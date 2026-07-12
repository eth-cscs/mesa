use core::time;
#[cfg(feature = "commands-admin")]
use std::collections::BTreeMap;

use futures::{AsyncBufRead, AsyncBufReadExt, StreamExt, TryStreamExt};

#[cfg(feature = "commands-admin")]
use k8s_openapi::api::core::v1::ConfigMap;
use k8s_openapi::api::core::v1::{Container, ContainerStatus, Pod};
use kube::runtime::reflector::Lookup;
use kube::{
  Api,
  api::AttachedProcess,
  config::{
    AuthInfo, Cluster, Context, KubeConfigOptions, Kubeconfig, NamedAuthInfo,
    NamedCluster, NamedContext,
  },
};

use serde_json::Value;

use crate::error::Error;
use secrecy::SecretBox;

/// Name of the `services`-namespace `ConfigMap` that CSM exposes the
/// product catalog through. CFS layers and SAT workflows read it to
/// resolve product versions to git URLs and commit ids.
#[cfg(feature = "commands-admin")]
pub(crate) const CRAY_PRODUCT_CATALOG_CONFIGMAP: &str = "cray-product-catalog";

/// Build a `kube::Client` from a CSM-side Vault secret bundle.
///
/// `shasta_k8s_secrets` is the JSON object returned by
/// [`crate::common::vault::http_client::fetch_shasta_k8s_secrets_from_vault`];
/// it must contain
/// `certificate-authority-data`, `client-certificate-data`, and
/// `client-key-data`.
pub async fn get_client(
  k8s_api_url: &str,
  shasta_k8s_secrets: Value,
) -> Result<kube::Client, Error> {
  let k8s_credential_name = "certificate-authority-data";

  let certificate_authority_data = shasta_k8s_secrets
    .get(k8s_credential_name)
    .ok_or_else(|| {
      Error::K8sCredentialMissingError(k8s_credential_name.to_string())
    })?
    .as_str()
    .ok_or_else(|| {
      Error::K8sCredentialNotStringError(k8s_credential_name.to_string())
    })?;

  let k8s_credential_name = "client-certificate-data";

  let client_certificate_data = shasta_k8s_secrets
    .get(k8s_credential_name)
    .ok_or_else(|| {
      Error::K8sCredentialMissingError(k8s_credential_name.to_string())
    })?
    .as_str()
    .ok_or_else(|| {
      Error::K8sCredentialNotStringError(k8s_credential_name.to_string())
    })?;

  let k8s_credential_name = "client-key-data";

  let client_key_data = shasta_k8s_secrets
    .get(k8s_credential_name)
    .ok_or_else(|| {
      Error::K8sCredentialMissingError(k8s_credential_name.to_string())
    })?
    .as_str()
    .ok_or_else(|| {
      Error::K8sCredentialNotStringError(k8s_credential_name.to_string())
    })?
    .to_string();

  let shasta_cluster = Cluster {
    server: Some(k8s_api_url.to_string()),
    tls_server_name: Some("kube-apiserver".to_string()), // The value "kube-apiserver" has been taken from the
    insecure_skip_tls_verify: Some(true),
    certificate_authority: None,
    certificate_authority_data: Some(String::from(certificate_authority_data)),
    proxy_url: None,
    extensions: None,
    disable_compression: None,
  };

  let shasta_named_cluster = NamedCluster {
    name: String::from("shasta"),
    cluster: Some(shasta_cluster),
  };

  let shasta_auth_info = AuthInfo {
    username: None,
    password: None,
    token: None,
    token_file: None,
    client_certificate: None,
    client_certificate_data: Some(String::from(client_certificate_data)),
    client_key: None,
    client_key_data: Some(SecretBox::from(client_key_data)),
    impersonate: None,
    impersonate_groups: None,
    auth_provider: None,
    exec: None,
  };

  let shasta_named_auth_info = NamedAuthInfo {
    name: String::from("kubernetes-admin"),
    auth_info: Some(shasta_auth_info),
  };

  let shasta_context = Context {
    cluster: String::from("shasta"),
    user: Some(String::from("kubernetes-admin")),
    namespace: None,
    extensions: None,
  };

  let shasta_named_context = NamedContext {
    name: String::from("kubernetes-admin@kubernetes"),
    context: Some(shasta_context),
  };

  let kube_config = Kubeconfig {
    preferences: None,
    clusters: vec![shasta_named_cluster],
    auth_infos: vec![shasta_named_auth_info],
    contexts: vec![shasta_named_context],
    current_context: Some(String::from("kubernetes-admin@kubernetes")),
    extensions: None,
    kind: None,
    api_version: None,
  };

  let kube_config_options = KubeConfigOptions {
    context: Some(String::from("kubernetes-admin@kubernetes")),
    cluster: Some(String::from("shasta")),
    user: Some(String::from("kubernetes-admin")),
  };

  let config =
    kube::Config::from_custom_kubeconfig(kube_config, &kube_config_options)
      .await
      .map_err(|e| Error::K8sError(e.to_string()))?;

  let client = kube::Client::try_from(config)
    .map_err(|e| Error::K8sError(e.to_string()))?;

  Ok(client)
}

