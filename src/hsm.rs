pub mod service {

    pub mod values {

        pub mod role {
            pub mod r#struct {
                use serde::{Deserialize, Serialize};

                #[derive(Debug, Serialize, Deserialize, Clone)]
                pub struct Role {
                    #[serde(rename(serialize = "Role"))]
                    pub role: Vec<String>,
                }
            }

            pub mod http_client {
                use crate::error::Error;

                use super::r#struct::Role;

                /// Get list of Roles
                pub async fn get(
                    shasta_token: &str,
                    shasta_base_url: &str,
                    shasta_root_cert: &[u8],
                ) -> Result<Vec<String>, Error> {
                    let client_builder = reqwest::Client::builder()
                        .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

                    // Build client
                    let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                        // socks5 proxy
                        log::debug!("SOCKS5 enabled");
                        let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                        // rest client to authenticate
                        client_builder.proxy(socks5proxy).build()?
                    } else {
                        client_builder.build()?
                    };

                    let api_url: String =
                        shasta_base_url.to_owned() + "/smd/hsm/v2/service/values/role";

                    let payload = client
                        .get(api_url)
                        .bearer_auth(shasta_token)
                        .send()
                        .await
                        .map_err(|error| Error::NetError(error))?
                        .json::<Role>()
                        .await;

                    payload
                        .map(|role| role.role)
                        .map_err(|error| Error::NetError(error))
                }
            }

            pub mod hardcoded_values {
                pub fn get() -> Vec<String> {
                    vec![
                        "Storage".to_string(),
                        "Management".to_string(),
                        "Compute".to_string(),
                        "Service".to_string(),
                        "System".to_string(),
                        "Application".to_string(),
                    ]
                }
            }
        }
    }
}

/// Refs:
/// Member/node state --> https://apidocs.svc.cscs.ch/iaas/hardware-state-manager/overview/#section/Valid-State-Transistions
pub mod group {

