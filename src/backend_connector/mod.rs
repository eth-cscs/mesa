//! Backend-dispatcher integration layer.
//!
//! Implements the trait surface defined by
//! [`manta_backend_dispatcher`](https://crates.io/crates/manta-backend-dispatcher)
//! so that csm-rs can be plugged into Manta (or any other dispatcher
//! consumer) as a concrete CSM backend.
//!
//! Each submodule below wires one trait family to the corresponding
//! csm-rs API surface — see the inline annotations for which trait each
//! file implements. The impls live directly on [`crate::ShastaClient`],
//! which carries the connection metadata (base URL, root cert) those
//! impls need; per-request bearer tokens are passed in by the
//! dispatcher.
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

/// Backward-compatibility alias for [`crate::ShastaClient`].
///
/// The dispatcher trait impls used to live on a separate `Csm` wrapper
/// that owned a `ShastaClient` as a field. The two structs had
/// identical connection metadata (base URL, PEM root cert), so the
/// wrapper was redundant. As of v1.0.0-beta.14,
/// every `impl XxxTrait for Csm` block has moved directly onto
/// `ShastaClient`, and this alias is preserved for one release cycle so
/// that downstream code importing `csm_rs::backend_connector::Csm`
/// keeps compiling.
///
/// New code should reach for [`crate::ShastaClient`] directly.
#[deprecated(
  since = "1.0.0-beta.14",
  note = "use `csm_rs::ShastaClient` directly; this alias will be \
          removed in a future release"
)]
pub type Csm = crate::ShastaClient;
