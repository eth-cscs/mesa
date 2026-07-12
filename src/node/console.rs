//! Open and interact with a node serial console via the CSM `cray-console-*` services.

use core::time;

use k8s_openapi::api::core::v1::Pod;
use kube::{
  Api,
  api::{AttachParams, AttachedProcess},
};
use serde_json::Value;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

use crate::{
  common::kubernetes::{self, get_client},
  error::Error,
};

/// Attach to the `cray-console-node` pod that owns the given xname's
/// serial console (via conman) and return the open process handle.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_container_attachment_to_conman(
  xname: &str,
  k8s_api_url: &str,
  shasta_k8s_secrets: Value,
) -> Result<AttachedProcess, Error> {
  log::info!("xname: {xname}");

  let client = get_client(k8s_api_url, shasta_k8s_secrets).await?;

  let pods_fabric: Api<Pod> = Api::namespaced(client, "services");

  let params = kube::api::ListParams::default()
    .limit(1)
    .labels("app.kubernetes.io/name=cray-console-operator");

  let pods_objects = pods_fabric.list(&params).await?;

  let console_operator_pod = pods_objects.items.first().ok_or_else(|| {
    Error::K8sError(
      "No 'cray-console-operator' pod found in namespace 'services'"
        .to_string(),
    )
  })?;
  let console_operator_pod_name =
    console_operator_pod.metadata.name.as_ref().ok_or_else(|| {
      Error::K8sError("Pod related to console has no name".to_string())
    })?;

  log::info!("Console operator pod name '{console_operator_pod_name}'");

  let mut attached = pods_fabric
    .exec(
      console_operator_pod_name,
      vec!["sh", "-c", &format!("/app/get-node {xname}")],
      &AttachParams::default()
        .container("cray-console-operator")
        .stderr(false),
    )
    .await?;

  let stdout = attached.stdout().ok_or_else(|| {
    Error::K8sError(
      "attached console-operator process has no stdout".to_string(),
    )
  })?;
  let mut stdout_stream = ReaderStream::new(stdout);
  let Some(next_stdout_frame) = stdout_stream.next().await else {
    return Err(Error::K8sError(
      "console-operator stdout stream ended without yielding any frame"
        .to_string(),
    ));
  };
  let next_stdout = next_stdout_frame?;
  let stdout_str = std::str::from_utf8(&next_stdout)?;
  let output_json: Value = serde_json::from_str(stdout_str)?;

  let console_pod_name = output_json
    .get("podname")
    .and_then(Value::as_str)
    .ok_or_else(|| {
      Error::ConsoleError(format!(
        "console-operator response missing string field 'podname' (got: {output_json})"
      ))
    })?;

  let command = vec!["conman", "-j", xname]; // Enter the container and open conman to access node's console
  // let command = vec!["bash"]; // Enter the container and open bash to start an interactive
  // terminal session

  log::info!("Console pod name: {console_pod_name}");

  log::info!("Connecting to console {xname}");

  pods_fabric
        .exec(
            console_pod_name,
            command,
            &AttachParams::default()
                .container("cray-console-node")
                .stdin(true)
                .stdout(true)
                .stderr(false) // Note to self: tty and stderr cannot both be true
                .tty(true),
        )
        .await
        .map_err(|e| {
            Error::ConsoleError(format!(
                "Error attaching to container 'cray-console-node' in pod '{console_pod_name}'. Reason:\n{e}. Exit"
            ))
        })
}