    pub mod r#struct {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, Clone)]
        pub struct HsmGroup {
            pub label: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub description: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub tags: Option<Vec<String>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub members: Option<Member>,
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename(serialize = "exclusiveGroup"))]
            pub exclusive_group: Option<String>,
        }

        #[derive(Debug, Serialize, Deserialize, Default, Clone)]
        pub struct Member {
            #[serde(skip_serializing_if = "Option::is_none")]
            pub ids: Option<Vec<String>>,
        }
        #[derive(Debug, Serialize, Deserialize, Default, Clone)]
        pub struct XnameId {
            #[serde(skip_serializing_if = "Option::is_none")]
            pub id: Option<String>,
        }
    }

    pub mod http_client {
        use crate::{
            error::Error,
            hsm::group::r#struct::{HsmGroup, Member, XnameId},
        };

        use super::hacks::filter_system_hsm_groups;

        /// Get list of HSM group using --> shttps://apidocs.svc.cscs.ch/iaas/hardware-state-manager/operation/doGroupsGet/
        pub async fn get_raw(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            group_name_opt: Option<&String>,
        ) -> Result<reqwest::Response, Error> {
            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                // rest client to authenticate
                client_builder.proxy(socks5proxy).build()?
            } else {
                client_builder.build()?
            };

            let api_url: String = if let Some(group_name) = group_name_opt {
                shasta_base_url.to_owned() + "/smd/hsm/v2/groups/" + group_name
            } else {
                shasta_base_url.to_owned() + "/smd/hsm/v2/groups"
            };

            client
                .get(api_url)
                .bearer_auth(shasta_token)
                .send()
                .await
                .map_err(|error| Error::NetError(error))
        }

        /// Gets list of HSM groups from CSM api. It also does a hack where the list returned by
        /// CSM API gets shrinked by removing the CSM wide HSM groups like `alps`, `alpsm`,
        /// `alpsb`, etc
        pub async fn get(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            group_name_opt: Option<&String>,
        ) -> Result<Vec<HsmGroup>, Error> {
            let response = get_raw(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                group_name_opt,
            )
            .await?;

            if response.status().is_success() {
                if group_name_opt.is_some() {
                    let payload = response
                        .json::<HsmGroup>()
                        .await
                        .map_err(|error| Error::NetError(error))?;

                    let hsm_group_vec_rslt = Ok(vec![payload]);

                    //FIXME: Get rid of this by making sure CSM admins don't create HSM groups for system
                    //wide operations instead of using roles
                    filter_system_hsm_groups(hsm_group_vec_rslt)
                } else {
                    let hsm_group_vec_rslt = response
                        .json()
                        .await
                        .map_err(|error| Error::NetError(error));

                    //FIXME: Get rid of this by making sure CSM admins don't create HSM groups for system
                    //wide operations instead of using roles
                    filter_system_hsm_groups(hsm_group_vec_rslt)
                }
            } else {
                let payload = response
                    .text()
                    .await
                    .map_err(|error| Error::NetError(error))?;

                Err(Error::Message(payload))
            }
        }

        pub async fn get_all(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
        ) -> Result<Vec<HsmGroup>, Error> {
            get(shasta_token, shasta_base_url, shasta_root_cert, None).await
        }

        /// Get list of HSM groups using --> https://apidocs.svc.cscs.ch/iaas/hardware-state-manager/operation/doGroupsGet/
        /// NOTE: this returns all HSM groups which name contains hsm_groupu_name param value
        /// FIXME: change `hsm_group_name_opt` type from `Option<&String>` to Option<`&str`>
        pub async fn get_hsm_group_vec(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_group_name_opt: Option<&String>,
        ) -> Result<Vec<HsmGroup>, Error> {
            let json_response = get_all(shasta_token, shasta_base_url, shasta_root_cert).await?;

            let mut hsm_groups: Vec<HsmGroup> = Vec::new();

            if let Some(hsm_group_name) = hsm_group_name_opt {
                for hsm_group in json_response {
                    if hsm_group.label.contains(hsm_group_name) {
                        hsm_groups.push(hsm_group.clone());
                    }
                }
            }

            Ok(hsm_groups)
        }

        pub async fn post_member(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_group_name: &str,
            member_id: &str,
        ) -> Result<(), reqwest::Error> {
            log::info!("Add member {}/{}", hsm_group_name, member_id);
            let client;

            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            if std::env::var("SOCKS5").is_ok() {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

                // rest client to authenticate
                client = client_builder.proxy(socks5proxy).build()?;
            } else {
                client = client_builder.build()?;
            }

            let api_url: String =
                shasta_base_url.to_owned() + "/smd/hsm/v2/groups/" + hsm_group_name + "/members";

            let xname = XnameId {
                id: Some(member_id.to_owned()),
            };

            client
                .post(api_url)
                .header("Authorization", format!("Bearer {}", shasta_token))
                .json(&xname) // make sure this is not a string!
                .send()
                .await?
                .error_for_status()?;
            // TODO Parse the output!!!
            // TODO add some debugging output

            Ok(())
        }

        pub async fn delete_member(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_group_name: &str,
            member_id: &str,
        ) -> Result<(), reqwest::Error> {
            log::info!("Delete member {}/{}", hsm_group_name, member_id);
            let client;

            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            if std::env::var("SOCKS5").is_ok() {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

                // rest client to authenticate
                client = client_builder.proxy(socks5proxy).build()?;
            } else {
                client = client_builder.build()?;
            }

            let api_url: String = shasta_base_url.to_owned()
                + "/smd/hsm/v2/groups/"
                + hsm_group_name
                + "/members/"
                + member_id;

            client
                .delete(api_url)
                .header("Authorization", format!("Bearer {}", shasta_token))
                .send()
                .await?
                .error_for_status()?;

            // TODO Parse the output!!!
            // TODO add some debugging output
            Ok(())
        }

        /// https://github.com/Cray-HPE/docs-csm/blob/release/1.5/api/smd.md#post-groups
        pub async fn create_new_hsm_group(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_group_name_opt: &str, // label in HSM
            xnames: &[String],
            exclusive: &str,
            description: &str,
            tags: &[String],
        ) -> Result<Vec<HsmGroup>, reqwest::Error> {
            let client;

            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            if std::env::var("SOCKS5").is_ok() {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

                // rest client to authenticate
                client = client_builder.proxy(socks5proxy).build()?;
            } else {
                client = client_builder.build()?;
            }
            // Example body to create a new group:
            // {
            //   "label": "blue",
            //   "description": "This is the blue group",
            //   "tags": [
            //     "optional_tag1",
            //     "optional_tag2"
            //   ],
            //   "exclusiveGroup": "optional_excl_group",
            //   "members": {
            //     "ids": [
            //       "x1c0s1b0n0",
            //       "x1c0s1b0n1",
            //       "x1c0s2b0n0",
            //       "x1c0s2b0n1"
            //     ]
            //   }
            // }
            // Describe the JSON object

            // Create the variables that represent our JSON object
            let myxnames = Member {
                ids: Some(xnames.to_owned()),
            };

            let hsm_group_json = HsmGroup {
                label: hsm_group_name_opt.to_owned(),
                description: Option::from(description.to_string().clone()),
                tags: Option::from(tags.to_owned()),
                exclusive_group: Option::from(exclusive.to_string().clone()),
                members: Some(myxnames),
            };

            let hsm_group_json_body = match serde_json::to_string(&hsm_group_json) {
                    Ok(m) => m,
                    Err(_) => panic!("Error parsing the JSON generated, one or more of the fields could have invalid chars."),
                };

            println!("{:#?}", &hsm_group_json_body);

            let url_api = shasta_base_url.to_owned() + "/smd/hsm/v2/groups";

            client
                .post(url_api)
                .header("Authorization", format!("Bearer {}", shasta_token))
                .json(&hsm_group_json) // make sure this is not a string!
                .send()
                .await?
                .error_for_status()?
                .json()
                .await
        }

        pub async fn delete_hsm_group(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_group_name_opt: &String, // label in HSM
        ) -> Result<String, reqwest::Error> {
            let client;

            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            if std::env::var("SOCKS5").is_ok() {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

                // rest client to authenticate
                client = client_builder.proxy(socks5proxy).build()?;
            } else {
                client = client_builder.build()?;
            }
            let url_api = shasta_base_url.to_owned() + "/smd/hsm/v2/groups/" + &hsm_group_name_opt;

            client
                .delete(url_api)
                .header("Authorization", format!("Bearer {}", shasta_token))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await
        }
    }

    pub mod hacks {
        use crate::error::Error;

        use super::r#struct::HsmGroup;

        pub fn filter_system_hsm_groups(
            hsm_group_vec_rslt: Result<Vec<HsmGroup>, Error>,
        ) -> Result<Vec<HsmGroup>, Error> {
            //TODO: Get rid of this by making sure CSM admins don't create HSM groups for system
            //wide operations instead of using roles
            let hsm_group_to_ignore_vec = ["alps", "prealps", "alpse", "alpsb"];
            let hsm_group_vec_filtered_rslt: Result<Vec<HsmGroup>, Error> = hsm_group_vec_rslt
                .and_then(|hsm_group_vec| {
                    Ok(hsm_group_vec
                        .iter()
                        .filter(|hsm_group| {
                            let label = hsm_group.label.as_str();
                            !hsm_group_to_ignore_vec.contains(&label)
                        })
                        .cloned()
                        .collect::<Vec<HsmGroup>>())
                });

            if let Ok([]) = hsm_group_vec_filtered_rslt.as_deref() {
                Err(Error::Message(
                    "HSM groups 'alps, prealps, alpse, alpsb' not allowed.".to_string(),
                ))
            } else {
                hsm_group_vec_filtered_rslt
            }
        }

        pub fn filter_system_hsm_group_names(hsm_group_name_vec: Vec<String>) -> Vec<String> {
            //FIXME: Get rid of this by making sure CSM admins don't create HSM groups for system
            //wide operations instead of using roles
            let hsm_group_to_ignore_vec = ["alps", "prealps", "alpse", "alpsb"];

            hsm_group_name_vec
                .into_iter()
                .filter(|hsm_group_name| {
                    !hsm_group_to_ignore_vec.contains(&hsm_group_name.as_str())
                })
                .collect()
        }
    }

    pub mod utils {

        use std::{
            collections::{HashMap, HashSet},
            sync::Arc,
            time::Instant,
        };

        use serde_json::Value;
        use tokio::sync::Semaphore;

        use crate::{
            cfs::session::http_client::v3::r#struct::CfsSessionGetResponse,
            error::Error,
            hsm::group::{
                http_client::{get, post_member},
                r#struct::HsmGroup,
            },
            node::utils::validate_xnames_format_and_membership_agaisnt_single_hsm,
        };

        use super::http_client::{self, delete_member};

        /// Add a list of xnames to target HSM group
        /// Returns the new list of nodes in target HSM group
        pub async fn add_hsm_members(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            target_hsm_group_name: &str,
            new_target_hsm_members: Vec<&str>,
            dryrun: bool,
        ) -> Result<Vec<String>, Error> {
            // get list of target HSM group members
            let mut target_hsm_group_member_vec: Vec<String> = get_member_vec_from_hsm_group_name(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                target_hsm_group_name,
            )
            .await;

            // merge HSM group list with the list of xnames provided by the user
            target_hsm_group_member_vec
                .extend(new_target_hsm_members.iter().map(|xname| xname.to_string()));

            target_hsm_group_member_vec.sort();
            target_hsm_group_member_vec.dedup();

            // *********************************************************************************************************
            // UPDATE HSM GROUP MEMBERS IN CSM
            if dryrun {
                println!(
                    "Add following nodes to HSM group {}:\n{:?}",
                    target_hsm_group_name, new_target_hsm_members
                );

                println!("dry-run enabled, changes not persisted.");
            } else {
                for xname in new_target_hsm_members {
                    let _ = post_member(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        target_hsm_group_name,
                        xname,
                    )
                    .await;
                }
            }

            Ok(target_hsm_group_member_vec)
        }

        /// Removes list of xnames from  HSM group
        pub async fn remove_hsm_members(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            target_hsm_group_name: &str,
            new_target_hsm_members: Vec<&str>,
            dryrun: bool,
        ) -> Result<Vec<String>, Error> {
            // Check nodes are valid xnames and they belong to parent HSM group
            if !validate_xnames_format_and_membership_agaisnt_single_hsm(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                new_target_hsm_members.as_slice(),
                Some(&target_hsm_group_name.to_string()),
            )
            .await
            {
                let error_msg = format!("Nodes '{}' not valid", new_target_hsm_members.join(", "));
                return Err(Error::Message(error_msg));
                /* eprintln!("Nodes '{}' not valid", new_target_hsm_members.join(", "));
                std::process::exit(1); */
            }

            // get list of parent HSM group members
            let mut target_hsm_group_member_vec: Vec<String> = get_member_vec_from_hsm_group_name(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                target_hsm_group_name,
            )
            .await;

            target_hsm_group_member_vec
                .retain(|parent_member| !new_target_hsm_members.contains(&parent_member.as_str()));

            target_hsm_group_member_vec.sort();
            target_hsm_group_member_vec.dedup();

            // *********************************************************************************************************
            // UPDATE HSM GROUP MEMBERS IN CSM
            if dryrun {
                println!(
                    "Remove following nodes from HSM group {}:\n{:?}",
                    target_hsm_group_name, new_target_hsm_members
                );

                println!("dry-run enabled, changes not persisted.");
            } else {
                for xname in new_target_hsm_members {
                    let _ = delete_member(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        target_hsm_group_name,
                        xname,
                    )
                    .await;
                }
            }

            Ok(target_hsm_group_member_vec)
        }

        /// Moves list of xnames from parent to target HSM group
        pub async fn migrate_hsm_members(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            target_hsm_group_name: &str,
            parent_hsm_group_name: &str,
            new_target_hsm_members: Vec<&str>,
            nodryrun: bool,
        ) -> Result<(Vec<String>, Vec<String>), Error> {
            // Check nodes are valid xnames and they belong to parent HSM group
            if !validate_xnames_format_and_membership_agaisnt_single_hsm(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                new_target_hsm_members.as_slice(),
                Some(&parent_hsm_group_name.to_string()),
            )
            .await
            {
                let error_msg = format!("Nodes '{}' not valid", new_target_hsm_members.join(", "));
                return Err(Error::Message(error_msg));
                /* eprintln!("Nodes '{}' not valid", new_target_hsm_members.join(", "));
                std::process::exit(1); */
            }

            // get list of target HSM group members
            let mut target_hsm_group_member_vec: Vec<String> = get_member_vec_from_hsm_group_name(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                target_hsm_group_name,
            )
            .await;

            // merge HSM group list with the list of xnames provided by the user
            target_hsm_group_member_vec
                .extend(new_target_hsm_members.iter().map(|xname| xname.to_string()));

            target_hsm_group_member_vec.sort();
            target_hsm_group_member_vec.dedup();

            // get list of parent HSM group members
            let mut parent_hsm_group_member_vec: Vec<String> = get_member_vec_from_hsm_group_name(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                parent_hsm_group_name,
            )
            .await;

            parent_hsm_group_member_vec
                .retain(|parent_member| !target_hsm_group_member_vec.contains(parent_member));

            parent_hsm_group_member_vec.sort();
            parent_hsm_group_member_vec.dedup();

            // *********************************************************************************************************
            // UPDATE HSM GROUP MEMBERS IN CSM
            if !nodryrun {
                let target_hsm_group = serde_json::json!({
                    "label": target_hsm_group_name,
                    "decription": "",
                    "members": target_hsm_group_member_vec,
                    "tags": []
                });

                println!(
                    "Target HSM group:\n{}",
                    serde_json::to_string_pretty(&target_hsm_group).unwrap()
                );

                let parent_hsm_group = serde_json::json!({
                    "label": parent_hsm_group_name,
                    "decription": "",
                    "members": parent_hsm_group_member_vec,
                    "tags": []
                });

                println!(
                    "Parent HSM group:\n{}",
                    serde_json::to_string_pretty(&parent_hsm_group).unwrap()
                );

                println!("dry-run enabled, changes not persisted.");
            } else {
                for xname in new_target_hsm_members {
                    let _ = post_member(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        target_hsm_group_name,
                        xname,
                    )
                    .await;

                    let _ = delete_member(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        parent_hsm_group_name,
                        xname,
                    )
                    .await;
                }
            }

            Ok((target_hsm_group_member_vec, parent_hsm_group_member_vec))
        }

        /// Receives 2 lists of xnames old xnames to remove from parent HSM group and new xhanges to add to target HSM group, and does just that
        pub async fn update_hsm_group_members(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_group_name: &str,
            old_target_hsm_group_members: &Vec<String>,
            new_target_hsm_group_members: &Vec<String>,
        ) -> Result<(), Error> {
            // Delete members
            for old_member in old_target_hsm_group_members {
                if !new_target_hsm_group_members.contains(old_member) {
                    let _ = delete_member(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        hsm_group_name,
                        old_member,
                    )
                    .await;
                }
            }

            // Add members
            for new_member in new_target_hsm_group_members {
                if !old_target_hsm_group_members.contains(new_member) {
                    let _ = post_member(
                        shasta_token,
                        shasta_base_url,
                        shasta_root_cert,
                        hsm_group_name,
                        new_member,
                    )
                    .await;
                }
            }

            Ok(())
        }

        // Returns a HashMap with keys being the hsm names/labels the user has access a curated list of xnames
        // for each hsm name as values
        pub async fn get_hsm_map_and_filter_by_hsm_name_vec(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_name_vec: Vec<&str>,
        ) -> Result<HashMap<String, Vec<String>>, Error> {
            let hsm_group_vec =
                http_client::get_all(shasta_token, shasta_base_url, shasta_root_cert).await?;

            Ok(filter_and_convert_to_map(hsm_name_vec, hsm_group_vec))
        }

        /// Given a list of HsmGroup struct and a list of Hsm group names, it will filter out those
        /// not in the Hsm group names and convert from HsmGroup struct to HashMap
        pub fn filter_and_convert_to_map(
            hsm_name_vec: Vec<&str>,
            hsm_group_vec: Vec<HsmGroup>,
        ) -> HashMap<String, Vec<String>> {
            let mut hsm_group_map: HashMap<String, Vec<String>> = HashMap::new();

            for hsm_group in hsm_group_vec {
                if hsm_name_vec.contains(&hsm_group.label.as_str()) {
                    hsm_group_map.entry(hsm_group.label).or_insert(
                        hsm_group
                            .members
                            .and_then(|members| Some(members.ids.unwrap_or_default()))
                            .unwrap(),
                    );
                }
            }

            hsm_group_map
        }

        pub fn get_member_vec_from_hsm_group_value(hsm_group: &Value) -> Vec<String> {
            // Take all nodes for all hsm_groups found and put them in a Vec
            hsm_group["members"]["ids"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|xname| xname.as_str().unwrap().to_string())
                .collect()
        }

        pub fn get_member_vec_from_hsm_group(hsm_group: &HsmGroup) -> Vec<String> {
            // Take all nodes for all hsm_groups found and put them in a Vec
            hsm_group
                .members
                .as_ref()
                .unwrap()
                .ids
                .as_ref()
                .unwrap_or(&Vec::new())
                .clone()
        }

        /// Get the list of xnames which are members of a list of HSM groups.
        /// eg:
        /// given following HSM groups:
        /// tenant_a: [x1003c1s7b0n0, x1003c1s7b0n1]
        /// tenant_b: [x1003c1s7b1n0]
        /// Then calling this function with hsm_name_vec: &["tenant_a", "tenant_b"] should return [x1003c1s7b0n0, x1003c1s7b0n1, x1003c1s7b1n0]
        pub async fn get_member_vec_from_hsm_name_vec(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_name_vec: Vec<String>,
        ) -> Vec<String> {
            log::info!("Get xnames for HSM groups: {:?}", hsm_name_vec);

            let start = Instant::now();

            /* let mut hsm_group_value_vec =
                http_client::get_all(shasta_token, shasta_base_url, shasta_root_cert)
                    .await
                    .unwrap();

            hsm_group_value_vec.retain(|hsm_value| hsm_name_vec.contains(&hsm_value.label));

            Vec::from_iter(
                get_member_vec_from_hsm_group_vec(&hsm_group_value_vec)
                    .iter()
                    .cloned(),
            ) */

            let mut hsm_group_member_vec: Vec<String> = Vec::new();

            let pipe_size = 10;

            let mut tasks = tokio::task::JoinSet::new();

            let sem = Arc::new(Semaphore::new(pipe_size)); // CSM 1.3.1 higher number of concurrent tasks won't
                                                           //
            for hsm_name in hsm_name_vec {
                let shasta_token_string = shasta_token.to_string();
                let shasta_base_url_string = shasta_base_url.to_string();
                let shasta_root_cert_vec = shasta_root_cert.to_vec();

                let permit = Arc::clone(&sem).acquire_owned().await;

                tasks.spawn(async move {
                    let _permit = permit; // Wait semaphore to allow new tasks https://github.com/tokio-rs/tokio/discussions/2648#discussioncomment-34885

                    get(
                        &shasta_token_string,
                        &shasta_base_url_string,
                        &shasta_root_cert_vec,
                        Some(&hsm_name),
                    )
                    .await
                });
            }

            while let Some(message) = tasks.join_next().await {
                match message {
                    Ok(Ok(hsm_group_vec)) => {
                        let mut hsm_grop_members = hsm_group_vec
                            .first()
                            .unwrap()
                            .members
                            .as_ref()
                            .unwrap()
                            .ids
                            .clone()
                            .unwrap();

                        hsm_group_member_vec.append(&mut hsm_grop_members);
                    }
                    Ok(Err(error)) => log::warn!("{error}"),
                    Err(error) => {
                        log::warn!("{error}");
                    }
                }
                /* if let Ok(hsm_group_vec) = message {
                    let mut hsm_grop_members = hsm_group_vec
                        .first()
                        .unwrap()
                        .members
                        .as_ref()
                        .unwrap()
                        .ids
                        .clone()
                        .unwrap();

                    hsm_group_member_vec.append(&mut hsm_grop_members);
                } */
            }

            let duration = start.elapsed();
            log::info!("Time elapsed to get HSM members is: {:?}", duration);

            hsm_group_member_vec
        }

        pub fn get_member_vec_from_hsm_group_value_vec(hsm_groups: &[Value]) -> HashSet<String> {
            hsm_groups
                .iter()
                .flat_map(get_member_vec_from_hsm_group_value)
                .collect()
        }

        pub fn get_member_vec_from_hsm_group_vec(hsm_groups: &[HsmGroup]) -> HashSet<String> {
            hsm_groups
                .iter()
                .flat_map(get_member_vec_from_hsm_group)
                .collect()
        }

        /// Returns a Map with nodes and the list of hsm groups that node belongs to.
        /// eg "x1500b5c1n3 --> [ psi-dev, psi-dev_cn ]"
        pub fn group_members_by_hsm_group_from_hsm_groups_value(
            hsm_groups: &Vec<Value>,
        ) -> HashMap<String, Vec<String>> {
            let mut member_hsm_map: HashMap<String, Vec<String>> = HashMap::new();
            for hsm_group_value in hsm_groups {
                let hsm_group_name = hsm_group_value["label"].as_str().unwrap().to_string();
                for member in get_member_vec_from_hsm_group_value(hsm_group_value) {
                    member_hsm_map
                        .entry(member)
                        .and_modify(|hsm_groups| hsm_groups.push(hsm_group_name.clone()))
                        .or_insert_with(|| vec![hsm_group_name.clone()]);
                }
            }

            member_hsm_map
        }

        pub async fn get_member_vec_from_hsm_group_name_opt(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_group: &str,
        ) -> Option<Vec<String>> {
            // Take all nodes for all hsm_groups found and put them in a Vec
            http_client::get(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                Some(&hsm_group.to_string()),
            )
            .await
            .unwrap()
            .first()
            .unwrap()
            .members
            .as_ref()
            .unwrap()
            .ids
            .clone()
        }

        pub async fn get_member_vec_from_hsm_group_name(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_group: &str,
        ) -> Vec<String> {
            // Take all nodes for all hsm_groups found and put them in a Vec
            http_client::get(
                shasta_token,
                shasta_base_url,
                shasta_root_cert,
                Some(&hsm_group.to_string()),
            )
            .await
            .unwrap()
            .first()
            .unwrap()
            .members
            .as_ref()
            .unwrap()
            .ids
            .as_ref()
            .unwrap_or(&Vec::new())
            .clone()
        }

        pub async fn get_hsm_group_from_xname(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname: &String,
        ) -> Option<Vec<String>> {
            let mut hsm_group_vec =
                http_client::get_all(shasta_token, shasta_base_url, shasta_root_cert)
                    .await
                    .unwrap();

            hsm_group_vec.retain(|hsm_group| {
                hsm_group
                    .members
                    .as_ref()
                    .unwrap()
                    .ids
                    .as_ref()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .any(|hsm_group_member| hsm_group_member == xname)
            });

            if hsm_group_vec.is_empty() {
                None
            } else {
                Some(
                    hsm_group_vec
                        .iter()
                        .flat_map(|hsm_group| {
                            hsm_group
                                .members
                                .as_ref()
                                .unwrap()
                                .ids
                                .as_ref()
                                .cloned()
                                .unwrap()
                        })
                        .collect(),
                )
            }
        }

        /// Returns the list of HSM group names related to a list of nodes
        pub async fn get_hsm_group_vec_from_xname_vec(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname_vec: &[&str],
        ) -> Vec<String> {
            let mut hsm_group_vec =
                http_client::get_all(shasta_token, shasta_base_url, shasta_root_cert)
                    .await
                    .unwrap();

            hsm_group_vec.retain(|hsm_group_value| {
                hsm_group_value
                    .members
                    .as_ref()
                    .unwrap()
                    .ids
                    .as_ref()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .any(|hsm_group_member| xname_vec.contains(&hsm_group_member.as_str()))
            });

            hsm_group_vec
                .iter()
                .map(|hsm_group_value| hsm_group_value.label.clone())
                .collect::<Vec<String>>()
        }

        pub fn get_hsm_group_from_cfs_session_related_to_cfs_configuration(
            cfs_session_value_vec: &[Value],
            cfs_configuration: &str,
        ) -> Vec<String> {
            let mut hsm_group_from_cfs_session_vec = cfs_session_value_vec
                .iter()
                .filter(|cfs_session| {
                    cfs_session
                        .pointer("/configuration/name")
                        .unwrap()
                        .eq(cfs_configuration)
                })
                .flat_map(|cfs_session| {
                    cfs_session
                        .pointer("/target/groups")
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|group| group["name"].as_str().unwrap().to_string())
                })
                .collect::<Vec<String>>();

            hsm_group_from_cfs_session_vec.sort();
            hsm_group_from_cfs_session_vec.dedup();

            hsm_group_from_cfs_session_vec
        }

        pub fn get_hsm_group_from_bos_sessiontimplate_related_to_cfs_configuration(
            bos_sessiontemplate_value_vec: &[Value],
            cfs_configuration: &str,
        ) -> Vec<String> {
            let hsm_group_from_bos_sessiontemplate_computer_related_to_cfs_configuration =
                bos_sessiontemplate_value_vec
                    .iter()
                    .filter(|bos_sessiontemplate| {
                        bos_sessiontemplate
                            .pointer("/cfs/configuration")
                            .unwrap()
                            .eq(cfs_configuration)
                    })
                    .flat_map(|bos_sessiontemplate| {
                        bos_sessiontemplate
                            .pointer("/boot_sets/compute/node_groups")
                            .unwrap()
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|node_group| node_group.as_str().unwrap().to_string())
                    });

            let hsm_group_from_bos_sessiontemplate_uan_related_to_cfs_configuration =
                bos_sessiontemplate_value_vec
                    .iter()
                    .filter(|bos_sessiontemplate| {
                        bos_sessiontemplate
                            .pointer("/cfs/configuration")
                            .unwrap()
                            .eq(cfs_configuration)
                            && bos_sessiontemplate
                                .pointer("/boot_sets/uan/node_groups")
                                .is_some()
                    })
                    .flat_map(|bos_sessiontemplate| {
                        bos_sessiontemplate
                            .pointer("/boot_sets/uan/node_groups")
                            .unwrap()
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|node_group| node_group.as_str().unwrap().to_string())
                    });

            let mut hsm_group_from_bos_sessiontemplate_vec =
                hsm_group_from_bos_sessiontemplate_computer_related_to_cfs_configuration
                    .chain(hsm_group_from_bos_sessiontemplate_uan_related_to_cfs_configuration)
                    .collect::<Vec<String>>();

            hsm_group_from_bos_sessiontemplate_vec.sort();
            hsm_group_from_bos_sessiontemplate_vec.dedup();

            hsm_group_from_bos_sessiontemplate_vec
        }

        /// This method will verify the HSM group in user config file and the HSM group the user is
        /// trying to access and it will verify if this access is granted.
        /// config_hsm_group is the HSM group name in manta config file (~/.config/manta/config) and
        /// hsm_group_accessed is the hsm group the user is trying to access (either trying to access a
        /// CFS session or in a SAT file.)
        pub async fn validate_config_hsm_group_and_hsm_group_accessed(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            hsm_group: Option<&String>,
            session_name: Option<&String>,
            cfs_sessions: &[CfsSessionGetResponse],
        ) {
            if let Some(hsm_group_name) = hsm_group {
                let hsm_group_details = crate::hsm::group::http_client::get_hsm_group_vec(
                    shasta_token,
                    shasta_base_url,
                    shasta_root_cert,
                    hsm_group,
                )
                .await
                .unwrap();
                let hsm_group_members = get_member_vec_from_hsm_group_vec(&hsm_group_details);
                let cfs_session_hsm_groups: Vec<String> = cfs_sessions
                    .last()
                    .unwrap()
                    .target
                    .as_ref()
                    .unwrap()
                    .groups
                    .as_ref()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|group| group.name.clone())
                    .collect();
                let cfs_session_members: Vec<String> = cfs_sessions
                    .last()
                    .unwrap()
                    .ansible
                    .as_ref()
                    .unwrap()
                    .limit
                    .clone()
                    .unwrap_or_default()
                    .split(',')
                    .map(|xname| xname.to_string())
                    .collect();
                if !cfs_session_hsm_groups.contains(hsm_group_name)
                    && !cfs_session_members
                        .iter()
                        .all(|cfs_session_member| hsm_group_members.contains(cfs_session_member))
                {
                    println!(
                        "CFS session {} does not apply to HSM group {}",
                        session_name.unwrap(),
                        hsm_group_name
                    );
                    std::process::exit(1);
                }
            }
        }
    }
}

