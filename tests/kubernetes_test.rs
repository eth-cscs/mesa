#[cfg(test)]
mod test {

    use k8s_openapi::api::core::v1::Pod;
    use kube::{
        api::{AttachParams, ListParams},
        Api,
    };
    use mesa::common::{
        kubernetes::{self, get_k8s_client_programmatically},
        vault::http_client::fetch_shasta_k8s_secrets,
    };

    #[tokio::test]
    async fn test_connection_to_k8s() {
        std::env::set_var("SOCKS5", "socks5h://127.0.0.1:1080");
        let k8s_api_url = "https://10.252.1.12:6442";
        let vault_base_url = "https://hashicorp-vault.cscs.ch:8200";
        let vault_secret_path = "shasta";
        let vault_role_id = "b15517de-cabb-06ba-af98-633d216c6d99";
        let pod_name = "cfs-7e54c14a-89fb-4564-886e-d11d69866212-d25rn";

        let shasta_k8s_secrets =
            fetch_shasta_k8s_secrets(vault_base_url, vault_secret_path, vault_role_id).await;

        let client = get_k8s_client_programmatically(k8s_api_url, shasta_k8s_secrets)
            .await
            .unwrap();

        let api_pods: Api<Pod> = Api::namespaced(client, "services");

        let lp = ListParams::default().limit(1);
        let pod_detail_list = api_pods.list(&lp).await;

        println!("Pods:\n{:#?}", pod_detail_list);

        let ap = AttachParams::default().container("ansible");

        let attached = api_pods
            .exec(pod_name, vec!["sh", "-c", "echo $LAYER_CURRENT"], &ap)
            .await
            .unwrap();

        let output = kubernetes::get_output(attached).await;

        println!("Current CFS configurarion layer is {}", output);
    }
}
