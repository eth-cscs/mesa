# Remove SOCKS5 proxy support from csm-rs and ochami-rs

**Date:** 2026-07-12
**Status:** Design — approved for planning
**Applies to:** `csm-rs`, `ochami-rs`
**Depends on:** `manta-backend-dispatcher` trait change (see Section 4)

## 1. Motivation

Both `csm-rs` and `ochami-rs` are HPC-cluster management client libraries. They only communicate with CSM / OpenCHAMI backends that live inside a trusted management network where all necessary routes are already open. A SOCKS5 tunnel from the client has no operational role in these deployments.

Keeping SOCKS5 alive has a concrete cost. In `csm-rs`, the IMS S3 client wires SOCKS5 into the AWS SDK via `hyper_socks2::SocksConnector` + `hyper::client::HttpConnector` + `aws_smithy_runtime`'s `hyper_014::HyperClientBuilder`. That connector chain pins the S3 code path to hyper 0.14 and was about to force a nontrivial hyper 1.x migration (see the deleted `docs/superpowers/{specs,plans}/2026-07-12-s3-socks5-hyper1-migration-*` docs). Removing SOCKS5 removes the reason for that migration and shrinks the transitive dependency graph across both crates.

## 2. Goals and non-goals

**Goals**

- Delete every SOCKS5 code path, parameter, struct field, and dependency from both crates.
- Rewrite `csm-rs`'s IMS S3 client on the AWS SDK's default rustls HTTP client so the hyper-0.14 wiring can be removed entirely.
- Coordinate the removal across `manta-backend-dispatcher`, `csm-rs`, and `ochami-rs` so the traits and their impls stay in sync.

**Non-goals**

- Not migrating `csm-rs`'s IMS S3 client to `aws-sdk-s3`-native operation beyond dropping the custom HTTP client wiring.
- Not changing TLS trust behavior for any code path (see Section 3.4).
- Not updating the `manta` CLI in this spec; the CLI drops its `--socks5-proxy` flag as a follow-up once both client crates have released the breaking change.
- Not renaming or refactoring unrelated code encountered during the sweep.

## 3. Design

### 3.1 Overall approach

- **Hard-break the public API in both client crates.** Every function signature and struct field that carries a `socks5_proxy: Option<&str>` / `Option<String>` is removed. Consumers must update.
- **Version bumps.** Both crates are on `1.0.0-beta.*`, so a beta bump is sufficient — no formal semver-major bump is required. `csm-rs` releases `1.0.0-beta.20`; `ochami-rs` bumps its next beta at the tail of the sweep.
- **Cross-repo coordination.** The `manta-backend-dispatcher` trait change lands first and is the single upstream prerequisite for both client sweeps. Once the dispatcher release is out, `csm-rs` and `ochami-rs` sweeps are independent and can proceed in parallel.

### 3.2 csm-rs — Cargo.toml changes

| Line | Before | After |
|---|---|---|
| `reqwest` features | `["blocking", "json", "rustls-tls", "socks"]` | `["blocking", "json", "rustls-tls"]` |
| `kube` features | `["ws", "socks5", "runtime"]` | `["ws", "runtime"]` |
| `hyper-socks2 = { version = "0.8.0", … }` | present, optional | **deleted** |
| `hyper = { version = "0.14", optional = true }` | present | **deleted** |
| `ims-s3` feature deps | includes `"dep:hyper", "dep:hyper-socks2"` | those two entries removed |
| Doc comments on `ims-s3` feature and the two hyper deps | describe the "SOCKS5 / hyper-0.14 glue" | rewritten to reflect the reqwest-only, hyper-1.x-only story |

`Cargo.lock` loses two entries transitively: `hyper-socks2` and the hyper 0.14.x tree.

### 3.3 csm-rs — public API surface

**`src/client.rs`**

- Remove struct field `pub(crate) socks5_proxy: Option<String>`.
- Change `Client::new(base_url, root_cert, socks5_proxy)` to `Client::new(base_url, root_cert)`; drop the corresponding assignment.
- Remove getter `pub fn socks5_proxy(&self) -> Option<&str>`.
- Update the `http::build_client(&root_cert, socks5_proxy.as_deref())` call inside `new()` to `http::build_client(&root_cert)`.
- Delete tests `new_with_socks5_proxy_succeeds` and the `assert!(client.socks5_proxy().is_none())` / clone-equality assertions on the field.

**`src/common/http.rs`**

- Remove `socks5_proxy: Option<&str>` from `build_client` and `build_client_with_auth`.
- Drop the `match socks5_proxy { Some(url) => builder.proxy(reqwest::Proxy::all(url)?), None => builder }` branch in both.
- Delete test `build_client_with_socks5_proxy_succeeds`.

