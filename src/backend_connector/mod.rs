//! Backend-dispatcher integration layer.
//!
//! Implements the trait surface defined by
//! [`manta_backend_dispatcher`](https://crates.io/crates/manta-backend-dispatcher)
//! so that csm-rs can be plugged into Manta (or any other dispatcher
//! consumer) as a concrete CSM backend.
//!
//! Each submodule below wires one trait family to the corresponding
//! csm-rs API surface — see the inline annotations for which trait each
//! file implements. The impls live directly on [`Csm`], which carries
//! the connection metadata (base URL, root cert) those impls need;
//! per-request bearer tokens are passed in by the dispatcher.
//!
//! As a rule, dispatcher trait impls call into the domain namespaces
//! (`crate::cfs`, `crate::ims`, `crate::hsm`, ...) rather than into
//! `crate::commands`. The remaining `crate::commands::*` reaches are:
//!
//! - `bos::ApplySessionTrait` → `crate::commands::apply_session` — the
//!   workflow needs a Gitea token + `playbook_yaml_file_name_opt`, so
//!   it is intrinsically command-shaped.
//! - `sat` (gated) → `crate::commands::i_apply_sat_file` /
//!   `crate::commands::apply_hw_cluster_pin` — admin workflows; both
//!   sides ride the same `commands-admin` Cargo feature.
//! - `migrate` (gated) → `crate::commands::migrate_backup` /
//!   `crate::commands::migrate_restore` — admin workflows; deferred
//!   for a follow-up refactor to take `&ShastaClient` and lift the
//!   logic out.
//!
//! Consumers that talk to CSM directly should reach for
//! [`crate::ShastaClient`] instead — this module exists specifically to
//! satisfy the dispatcher contract.

pub mod authentication;
pub mod bos; // ApplySessionTrait, ClusterSessionTrait, ClusterTemplateTrait
pub mod bss; // BootParametersTrait
pub mod cfs; // CfsTrait
pub mod cleanup; // DeleteConfigurationsAndDataRelatedTrait
// `ConsoleTrait` attaches to the in-cluster `cray-console-node` pod
// via `kube`, so it requires the same `k8s-console` feature as the
// underlying console helpers.
#[cfg(feature = "k8s-console")]
pub mod console; // ConsoleTrait
pub mod group; // GroupTrait
pub mod hsm; // HardwareInventory, ComponentTrait, ComponentEthernetInterfaceTrait, RedfishEndpointTrait
pub mod ims; // ImsTrait, GetImagesAndDetailsTrait
// `MigrateRestoreTrait`/`MigrateBackupTrait` and `SatTrait` are
// implemented in terms of the CLI-shaped admin workflows under
// `commands::{migrate_*, i_apply_sat_file}`, so they are gated behind
// the same `commands-admin` feature.
#[cfg(feature = "commands-admin")]
pub mod migrate; // MigrateRestoreTrait, MigrateBackupTrait
pub mod pcs; // PCSTrait
#[cfg(feature = "commands-admin")]
pub mod sat; // SatTrait, ApplyHwClusterPin

/// Connection metadata for one Shasta installation, used by the
/// [`manta_backend_dispatcher`] trait implementations in this module.
///
/// Holds the base URL, PEM root cert, and a pre-built
/// [`crate::ShastaClient`] (constructed once at [`Csm::new`] time and
/// shared across every dispatcher call). Bearer tokens are passed in
/// per request by the dispatcher and are **not** stored.
#[derive(Debug, Clone)]
pub struct Csm {
  pub(crate) base_url: String,
  pub(crate) root_cert: Vec<u8>,
  pub(crate) client: crate::ShastaClient,
}

impl Csm {
  /// Construct a `Csm` from a base URL and a PEM-encoded root cert.
  ///
  /// Builds the underlying `reqwest::Client` (cert parse, connection
  /// pool, DNS resolver, TLS context) once and caches it on
  /// `self.client`; trait-method implementations reuse it across all
  /// calls.
  ///
  /// # Errors
  ///
  /// Returns an error if [`crate::ShastaClient::new`] fails — typically
  /// because the cert bytes are unparseable.
  #[must_use = "constructing a Csm without using it is a no-op"]
  pub fn new(
    base_url: &str,
    root_cert: &[u8],
  ) -> Result<Self, manta_backend_dispatcher::error::Error> {
    let client = crate::ShastaClient::new(base_url, root_cert.to_vec())
      .map_err(manta_backend_dispatcher::error::Error::from)?;
    Ok(Self {
      base_url: base_url.to_string(),
      root_cert: root_cert.to_vec(),
      client,
    })
  }

  /// Borrow the cached [`crate::ShastaClient`].
  pub(crate) fn shasta_client(&self) -> &crate::ShastaClient {
    &self.client
  }
}