/// Stream the full set of CFS-session container logs to stdout.
///
/// Tails the `git-clone`, `inventory`, `ansible`, and `teardown`
/// containers of the pod backing `cfs_session_name` in order, retrying
/// the pod-lookup up to three times.
///
/// Emits each line through `log::debug!`, so output is routed by the
/// caller's `log` backend (no direct stdout writes). Callers wanting
/// to consume the lines themselves should compose
/// [`get_cfs_session_init_container_git_clone_logs_stream`],
/// [`get_cfs_session_container_inventory_logs_stream`], and
/// [`get_cfs_session_container_ansible_logs_stream`] instead.
pub async fn i_print_cfs_session_logs(
  client: kube::Client,
  cfs_session_name: &str,
  timestamps: bool,
) -> Result<(), Error> {
  let max_attempts = 3;

  let namespace = "services";

  let mut attempt = 0;

  let container_name = "git-clone";

  let mut result = i_print_init_container_logs(
    client.clone(),
    cfs_session_name,
    container_name,
    namespace,
    timestamps,
  )
  .await;

  while result.is_err() && attempt < max_attempts {
    attempt += 1;

    log::debug!(
      "Could not get logs for init container '{}'. Trying again. Attempt {} of {}",
      container_name,
      attempt + 1,
      max_attempts
    );
    result = i_print_init_container_logs(
      client.clone(),
      cfs_session_name,
      container_name,
      namespace,
      timestamps,
    )
    .await;
  }

  let mut attempt = 0;

  let container_name = "inventory";

  let mut result = i_print_container_logs(
    client.clone(),
    cfs_session_name,
    container_name,
    namespace,
    timestamps,
  )
  .await;

  while result.is_err() && attempt < max_attempts {
    attempt += 1;

    log::debug!(
      "Could not get logs for init container '{}'. Trying again. Attempt {} of {}",
      container_name,
      attempt + 1,
      max_attempts
    );
    result = i_print_init_container_logs(
      client.clone(),
      cfs_session_name,
      container_name,
      namespace,
      timestamps,
    )
    .await;
  }

  let mut attempt = 0;

  let container_name = "ansible";

  let mut result = i_print_container_logs(
    client.clone(),
    cfs_session_name,
    container_name,
    namespace,
    timestamps,
  )
  .await;

  while result.is_err() && attempt < max_attempts {
    log::debug!(
      "Could not get logs from container '{container_name}'. Trying again. Attempt {attempt} of {max_attempts}"
    );

    attempt += 1;

    result = i_print_container_logs(
      client.clone(),
      cfs_session_name,
      container_name,
      namespace,
      timestamps,
    )
    .await;
  }

  let mut attempt = 0;

  let container_name = "teardown";

  let mut result = i_print_container_logs(
    client.clone(),
    cfs_session_name,
    container_name,
    namespace,
    timestamps,
  )
  .await;

  while result.is_err() && attempt < max_attempts {
    log::debug!(
      "Could not get logs from container '{container_name}'. Trying again. Attempt {attempt} of {max_attempts}"
    );

    attempt += 1;

    result = i_print_container_logs(
      client.clone(),
      cfs_session_name,
      container_name,
      namespace,
      timestamps,
    )
    .await;
  }

  Ok(())
}