/// Attach to the Ansible container of a CFS session's image-build pod
/// so the caller can stream its logs / shell.
///
/// # Errors
///
/// Returns an [`Error`] variant on CSM, transport, or
/// deserialization failure; see the crate-level `Error` enum
/// for the full set.
pub async fn get_container_attachment_to_cfs_session_image_target(
  cfs_session_name: &str,
  k8s_api_url: &str,
  shasta_k8s_secrets: Value,
) -> Result<AttachedProcess, Error> {
  let client = get_client(k8s_api_url, shasta_k8s_secrets).await?;

  let pods_fabric: Api<Pod> = Api::namespaced(client.clone(), "services");

  let params = kube::api::ListParams::default()
    .limit(1)
    .labels(format!("cfsession={cfs_session_name}").as_str());

  let mut pods = pods_fabric.list(&params).await?;

  let mut i = 0;
  let max = 30;

  // Waiting for pod to start
  while pods.items.is_empty() && i <= max {
    log::info!(
      "Pod for cfs session {} not ready. Trying again in 2 secs. Attempt {} of {}",
      cfs_session_name,
      i + 1,
      max
    );
    i += 1;
    tokio::time::sleep(time::Duration::from_secs(2)).await;
    pods = pods_fabric.list(&params).await?;
  }

  if pods.items.is_empty() {
    return Err(Error::ConsoleError(format!(
      "Pod for cfs session {cfs_session_name} not ready. Aborting operation"
    )));
  }

  let console_operator_pod_name = &pods
    .items
    .first()
    .and_then(|pod| pod.metadata.name.as_ref())
    .ok_or_else(|| {
      Error::K8sError("Pod related to console has no name".to_string())
    })?;

  log::info!("Ansible pod name: {console_operator_pod_name}");

  let attached = pods_fabric
        .exec(
            console_operator_pod_name,
            vec![
                "sh",
                "-c",
                "cat /inventory/hosts/01-cfs-generated.yaml | grep cray-ims- | head -n 1",
            ],
            &AttachParams::default().container("ansible").stderr(false),
        )
        .await?;

  let mut output = kubernetes::get_output(attached).await;
  log::info!("{output}");

  output = output.trim().to_string();

  log::info!("{output}");

  output = output
    .strip_prefix("ansible_host: ")
    .and_then(|output| output.strip_suffix("-service.ims.svc.cluster.local"))
    .ok_or_else(|| Error::Message("Output neither contains prefix 'ansible_host: ' not suffix '-service.ims.svc.cluster.local'".to_string()))?
    .to_string();

  log::info!("{output}");

  let ansible_target_container_label = output + "-customize";

  log::info!("{ansible_target_container_label}");

  // Find ansible target container

  let pods_fabric: Api<Pod> = Api::namespaced(client, "ims");

  let params = kube::api::ListParams::default()
    .limit(1)
    .labels(format!("job-name={ansible_target_container_label}").as_str());

  let mut pods = pods_fabric.list(&params).await?;

  let mut i = 0;
  let max = 30;

  // Waiting for pod to start
  while pods.items.is_empty() && i <= max {
    log::info!(
      "Pod for cfs session {} not ready. Trying again in 2 secs. Attempt {} of {}",
      cfs_session_name,
      i + 1,
      max
    );
    i += 1;
    tokio::time::sleep(time::Duration::from_secs(2)).await;
    pods = pods_fabric.list(&params).await?;
  }

  if pods.items.is_empty() {
    return Err(Error::ConsoleError(format!(
      "Pod for cfs session {cfs_session_name} not ready. Aborting operation"
    )));
  }

  let console_operator_pod = &pods.items[0].clone();

  log::info!("Connecting to console ansible target container");

  let console_operator_pod_name =
    console_operator_pod.metadata.name.as_ref().ok_or_else(|| {
      Error::K8sError("Pod related to console has no name".to_string())
    })?;

  let command = vec!["bash"]; // Enter the container and open conman to access node's console
  // let command = vec!["bash"]; // Enter the container and open bash to start an interactive
  // terminal session

  pods_fabric
    .exec(
      console_operator_pod_name,
      command,
      &AttachParams::default()
        .container("sshd")
        .stdin(true)
        .stdout(true)
        .stderr(false) // Note to self: tty and stderr cannot both be true
        .tty(true),
    )
    .await
    .map_err(|e| {
      Error::ConsoleError(format!(
        "Error attaching to container 'sshd' in pod '{console_operator_pod_name}'. Reason\n{e}\n. Exit"
      ))
    })
}
