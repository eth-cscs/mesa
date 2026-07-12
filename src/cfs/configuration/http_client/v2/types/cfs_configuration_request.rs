use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::{
  common::{
    gitea,
    yaml::{as_yaml_str, yaml_seq, yaml_str},
  },
  error::Error,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Layer {
  pub name: String,
  #[serde(rename = "cloneUrl")]
  pub clone_url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  // Either commit or branch is passed
  pub commit: Option<String>,
  pub(super) playbook: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  // Either commit or branch is passed
  pub branch: Option<String>,
  // #[serde(skip_serializing_if = "Option::is_none")]
  // pub tag: Option<String>,
  #[serde(rename = "specialParameters")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub special_parameters: Option<Vec<SpecialParameter>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpecialParameter {
  #[serde(rename = "imsRequiredDkms")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(super) ims_required_dkms: Option<bool>,
}

/// CFS v2 configuration request body.
///
/// The configuration name is **not** part of this body: CFS v2 uses
/// `PUT /cfs/v2/configurations/{name}` and takes only `{ "layers": ... }`
/// in the body. Callsites that need a name pass it as a separate
/// `configuration_name: &str` parameter (see `cfs_configuration_v2_put`,
/// `put_configuration`, and the SAT parser, which returns
/// `(String, CfsConfigurationRequest)`).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CfsConfigurationRequest {
  pub layers: Vec<Layer>,
}

impl Layer {
  pub fn new(
    clone_url: String,
    commit: Option<String>,
    name: String,
    playbook: String,
    branch: Option<String>,
    _tag: Option<String>,
    special_parameters: Option<Vec<SpecialParameter>>,
  ) -> Self {
    Self {
      name,
      clone_url,
      commit,
      playbook,
      branch,
      // tag,
      special_parameters,
    }
  }
}

impl Default for CfsConfigurationRequest {
  fn default() -> Self {
    Self::new()
  }
}

impl CfsConfigurationRequest {
  #[must_use]
  pub fn new() -> Self {
    Self {
      layers: Vec::default(),
    }
  }

  pub fn add_layer(&mut self, layer: Layer) {
    self.layers.push(layer);
  }

