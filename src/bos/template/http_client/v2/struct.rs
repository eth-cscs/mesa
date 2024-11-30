use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Link {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cfs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<String>,
    /* #[serde(skip_serializing_if = "Option::is_none")]
    pub clone_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playbook: Option<String>, */
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BootSet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfs: Option<Cfs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_parameters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_list: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_roles_groups: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_groups: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>, // TODO: use Arch enum instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs_provider_passthrough: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub boot_ordinal: Option<u64>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub shutdown_ordinal: Option<u64>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub network: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub property: Option<String>,
}

// TODO: use strum crate to implement functions to convert to/from String
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Arch {
    X86,
    ARM,
    Other,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BosSessionTemplate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_cfs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfs: Option<Cfs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_sets: Option<HashMap<String, BootSet>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<Link>>,
    // #[serde(rename = "templateUrl")]
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub template_url: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub cfs_url: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub cfs_branch: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub partition: Option<String>,
}

impl BosSessionTemplate {
    pub fn get_target(&self) -> Vec<String> {
        let target_hsm = self.get_target_hsm();
        let target_xname = self.get_target_xname();

        [target_hsm, target_xname].concat()
    }

    /// Returns HSM group names related to the BOS sessiontemplate
    pub fn get_target_hsm(&self) -> Vec<String> {
        self.boot_sets
            .as_ref()
            .unwrap()
            .iter()
            .flat_map(|(_, boot_param)| boot_param.node_groups.clone().unwrap_or(Vec::new()))
            .collect()
    }

    pub fn get_target_xname(&self) -> Vec<String> {
        self.boot_sets
            .as_ref()
            .unwrap()
            .iter()
            .flat_map(|(_, boot_param)| boot_param.node_list.clone().unwrap_or(Vec::new()))
            .collect()
    }

    pub fn get_confguration(&self) -> Option<String> {
        self.cfs.as_ref().and_then(|cfs| cfs.configuration.clone())
    }

    pub fn get_path_vec(&self) -> Vec<String> {
        self.boot_sets
            .as_ref()
            .unwrap()
            .iter()
            .map(|(_, boot_param)| boot_param.path.clone().unwrap_or_default())
            .collect()
    }

    /// Returns all images related to this BOS sessiontemplate
    pub fn get_image_vec(&self) -> Vec<String> {
        // FIXME: Try to use a function in mesa boot parameters to get the image id from a
        // CSM/S3 URI instead of calling
        // tim_start_matches("s3://boot-images").trim_end_matches("/manifest.json")?
        self.boot_sets
            .as_ref()
            .unwrap()
            .iter()
            .map(|(_, boot_param)| {
                boot_param
                    .path
                    .clone()
                    .unwrap_or_default()
                    .trim_start_matches("s3://boot-images/")
                    .trim_end_matches("/manifest.json")
                    .to_string()
            })
            .collect()
    }

    pub fn new_for_hsm_group(
        tenant_opt: Option<String>,
        cfs_configuration_name: String,
        bos_session_template_name: String,
        ims_image_name: String,
        ims_image_path: String,
        ims_image_type: String,
        ims_image_etag: String,
        hsm_group: String,
        kernel_params: String,
        arch_opt: Option<String>,
    ) -> Self {
        let cfs = Cfs {
            configuration: Some(cfs_configuration_name),
        };

        let boot_set = BootSet {
            name: Some(ims_image_name),
            // boot_ordinal: Some(2),
            // shutdown_ordinal: None,
            path: Some(ims_image_path),
            r#type: Some(ims_image_type.clone()),
            etag: Some(ims_image_etag),
            kernel_parameters: Some(kernel_params),
            // kernel_parameters: Some(
            //     "ip=dhcp quiet ksocklnd.skip_mr_route_setup=1 cxi_core.disable_default_svc=0 cxi_core.enable_fgfc=1 cxi_core.disable_default_svc=0 cxi_core.sct_pid_mask=0xf spire_join_token=${SPIRE_JOIN_TOKEN}".to_string(),
            // ),
            // network: Some("nmn".to_string()),
            node_list: None,
            node_roles_groups: None,
            node_groups: Some(vec![hsm_group]),
            rootfs_provider: Some("cpss3".to_string()),
            rootfs_provider_passthrough: Some("dvs:api-gw-service-nmn.local:300:nmn0".to_string()),
            cfs: Some(cfs.clone()),
            arch: arch_opt,
        };

        let mut boot_set_map = HashMap::<String, BootSet>::new();

        boot_set_map.insert(ims_image_type, boot_set);

        BosSessionTemplate {
            name: Some(bos_session_template_name),
            // template_url: None,
            description: None,
            // cfs_url: None,
            // cfs_branch: None,
            enable_cfs: Some(true),
            cfs: Some(cfs),
            // partition: None,
            boot_sets: Some(boot_set_map),
            links: None,
            tenant: tenant_opt,
        }
    }
}

