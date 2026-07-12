//! # csm-rs
//!
//! A Rust library for talking to the HPE Cray Shasta CSM (Cray System
//! Management) API.
//!
//! ## Quick start
//!
//! All HTTP calls hang off [`ShastaClient`]. Construct one per Shasta
//! installation and reuse it across calls — it caches a pre-built
//! `reqwest::Client` (with connection pool, TLS context, DNS resolver).
//! The bearer token is supplied per call so a single client can serve
//! many tokens:
//!
//! ```no_run
//! # async fn example() -> Result<(), csm_rs::Error> {
//! let client = csm_rs::ShastaClient::new(
//!     "https://api.shasta.example.com",
//!     std::fs::read("/etc/shasta/ca.crt").unwrap(),
//! )?;
//!
//! let token = "your-bearer-token";
//! // Methods are namespaced by API module: `<module>_<resource>_<verb>`.
//! // The first arg is always the bearer token.
//! let images = client.ims_image_get_all(token).await?;
//! let groups = client.hsm_group_get_all(token).await?;
//! let configs = client.cfs_configuration_v2_get_all(token).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Migrating from earlier releases
//!
//! - **Releases ≤ 0.106**: exposed each HTTP call as a free function with
//!   a 4-parameter auth quartet (`token`, `base_url`, `root_cert`,
//!   `proxy`). Removed in 0.107.
//! - **0.107.x**: replaced the free functions with methods on
//!   [`ShastaClient`]; the token was stored on the client.
//! - **0.108 (this release)**: removed the token from
//!   [`ShastaClient`] — it is now passed per call as the first method
//!   argument. One client can serve many tokens, and the underlying
//!   `reqwest::Client` (with its connection pool) is reused across all
//!   of them.
//! - **1.0.0-beta.20**: `ShastaClient::new` signature changed from 3 to 2
//!   arguments; call sites drop the third positional argument (the removed
//!   proxy parameter).
//!
//! ```ignore
//! // 0.107.x
//! let client = ShastaClient::new(base_url, token, cert, proxy)?;
//! client.ims_image_get_all().await?;
//!
//! // 0.108 – 1.0.0-beta.19
//! let client = ShastaClient::new(base_url, cert, proxy)?;
//! client.ims_image_get_all(token).await?;
//!
//! // 1.0.0-beta.20+
//! let client = ShastaClient::new(base_url, cert)?;
//! client.ims_image_get_all(token).await?;
//! ```
//!
//! ## Source layout
//!
//! Every CSM API namespace under `src/` follows the same shape so
//! reviewers don't have to track per-module conventions:
//!
//! ```text
//! <namespace>/                 // bos, bss, capmc, cfs, hsm, ims, pcs
//!   mod.rs                     // module docs + canonical `pub use` aliases
//!   <resource>/                // e.g. cfs/configuration, bos/session
//!     mod.rs                   // resource docs; declares the items below
//!     types.rs                 // wire-format request/response structs
//!     http_client/             // `impl ShastaClient` HTTP method blocks
//!       mod.rs                 // unversioned methods (or `v{N}/mod.rs` for
//!       v2/mod.rs              // version-split CSM APIs — currently
//!       v3/mod.rs              // `cfs/*` and `bos/{session, template}`)
//!     dispatcher_conv.rs       // `From` impls between local and
//!                              // `manta-backend-dispatcher` types
//!                              // (gated by the `manta-dispatcher` feature)
//!     utils.rs                 // helpers built on the raw HTTP methods
//! ```
//!
//! Higher-level composed operations that combine multiple namespaces
//! live in `commands/`, with the most CLI-shaped ones (file I/O, YAML,
//! progress bars) gated behind the `commands-admin` Cargo feature.

#![allow(clippy::doc_lazy_continuation)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(missing_docs)]
// Promote clippy::pedantic to a warn baseline. The categories below
// are silenced because they generate too much noise for too little
// signal in this crate's shape (lots of moved-by-value tokens,
// many-arg HTTP wrappers, HashMap-on-the-boundary types).
#![warn(clippy::pedantic)]
#![allow(
  clippy::needless_pass_by_value,      // signatures take owned tokens by design
  clippy::implicit_hasher,             // HashMap<_, _> on API boundary is fine
  clippy::too_many_lines,              // long workflow fns are unavoidable
  clippy::cast_precision_loss,         // f32 used for normalized scarcity scores
  clippy::cast_possible_truncation,    // ditto + intentional u8/u16 narrowing
  clippy::cast_sign_loss,              // ditto
  clippy::module_name_repetitions,     // accepted in this crate's layout
  clippy::missing_panics_doc,          // covered by `# Errors` + invariants
  clippy::doc_markdown,                // tolerates CSM/HSM/IMS acronyms
  clippy::struct_excessive_bools,      // a few config structs have several bools
  clippy::fn_params_excessive_bools,   // ditto for some workflow entry points
  clippy::missing_errors_doc,          // already addressed in the doc-hygiene pass
  clippy::redundant_else,              // tolerated in HTTP error-handling shape
  clippy::assigning_clones,            // x = y.clone() reads fine; .clone_from is unidiomatic at call sites
  clippy::unreadable_literal,          // status codes (404 etc.) read better unseparated
)]

/// Backend-dispatcher integration layer. Implements the trait families
/// from the `manta-backend-dispatcher` crate so csm-rs can be plugged
/// into Manta (or any compatible dispatcher consumer) as a CSM backend.
///
/// Requires the `manta-dispatcher` Cargo feature (enabled by default).
/// Direct CSM clients should reach for [`ShastaClient`] instead — this
/// module exists specifically to satisfy the dispatcher contract.
#[cfg(feature = "manta-dispatcher")]
pub mod backend_connector;
pub mod bos;
pub mod bss;
// pub mod capmc;
pub mod cfs;
// `client` and `error` are not `pub mod` — the canonical paths are
// `csm_rs::ShastaClient` and `csm_rs::Error` (re-exports below). The
// modules stay private so a future internal reshuffle doesn't break
// downstream `csm_rs::{client, error}::*` paths.
mod client;
pub mod commands;
pub(crate) mod common;
pub mod error;
pub mod hsm;
pub mod ims;
pub mod node;
pub mod pcs;

pub use client::ShastaClient;
pub use error::Error;

// Canonical type re-exports lifted from each namespace's `mod.rs`. Only
// types that are already curated as the namespace-level canonical name
// are re-exported here; deep `*::http_client::*::types::*` paths stay
// internal so a future version bump is a single edit in the namespace.
pub use bos::{BosSession, BosSessionTemplate};
pub use bss::BootParameters;
// IMS types are prefixed at the lib root to avoid collisions with
// likely-overlapping downstream names (`Link`, `PublicKey`, `PatchImage`).
// `Image` keeps its un-prefixed name as the central IMS type. The deep
// `csm_rs::ims::*` paths still resolve un-prefixed.
pub use ims::{
  Image, Link as ImsLink, PatchImage as ImsPatchImage,
  PublicKey as ImsPublicKey,
};
// CAPMC types stay namespaced under `csm_rs::capmc::*` — they were
// previously lifted to the lib root but CAPMC is not a daily-driver
// namespace, so the un-prefixed lift was an overcorrection.
// CFS exposes both v2 and v3 endpoints with structurally different
// wire types, so the canonical surface is the `cfs::v2` and `cfs::v3`
// submodules rather than a single crate-root alias.