/// Fetch the `.data` of a `ConfigMap` in the `services` namespace.
///
/// Used by callers that need CSM-side state surfaced through `ConfigMaps`
/// (e.g. the `cray-product-catalog`).
///
/// # Errors
///
/// Returns [`Error::K8sError`] if the `ConfigMap` is missing or has no
/// `data` field.
#[cfg(feature = "commands-admin")]
pub async fn try_get_configmap(
  client: kube::Client,
  configmap_name: &str,
) -> Result<BTreeMap<String, String>, Error> {
  let configmap_api: kube::Api<ConfigMap> =
    kube::Api::namespaced(client, "services");

  let params = kube::api::ListParams::default()
    .fields(&("metadata.name=".to_owned() + configmap_name));

  let configmap = configmap_api
    .list(&params)
    .await
    .map_err(|e| Error::K8sError(e.to_string()))?;

  let configmap_data = configmap
    .items
    .first()
    .ok_or_else(|| {
      Error::K8sError("ERROR - There is no configmap".to_string())
    })?
    .clone();

  configmap_data.data.ok_or_else(|| {
    Error::K8sError("ERROR - There is no data in the configmap".to_string())
  })
}

pub(crate) async fn i_print_init_container_logs(
  client: kube::Client,
  cfs_session_name: &str,
  init_container_name: &str,
  namespace: &str,
  timestamps: bool,
) -> Result<(), Error> {
  let mut log_stream = get_init_container_logs_stream(
    client,
    cfs_session_name.to_string(),
    init_container_name,
    namespace,
    format!("cfsession={cfs_session_name}"),
    timestamps,
  )
  .await?
  .0
  .lines();

  while let Some(line) = log_stream.try_next().await? {
    log::debug!("{line}");
  }

  Ok(())
}

/// Stream the `git-clone` init-container logs for a CFS session.
///
/// Returns an [`AsyncBufRead`] over the tail of the container's stdout
/// along with the container exit code captured at attach time. Pairs
/// with the other `get_cfs_session_*_logs_stream` helpers so callers
/// can consume CFS-session output without involving stdout.
///
/// # Cancellation
///
/// The returned reader is a thin wrapper around the hyper response
/// body that `kube_client::Client::request_stream` produces. There is
/// no background watcher task — dropping the reader drops the hyper
/// `Response`, which closes the connection to the Kubernetes API
/// server. The API server then stops shipping log lines. No watcher
/// leak.
pub async fn get_cfs_session_init_container_git_clone_logs_stream(
  client: kube::Client,
  cfs_session_name: String,
  timestamps: bool,
) -> Result<(impl AsyncBufRead, i32), Error> {
  get_init_container_logs_stream(
    client,
    cfs_session_name.clone(),
    "git-clone",
    "services",
    format!("cfsession={cfs_session_name}"),
    timestamps,
  )
  .await
}

pub(crate) async fn i_print_container_logs(
  client: kube::Client,
  cfs_session_name: &str,
  container_name: &str,
  namespace: &str,
  timestamps: bool,
) -> Result<(), Error> {
  let mut log_stream = get_container_logs_stream(
    client,
    cfs_session_name.to_string(),
    container_name,
    namespace,
    format!("cfsession={cfs_session_name}"),
    timestamps,
  )
  .await?
  .lines();

  while let Some(line) = log_stream.try_next().await? {
    log::debug!("{line}");
  }

  Ok(())
}

/// Stream the `inventory` container logs for a CFS session.
///
/// See [`get_cfs_session_init_container_git_clone_logs_stream`] for the
/// shared pattern.
pub async fn get_cfs_session_container_inventory_logs_stream(
  client: kube::Client,
  cfs_session_name: String,
  timestamps: bool,
) -> Result<impl AsyncBufRead, Error> {
  get_container_logs_stream(
    client,
    cfs_session_name.clone(),
    "inventory",
    "services",
    format!("cfsession={cfs_session_name}"),
    timestamps,
  )
  .await
}

