# csm-rs

A Rust library for interacting with the HPE Cray Shasta CSM (Cray System
Management) API.

`csm-rs` (formerly *Mesa*) is the foundation used by applications like
[Manta](https://github.com/eth-cscs/manta) to integrate with Shasta-based
systems. It avoids `unsafe` code and aims to provide a safe, ergonomic
async interface to the CSM control plane.

Typical use cases:

- Building applications that integrate Shasta/CSM systems into your
  ecosystem.
- Simplifying or scripting common CSM operations.
- Extending CSM functionality beyond what the official CLIs expose.

## Supported APIs

The crate currently wraps the following CSM components:

- HSM (Hardware State Manager) — `hsm`
- CFS (Configuration Framework Service) — configurations & sessions
- BOS (Boot Orchestration Service) — `bos`
- BSS (Boot Script Service) — `bss`
- CAPMC (Cray Advanced Platform Monitoring and Control) — `capmc`
- IMS (Image Management Service) — `ims`
- PCS (Power Control Service) — `pcs`
- Node operations — `node`
- Kubernetes & Keycloak helpers — `common`

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
csm-rs = "0.108"
tokio = { version = "1", features = ["full"] }
```

## Quick start

All HTTP calls are exposed as methods on [`ShastaClient`]. Construct one
per Shasta installation and reuse it — it caches a pre-built
`reqwest::Client` (connection pool, TLS context, DNS resolver). The
bearer token is supplied per call, so one client can serve many tokens:

```rust,no_run
use csm_rs::ShastaClient;

#[tokio::main]
async fn main() -> Result<(), csm_rs::error::Error> {
    let client = ShastaClient::new(
        "https://api.shasta.example.com",
        std::fs::read("/etc/shasta/ca.crt").unwrap()
    )?;

    let token = "your-bearer-token";

    // Methods are namespaced by API module: `<module>_<resource>_<verb>`.
    // The first argument is always the bearer token.
    let images  = client.ims_image_get_all(token).await?;
    let groups  = client.hsm_group_get_all(token).await?;
    let configs = client.cfs_configuration_v2_get_all(token).await?;

    Ok(())
}
```

Method names are versioned where the underlying API is — e.g.
`cfs_session_v2_get` vs `cfs_session_v3_get`. See each module's rustdoc
for the full list.

## Examples

Runnable programs under [`examples/`](examples/):

- [`list_hsm_groups`](examples/list_hsm_groups.rs) — minimal client
  construction plus one GET.
- [`list_cfs_sessions`](examples/list_cfs_sessions.rs) — paginated CFS
  v3 session listing.
- [`power_cycle_nodes`](examples/power_cycle_nodes.rs) — PCS transition
  with synchronous wait.

Each reads `CSM_BASE_URL`, `CSM_TOKEN`, and `CSM_ROOT_CERT_PATH` from
the environment. Run with `cargo run --example <name>`.

## Migrating between releases

- **≤ 0.106**: exposed each HTTP call as a free function taking a
  4-parameter auth quartet (`token`, `base_url`, `root_cert`, `proxy`).
  Removed in 0.107.
- **0.107.x**: free functions replaced by methods on [`ShastaClient`];
  the token was stored on the client.
- **0.108**: the token was removed from [`ShastaClient`] —
  it is now passed per call as the method's first argument. One client
  can serve many tokens; the underlying `reqwest::Client` (and its
  connection pool) is reused across all of them.

```rust,ignore
// 0.107.x
let client = ShastaClient::new(base_url, token, cert, proxy)?;
client.ims_image_get_all().await?;

// 0.108 – 1.0.0-beta.19
let client = ShastaClient::new(base_url, cert, proxy)?;
client.ims_image_get_all(token).await?;
```

- **1.0.0-beta.20**: `ShastaClient::new` no longer accepts a `socks5_proxy` argument; call sites drop the third positional argument.

- **1.0.0-beta (current)**: HSM, CFS, BSS, BOS, and PCS are now
  generated from the upstream OpenAPI specs via
  [`progenitor`](https://crates.io/crates/progenitor) at build time. A
  thin wrapper layer at `src/<module>/wrapper/` preserves the historical
  `<module>_<resource>_<verb>` method names, but a handful of return
  types tightened where the hand-written shapes had silently diverged
  from the spec:

  - `pcs_power_cap_get` returns `PowerCapTaskList` (was wrongly typed
    as a single `PowerCapTaskInfo`; the endpoint always returned a
    list).
  - `pcs_power_cap_get_task_id` returns `PowerCapsRetdata`.
  - `pcs_power_cap_post_snapshot` and `pcs_power_cap_patch` return
    `OpTaskStartResponse` (the `{taskID: …}` envelope) instead of the
    full `PowerCapTaskInfo`.
  - `pcs_power_cap_patch` correctly issues `PATCH /power-cap` (the
    previous code sent `PUT /power-cap/snapshot`).
  - Several HSM types adopted spec-conformant casing (`MACAddress`,
    `IPAddresses`, `RediscoverOnUpdate`); fields like
    `EthernetInterface.ip_addresses` are now `Vec<IpAddressMapping>`
    instead of the broken singular `ip_address: Option<String>`.
  - HSM `Group.label`, `Group.exclusive_group`, and `Members.ids`
    became newtypes (`ResourceName`, `XNameRw100`); access the inner
    `String` via `.0` or `Deref`.

  No `ShastaClient::*` method signature changed beyond return-type
  swaps; per-call signatures are preserved. The CapMC module is
  currently disabled in `lib.rs` while it waits for its own migration
  pass.

## Notes

`hyper 0.14` remains as a transitive dependency via `aws-smithy-http-client 1.1.13` → `hyper-rustls 0.24.2`. It will drop once `aws-smithy-http-client` migrates to `hyper 1.x`. csm-rs itself no longer depends on `hyper 0.14` directly.

## Building & testing

Build:

```sh
cargo build
```

Run the test suite (some tests require access to a live Shasta backend
and are gated accordingly):

```sh
cargo test -- --show-output
```

Generate API documentation locally:

```sh
cargo doc --open
```

## Release

Releases are cut with [`cargo-release`](https://github.com/crate-ci/cargo-release):

```sh
cargo release patch --execute
```

## Security advisories

`cargo audit` currently reports three advisories against
`rustls-webpki 0.101.7`, all pulled in transitively through the AWS
SDK chain:

- [RUSTSEC-2026-0098](https://rustsec.org/advisories/RUSTSEC-2026-0098)
  — name constraints for URI names were incorrectly accepted.
- [RUSTSEC-2026-0099](https://rustsec.org/advisories/RUSTSEC-2026-0099)
  — name constraints accepted for certificates asserting a wildcard
  name.
- [RUSTSEC-2026-0104](https://rustsec.org/advisories/RUSTSEC-2026-0104)
  — reachable panic in certificate revocation list parsing.

Dependency chain:

```
csm-rs
  └─ aws-smithy-runtime
       └─ aws-smithy-http-client 1.1.12
            └─ hyper-rustls 0.24.2
                 └─ rustls 0.21.12
                      └─ rustls-webpki 0.101.7   ← vulnerable
```


### Severity assessment for csm-rs

These advisories are reachable only through TLS validation performed
by the IMS S3 client (`src/ims/s3_client.rs`). In the standard
Manta-style deployment, the S3 endpoint is on the CSM-internal
network with a CSM-provisioned certificate chain, so the
attacker-controlled-cert preconditions are weak. Downstream users
should still make their own call.

### Workaround

Build without the `ims-s3` feature, which removes the AWS SDK
entirely:

```toml
[dependencies]
csm-rs = { version = "1.0.0-beta", default-features = false, features = ["manta-dispatcher", "k8s-console"] }
```

The advisories are allowlisted in `.cargo/audit.toml` with a comment
linking back here, so CI `cargo audit` runs stay actionable. Delete
the three IDs from that file once the AWS chain ships a fix.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under the terms of the [LICENSE](LICENSE) file in the repository
root.