pub mod memberships {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Membership {
        pub id: String,
        #[serde(rename = "partitionName")]
        pub partition_name: String,
        #[serde(rename = "groupLabels")]
        pub group_labels: Vec<String>,
    }

    pub mod http_client {
        use serde_json::Value;

        use crate::error::Error;

        use super::Membership;

        pub async fn get_all(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
        ) -> Result<Vec<Membership>, Error> {
            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                // rest client to authenticate
                client_builder.proxy(socks5proxy).build()?
            } else {
                client_builder.build()?
            };

            let api_url = format!("{}/smd/hsm/v2/memberships", shasta_base_url);

            let response = client
                .get(api_url.clone())
                .header("Authorization", format!("Bearer {}", shasta_token))
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                Ok(response
                    .json::<Vec<Membership>>()
                    .await
                    .map_err(|error| Error::NetError(error))
                    .unwrap())
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))?;
                Err(Error::CsmError(payload))
            }
        }

        pub async fn get_xname(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname: &str,
        ) -> Result<Membership, Error> {
            log::info!("Get membership of node '{}'", xname);
            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                // rest client to authenticate
                client_builder.proxy(socks5proxy).build()?
            } else {
                client_builder.build()?
            };

            let api_url = format!("{}/smd/hsm/v2/memberships/{}", shasta_base_url, xname);

            let response = client
                .get(api_url.clone())
                .header("Authorization", format!("Bearer {}", shasta_token))
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                Ok(response
                    .json::<Membership>()
                    .await
                    .map_err(|error| Error::NetError(error))
                    .unwrap())
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))?;
                Err(Error::CsmError(payload))
            }
        }
    }
}