/// Stream the `ansible` container logs for a CFS session.
///
/// See [`get_cfs_session_init_container_git_clone_logs_stream`] for the
/// shared pattern.
pub async fn get_cfs_session_container_ansible_logs_stream(
  client: kube::Client,
  cfs_session_name: String,
  timestamps: bool,
) -> Result<impl AsyncBufRead, Error> {
  get_container_logs_stream(
    client,
    cfs_session_name.clone(),
    "ansible",
    "services",
    format!("cfsession={cfs_session_name}"),
    timestamps,
  )
  .await
}

pub(crate) fn get_init_container<'a>(
  pod: &'a Pod,
  name: &str,
) -> Option<&'a Container> {
  pod
    .spec
    .as_ref()
    .and_then(|pod_spec| pod_spec.init_containers.as_ref())
    .and_then(|init_container_vec| {
      init_container_vec
        .iter()
        .find(|container| container.name == name)
    })
}

pub(crate) fn init_container_status<'a>(
  pod: &'a Pod,
  container_name: &str,
) -> Option<&'a ContainerStatus> {
  pod
    .status
    .as_ref()
    .and_then(|pod_status| pod_status.init_container_statuses.as_ref())
    .and_then(|status_vec| {
      status_vec
        .iter()
        .find(|container_status| container_status.name == container_name)
    })
}

pub(crate) fn init_container_exit_code(
  pod: &Pod,
  container_name: &str,
) -> Option<i32> {
  init_container_status(pod, container_name)
    .and_then(|container_status| container_status.state.as_ref())
    .and_then(|container_state| container_state.terminated.as_ref())
    .map(|terminated_state| terminated_state.exit_code)
}

pub(crate) fn is_init_container_state_unknown(
  pod: &Pod,
  container_name: &str,
) -> bool {
  init_container_status(pod, container_name)
    .is_some_and(|container_status| container_status.state.as_ref().is_none())
}

pub(crate) fn is_init_container_state_waiting(
  pod: &Pod,
  container_name: &str,
) -> bool {
  init_container_status(pod, container_name).is_some_and(|container_status| {
    container_status
      .state
      .as_ref()
      .is_some_and(|container_state| container_state.waiting.is_some())
  })
}

pub(crate) fn get_container<'a>(
  pod: &'a Pod,
  name: &str,
) -> Option<&'a Container> {
  pod.spec.as_ref().and_then(|pod_spec| {
    pod_spec
      .containers
      .iter()
      .find(|container| container.name == name)
  })
}

pub(crate) fn container_status<'a>(
  pod: &'a Pod,
  container_name: &str,
) -> Option<&'a ContainerStatus> {
  pod
    .status
    .as_ref()
    .and_then(|pod_status| pod_status.container_statuses.as_ref())
    .and_then(|status_vec| {
      status_vec
        .iter()
        .find(|container_status| container_status.name == container_name)
    })
}

pub(crate) fn is_container_state_unknown(
  pod: &Pod,
  container_name: &str,
) -> bool {
  container_status(pod, container_name)
    .is_some_and(|container_status| container_status.state.as_ref().is_none())
}

pub(crate) fn is_container_state_waiting(
  pod: &Pod,
  container_name: &str,
) -> bool {
  container_status(pod, container_name).is_some_and(|container_status| {
    container_status
      .state
      .as_ref()
      .is_some_and(|container_state| container_state.waiting.is_some())
  })
}

