use core::time;
use std::{error::Error, str::FromStr, pin::Pin, thread};

use hyper::Uri;
use hyper_socks2::SocksConnector;
use futures_util::{Stream, StreamExt};
use k8s_openapi::api::core::v1::{Container, Pod};
use kube::{
    client::ConfigExt,
    config::{
        AuthInfo, Cluster, Context, KubeConfigOptions, Kubeconfig, NamedAuthInfo, NamedCluster,
        NamedContext,
    }, Api, api::ListParams,
};

use secrecy::SecretString;
use serde_json::Value;

pub async fn get_k8s_client_programmatically(
    k8s_api_url: &str,
    shasta_k8s_secrets: Value,
) -> Result<kube::Client, Box<dyn Error>> {
    let shasta_cluster = Cluster {
        server: Some(k8s_api_url.to_string()),
        tls_server_name: Some("kube-apiserver".to_string()), // The value "kube-apiserver" has been taken from the
        // Subject: CN value in the Shasta certificate running
        // this command echo | openssl s_client -showcerts -servername 10.252.1.12 -connect 10.252.1.12:6442 2>/dev/null | openssl x509 -inform pem -noout -text
        insecure_skip_tls_verify: Some(true),
        certificate_authority: None,
        certificate_authority_data: Some(String::from(
            shasta_k8s_secrets["certificate-authority-data"]
                .as_str()
                .unwrap(),
        )),
        proxy_url: None,
        extensions: None,
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
        client_certificate_data: Some(String::from(
            shasta_k8s_secrets["client-certificate-data"]
                .as_str()
                .unwrap(),
        )),
        client_key: None,
        client_key_data: Some(
            SecretString::from_str(shasta_k8s_secrets["client-key-data"].as_str().unwrap())
                .unwrap(),
        ),
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
        user: String::from("kubernetes-admin"),
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

    let config = kube::Config::from_custom_kubeconfig(kube_config, &kube_config_options).await?;

    // OPTION 1 --> Native TLS - WORKING
    /* let client = if std::env::var("SOCKS5").is_ok() {
        log::debug!("SOCKS5 enabled");
        let connector = {
            let mut http = hyper::client::HttpConnector::new();
            http.enforce_http(false);
            let proxy = hyper_socks2::SocksConnector {
                proxy_addr: std::env::var("SOCKS5").unwrap().parse::<Uri>().unwrap(),
                auth: None,
                connector: http,
            };
            let mut native_tls_builder = native_tls::TlsConnector::builder();
            native_tls_builder.danger_accept_invalid_certs(true);
            native_tls_builder.danger_accept_invalid_hostnames(true);
            native_tls_builder.use_sni(false);

            let tls = tokio_native_tls::TlsConnector::from(config.native_tls_connector()?);
            hyper_tls::HttpsConnector::from((proxy, tls))
        };

        let service = tower::ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .option_layer(config.auth_layer()?)
            .service(hyper::Client::builder().build(connector));

        kube::Client::new(service, config.default_namespace)
    } else {
        let https = config.openssl_https_connector()?;
        let service = tower::ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .service(hyper::Client::builder().build(https));
        kube::Client::new(service, config.default_namespace)
    }; */

    // OPTION 2 --> rustls - Not working. Probably failing hyper client
    /* let client = if std::env::var("SOCKS5").is_ok() {
        log::debug!("SOCKS5 enabled");
        // let https = config.rustls_https_connector()?;
        let rustls_config = std::sync::Arc::new(config.rustls_client_config()?);
        println!("rustls_config:\n{:#?}", config.rustls_client_config());
        let mut http_connector = hyper::client::HttpConnector::new();
        http_connector.enforce_http(false);
        let socks_http_connector = SocksConnector {
            proxy_addr: std::env::var("SOCKS5").unwrap().parse::<Uri>().unwrap(), // scheme is required by HttpConnector
            auth: None,
            connector: http_connector.clone(),
        };
        // let socks = socks_http_connector.clone().with_tls()?;
        let https_socks_http_connector = hyper_rustls::HttpsConnector::from((
            socks_http_connector.clone(),
            rustls_config.clone(),
        ));

        println!(
            "https_socks_http_connector:\n{:#?}",
            https_socks_http_connector
        );
        // let https_http_connector = hyper_rustls::HttpsConnector::from((http_connector, rustls_config));
        let service = tower::ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .service(hyper::Client::builder().build(https_socks_http_connector));
        kube::Client::new(service, config.default_namespace)
    } else {
        let https = config.openssl_https_connector()?;
        let service = tower::ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .service(hyper::Client::builder().build(https));
        kube::Client::new(service, config.default_namespace)
    }; */

    let client = if std::env::var("SOCKS5").is_ok() {
        log::debug!("SOCKS5 enabled");
        let mut http_connector = hyper::client::HttpConnector::new();
        http_connector.enforce_http(false);
        let socks_http_connector = SocksConnector {
            proxy_addr: std::env::var("SOCKS5").unwrap().parse::<Uri>().unwrap(), // scheme is required by HttpConnector
            auth: None,
            connector: http_connector.clone(),
        };

        // HttpsConnector following https://github.com/rustls/hyper-rustls/blob/main/examples/client.rs
        // Get CA root cert
        let mut ca_root_cert_pem_decoded: &[u8] = &base64::decode(
            shasta_k8s_secrets["certificate-authority-data"]
                .as_str()
                .unwrap(),
        )?;

        let ca_root_cert = rustls_pemfile::certs(&mut ca_root_cert_pem_decoded)?;

        // Import CA cert into rustls ROOT certificate store
        let mut root_cert_store = tokio_rustls::rustls::RootCertStore::empty();

        root_cert_store.add_parsable_certificates(&ca_root_cert);

        // Prepare client authentication https://github.com/rustls/rustls/blob/0018e7586c2dc689eb9e1ba8e0283c0f24b9fe8c/examples/src/bin/tlsclient-mio.rs#L414-L426
        // Get client cert
        let mut client_cert_pem_decoded: &[u8] = &base64::decode(
            shasta_k8s_secrets["client-certificate-data"]
                .as_str()
                .unwrap(),
        )?;

        let client_certs = rustls_pemfile::certs(&mut client_cert_pem_decoded)
            .unwrap()
            .iter()
            .map(|cert| tokio_rustls::rustls::Certificate(cert.clone()))
            .collect();

        // Get client key
        let mut client_key_decoded: &[u8] =
            &base64::decode(shasta_k8s_secrets["client-key-data"].as_str().unwrap())?;

        let client_key = match rustls_pemfile::read_one(&mut client_key_decoded)
            .expect("cannot parse private key .pem file")
        {
            Some(rustls_pemfile::Item::RSAKey(key)) => tokio_rustls::rustls::PrivateKey(key),
            Some(rustls_pemfile::Item::PKCS8Key(key)) => tokio_rustls::rustls::PrivateKey(key),
            Some(rustls_pemfile::Item::ECKey(key)) => tokio_rustls::rustls::PrivateKey(key),
            _ => tokio_rustls::rustls::PrivateKey(Vec::new()),
        };

        // Create HTTPS connector
        let rustls_config = tokio_rustls::rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            // .with_no_client_auth();
            .with_single_cert(client_certs, client_key)?;

        let rustls_config = std::sync::Arc::new(rustls_config);

        let args = (socks_http_connector, rustls_config);
        let https_socks_http_connector = hyper_rustls::HttpsConnector::from(args);

        /* let https_socks_http_connector = socks_http_connector
        .with_rustls_root_cert_store(root_cert_store); */

        // Create HTTPS client
        let hyper_client = hyper::Client::builder().build(https_socks_http_connector);

        let service = tower::ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .service(hyper_client);

        kube::Client::new(service, config.default_namespace)
    } else {
        let https = config.openssl_https_connector()?;
        let service = tower::ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .service(hyper::Client::builder().build(https));
        kube::Client::new(service, config.default_namespace)
    };

    Ok(client)
}

pub async fn get_container_logs_stream(
    cfs_session_layer_container: &Container,
    cfs_session_pod: &Pod,
    pods_api: &Api<Pod>,
    params: &ListParams,
) -> Result<
    Pin<Box<dyn Stream<Item = Result<hyper::body::Bytes, kube::Error>> + std::marker::Send>>,
    Box<dyn Error + Sync + Send>,
> {
    let mut container_log_stream: Pin<
        Box<dyn Stream<Item = Result<hyper::body::Bytes, kube::Error>> + std::marker::Send>,
    > = tokio_stream::once(Ok(hyper::body::Bytes::copy_from_slice(
        format!(
            "\nFetching logs for container {}\n",
            cfs_session_layer_container.name
        )
        .as_bytes(),
    )))
    .boxed();

    // Check if container exists in pod
    let container_exists = cfs_session_pod
        .spec
        .as_ref()
        .unwrap()
        .containers
        .iter()
        .find(|x| x.name.eq(&cfs_session_layer_container.name));

    if container_exists.is_none() {
        return Err(format!(
            "Container {} does not exists. Aborting",
            cfs_session_layer_container.name
        )
        .into());
    }

    let mut container_state =
        get_container_state(cfs_session_pod, &cfs_session_layer_container.name);

    let mut i = 0;
    let max = 300;

    // Waiting for container ansible-x to start
    while container_state.as_ref().unwrap().waiting.is_some() && i <= max {
        container_log_stream = container_log_stream
            .chain(tokio_stream::once(Ok(hyper::body::Bytes::copy_from_slice(
                format!(
                "\nWaiting for container {} to be ready. Checking again in 2 secs. Attempt {} of {}\n",
                cfs_session_layer_container.name,
                i + 1,
                max
            )
                .as_bytes(),
            ))))
            .boxed();
        i += 1;
        thread::sleep(time::Duration::from_secs(2));
        let pods = pods_api.list(params).await?;
        container_state = get_container_state(&pods.items[0], &cfs_session_layer_container.name);
        log::debug!("Container state:\n{:#?}", container_state.as_ref().unwrap());
    }

    if container_state.as_ref().unwrap().waiting.is_some() {
        return Err(format!(
            "Container {} not ready. Aborting operation",
            cfs_session_layer_container.name
        )
        .into());
    }

    let logs_stream = pods_api
        .log_stream(
            cfs_session_pod.metadata.name.as_ref().unwrap(),
            &kube::api::LogParams {
                follow: true,
                container: Some(cfs_session_layer_container.name.clone()),
                ..kube::api::LogParams::default()
            },
        )
        .await?;

    // We are going to use chain method (https://dtantsur.github.io/rust-openstack/tokio/stream/trait.StreamExt.html#method.chain) to join streams coming from kube_client::api::subresource::Api::log_stream which returns Result<impl Stream<Item = Result<Bytes>>> or Result<hyper::body::Bytes>, we will consume the Result hence we will be chaining streams of hyper::body::Bytes
    container_log_stream = container_log_stream.chain(logs_stream).boxed();

    Ok(container_log_stream)
}

pub async fn get_cfs_session_logs_stream(
    client: kube::Client,
    cfs_session_name: &str,
    layer_id: Option<&u8>,
) -> Result<
    Pin<Box<dyn futures_util::Stream<Item = Result<hyper::body::Bytes, kube::Error>> + std::marker::Send>>,
    Box<dyn Error + Sync + Send>,
> {
    let mut container_log_stream: Pin<
        Box<dyn futures_util::Stream<Item = Result<hyper::body::Bytes, kube::Error>> + std::marker::Send>,
    > = tokio_stream::once(Ok(hyper::body::Bytes::copy_from_slice(
        format!("\nFetching logs for CFS session {}\n", cfs_session_name).as_bytes(),
    )))
    .boxed();

    let pods_api: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(client, "services");

    log::debug!("cfs session: {}", cfs_session_name);

    let params = kube::api::ListParams::default()
        .limit(1)
        .labels(format!("cfsession={}", cfs_session_name).as_str());

    let mut pods = pods_api.list(&params).await?;

    let mut i = 0;
    let max = 300;

    // Waiting for pod to start
    while pods.items.is_empty() && i <= max {
        container_log_stream = container_log_stream
            .chain(tokio_stream::once(Ok(hyper::body::Bytes::copy_from_slice(
                format!(
                    "\nPod for cfs session {} not ready. Trying again in 2 secs. Attempt {} of {}\n",
                    cfs_session_name,
                    i + 1,
                    max
                )
                .as_bytes(),
            ))))
            .boxed();
        i += 1;
        thread::sleep(time::Duration::from_secs(2));
        pods = pods_api.list(&params).await?;
    }

    if pods.items.is_empty() {
        return Err(format!(
            "Pod for cfs session {} not ready. Aborting operation",
            cfs_session_name
        )
        .into());
    }

    let cfs_session_pod = &pods.items[0].clone();

    let cfs_session_pod_name = cfs_session_pod.metadata.name.clone().unwrap();
    log::info!("Pod name: {}", cfs_session_pod_name);

    // CSM 1.2.x
    let ansible_containers: Vec<&k8s_openapi::api::core::v1::Container> = if layer_id.is_some() {
        // Printing a CFS session layer logs

        let layer = layer_id.unwrap().to_string();

        let container_name = format!("ansible-{}", layer);

        // Get ansible-x containers
        cfs_session_pod
            .spec
            .as_ref()
            .unwrap()
            .containers
            .iter()
            .filter(|container| container.name.eq(&container_name))
            .collect()
    } else {
        // Get ansible-x containers
        cfs_session_pod
            .spec
            .as_ref()
            .unwrap()
            .containers
            .iter()
            .filter(|container| container.name.contains("ansible-"))
            .collect()
    };

    // CSM 1.3.x
    let ansible_containers: Vec<&k8s_openapi::api::core::v1::Container> = if ansible_containers.is_empty() {
        log::info!("No containers found with name 'ansible-x' trying 'ansible'");
        // Get ansible container
        cfs_session_pod
            .spec
            .as_ref()
            .unwrap()
            .containers
            .iter()
            .filter(|container| container.name.contains("ansible"))
            .collect()
    } else {
        eprintln!("No container related to CFS session logs found. Exit");
        std::process::exit(1);
    };

    for ansible_container in ansible_containers {
        container_log_stream = container_log_stream
            .chain(tokio_stream::once(Ok(hyper::body::Bytes::copy_from_slice(
                format!(
                    "\n*** Starting logs for container {}\n",
                    ansible_container.name
                )
                .as_bytes(),
            ))))
            .boxed();

        let logs_stream =
            get_container_logs_stream(ansible_container, cfs_session_pod, &pods_api, &params)
                .await
                .unwrap();

        // We are going to use chain method (https://dtantsur.github.io/rust-openstack/tokio/stream/trait.StreamExt.html#method.chain) to join streams coming from kube_client::api::subresource::Api::log_stream which returns Result<impl Stream<Item = Result<Bytes>>> or Result<hyper::body::Bytes>, we will consume the Result hence we will be chaining streams of hyper::body::Bytes
        container_log_stream = container_log_stream.chain(logs_stream).boxed();
    }

    Ok(container_log_stream)
}

fn get_container_state(pod: &k8s_openapi::api::core::v1::Pod, container_name: &String) -> Option<k8s_openapi::api::core::v1::ContainerState> {
    let container_status = pod
        .status
        .as_ref()
        .unwrap()
        .container_statuses
        .as_ref()
        .unwrap()
        .iter()
        .find(|container_status| container_status.name.eq(container_name));

    match container_status {
        Some(container_status_aux) => container_status_aux.state.clone(),
        None => None,
    }
}

#[cfg(test)]
mod test {

    use k8s_openapi::api::core::v1::Pod;
    use kube::{api::ListParams, Api};

    use crate::common::vault::http_client::fetch_shasta_k8s_secrets;

    use super::get_k8s_client_programmatically;

    #[tokio::test]
    async fn my_test() {
        std::env::set_var("SOCKS5", "socks5h://127.0.0.1:1080");
        let k8s_api_url = "https://10.252.1.12:6442";
        let vault_base_url = "https://hashicorp-vault.cscs.ch:8200";
        let vault_secret_path = "shasta";
        let vault_role_id = "b15517de-cabb-06ba-af98-633d216c6d99";

        let shasta_k8s_secrets = fetch_shasta_k8s_secrets(vault_base_url, vault_secret_path, vault_role_id).await;

        println!("\nhasta k8s secrets:\n{:#?}\n", shasta_k8s_secrets);

        let client = get_k8s_client_programmatically(k8s_api_url, shasta_k8s_secrets)
            .await
            .unwrap();

        let api_pods: Api<Pod> = Api::namespaced(client, "cicd");

        let lp = ListParams::default().limit(1);
        let pod_detail_list = api_pods.list(&lp).await;

        println!("\nPods:\n{:#?}\n", pod_detail_list);
    }
}