pub mod component_status {
    pub mod http_client {

        use reqwest::Url;
        use serde_json::Value;

        use crate::error::Error;

        pub async fn get_raw(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname_vec: &[String],
        ) -> Result<Vec<Value>, Error> {
            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                // rest client to authenticate
                client_builder.proxy(socks5proxy).build()?
            } else {
                client_builder.build()?
            };

            let url_params: Vec<_> = xname_vec.iter().map(|xname| ("id", xname)).collect();

            let api_url = Url::parse_with_params(
                &format!("{}/smd/hsm/v2/State/Components", shasta_base_url),
                &url_params,
            )
            .unwrap();

            let response = client
                .get(api_url.clone())
                .header("Authorization", format!("Bearer {}", shasta_token))
                .send()
                .await
                .map_err(|error| Error::NetError(error))?;

            if response.status().is_success() {
                Ok(response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))
                    .unwrap()["Components"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .clone())
            } else {
                let payload = response
                    .json::<Value>()
                    .await
                    .map_err(|error| Error::NetError(error))?;
                Err(Error::CsmError(payload))
            }
        }

        /// Fetches nodes/compnents details using HSM v2 ref --> https://apidocs.svc.cscs.ch/iaas/hardware-state-manager/operation/doComponentsGet/
        pub async fn get(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            xname_vec: &[String],
        ) -> Result<Vec<Value>, Error> {
            let chunk_size = 30;

            let mut hsm_component_status_vec: Vec<Value> = Vec::new();

            let mut tasks = tokio::task::JoinSet::new();

            for sub_node_list in xname_vec.chunks(chunk_size) {
                let shasta_token_string = shasta_token.to_string();
                let shasta_base_url_string = shasta_base_url.to_string();
                let shasta_root_cert_vec = shasta_root_cert.to_vec();

                // let hsm_subgroup_nodes_string: String = sub_node_list.join(",");

                let node_vec = sub_node_list.to_vec();

                tasks.spawn(async move {
                    get_raw(
                        &shasta_token_string,
                        &shasta_base_url_string,
                        &shasta_root_cert_vec,
                        &node_vec,
                    )
                    .await
                    .unwrap()
                });
            }

            while let Some(message) = tasks.join_next().await {
                if let Ok(mut node_status_vec) = message {
                    hsm_component_status_vec.append(&mut node_status_vec);
                }
            }

            Ok(hsm_component_status_vec)
        }
    }
}

