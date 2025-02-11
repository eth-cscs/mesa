use core::time;

use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{AttachParams, AttachedProcess},
    Api,
};
use serde_json::Value;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

use crate::{
    common::kubernetes::{self, get_k8s_client_programmatically},
    error::Error,
};

pub async fn get_container_attachment_to_conman(
    xname: &String,
    k8s_api_url: &str,
    shasta_k8s_secrets: Value,
) -> Result<AttachedProcess, Error> {
    log::info!("xname: {}", xname);

    let client = get_k8s_client_programmatically(k8s_api_url, shasta_k8s_secrets)
        .await
        .unwrap();

    let pods_fabric: Api<Pod> = Api::namespaced(client, "services");

    let params = kube::api::ListParams::default()
        .limit(1)
        .labels("app.kubernetes.io/name=cray-console-operator");

    let pods_objects = pods_fabric.list(&params).await.unwrap();

    let console_operator_pod = &pods_objects.items[0];
    let console_operator_pod_name = console_operator_pod.metadata.name.clone().unwrap();

    log::info!("Console operator pod name '{}'", console_operator_pod_name);

    let mut attached = pods_fabric
        .exec(
            &console_operator_pod_name,
            vec!["sh", "-c", &format!("/app/get-node {}", xname)],
            &AttachParams::default()
                .container("cray-console-operator")
                .stderr(false),
        )
        .await
        .unwrap();

    let mut stdout_stream = ReaderStream::new(attached.stdout().unwrap());
    let next_stdout = stdout_stream.next().await.unwrap().unwrap();
    let stdout_str = std::str::from_utf8(&next_stdout).unwrap();
    let output_json: Value = serde_json::from_str(stdout_str).unwrap();

    let console_pod_name = output_json["podname"].as_str().unwrap();

    let command = vec!["conman", "-j", xname]; // Enter the container and open conman to access node's console
                                               // let command = vec!["bash"]; // Enter the container and open bash to start an interactive
                                               // terminal session

    log::info!("Console pod name: {}", console_pod_name,);

    log::info!("Connecting to console {}", xname);

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
                "Error attaching to container 'cray-console-node' in pod '{}'. Reason:\n{}. Exit",
                console_pod_name, e
            ))
        })
}

pub async fn get_container_attachment_to_cfs_session_image_target(
    cfs_session_name: &str,
    /* vault_base_url: &str,
    vault_secret_path: &str,
    vault_role_id: &str, */
    k8s_api_url: &str,
    shasta_k8s_secrets: Value,
) -> Result<AttachedProcess, Error> {
    /* let shasta_k8s_secrets =
    fetch_shasta_k8s_secrets(vault_base_url, vault_secret_path, vault_role_id).await?; */

    let client = get_k8s_client_programmatically(k8s_api_url, shasta_k8s_secrets).await?;

    let pods_fabric: Api<Pod> = Api::namespaced(client.clone(), "services");

    let params = kube::api::ListParams::default()
        .limit(1)
        .labels(format!("cfsession={}", cfs_session_name).as_str());

    let mut pods = pods_fabric.list(&params).await.unwrap();

    let mut i = 0;
    let max = 30;

    // Waiting for pod to start
    while pods.items.is_empty() && i <= max {
        println!(
            "Pod for cfs session {} not ready. Trying again in 2 secs. Attempt {} of {}",
            cfs_session_name,
            i + 1,
            max
        );
        i += 1;
        tokio::time::sleep(time::Duration::from_secs(2)).await;
        pods = pods_fabric.list(&params).await.unwrap();
    }

    if pods.items.is_empty() {
        return Err(Error::ConsoleError(format!(
            "Pod for cfs session {} not ready. Aborting operation",
            cfs_session_name
        )));
    }

    let console_operator_pod = &pods.items[0].clone();

    let console_operator_pod_name = console_operator_pod.metadata.name.clone().unwrap();

    log::info!("Ansible pod name: {}", console_operator_pod_name);

    let attached = pods_fabric
        .exec(
            &console_operator_pod_name,
            vec![
                "sh",
                "-c",
                "cat /inventory/hosts/01-cfs-generated.yaml | grep cray-ims- | head -n 1",
            ],
            &AttachParams::default().container("ansible").stderr(false),
        )
        .await
        .unwrap();

    let mut output = kubernetes::get_output(attached).await;
    log::info!("{output}");

    output = output.trim().to_string();

    log::info!("{output}");

    output = output.strip_prefix("ansible_host: ").unwrap().to_string();

    output = output
        .strip_suffix("-service.ims.svc.cluster.local")
        .unwrap()
        .to_string();

    log::info!("{output}");

    let ansible_target_container_label = output + "-customize";

    log::info!("{ansible_target_container_label}");

    // Find ansible target container

    let pods_fabric: Api<Pod> = Api::namespaced(client, "ims");

    let params = kube::api::ListParams::default()
        .limit(1)
        .labels(format!("job-name={}", ansible_target_container_label).as_str());

    let mut pods = pods_fabric.list(&params).await.unwrap();

    let mut i = 0;
    let max = 30;

    // Waiting for pod to start
    while pods.items.is_empty() && i <= max {
        println!(
            "Pod for cfs session {} not ready. Trying again in 2 secs. Attempt {} of {}",
            cfs_session_name,
            i + 1,
            max
        );
        i += 1;
        tokio::time::sleep(time::Duration::from_secs(2)).await;
        pods = pods_fabric.list(&params).await.unwrap();
    }

    if pods.items.is_empty() {
        return Err(Error::ConsoleError(format!(
            "Pod for cfs session {} not ready. Aborting operation",
            cfs_session_name
        )));
    }

    let console_operator_pod = &pods.items[0].clone();

    log::info!("Connecting to console ansible target container");

    let console_operator_pod_name = console_operator_pod.metadata.name.clone().unwrap();

    let command = vec!["bash"]; // Enter the container and open conman to access node's console
                                // let command = vec!["bash"]; // Enter the container and open bash to start an interactive
                                // terminal session

    pods_fabric
        .exec(
            &console_operator_pod_name,
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
                "Error attaching to container 'sshd' in pod '{}'. Reason\n{}\n. Exit",
                console_operator_pod_name, e
            ))
        })
}