### 3.4 csm-rs — IMS S3 client rewrite (`src/ims/s3_client.rs`)

Every public function loses the `socks5_proxy: Option<&str>` argument: `s3_auth`, `s3_get_object_size`, `s3_download_object`, `s3_upload_object`, `s3_remove_object`, `s3_multipart_upload_object`.

`setup_client(sts_value, socks5_proxy)` becomes `setup_client(sts_value)` and the body simplifies to:

```rust
async fn setup_client(sts_value: &Value) -> Result<Client, Error> {
    let (credentials, endpoint_url) = parse_sts_credentials(sts_value)?;

    let region_provider = aws_config::meta::region::RegionProviderChain::default_provider()
        .or_else("us-east-1");
    let app_name = aws_config::AppName::new("manta")
        .map_err(|e| Error::S3Transport(format!("Error setting app name: {e}")))?;
    let timeout_config = TimeoutConfig::builder()
        .operation_timeout(S3_OPERATION_TIMEOUT)
        .build();

    let config: SdkConfig = aws_config::from_env()
        .region(region_provider)
        .endpoint_url(endpoint_url)
        .app_name(app_name)
        .credentials_provider(credentials)
        .timeout_config(timeout_config)
        .load()
        .await;

    Ok(aws_sdk_s3::Client::from_conf(
        aws_sdk_s3::Client::new(&config)
            .config()
            .to_builder()
            .force_path_style(true)
            .build(),
    ))
}
```

Removed imports at the top of the file: `aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder`, `hyper::client::HttpConnector`, `hyper_socks2::SocksConnector`, `hyper::Uri`. `aws-sdk-s3`'s default rustls HTTP client (already enabled via the crate's `rustls` feature) takes over — no custom HTTP-client wiring at all.

**TLS trust anchor — preserved behavior.** The current S3 code never passes the CSM root cert into the AWS SDK client; the `shasta_root_cert` argument on `s3_auth` is only used for the reqwest STS call, and the AWS SDK talks to S3 using whatever the platform trust store contains. This spec preserves that behavior verbatim. If a deployment's platform trust store isn't sufficient, that is a pre-existing gap that predates this change and is out of scope.

### 3.5 csm-rs — call-site sweep

`socks5_proxy: Option<&str>` is threaded through ~40 files. The sweep is mechanical: remove the parameter from every function signature and remove the corresponding argument (`socks5_proxy`, `socks5_proxy.as_deref()`, `client.socks5_proxy()`, `self.client.socks5_proxy()`) at every call site.

The work groups naturally into the following review chunks, listed in the order the spec expects the plan to walk through them. The exact landing sequence — i.e. which of these are individual commits vs. rolled together, and how to keep the workspace compiling in intermediate states — is a plan-level decision, not a spec-level one:

1. `common::http` + `client` (foundation).
2. `common::{authentication, gitea, vault, kubernetes}`.
3. `ims::*` (S3 client + callers).
4. `hsm::*`.
5. `cfs::*`.
6. `bos::*`, `bss::*`, `pcs::*`, `node::*`.
7. `backend_connector::*` (dispatcher-trait impls; requires the new dispatcher release pinned in `Cargo.toml`).
8. `commands::*` including `migrate_backup`, `migrate_restore`, `apply_session`, `apply_hw_cluster_pin`, `i_apply_sat_file`.

### 3.6 ochami-rs — Cargo.toml changes

| Line | Before | After |
|---|---|---|
| `reqwest` features | `["blocking", "json", "rustls-tls", "socks"]` | `["blocking", "json", "rustls-tls"]` |

That is the entire SOCKS5 dependency footprint in `ochami-rs` — no hyper 0.14, no hyper-socks2, no kube feature. The rest of the removal is purely code-level.

### 3.7 ochami-rs — public API surface

- **`src/http.rs`** — two builder functions each carry a `socks5_proxy: Option<&str>` param and a `match socks5_proxy { … }` branch. Remove both parameters and both branches.
- **`src/backend_connector.rs`** — the connector struct holds `socks5_proxy: Option<String>`; its constructor takes `socks5_proxy: Option<&str>`; ~30 trait method bodies call `self.socks5_proxy.as_deref()` on their way into module helpers. Remove the field, the constructor parameter, and every `.as_deref()` argument.
- The `http_client.rs` files under `src/hsm/`, `src/bss/`, `src/pcs/`, and `src/node/` (per the ripgrep census in Section 5), plus `src/hsm/group/utils.rs` and `src/node/utils.rs` — each function loses its `socks5_proxy: Option<&str>` parameter, and the `build_client(root_cert, socks5_proxy)` call becomes `build_client(root_cert)`.

