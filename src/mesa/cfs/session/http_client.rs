pub mod http_client {

    use serde_json::Value;

    use crate::{mesa::cfs::session::get_response_struct::GetResponse, shasta};

    /// Fetch CFS sessions ref --> https://apidocs.svc.cscs.ch/paas/cfs/operation/get_sessions/
    /// Returns list of CFS sessions ordered by start time
    pub async fn get(
        shasta_token: &str,
        shasta_base_url: &str,
        is_succeded: Option<bool>,
    ) -> Result<Vec<GetResponse>, reqwest::Error> {
        let cfs_session_response =
            shasta::cfs::session::http_client::get_raw(shasta_token, shasta_base_url, is_succeded)
                .await;

        let cfs_session_response_value: Value = match cfs_session_response {
            Ok(cfs_session_value) => cfs_session_value.json().await.unwrap(),
            Err(error) => return Err(error),
        };

        let mut cfs_session_vec = Vec::new();

        if cfs_session_response_value.is_array() {
            for cfs_session_value in cfs_session_response_value.as_array().unwrap() {
                cfs_session_vec.push(GetResponse::from_csm_api_json(cfs_session_value.clone()));
            }
        } else {
            cfs_session_vec.push(GetResponse::from_csm_api_json(cfs_session_response_value));
        }

        Ok(cfs_session_vec)
    }
}

pub mod utils {
    use std::collections::HashSet;

    use termion::color;

    use crate::{mesa::cfs::session::get_response_struct::GetResponse, shasta};

    pub async fn filter(
        shasta_token: &str,
        shasta_base_url: &str,
        cfs_session_vec: &mut Vec<GetResponse>,
        hsm_group_name_opt: Option<&String>,
        cfs_session_name_opt: Option<&String>,
        limit_number_opt: Option<&u8>,
    ) -> Vec<GetResponse> {
        if let Some(hsm_group_name) = hsm_group_name_opt {
            let hsm_group_resp = crate::shasta::hsm::http_client::get_hsm_group(
                shasta_token,
                shasta_base_url,
                hsm_group_name,
            )
            .await;

            let hsm_group_nodes = if hsm_group_resp.is_ok() {
                shasta::hsm::utils::get_members_from_hsm_group_serde_value(&hsm_group_resp.unwrap())
            } else {
                eprintln!(
                    "No HSM group {}{}{} found!",
                    color::Fg(color::Red),
                    hsm_group_name,
                    color::Fg(color::Reset)
                );
                std::process::exit(1);
            };

            // Checks either target.groups contains hsm_group_name or ansible.limit is a subset of
            // hsm_group.members.ids
            cfs_session_vec.retain(|cfs_session| {
                cfs_session
                    .target
                    .clone()
                    .unwrap()
                    .groups
                    .unwrap_or(Vec::new())
                    .iter()
                    .any(|group| group.name.clone().unwrap().to_string().eq(hsm_group_name))
                    || cfs_session
                        .ansible
                        .clone()
                        .unwrap()
                        .limit
                        .unwrap_or("".to_string())
                        .split(',')
                        .map(|node| node.trim().to_string())
                        .collect::<HashSet<_>>()
                        .is_subset(&HashSet::from_iter(hsm_group_nodes.clone()))
            });
        }

        if let Some(session_name) = cfs_session_name_opt {
            cfs_session_vec
                .retain(|cfs_session| cfs_session.name.clone().unwrap().eq(session_name));
        }

        // Sort CFS sessions by start time order ASC
        cfs_session_vec.sort_by(|cfs_session_1, cfs_session_2| {
            cfs_session_1
                .status
                .clone()
                .unwrap()
                .session
                .unwrap()
                .start_time
                .unwrap()
                .cmp(
                    &cfs_session_2
                        .status
                        .clone()
                        .unwrap()
                        .session
                        .unwrap()
                        .start_time
                        .unwrap(),
                )
        });

        if let Some(limit_number) = limit_number_opt {
            // Limiting the number of results to return to client

            *cfs_session_vec = cfs_session_vec
                [cfs_session_vec.len().saturating_sub(*limit_number as usize)..]
                .to_vec();
        }

        cfs_session_vec.to_vec()
    }
}

#[tokio::test]
async fn test_cfs_session_serde_json_to_struct_conversion() {
    let cfs_session_value = serde_json::json!({
      "ansible": {
        "config": "cfs-default-ansible-cfg",
        "limit": "x1005c1s2b0n0,x1005c0s3b0n0",
        "passthrough": null,
        "verbosity": 0
      },
      "configuration": {
        "limit": "",
        "name": "clariden-cos-config-2.3.110-96-3"
      },
      "name": "batcher-e5c059a8-20c1-4779-9c0b-a270ff081d63",
      "status": {
        "artifacts": [],
        "session": {
          "completionTime": "2023-10-10T08:46:34",
          "job": "cfs-298b9145-7504-4241-a985-7a2f301cdd9f",
          "startTime": "2023-10-10T08:36:40",
          "status": "complete",
          "succeeded": "true"
        }
      },
      "tags": {
        "bos_session": "d452344f-4aad-4747-bfcb-8d016b5524bc"
      },
      "target": {
        "definition": "dynamic",
        "groups": null
      }
    });

    let cfs_session =
        crate::mesa::cfs::session::get_response_struct::GetResponse::from_csm_api_json(
            cfs_session_value,
        );

    println!("{:#?}", cfs_session);
}
