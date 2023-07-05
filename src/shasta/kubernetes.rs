use std::{error::Error, str::FromStr};

use hyper::Uri;
use hyper_socks2::SocksConnector;
use kube::{
    client::ConfigExt,
    config::{
        AuthInfo, Cluster, Context, KubeConfigOptions, Kubeconfig, NamedAuthInfo, NamedCluster,
        NamedContext,
    },
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
        let vault_role_id = "b15517de-cabb-06ba-af98-633d216c6d99";

        let shasta_k8s_secrets = fetch_shasta_k8s_secrets(vault_base_url, vault_role_id).await;

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
