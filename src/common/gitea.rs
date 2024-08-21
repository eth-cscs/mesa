#![allow(dead_code, unused_imports)] // TODO: to avoid compiler from complaining about unused methods

pub mod http_client {

    use std::str::FromStr;

    use crate::{config, error::Error};
    use serde_json::Value;

    /// Get all refs for a repository
    /// Used when getting repo details
    pub async fn get_all_refs_from_repo_url(
        gitea_base_url: &str,
        gitea_token: &str,
        repo_url: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Vec<Value>, crate::error::Error> {
        let gitea_internal_base_url = "https://api-gw-service-nmn.local/vcs/cray/";
        // let gitea_external_base_url = "https://api.cmn.alps.cscs.ch/vcs/";

        let repo_name = repo_url
            .trim_start_matches(gitea_internal_base_url)
            .trim_end_matches(".git");

        get_all_refs(gitea_base_url, gitea_token, repo_name, shasta_root_cert).await
    }

    /// Get all refs for a repository
    /// Used when getting repo details
    pub async fn get_all_refs(
        gitea_base_url: &str,
        gitea_token: &str,
        repo_name: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Vec<Value>, crate::error::Error> {
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

        let api_url = format!(
            "{}/api/v1/repos/cray/{}/git/refs",
            gitea_base_url, repo_name
        );

        log::debug!("Get refs in gitea using through API call: {}", api_url);

        let resp_rslt = client
            .get(api_url)
            .header("Authorization", format!("token {}", gitea_token))
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<Value>>()
            .await;

        match resp_rslt {
            Ok(resp) => Ok(resp),
            Err(error) => Err(Error::NetError(error)),
        }
    }

    /// Get most commit id (sha) pointed by a branch
    pub async fn get_commit_pointed_by_branch(
        gitea_base_url: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
        repo_url: &str,
        branch_name: &str,
    ) -> Result<String, crate::error::Error> {
        let all_ref_vec =
            get_all_refs_from_repo_url(gitea_base_url, gitea_token, repo_url, shasta_root_cert)
                .await?;

        let ref_details_opt = all_ref_vec.into_iter().find(|ref_details| {
            ref_details["ref"].as_str().unwrap() == format!("refs/heads/{}", branch_name)
        });

        match ref_details_opt {
            Some(ref_details) => Ok(ref_details["object"]["sha"].as_str().unwrap().to_string()),
            None => Err(Error::Message("SHA for branch not found".to_string())),
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
    ) -> Result<Value, reqwest::Error> {
        let gitea_internal_base_url = "https://api-gw-service-nmn.local/vcs/";
        let gitea_external_base_url = "https://api.cmn.alps.cscs.ch/vcs/";

        let gitea_api_base_url = gitea_internal_base_url.to_owned() + "api/v1";

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

        log::debug!("Request to {}", api_url);

        client
            .get(api_url)
            .header("Authorization", format!("token {}", gitea_token))
            .send()
            .await?
            .json()
            .await
    }

    /// Returns the commit id (sha) related to a tag name
    /// Used to translate CFS configuration layer tag name into commit id values when processing
    /// SAT files
    pub async fn get_commit_from_tag(
        gitea_api_tag_url: &str,
        tag: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Value, Error> {
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

        log::debug!("Request to {}", api_url);

        let response_rslt = client
            .get(api_url.clone())
            .header("Authorization", format!("token {}", gitea_token))
            .send()
            .await;

        match response_rslt {
            Ok(response) => Ok(response.json::<Value>().await?),
            Err(error) => Err(Error::NetError(error)),
        }
    }

    // Get commit details.
    // NOTE: repo_name value must not contain the group (eg in CSSC gitlab we have
    // alps/csm-config/template-management and in gitea is vcs/api/v1/repos/template-management
    pub async fn get_commit_details_from_external_url(
        repo_url: &str,
        commitid: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Value, crate::error::Error> {
        let gitea_external_base_url = "https://api.cmn.alps.cscs.ch/vcs/";

        let repo_name = repo_url
            .trim_end_matches(".git")
            .trim_start_matches(gitea_external_base_url);

        if repo_name.ne(repo_url) {
            crate::error::Error::Message(
                "repo url provided does not match gitea internal URL".to_string(),
            );
        }

        get_commit_details(
            gitea_external_base_url,
            repo_name,
            commitid,
            gitea_token,
            shasta_root_cert,
        )
        .await
    }

    pub async fn get_commit_details_from_internal_url(
        repo_url: &str,
        commitid: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Value, crate::error::Error> {
        let gitea_internal_base_url = "https://api-gw-service-nmn.local/vcs/";

        let repo_name = repo_url
            .trim_end_matches(".git")
            .trim_start_matches(gitea_internal_base_url);

        if repo_name.ne(repo_url) {
            crate::error::Error::Message(
                "repo url provided does not match gitea internal URL".to_string(),
            );
        }

        get_commit_details(
            gitea_internal_base_url,
            repo_name,
            commitid,
            gitea_token,
            shasta_root_cert,
        )
        .await
    }

    pub async fn get_commit_details(
        gitea_base_url: &str,
        repo_name: &str,
        commitid: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> Result<Value, crate::error::Error> {
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
            "{}/api/v1/repos/cray/{}/git/commits/{}",
            gitea_base_url, repo_name, commitid
        );

        log::debug!("Request to {}", api_url);

        let response = client
            .get(api_url)
            .header("Authorization", format!("token {}", gitea_token))
            .send()
            .await?;

        if response.status().is_success() {
            // Make sure we return a vec if user requesting a single value
            response
                .json()
                .await
                .map_err(|error| Error::NetError(error))
        } else {
            let payload = response
                .json::<Value>()
                .await
                .map_err(|error| Error::NetError(error))?;
            Err(Error::CsmError(payload))
        }
    }

    pub async fn get_last_commit_from_repo_name(
        gitea_api_base_url: &str,
        repo_name: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> core::result::Result<Value, reqwest::Error> {
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

        let mut resp: Vec<Value> = client
            .get(repo_url)
            .header("Authorization", format!("token {}", gitea_token))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        resp.sort_by(|a, b| {
            a["commit"]["committer"]["date"]
                .to_string()
                .cmp(&b["commit"]["committer"]["date"].to_string())
        });

        Ok(resp.last().unwrap().clone())
    }

    pub async fn get_last_commit_from_url(
        gitea_api_base_url: &str,
        repo_url: &str,
        gitea_token: &str,
        shasta_root_cert: &[u8],
    ) -> core::result::Result<Value, reqwest::Error> {
        let repo_name = repo_url
            .trim_start_matches("https://api-gw-service-nmn.local/vcs/")
            .trim_end_matches(".git");

        get_last_commit_from_repo_name(gitea_api_base_url, repo_name, gitea_token, shasta_root_cert)
            .await
    }
}