Roughly 20 files, all mechanical.

### 3.8 Shared upstream — `manta-backend-dispatcher` trait change

The dispatcher traits currently declare `socks5_proxy: Option<&str>` on the methods that `csm-rs` and `ochami-rs` implement. That parameter is removed from every affected trait method. `manta-backend-dispatcher` cuts a beta release (target: `1.0.0-beta.14`, exact number determined by the state of that repo when the work starts). Both client crates then pin the new dispatcher release and their `backend_connector` impls compile clean against the new trait shape.

### 3.9 Documentation updates

- **`csm-rs/README.md`** — 7 SOCKS5 references (examples, feature list, deployment notes). Update example invocations, delete the "SOCKS5 tunnel from your workstation" paragraph, and update the feature-flag documentation for `ims-s3`.
- **`csm-rs/Cargo.toml`** doc comments — rewrite the `ims-s3` feature block and delete the block explaining the hyper-0.14 / hyper-socks2 glue.
- **`ochami-rs/README.md`** — remove any SOCKS5 references (grep during implementation).

## 4. Cross-repo ordering

1. **manta-backend-dispatcher** — drop `socks5_proxy` from every trait method; release (e.g. `1.0.0-beta.14`).
2. **csm-rs** and **ochami-rs** — sweep in parallel on `feat/remove-socks5` branches. Each pins the new dispatcher release, does its own module-by-module commits, verifies its own ripgrep gate, and cuts its own release (`csm-rs 1.0.0-beta.20`; ochami-rs beta bump tracked in that repo).
3. **manta CLI** — bumps both client dependencies in a single follow-up PR and drops any `--socks5-proxy` CLI flag. Not covered by this spec.

If the dispatcher release lags during development, either client crate can use a `path = "…"` or `branch = "…"` dep on `manta-backend-dispatcher` in its `Cargo.toml` until step 1 lands, then switch back to a pinned version before releasing.

## 5. Verification

For each crate, the following must pass on the release commit:

- `cargo check --all-features` (for csm-rs; `ochami-rs` has no feature gates to worry about).
- `cargo test --all-features`.
- `cargo clippy --all-features -- -D warnings`.
- Ripgrep gate — **zero hits** from:
  - `rg -i "socks|hyper_socks2|hyper::client::HttpConnector" csm-rs/src/ csm-rs/Cargo.toml csm-rs/README.md`
  - `rg -i "socks" ochami-rs/src/ ochami-rs/Cargo.toml ochami-rs/README.md`

The ripgrep gate is what proves the removal is complete; the implementation plan must include it as a mandatory step before opening the release PR.

## 6. Risks and rollback

- **Any consumer actually running through a SOCKS5 tunnel loses connectivity.** Per the premise of this work, no such consumer exists — both crates are deployed inside the trusted management network. This is called out in the release notes so anyone reading them understands the intent.
- **S3 TLS trust anchor** — unchanged from current behavior; see Section 3.4. Not a regression introduced by this change.
- **Rollback** — `git revert` of the branch merge commit is clean in each repo. No schema or data changes are involved. Downstream consumers can stay on the older betas until they choose to upgrade.

## 7. Release notes (drafts)

### csm-rs 1.0.0-beta.20

- **Breaking:** removed SOCKS5 proxy support. `Client::new` no longer takes a `socks5_proxy` argument, and every public function in `common::http`, `ims::s3_client`, and all callers loses its `socks5_proxy` parameter.
- Dropped `hyper 0.14` and `hyper-socks2` dependencies; the IMS S3 client now uses the AWS SDK's default rustls HTTP client.
- Dropped `reqwest`'s `socks` feature and `kube`'s `socks5` feature.
- Requires `manta-backend-dispatcher` ≥ the release that removes the proxy parameter from its trait methods.

### ochami-rs (next beta)

- **Breaking:** removed SOCKS5 proxy support. The connector constructor and every public function in `http.rs` and the various `http_client.rs` modules lose their `socks5_proxy` parameter.
- Dropped `reqwest`'s `socks` feature.
- Requires `manta-backend-dispatcher` ≥ the release that removes the proxy parameter from its trait methods.

## 8. Spec locations

Canonical copy: `csm-rs/docs/superpowers/specs/2026-07-12-remove-socks5-design.md` (this file).
Mirror copy: `ochami-rs/docs/superpowers/specs/2026-07-12-remove-socks5-design.md`, byte-identical. Any future edits happen in both places. The `ochami-rs/docs/superpowers/specs/` directory is created by the commit that adds the mirror.