pub(crate) async fn get_pod_and_wait_items(
  pods_api: &Api<Pod>,
  cfs_session_name: &str,
  label_selector: &str,
) -> Result<Pod, Error> {
  let params = kube::api::ListParams::default()
    .limit(1)
    .labels(label_selector);

  let mut cfs_session_pods = pods_api
    .list(&params)
    .await
    .map_err(|e| Error::K8sError(format!("{e}")))?;

  let mut i = 0;
  let max = 150;
  let delay_secs = 2;

  // Waiting for pod to start
  while cfs_session_pods.items.is_empty() && i <= max {
    log::debug!(
      "Waiting k8s to create pod for cfs session '{}'. Trying again in {} secs. Attempt {} of {}",
      cfs_session_name,
      delay_secs,
      i + 1,
      max
    );

    i += 1;

    tokio::time::sleep(time::Duration::from_secs(delay_secs)).await;

    cfs_session_pods = pods_api
      .list(&params)
      .await
      .map_err(|e| Error::K8sError(format!("{e}")))?;
  }

  if cfs_session_pods.items.is_empty() {
    return Err(Error::K8sError(format!(
      "Pod for cfs session {cfs_session_name} missing. Aborting operation"
    )));
  }

  if cfs_session_pods.items.len() > 1 {
    return Err(Error::K8sError(format!(
      "Multiple pods found for cfs session '{cfs_session_name}'. Using the first one."
    )));
  }

  let cfs_session_pod = cfs_session_pods.items.first().ok_or_else(|| {
    Error::K8sError(format!(
      "Pod related to CFS session '{cfs_session_name}' not found"
    ))
  })?;

  Ok(cfs_session_pod.clone())
}

pub(crate) async fn get_init_container_and_wait_to_ready(
  cfs_session_pod: &Pod,
  init_container_name: &str,
) -> Result<Container, Error> {
  let cfs_session_pod_name = cfs_session_pod.name().ok_or_else(|| {
    Error::K8sError("Pod related to CFS session has no name".to_string())
  })?;

  let init_container_opt =
    get_init_container(cfs_session_pod, init_container_name);

  if init_container_opt.is_none() {
    return Err(Error::K8sError(format!(
      "Init container '{init_container_name}' not found in pod '{cfs_session_pod_name}'",
    )));
  }

  // Waiting for init container to start
  let init_container = get_init_container(cfs_session_pod, init_container_name)
    .ok_or(Error::K8sError(format!(
      "Init container '{init_container_name}' not found in pod '{cfs_session_pod_name}'",
    )))?;

  let mut i = 0;
  let max = 60;

  while (is_init_container_state_unknown(cfs_session_pod, &init_container.name)
    || is_init_container_state_waiting(cfs_session_pod, &init_container.name))
    && i <= max
  {
    log::debug!(
      "Init Container name: '{}' container state {:?}",
      init_container.name,
      init_container_status(cfs_session_pod, &init_container.name)
    );
    log::debug!(
      "Waiting for init container '{}' to be ready. Checking again in 2 secs. Attempt {} of {}",
      init_container.name,
      i + 1,
      max
    );

    i += 1;
    tokio::time::sleep(time::Duration::from_secs(2)).await;
  }

  Ok(init_container.clone())
}

pub(crate) async fn get_init_container_logs_stream(
  client: kube::Client,
  cfs_session_name: String,
  init_container_name: &str,
  namespace: &str,
  label_selector: String,
  timestamps: bool,
) -> Result<(impl AsyncBufRead, i32), Error> {
  let pods_api: Api<Pod> = Api::namespaced(client, namespace);

  let cfs_session_pod =
    &get_pod_and_wait_items(&pods_api, &cfs_session_name, &label_selector)
      .await?;

  let cfs_session_pod_name = cfs_session_pod.name().ok_or_else(|| {
    Error::K8sError(format!(
      "Pod related to CFS session '{cfs_session_name}' has no name"
    ))
  })?;

  log::debug!(
    "Fetching logs from init container '{init_container_name}' in namespace/pod '{namespace}/{cfs_session_pod_name}'",
  );

  let init_container =
    get_init_container_and_wait_to_ready(cfs_session_pod, init_container_name)
      .await?;

  if is_init_container_state_unknown(cfs_session_pod, &init_container.name)
    || is_init_container_state_waiting(cfs_session_pod, &init_container.name)
  {
    return Err(Error::K8sError(format!(
      "Init container '{init_container_name}' not in 'running' state. Aborting operation"
    )));
  }

  let exit_code =
    init_container_exit_code(cfs_session_pod, &init_container.name)
      .unwrap_or(-1);

  log::debug!(
    "Fetching logs from init container '{init_container_name}' in namespace/pod '{namespace}/{cfs_session_pod_name}'",
  );

  let container_log_stream = pods_api
    .log_stream(
      cfs_session_pod_name.as_ref(),
      &kube::api::LogParams {
        follow: true,
        container: Some(init_container_name.to_string()),
        limit_bytes: None,
        pretty: true,
        previous: false,
        since_seconds: None,
        since_time: None,
        tail_lines: None,
        timestamps,
      },
    )
    .await
    .map_err(|e| Error::K8sError(format!("{e}")))?;

  Ok((container_log_stream, exit_code))
}

