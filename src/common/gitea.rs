#![allow(dead_code, unused_imports)] // TODO: to avoid compiler from complaining about unused methods

pub mod http_client {

    use std::error::Error;

    use crate::{cfs::configuration::mesa::r#struct::cfs_configuration_response::ApiError, config};
    use serde_json::Value;

    /// Get all refs for a repository
    /// Used when getting repo details
    pub async fn get_all_refs(
        repo_url: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Vec<Value>, ApiError> {
        let gitea_internal_base_url = "https://api-gw-service-nmn.local/vcs/";
        let gitea_external_base_url = "https://api.cmn.alps.cscs.ch/vcs/";

        let gitea_api_base_url = gitea_external_base_url.to_owned() + "api/v1";

        let repo_name = repo_url
            .trim_start_matches(gitea_internal_base_url)
            .trim_end_matches(".git");
        let repo_name = repo_name
            .trim_start_matches(gitea_external_base_url)
            .trim_end_matches(".git");

        /* log::info!("repo_url: {}", repo_url);
        log::info!("gitea_base_url: {}", gitea_internal_base_url);
        log::info!("repo_name: {}", repo_name); */

        let client;

        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert).unwrap());

        // Build client
        if std::env::var("SOCKS5").is_ok() {
            // socks5 proxy
            let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap()).unwrap();

            // rest client to authenticate
            client = client_builder.proxy(socks5proxy).build().unwrap();
        } else {
            client = client_builder.build().unwrap();
        }

        let api_url = format!("{}/repos/{}/git/refs", gitea_api_base_url, repo_name);

        log::info!("Get refs in gitea using through API call: {}", api_url);

        let resp = client
            .get(api_url)
            .header("Authorization", format!("token {}", gitea_token))
            .send()
            .await
            .unwrap();