pub mod hw_inventory {

    pub mod hw_component {
        pub mod r#struct {
            use serde::{Deserialize, Serialize};
            use serde_json::Value;
            use std::str::FromStr;
            use std::string::ToString;
            use strum_macros::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

            #[derive(
                Debug,
                EnumIter,
                EnumString,
                IntoStaticStr,
                AsRefStr,
                Display,
                Serialize,
                Deserialize,
                Clone,
            )]
            pub enum ArtifactType {
                Memory,
                Processor,
                NodeAccel,
                NodeHsnNic,
                Drive,
                CabinetPDU,
                CabinetPDUPowerConnector,
                CMMRectifier,
                NodeAccelRiser,
                NodeEnclosurePowerSupplie,
                NodeBMC,
                RouterBMC,
            }

            #[derive(Debug, Serialize, Deserialize, Clone)]
            pub struct NodeSummary {
                pub xname: String,
                pub r#type: String,
                pub processors: Vec<ArtifactSummary>,
                pub memory: Vec<ArtifactSummary>,
                pub node_accels: Vec<ArtifactSummary>,
                pub node_hsn_nics: Vec<ArtifactSummary>,
            }

            impl NodeSummary {
                pub fn from_csm_value(hw_artifact_value: Value) -> Self {
                    let processors = hw_artifact_value["Processors"]
                        .as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(|processor_value| {
                            ArtifactSummary::from_processor_value(processor_value.clone())
                        })
                        .collect();

                    let memory = hw_artifact_value["Memory"]
                        .as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(|memory_value| {
                            ArtifactSummary::from_memory_value(memory_value.clone())
                        })
                        .collect();

                    let node_accels = hw_artifact_value["NodeAccels"]
                        .as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(|nodeaccel_value| {
                            ArtifactSummary::from_nodeaccel_value(nodeaccel_value.clone())
                        })
                        .collect();

                    let node_hsn_nics = hw_artifact_value["NodeHsnNics"]
                        .as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(|nodehsnnic_value| {
                            ArtifactSummary::from_nodehsnnics_value(nodehsnnic_value.clone())
                        })
                        .collect();

                    Self {
                        xname: hw_artifact_value["ID"].as_str().unwrap().to_string(),
                        r#type: hw_artifact_value["Type"].as_str().unwrap().to_string(),
                        processors,
                        memory,
                        node_accels,
                        node_hsn_nics,
                    }
                }
            }

            #[derive(Debug, Serialize, Deserialize, Clone)]
            pub struct ArtifactSummary {
                pub xname: String,
                pub r#type: ArtifactType,
                pub info: Option<String>,
            }

            impl ArtifactSummary {
                fn from_processor_value(processor_value: Value) -> Self {
                    Self {
                        xname: processor_value["ID"].as_str().unwrap().to_string(),
                        r#type: ArtifactType::from_str(processor_value["Type"].as_str().unwrap())
                            .unwrap(),
                        info: processor_value
                            .pointer("/PopulatedFRU/ProcessorFRUInfo/Model")
                            .map(|model| model.as_str().unwrap().to_string()),
                    }
                }

                fn from_memory_value(memory_value: Value) -> Self {
                    Self {
                        xname: memory_value["ID"].as_str().unwrap().to_string(),
                        r#type: ArtifactType::from_str(memory_value["Type"].as_str().unwrap())
                            .unwrap(),
                        info: memory_value
                            .pointer("/PopulatedFRU/MemoryFRUInfo/CapacityMiB")
                            .map(|capacity_mib| {
                                capacity_mib.as_number().unwrap().to_string() + " MiB"
                            }),
                    }
                }

                fn from_nodehsnnics_value(nodehsnnic_value: Value) -> Self {
                    Self {
                        xname: nodehsnnic_value["ID"].as_str().unwrap().to_string(),
                        r#type: ArtifactType::from_str(nodehsnnic_value["Type"].as_str().unwrap())
                            .unwrap(),
                        info: nodehsnnic_value
                            .pointer("/NodeHsnNicLocationInfo/Description")
                            .map(|description| description.as_str().unwrap().to_string()),
                    }
                }

                fn from_nodeaccel_value(nodeaccel_value: Value) -> Self {
                    Self {
                        xname: nodeaccel_value["ID"].as_str().unwrap().to_string(),
                        r#type: ArtifactType::from_str(nodeaccel_value["Type"].as_str().unwrap())
                            .unwrap(),
                        info: nodeaccel_value
                            .pointer("/PopulatedFRU/NodeAccelFRUInfo/Model")
                            .map(|model| model.as_str().unwrap().to_string()),
                    }
                }
            }
        }

        pub mod http_client {

            use serde_json::Value;

            use crate::error::Error;

            use super::r#struct::NodeSummary;

            pub async fn get(
                shasta_token: &str,
                shasta_base_url: &str,
                shasta_root_cert: &[u8],
                xname: &str,
            ) -> Result<NodeSummary, Error> {
                let client;

                let client_builder = reqwest::Client::builder()
                    .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

                // Build client
                if std::env::var("SOCKS5").is_ok() {
                    // socks5 proxy
                    log::debug!("SOCKS5 enabled");
                    let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

                    // rest client to authenticate
                    client = client_builder.proxy(socks5proxy).build()?;
                } else {
                    client = client_builder.build()?;
                }

                let api_url = format!(
                    "{}/smd/hsm/v2/Inventory/Hardware/Query/{}",
                    shasta_base_url, xname
                );

                let response = client
                    .get(api_url)
                    .header("Authorization", format!("Bearer {}", shasta_token))
                    .send()
                    .await
                    .map_err(|error| Error::NetError(error))?;

                if response.status().is_success() {
                    let payload = response
                        .json::<Value>()
                        .await
                        .map_err(|error| Error::NetError(error));

                    /* Ok(NodeSummary::from_csm_value(
                        payload.unwrap().pointer("/Nodes/0").unwrap().clone(),
                    )) */

                    match payload.unwrap().pointer("/Nodes/0") {
                        Some(node_value) => Ok(NodeSummary::from_csm_value(node_value.clone())),
                        None => Err(Error::Message(format!(
                            "ERROR - json section '/Node' missing in json response API for node '{}'",
                            xname
                        ))),
                    }
                } else {
                    let payload = response
                        .json::<Value>()
                        .await
                        .map_err(|error| Error::NetError(error))?;

                    Err(Error::CsmError(payload))
                }
            }

            pub async fn get_hw_inventory(
                shasta_token: &str,
                shasta_base_url: &str,
                shasta_root_cert: &[u8],
                xname: &str,
            ) -> Result<Value, Error> {
                let client;

                let client_builder = reqwest::Client::builder()
                    .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

                // Build client
                if std::env::var("SOCKS5").is_ok() {
                    // socks5 proxy
                    log::debug!("SOCKS5 enabled");
                    let socks5proxy = reqwest::Proxy::all(std::env::var("SOCKS5").unwrap())?;

                    // rest client to authenticate
                    client = client_builder.proxy(socks5proxy).build()?;
                } else {
                    client = client_builder.build()?;
                }

                let api_url = format!(
                    "{}/smd/hsm/v2/Inventory/Hardware/Query/{}",
                    shasta_base_url, xname
                );

                let response = client
                    .get(api_url)
                    .header("Authorization", format!("Bearer {}", shasta_token))
                    .send()
                    .await
                    .map_err(|error| Error::NetError(error))?;

                if response.status().is_success() {
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
        }

        pub mod utils {
            use std::collections::HashMap;

            use serde_json::Value;

            use super::r#struct::NodeSummary;

            pub fn get_list_processor_model_from_hw_inventory_value(
                hw_inventory: &Value,
            ) -> Option<Vec<String>> {
                hw_inventory["Nodes"].as_array().unwrap().first().unwrap()["Processors"]
                    .as_array()
                    .map(|processor_list: &Vec<Value>| {
                        processor_list
                            .iter()
                            .map(|processor| {
                                processor
                                    .pointer("/PopulatedFRU/ProcessorFRUInfo/Model")
                                    .unwrap()
                                    .as_str()
                                    .unwrap()
                                    .to_string()
                            })
                            .collect::<Vec<String>>()
                    })
            }

            pub fn get_list_accelerator_model_from_hw_inventory_value(
                hw_inventory: &Value,
            ) -> Option<Vec<String>> {
                hw_inventory["Nodes"].as_array().unwrap().first().unwrap()["NodeAccels"]
                    .as_array()
                    .map(|accelerator_list| {
                        accelerator_list
                            .iter()
                            .map(|accelerator| {
                                accelerator
                                    .pointer("/PopulatedFRU/NodeAccelFRUInfo/Model")
                                    .unwrap()
                                    .as_str()
                                    .unwrap()
                                    .to_string()
                            })
                            .collect::<Vec<String>>()
                    })
            }

            pub fn get_list_hsn_nics_model_from_hw_inventory_value(
                hw_inventory: &Value,
            ) -> Option<Vec<String>> {
                hw_inventory["Nodes"].as_array().unwrap().first().unwrap()["NodeHsnNics"]
                    .as_array()
                    .map(|hsn_nic_list| {
                        hsn_nic_list
                            .iter()
                            .map(|hsn_nic| {
                                hsn_nic
                                    .pointer("/NodeHsnNicLocationInfo/Description")
                                    .unwrap()
                                    .as_str()
                                    .unwrap()
                                    .to_string()
                            })
                            .collect::<Vec<String>>()
                    })
            }

            pub fn get_list_memory_capacity_from_hw_inventory_value(
                hw_inventory: &Value,
            ) -> Option<Vec<u64>> {
                hw_inventory["Nodes"].as_array().unwrap().first().unwrap()["Memory"]
                    .as_array()
                    .map(|memory_list| {
                        memory_list
                            .iter()
                            .map(|memory| {
                                memory
                                    .pointer("/PopulatedFRU/MemoryFRUInfo/CapacityMiB")
                                    .unwrap_or(&serde_json::json!(0))
                                    .as_u64()
                                    .unwrap()
                            })
                            .collect::<Vec<u64>>()
                    })
            }

            pub fn calculate_hsm_hw_component_summary(
                node_summary_vec: &Vec<NodeSummary>,
            ) -> HashMap<String, usize> {
                let mut node_hw_component_summary: HashMap<String, usize> = HashMap::new();

                for node_summary in node_summary_vec {
                    for artifact_summary in &node_summary.processors {
                        node_hw_component_summary
                            .entry(artifact_summary.info.as_ref().unwrap().to_string())
                            .and_modify(|summary_quantity| *summary_quantity += 1)
                            .or_insert(1);
                    }
                    for artifact_summary in &node_summary.node_accels {
                        node_hw_component_summary
                            .entry(artifact_summary.info.as_ref().unwrap().to_string())
                            .and_modify(|summary_quantity| *summary_quantity += 1)
                            .or_insert(1);
                    }
                    for artifact_summary in &node_summary.memory {
                        let memory_capacity = artifact_summary
                            .info
                            .as_ref()
                            .unwrap_or(&"ERROR NA".to_string())
                            .split(' ')
                            .collect::<Vec<_>>()
                            .first()
                            .unwrap()
                            .parse::<usize>()
                            .unwrap_or(0);
                        node_hw_component_summary
                            .entry(artifact_summary.r#type.to_string() + " (GiB)")
                            .and_modify(|summary_quantity| {
                                *summary_quantity += memory_capacity / 1024;
                            })
                            .or_insert(memory_capacity / 1024);
                    }
                    for artifact_summary in &node_summary.node_hsn_nics {
                        node_hw_component_summary
                            .entry(artifact_summary.info.as_ref().unwrap().to_string())
                            .and_modify(|summary_quantity| *summary_quantity += 1)
                            .or_insert(1);
                    }
                }

                node_hw_component_summary
            }
        }
    }

    pub mod ethernet_interfaces {
        use self::r#struct::{ComponentEthernetInterface, IpAddressMapping};

        pub mod r#struct {
            use serde::{Deserialize, Serialize};

            #[derive(Debug, Default, Serialize, Deserialize)]
            pub struct IpAddressMapping {
                pub ip_address: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                pub network: Option<String>,
            }

            #[derive(Debug, Default, Serialize, Deserialize)]
            pub struct ComponentEthernetInterface {
                #[serde(skip_serializing_if = "Option::is_none")]
                pub description: Option<String>,
                pub ip_addresses: Vec<IpAddressMapping>,
                #[serde(skip_serializing_if = "Option::is_none")]
                pub component_id: Option<String>,
            }

            #[derive(Debug, Serialize, Deserialize)]
            pub enum ComponentType {
                CDU,
                CabinetCDU,
                CabinetPDU,
                CabinetPDUOutlet,
                CabinetPDUPowerConnector,
                CabinetPDUController,
                r#Cabinet,
                Chassis,
                ChassisBMC,
                CMMRectifier,
                CMMFpga,
                CEC,
                ComputeModule,
                RouterModule,
                NodeBMC,
                NodeEnclosure,
                NodeEnclosurePowerSupply,
                HSNBoard,
                Node,
                Processor,
                Drive,
                StorageGroup,
                NodeNIC,
                Memory,
                NodeAccel,
                NodeAccelRiser,
                NodeFpga,
                HSNAsic,
                RouterFpga,
                RouterBMC,
                HSNLink,
                HSNConnector,
                INVALID,
            }

            #[derive(Debug, Default, Serialize, Deserialize)]
            pub struct EthernetInterface {
                #[serde(skip_serializing_if = "Option::is_none")]
                id: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                description: Option<String>,
                mac_address: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                ip_address: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                last_update: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                component_id: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                r#type: Option<ComponentType>,
            }
        }

        pub mod http_client {

            // Get list of network interfaces
            // ref --> https://csm12-apidocs.svc.cscs.ch/iaas/hardware-state-manager/operation/doCompEthInterfacesGetV2/
            pub async fn get(
                shasta_token: &str,
                shasta_base_url: &str,
                shasta_root_cert: &[u8],
                mac_address: &str,
                ip_address: &str,
                network: &str,
                component_id: &str, // Node's xname
                r#type: &str,
                olther_than: &str,
                newer_than: &str,
            ) -> Result<reqwest::Response, reqwest::Error> {
                let client_builder = reqwest::Client::builder()
                    .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

                // Build client
                let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                    // socks5 proxy
                    log::debug!("SOCKS5 enabled");
                    let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                    // rest client to authenticate
                    client_builder.proxy(socks5proxy).build()?
                } else {
                    client_builder.build()?
                };

                let api_url: String =
                    shasta_base_url.to_owned() + "/smd/hsm/v2/Inventory/EthernetInterfaces";

                let response_rslt = client
                    .get(api_url)
                    .query(&[
                        ("MACAddress", mac_address),
                        ("IPAddress", ip_address),
                        ("Network", network),
                        ("ComponentID", component_id),
                        ("Type", r#type),
                        ("OlderThan", olther_than),
                        ("NewerThan", newer_than),
                    ])
                    .bearer_auth(shasta_token)
                    .send()
                    .await;

                match response_rslt {
                    Ok(response) => response.error_for_status(),
                    Err(error) => Err(error),
                }
            }
        }

        pub async fn patch(
            shasta_token: &str,
            shasta_base_url: &str,
            shasta_root_cert: &[u8],
            eth_interface_id: &str,
            description: Option<&str>,
            component_id: &str,
            ip_address_mapping: (&str, &str), // [(<ip address>, <network>), ...], examle
                                              // [("192.168.1.10", "HMN"), ...]
        ) -> Result<reqwest::Response, reqwest::Error> {
            let ip_address = ip_address_mapping.0;
            let network = ip_address_mapping.1;
            let cei = ComponentEthernetInterface {
                description: description.map(|value| value.to_string()),
                ip_addresses: vec![IpAddressMapping {
                    ip_address: ip_address.to_string(),
                    network: Some(network.to_string()),
                }],
                component_id: Some(component_id.to_string()),
            };

            let client_builder = reqwest::Client::builder()
                .add_root_certificate(reqwest::Certificate::from_pem(shasta_root_cert)?);

            // Build client
            let client = if let Ok(socks5_env) = std::env::var("SOCKS5") {
                // socks5 proxy
                log::debug!("SOCKS5 enabled");
                let socks5proxy = reqwest::Proxy::all(socks5_env)?;

                // rest client to authenticate
                client_builder.proxy(socks5proxy).build()?
            } else {
                client_builder.build()?
            };

            let api_url: String = format!(
                "{}/smd/hsm/v2/Inventory/EthernetInterfaces/{}",
                shasta_base_url, eth_interface_id
            );

            let response_rslt = client
                .patch(api_url)
                .query(&[("ethInterfaceID", ip_address), ("ipAddress", ip_address)])
                .bearer_auth(shasta_token)
                .json(&cei)
                .send()
                .await;

            match response_rslt {
                Ok(response) => response.error_for_status(),
                Err(error) => Err(error),
            }
        }
    }
}