pub(crate) async fn get_container_logs_stream(
  client: kube::Client,
  cfs_session_name: String,
  container_name: &str,
  namespace: &str,
  label_selector: String,
  timestamps: bool,
) -> Result<impl AsyncBufRead, Error> {
  let pods_api: kube::Api<Pod> = kube::Api::namespaced(client, namespace);

  let cfs_session_pod =
    get_pod_and_wait_items(&pods_api, &cfs_session_name, &label_selector)
      .await?;

  let cfs_session_pod_name = cfs_session_pod.name().ok_or_else(|| {
    Error::K8sError(format!(
      "Pod related to CFS session '{cfs_session_name}' has no name"
    ))
  })?;

  log::debug!(
    "Fetching logs from container '{container_name}' in namespace/pod '{namespace}/{cfs_session_pod_name}'",
  );

  let container_opt = get_container(&cfs_session_pod, container_name);

  if container_opt.is_none() {
    return Err(Error::K8sError(format!(
      "Container '{container_name}' not found in pod '{cfs_session_pod_name}'",
    )));
  }

  // Waiting for container to start
  let container = get_container(&cfs_session_pod, container_name).ok_or(
    Error::K8sError(format!(
      "Container '{container_name}' not found in pod '{cfs_session_pod_name}'",
    )),
  )?;

  let mut i = 0;
  let max = 600;

  while (is_container_state_unknown(&cfs_session_pod, &container.name)
    || is_container_state_waiting(&cfs_session_pod, &container.name))
    && i <= max
  {
    log::debug!(
      "Container name: '{}' container state {:?}",
      container.name,
      container_status(&cfs_session_pod, &container.name)
    );
    log::debug!(
      "Waiting for container '{}' to be ready. Checking again in 2 secs. Attempt {} of {}",
      container.name,
      i + 1,
      max
    );

    i += 1;
    tokio::time::sleep(time::Duration::from_secs(2)).await;
  }

  if is_container_state_unknown(&cfs_session_pod, &container.name)
    || is_container_state_waiting(&cfs_session_pod, &container.name)
  {
    return Err(Error::K8sError(format!(
      "Container '{container_name}' not ready. Aborting operation"
    )));
  }

  pods_api
    .log_stream(
      cfs_session_pod_name.as_ref(),
      &kube::api::LogParams {
        follow: true,
        container: Some(container_name.to_string()),
        limit_bytes: None,
        pretty: true,
        previous: false,
        since_seconds: None,
        since_time: None,
        tail_lines: None,
        timestamps,
      },
    )
    .await
    .map_err(|e| Error::K8sError(format!("{e}")))
}

/// Collect the stdout of a `kube::exec` [`AttachedProcess`] into a
/// String, then join the process.
///
/// # Panics
///
/// Panics if the exec was started without a stdout stream (i.e. with
/// `AttachParams::default().stdout(false)`); the function is documented
/// to require stdout. Errors during `attached.join()` are logged but
/// not surfaced — the function is infallible by signature.
pub async fn get_output(mut attached: AttachedProcess) -> String {
  // `attached` is created by kube exec; in our callers the attach params
  // always request stdout, so `stdout()` should be `Some`. If kube ever
  // returns None we want a clear panic rather than a silent empty string.
  let stdout = tokio_util::io::ReaderStream::new(
    attached
      .stdout()
      .expect("kube exec was started without a stdout stream"),
  );
  let out = stdout
    .filter_map(|r| async {
      r.ok().and_then(|v| String::from_utf8(v.to_vec()).ok())
    })
    .collect::<Vec<_>>()
    .await
    .join("");

  // join() returns the process exit status; failures are logged rather than
  // surfaced because get_output is infallible by signature.
  if let Err(e) = attached.join().await {
    log::warn!("kube exec join failed: {e}");
  }

  out
}