        if resp.status().is_success() {
            let json_response: Vec<Value> = resp.json().await.unwrap();

            log::debug!(
                "Gitea response refs for repo '{}':\n{:#?}",
                repo_name,
                json_response
            );

            Ok(json_response)
        } else {
            let response_payload = resp.text().await.unwrap();

            Err(ApiError::MesaError(response_payload))
        }
    }

    /// Returns the commit id (sha) related to a tag name
    /// Used to translate CFS configuration layer tag name into commit id values when processing
    /// SAT files
    pub async fn get_tag_details(
        repo_url: &str,
        tag: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Value, Box<dyn Error>> {
        let gitea_internal_base_url = "https://api-gw-service-nmn.local/vcs/";
        let gitea_external_base_url = "https://api.cmn.alps.cscs.ch/vcs/";

        let gitea_api_base_url = gitea_external_base_url.to_owned() + "api/v1";

        let repo_name = repo_url
            .trim_start_matches(gitea_internal_base_url)
            .trim_end_matches(".git");
        let repo_name = repo_name
            .trim_start_matches(gitea_external_base_url)
            .trim_end_matches(".git");

        /* log::info!("repo_url: {}", repo_url);
        log::info!("gitea_base_url: {}", gitea_internal_base_url);
        log::info!("repo_name: {}", repo_name); */

        let client;

        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

        // Build client
        if std::env::var("SOCKS5").is_ok() {
            // socks5 proxy
            let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

            // rest client to authenticate
            client = client_builder.proxy(socks5proxy).build()?;
        } else {
            client = client_builder.build()?;
        }

        let api_url = format!("{}/repos/{}/tags/{}", gitea_api_base_url, repo_name, tag);

        log::info!("Request to {}", api_url);

        let resp = client
            .get(api_url)
            .header("Authorization", format!("token {}", gitea_token))
            .send()
            .await?;

        if resp.status().is_success() {
            let json_response: Value = resp.json().await?;

            log::debug!("{}", serde_json::to_string_pretty(&json_response)?);

            Ok(json_response)
        } else {
            let error_msg = format!("ERROR: tag {} not found in Shasta CVS. Please check gitea admin or wait sync to finish.", tag);

            Err(error_msg.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
        }
    }

    /// Returns the commit id (sha) related to a tag name
    /// Used to translate CFS configuration layer tag name into commit id values when processing
    /// SAT files
    pub async fn get_commit_from_tag(
        gitea_api_tag_url: &str,
        tag: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Value, Box<dyn Error>> {
        let repo_name: &str = gitea_api_tag_url
            .trim_start_matches("https://vcs.cmn.alps.cscs.ch/vcs/api/v1/repos/cray/")
            .split('/')
            .next()
            .unwrap();

        let api_url = format!(
            "https://api.cmn.alps.cscs.ch/vcs/api/v1/repos/cray/{}/tags/{}",
            repo_name, tag
        );

        let client;

        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

        // Build client
        if std::env::var("SOCKS5").is_ok() {
            // socks5 proxy
            let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

            // rest client to authenticate
            client = client_builder.proxy(socks5proxy).build()?;
        } else {
            client = client_builder.build()?;
        }

        log::info!("Request to {}", api_url);

        let resp = client
            .get(api_url.clone())
            .header("Authorization", format!("token {}", gitea_token))
            .send()
            .await?;

        if resp.status().is_success() {
            let json_response: Value = resp.json().await?;

            log::debug!("{}", serde_json::to_string_pretty(&json_response)?);

            Ok(json_response)
        } else {
            let error_msg = format!("ERROR: tag related to run '{}' not found in Shasta CVS. Please check gitea admin or wait sync to finish.", api_url);

            Err(error_msg.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
        }
    }

    pub async fn get_commit_details(
        repo_url: &str,
        commitid: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Value, Box<dyn Error>> {
        let gitea_internal_base_url = "https://api-gw-service-nmn.local/vcs/";
        let gitea_external_base_url = "https://api.cmn.alps.cscs.ch/vcs/";

        let gitea_api_base_url = gitea_external_base_url.to_owned() + "api/v1";

        let repo_name = repo_url
            .trim_start_matches(gitea_internal_base_url)
            .trim_end_matches(".git");
        let repo_name = repo_name
            .trim_start_matches(gitea_external_base_url)
            .trim_end_matches(".git");

        /* log::info!("repo_url: {}", repo_url);
        log::info!("gitea_base_url: {}", gitea_internal_base_url);
        log::info!("repo_name: {}", repo_name); */

        let client;

        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

        // Build client
        if std::env::var("SOCKS5").is_ok() {
            // socks5 proxy
            let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

            // rest client to authenticate
            client = client_builder.proxy(socks5proxy).build()?;
        } else {
            client = client_builder.build()?;
        }

        let api_url = format!(
            "{}/repos/{}/git/commits/{}",
            gitea_api_base_url, repo_name, commitid
        );

        log::info!("Request to {}", api_url);

        let resp = client
            .get(api_url)
            .header("Authorization", format!("token {}", gitea_token))
            .send()
            .await?;

        if resp.status().is_success() {
            let json_response: Value = resp.json().await?;

            log::debug!(
                "Gitea commit id '{}' details:\n{:#?}",
                commitid,
                json_response
            );

            Ok(json_response)
        } else {
            let error_msg = format!("ERROR: commit {} not found in Shasta CVS. Please check gitea admin or wait sync to finish.", commitid);

            Err(error_msg.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
        }
    }

    pub async fn get_last_commit_from_repo_name(
        gitea_api_base_url: &str,
        repo_name: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> core::result::Result<Value, Box<dyn std::error::Error>> {
        let repo_url = gitea_api_base_url.to_owned() + "/api/v1/repos" + repo_name + "/commits";

        let client;

        let client_builder = reqwest::Client::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

        // Build client
        if std::env::var("SOCKS5").is_ok() {
            // socks5 proxy
            let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

            // rest client to authenticate
            client = client_builder.proxy(socks5proxy).build()?;
        } else {
            client = client_builder.build()?;
        }

        let resp = client
            .get(repo_url)
            .header("Authorization", format!("token {}", gitea_token))
            .send()
            .await?;

        if resp.status().is_success() {
            let mut json_response: Vec<Value> = serde_json::from_str(&resp.text().await?)?;
            json_response.sort_by(|a, b| {
                a["commit"]["committer"]["date"]
                    .to_string()
                    .cmp(&b["commit"]["committer"]["date"].to_string())
            });

            println!("last commit: {:#?}", json_response.last().unwrap().clone());

            Ok(json_response.last().unwrap().clone())
        } else {
            eprintln!("FAIL request: {:#?}", resp);
            let response: String = resp.text().await?;
            eprintln!("FAIL response: {:#?}", response);
            Err(response.into()) // Black magic conversion from Err(Box::new("my error msg")) which does not
        }
    }

    pub async fn get_last_commit_from_url(
        gitea_api_base_url: &str,
        repo_url: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> core::result::Result<Value, Box<dyn std::error::Error>> {
        let repo_name = repo_url
            .trim_start_matches("https://api-gw-service-nmn.local/vcs/")
            .trim_end_matches(".git");

        get_last_commit_from_repo_name(gitea_api_base_url, repo_name, gitea_token, shasta_root_cert)
            .await
    }
}