  /// Converts a CFS configuration in the SAT file represented as a `serde_yaml::Value` into a
  /// `CfsConfigurationRequest` struct that we can use to create CFS configuration in CSM through its
  /// APIs. This function also resolves the git commit id for git layers in the SAT file if the
  /// user provides a git tag or a branch name instead of a commit id and it also resolves the git
  /// commit id for product layers in the SAT file if the user provides a branch name instead of a
  /// commit id. To resolve the git commit id, this function calls Gitea APIs and for that it needs
  /// Gitea base URL, token and Shasta root certificate to be able to call Gitea APIs in a secure
  /// way from Manta which may run outside the CSM local network.
  /// Returns the CFS configuration name and the `CfsConfigurationRequest` struct created from the
  /// SAT file.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] variant on CSM, transport, or
  /// deserialization failure; see the crate-level `Error` enum
  /// for the full set.
  pub async fn from_sat_file_serde_yaml(
    shasta_root_cert: &[u8],
    gitea_base_url: &str,
    gitea_token: &str,
    configuration_yaml: &serde_yaml::Value,
    cray_product_catalog: &BTreeMap<String, String>,
    site_name: &str,
  ) -> Result<(String, Self), Error> {
    let mut cfs_configuration = Self::new();

    let cfs_configuration_name =
      yaml_str(configuration_yaml, "name")?.to_string();

    for layer_yaml in yaml_seq(configuration_yaml, "layers")? {
      // log::debug!("\n\n### Layer:\n{:#?}\n", layer_json);

      if let Some(git_yaml) = layer_yaml.get("git") {
        // Git layer

        let layer_name = yaml_str(layer_yaml, "name")?.to_string();

        let repo_url = yaml_str(git_yaml, "url")?.to_string();

        let commit_id_value_opt =
          layer_yaml.get("git").and_then(|git| git.get("commit"));
        let tag_value_opt =
          layer_yaml.get("git").and_then(|git| git.get("tag"));
        let branch_value_opt =
          layer_yaml.get("git").and_then(|git| git.get("branch"));

        let commit_id_opt: Option<String> = if commit_id_value_opt.is_some() {
          // Git commit id
          layer_yaml
            .get("git")
            .and_then(|git| git.get("commit"))
            .and_then(Value::as_str)
            .map(str::to_string)
        } else if let Some(git_tag_value) = tag_value_opt {
          // Git tag
          let git_tag = as_yaml_str(git_tag_value)?;

          log::debug!("git tag: {git_tag}");

          let tag_details_rslt = gitea::http_client::get_tag_details(
            &repo_url,
            git_tag,
            gitea_token,
            shasta_root_cert,
            site_name,
          )
          .await;

          let tag_details = if let Ok(tag_details) = tag_details_rslt {
            log::debug!("tag details:\n{tag_details:#?}");
            tag_details
          } else {
            return Err(Error::Message(format!(
              "ERROR - Could not get details for git tag '{git_tag}' in CFS configuration '{cfs_configuration_name}'. Reason:\n{tag_details_rslt:#?}"
            )));
          };

          // Assuming user sets an existing tag name. It could be an annotated tag
          // (different object than the commit id with its own sha value) or a
          // lightweight tag (pointer to commit id, therefore the tag will have the
          // same sha as the commit id it points to), either way CFS session will
          // do a `git checkout` to the sha we found here, if an annotated tag, then,
          // git is clever enough to take us to the final commit id, if it is a
          // lightweight tag, then there is no problem because the sha is the same
          // as the commit id
          // NOTE: the `id` field is the tag's sha, note we are not taking the commit id
          // the tag points to and we should not use sha because otherwise we won't be
          // able to fetch the annotated tag using a commit sha through the Gitea APIs
          tag_details
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
        } else if let Some(branch_value) = branch_value_opt {
          // Branch name
          let branch_name = as_yaml_str(branch_value)?;
          Some(
            gitea::http_client::get_commit_pointed_by_branch(
              gitea_base_url,
              gitea_token,
              shasta_root_cert,
              &repo_url,
              branch_name,
            )
            .await?,
          )
        } else {
          // This should be an error but we will let CSM to handle this
          None
        };

        // IMPORTANT: CSM won't allow CFS configuration layers with both commit id and
        // branch name, therefore, we will set branch name to None if we already have a
        // commit id
        let branch_name = if commit_id_opt.is_some() {
          None
        } else {
          branch_value_opt.and_then(Value::as_str).map(str::to_string)
        };

        let layer = Layer::new(
          repo_url,
          commit_id_opt,
          layer_name,
          layer_yaml
            .get("playbook")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_default(),
          branch_name,
          None,
          None,
        );
        cfs_configuration.add_layer(layer);
      } else if let Some(product_yaml) = layer_yaml.get("product") {
        // Product layer

        let product_name = yaml_str(product_yaml, "name")?;
        let product_version = yaml_str(product_yaml, "version")?;
        let product_branch_value_opt = product_yaml.get("branch");

        let product =
          cray_product_catalog.get(product_name).ok_or_else(|| {
            Error::CrayProductCatalog(format!(
              "Product {product_name} not found in cray product catalog"
            ))
          })?;

        let cos_cray_product_catalog = serde_yaml::from_str::<Value>(product)?;

        let product_details = cos_cray_product_catalog
          .get(product_version)
          .and_then(|product| product.get("configuration"))
          .cloned()
          .ok_or_else(|| {
            Error::CrayProductCatalog(format!(
              "Product details for product name '{product_name}', product_version '{product_version}' and 'configuration' not found in cray product catalog"
            ))
          })?;

        log::debug!(
          "CRAY product catalog details for product: {product_name}, version: {product_version}:\n{product_details:#?}"
        );

        // Manta may run outside the CSM local network therefore we have to change the
        // internal URLs for the external one
        let repo_url = yaml_str(&product_details, "clone_url")?.replace(
          format!("vcs.cmn.{site_name}.cscs.ch").as_str(),
          crate::common::gitea::INTERNAL_API_HOST,
        );

        let commit_id_opt = if let Some(branch_value) = product_branch_value_opt
        {
          // If branch is provided, then ignore the commit id in the CRAY products table
          let branch_name = as_yaml_str(branch_value)?;
          Some(
            gitea::http_client::get_commit_pointed_by_branch(
              gitea_base_url,
              gitea_token,
              shasta_root_cert,
              &repo_url,
              branch_name,
            )
            .await?,
          )
        } else {
          product_details
            .get("commit")
            .and_then(Value::as_str)
            .map(str::to_string)
        };

        // IMPORTANT: CSM won't allow CFS configuration layers with both commit id and
        // branch name, therefore, we will set branch name to None if we already have a
        // commit id
        let branch_name = if commit_id_opt.is_some() {
          None
        } else {
          product_branch_value_opt
            .and_then(Value::as_str)
            .map(str::to_string)
        };

        // Create CFS configuration layer struct
        let layer = Layer::new(
          repo_url,
          commit_id_opt,
          product_name.to_string(),
          yaml_str(layer_yaml, "playbook")?.to_string(),
          branch_name,
          None,
          None,
        );
        cfs_configuration.add_layer(layer);
      } else {
        return Err(Error::Message("configurations section in SAT file error - CFS configuration layer error".to_string()));
      }
    }

    Ok((cfs_configuration_name, cfs_configuration))
  }
}
