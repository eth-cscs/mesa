# Remove SOCKS5 Proxy Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete every SOCKS5 code path, parameter, struct field, and dependency from `csm-rs` and `ochami-rs`, plus the transitive `reqwest` `socks` feature flag in `manta-backend-dispatcher`. Rewrite `csm-rs`'s IMS S3 client on the AWS SDK's default rustls HTTP client so the hyper-0.14 wiring can go too.

**Architecture:** Three repos, all independent — the dispatcher's public traits do not carry `socks5_proxy` (verified during planning), so no compile-time coordination is needed. Each repo does its own sweep on its own branch, verifies its own ripgrep gate, and cuts its own beta release.

**Tech Stack:** Rust 2024 edition, reqwest 0.12, `aws-sdk-s3` (csm-rs S3 client), kube 2.0.1 (csm-rs k8s console). No new dependencies introduced.

**Branches (create at task start):**
- `csm-rs`: `feat/remove-socks5` (branched from `main` at `b062516`)
- `ochami-rs`: `feat/remove-socks5` (branched from current tip of `main`)
- `manta-backend-dispatcher`: `feat/remove-socks5` (branched from current tip of `main`)

**Source of truth:** `docs/superpowers/specs/2026-07-12-remove-socks5-design.md`, present verbatim in all three repos.

**Phase ordering:** Phases A (csm-rs), B (ochami-rs), and C (manta-backend-dispatcher) are independent. They can be executed in any order or in parallel — the dispatcher does not gate the client sweeps (verified during planning; see Section 3.8 of the spec). Within a phase, tasks are strictly ordered.

## Global Constraints

- Ripgrep gate before releasing any repo — **zero hits** on the queries in Section 5 of the spec. Every task that touches source code must leave `rg -i "socks|hyper_socks2|hyper::client::HttpConnector" <paths>` closer to zero, never further.
- `csm-rs` must pass `cargo check --all-features`, `cargo test --all-features`, `cargo clippy --all-features -- -D warnings` on the release commit. `ochami-rs` and `manta-backend-dispatcher` have no feature gates that matter; plain `cargo check`, `cargo test`, `cargo clippy -- -D warnings` are sufficient.
- S3 TLS behavior in `csm-rs` must not change from what Section 3.4 of the spec describes: the AWS SDK still talks to CSM S3 via the platform trust store; the CSM root cert continues to be used only by reqwest for the STS token exchange.
- No unrelated cleanup, refactor, rename, or scope creep. If encountered during the sweep, stop and file a follow-up.
- Version bumps: `csm-rs` → `1.0.0-beta.20`; `manta-backend-dispatcher` → `1.0.0-beta.14`; `ochami-rs` → next beta (increment from current, tracked in that repo's Cargo.toml at execution time).

---

# Phase A — csm-rs sweep

## Task A1: Cut branch and remove SOCKS5 from `Cargo.toml`

**Files:**
- Modify: `csm-rs/Cargo.toml`

**Interfaces:**
- Consumes: nothing
- Produces: a Cargo manifest with no SOCKS5 dep or feature. Downstream tasks assume `hyper-socks2`, hyper 0.14, and the two socks-flavored `reqwest` / `kube` features are already gone.

- [ ] **Step 1: Create the branch**

Run from `csm-rs/`:
```bash
git checkout -b feat/remove-socks5
```

Expected: `Switched to a new branch 'feat/remove-socks5'`.

- [ ] **Step 2: Edit `Cargo.toml` — strip the `socks` feature from `reqwest`**

Locate the `reqwest = { … }` line (currently line 78):
```toml
reqwest = { version = "0.12.28", default-features = false, features = ["blocking", "json", "rustls-tls", "socks"] }
```
Change to:
```toml
reqwest = { version = "0.12.28", default-features = false, features = ["blocking", "json", "rustls-tls"] }
```

- [ ] **Step 3: Edit `Cargo.toml` — strip the `socks5` feature from `kube`**

Locate the `kube = { … }` line (currently line 92):
```toml
kube = { version = "2.0.1", features = ["ws", "socks5", "runtime"], optional = true }
```
Change to:
```toml
kube = { version = "2.0.1", features = ["ws", "runtime"], optional = true }
```

- [ ] **Step 4: Edit `Cargo.toml` — delete `hyper` and `hyper-socks2` dependency lines**

Delete these two lines (currently 105 and 106):
```toml
hyper-socks2 = { version = "0.8.0", default-features = false, optional = true }
hyper = { version = "0.14", optional = true }
```

- [ ] **Step 5: Edit `Cargo.toml` — trim `ims-s3` feature deps**

Locate the `ims-s3 = [ … ]` feature (currently lines 55–59):
```toml
ims-s3 = [
    "dep:aws-sdk-s3", "dep:aws-config", "dep:aws-smithy-runtime",
    "dep:aws-smithy-types", "dep:hyper",
    "dep:hyper-socks2", "dep:indicatif",
]
```
Change to:
```toml
ims-s3 = [
    "dep:aws-sdk-s3", "dep:aws-config", "dep:aws-smithy-runtime",
    "dep:aws-smithy-types", "dep:indicatif",
]
```

- [ ] **Step 6: Edit `Cargo.toml` — update doc comments that reference the hyper-0.14 / hyper-socks2 glue**

Replace the comment block above `ims-s3` (currently lines 51–54):
```toml
# IMS S3 image transport (`ims::s3_client`, the SOCKS5/hyper-0.14 glue
# the AWS SDK still requires). Pulls in `aws-sdk-s3`, `aws-config`,
# `aws-smithy-*`, `hyper`, `hyper-socks2`, `indicatif`. Default-on for
# backwards compatibility.
```
With:
```toml
# IMS S3 image transport (`ims::s3_client`). Pulls in `aws-sdk-s3`,
# `aws-config`, `aws-smithy-*`, `indicatif`. Uses the AWS SDK's default
# rustls HTTP client. Default-on for backwards compatibility.
```

Delete the multi-line comment block above the (now-deleted) `hyper-socks2` / `hyper` lines (currently 98–104):
```toml
# Needed by `ims::s3_client::setup_client`: the AWS SDK (aws-sdk-s3,
# aws-sdk-sts) still uses hyper 0.14 transitively via aws-smithy-http-client,
# and we glue SOCKS5 into the S3 transport using hyper_socks2::SocksConnector
# + hyper::client::HttpConnector + the AWS SDK's hyper_014::HyperClientBuilder.
# Drop these when aws-smithy-http-client upgrades to hyper 1.x (it will then
# also satisfy hyper-rustls 0.27+). kube-rs is unrelated -- kube 2.0.1 is
# already on hyper 1.x.
```

Also update the `reqwest` comment block (currently lines 74–77) to drop the SOCKS/legacy language if it mentions socks (it currently talks about the 0.12 pin, which stays — leave that part alone).

- [ ] **Step 7: Verify `Cargo.toml` no longer mentions SOCKS or hyper 0.14**

Run from `csm-rs/`:
```bash
rg -i "socks|hyper-socks2|hyper = |dep:hyper" Cargo.toml
```
Expected: no output.

- [ ] **Step 8: Do NOT run `cargo check` yet** — the source tree still references `socks5_proxy` everywhere and won't compile until Tasks A2–A5 land. `Cargo.lock` regeneration is deferred until Task A6.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml
git commit -m "chore(deps): remove SOCKS5 dependencies and features

Drop reqwest's socks feature, kube's socks5 feature, and the hyper 0.14
+ hyper-socks2 optional deps that previously fed the ims::s3_client
SOCKS5 glue. Removes both features from the ims-s3 feature list and
rewrites the surrounding doc comments.

Workspace does not compile after this commit; source-code sweep lands
in follow-up commits on the same branch. See
docs/superpowers/specs/2026-07-12-remove-socks5-design.md.
"
```

---

## Task A2: Strip SOCKS5 from `common::http` and `client`

**Files:**
- Modify: `csm-rs/src/common/http.rs`
- Modify: `csm-rs/src/client.rs`

**Interfaces:**
- Consumes: the SOCKS-free `Cargo.toml` from Task A1.
- Produces:
  - `pub fn build_client(shasta_root_cert: &[u8]) -> Result<reqwest::Client, Error>`
  - `pub fn build_client_with_auth(shasta_root_cert: &[u8], auth: Option<AuthCookies>) -> Result<reqwest::Client, Error>` (parameter list minus the `socks5_proxy` arg only; leave `auth` shape as-is)
  - `Client::new(base_url: impl Into<String>, root_cert: Vec<u8>) -> Result<Client, Error>` (drops the `socks5_proxy` arg only)
  - The `Client` struct no longer has a `socks5_proxy` field; no `socks5_proxy()` getter.

- [ ] **Step 1: Edit `src/common/http.rs` — drop `socks5_proxy` parameter from `build_client`**

Locate the current signature (around line 106):
```rust
pub fn build_client(
  shasta_root_cert: &[u8],
  socks5_proxy: Option<&str>,
) -> Result<reqwest::Client, Error> {
  build_client_with_auth(shasta_root_cert, socks5_proxy, None)
}
```
Change to:
```rust
pub fn build_client(
  shasta_root_cert: &[u8],
) -> Result<reqwest::Client, Error> {
  build_client_with_auth(shasta_root_cert, None)
}
```

- [ ] **Step 2: Edit `src/common/http.rs` — drop `socks5_proxy` parameter from `build_client_with_auth` and delete the SOCKS branch**

Locate the current signature and body (around line 120–160). It contains a `let client = match socks5_proxy { Some(url) => builder.proxy(reqwest::Proxy::all(url)?).build()?, None => builder.build()?, };` (exact shape). Remove the `socks5_proxy: Option<&str>,` parameter and replace the `match` with a plain `let client = builder.build()?;`.

Concrete transformation:
- Delete the `socks5_proxy: Option<&str>,` line inside the signature.
- Inside the body, the `match socks5_proxy { … }` returning a `reqwest::Client` becomes a straight `let client = builder.build()?;`. Do not delete anything above or below that match; only replace the match itself.
- Delete the `use reqwest::Proxy;` import if it exists at the top of the file and is now unused (Rust will warn if orphaned).

- [ ] **Step 3: Edit `src/common/http.rs` — delete the `build_client_with_socks5_proxy_succeeds` unit test**

Around line 499 of the file:
```rust
#[test]
fn build_client_with_socks5_proxy_succeeds() {
    let client =
      build_client(TEST_PEM.as_bytes(), Some("socks5://localhost:9050"));
    assert!(client.is_ok());
}
```
Delete the whole test function (including the `#[test]` attribute) and any trailing blank line it leaves behind.

- [ ] **Step 4: Edit `src/client.rs` — drop `socks5_proxy` field**

Locate the struct field declaration (around line 51):
```rust
pub(crate) socks5_proxy: Option<String>,
```
Delete the whole line.

- [ ] **Step 5: Edit `src/client.rs` — update `Client::new` signature and body**

Locate the constructor (around line 65–80). The current shape is roughly:
```rust
pub fn new(
    base_url: impl Into<String>,
    root_cert: Vec<u8>,
    socks5_proxy: Option<String>,
) -> Result<Self, Error> {
    let http = http::build_client(&root_cert, socks5_proxy.as_deref())?;
    Ok(Self { base_url: base_url.into(), root_cert, http, socks5_proxy })
}
```
Change to:
```rust
pub fn new(
    base_url: impl Into<String>,
    root_cert: Vec<u8>,
) -> Result<Self, Error> {
    let http = http::build_client(&root_cert)?;
    Ok(Self { base_url: base_url.into(), root_cert, http })
}
```
(Preserve any other struct fields verbatim; just remove `socks5_proxy` from both the parameter list and the struct-literal.)

- [ ] **Step 6: Edit `src/client.rs` — delete the `socks5_proxy` getter**

Locate:
```rust
/// The SOCKS5 proxy URL, if one was configured.
pub fn socks5_proxy(&self) -> Option<&str> {
    self.socks5_proxy.as_deref()
}
```
Delete the whole function including its doc comment.

- [ ] **Step 7: Edit `src/client.rs` — delete SOCKS5 tests**

Delete the following inside the `#[cfg(test)] mod tests { … }` block:
- `assert!(client.socks5_proxy().is_none());` — any occurrence.
- The whole `fn new_with_socks5_proxy_succeeds` test function.
- The line `assert_eq!(client.socks5_proxy(), cloned.socks5_proxy());` (if present).

If a test still references `Client::new(..., Some("socks5://…".to_string()))`, drop the third argument in that call site — the test body may still have other assertions worth keeping.

- [ ] **Step 8: Do NOT run `cargo check` yet** — every consumer of `build_client` / `Client::new` in the crate still passes `socks5_proxy` and won't compile until Task A4.

- [ ] **Step 9: Commit**

```bash
git add src/common/http.rs src/client.rs
git commit -m "refactor(http, client): drop socks5_proxy parameter and field

common::http::build_client and build_client_with_auth no longer accept
a socks5_proxy: Option<&str> argument, and the reqwest proxy branch is
gone. Client no longer stores a socks5_proxy field or exposes a getter,
and Client::new drops the corresponding parameter.

Unit tests exercising the removed SOCKS5 code paths are deleted. Rest
of the crate still references the removed parameter and does not
compile until subsequent tasks in the sweep.
"
```

---

## Task A3: Rewrite `ims::s3_client` on the AWS SDK's default HTTP client

**Files:**
- Modify: `csm-rs/src/ims/s3_client.rs`

**Interfaces:**
- Consumes: the parameter-free `common::http::build_client` from Task A2.
- Produces:
  - `pub async fn s3_auth(shasta_token: &str, shasta_base_url: &str, shasta_root_cert: &[u8]) -> Result<Value, Error>`
  - `pub async fn s3_get_object_size(sts_value: &Value, key: &str, bucket: &str) -> Result<i64, Error>`
  - `pub async fn s3_download_object(sts_value: &Value, object_path: &str, bucket: &str, destination_path: &str) -> Result<String, Error>`
  - `pub async fn s3_upload_object(sts_value: &Value, object_path: &str, bucket: &str, file_path: &str) -> Result<String, Error>`
  - `pub async fn s3_remove_object(sts_value: &Value, object_path: &str, bucket: &str) -> Result<String, Error>`
  - `pub async fn s3_multipart_upload_object(sts_value: &Value, object_path: &str, bucket: &str, file_path: &str) -> Result<String, Error>`
  - Private `async fn setup_client(sts_value: &Value) -> Result<Client, Error>`

- [ ] **Step 1: Delete imports at the top of `src/ims/s3_client.rs`**

Delete these lines from the header (currently line 4):
```rust
use hyper::client::HttpConnector;
```
There is no other `hyper::` or `hyper_socks2::` import at the file level; the `aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder` import lives inside `setup_client` and is removed in Step 3.

- [ ] **Step 2: Rewrite `setup_client`**

Locate the function (starts at line 92):
```rust
async fn setup_client(
  sts_value: &Value,
  socks5_proxy: Option<&str>,
) -> Result<Client, Error> {
  use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;

  let (credentials, endpoint_url) = parse_sts_credentials(sts_value)?;

  // Default provider fallback to us-east-1 since CSM doesn't use the concept of regions
  let region_provider =
    aws_config::meta::region::RegionProviderChain::default_provider()
      .or_else("us-east-1");
  let app_name = aws_config::AppName::new("manta")
    .map_err(|e| Error::S3Transport(format!("Error setting app name: {e}")))?;

  let timeout_config = TimeoutConfig::builder()
    .operation_timeout(S3_OPERATION_TIMEOUT)
    .build();
  let mut loader = aws_config::from_env()
    .region(region_provider)
    .endpoint_url(endpoint_url)
    .app_name(app_name)
    .credentials_provider(credentials)
    .timeout_config(timeout_config);

  if let Some(socks5_env) = socks5_proxy {
    log::debug!("SOCKS5 enabled");

    let mut http_connector: HttpConnector = hyper::client::HttpConnector::new();
    http_connector.enforce_http(false);

    let socks_http_connector = hyper_socks2::SocksConnector {
      proxy_addr: hyper::Uri::try_from(socks5_env)
        .map_err(|e| Error::S3Transport(e.to_string()))?,
      auth: None,
      connector: http_connector.clone(),
    };

    let http_client = HyperClientBuilder::new().build(socks_http_connector);
    loader = loader.http_client(http_client);
  }

  let config: SdkConfig = loader.load().await;

  let client = aws_sdk_s3::Client::from_conf(
    aws_sdk_s3::Client::new(&config)
      .config()
      .to_builder()
      .force_path_style(true)
      .build(),
  );

  Ok(client)
}
```
Replace the whole function with:
```rust
async fn setup_client(sts_value: &Value) -> Result<Client, Error> {
  let (credentials, endpoint_url) = parse_sts_credentials(sts_value)?;

  let region_provider =
    aws_config::meta::region::RegionProviderChain::default_provider()
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

- [ ] **Step 3: Drop `socks5_proxy` from each public function's signature and body**

For each of the six public functions in the file (`s3_auth`, `s3_get_object_size`, `s3_download_object`, `s3_upload_object`, `s3_remove_object`, `s3_multipart_upload_object`):

- Remove the `socks5_proxy: Option<&str>,` parameter from the function's parameter list.
- Remove the `socks5_proxy` argument from the `setup_client(sts_value, socks5_proxy)` call inside the body — it becomes `setup_client(sts_value).await?`.
- For `s3_auth`, additionally remove `socks5_proxy` from the `crate::common::http::build_client(shasta_root_cert, socks5_proxy)?` call — becomes `crate::common::http::build_client(shasta_root_cert)?`.

- [ ] **Step 4: Confirm no stale SOCKS references remain in the file**

Run from `csm-rs/`:
```bash
rg -i "socks|hyper_socks2|hyper::client::HttpConnector|HyperClientBuilder" src/ims/s3_client.rs
```
Expected: no output.

- [ ] **Step 5: Do NOT run `cargo check` yet** — every caller of the S3 helpers throughout `commands/`, `backend_connector/`, and `ims/` still passes `socks5_proxy`.

- [ ] **Step 6: Commit**

```bash
git add src/ims/s3_client.rs
git commit -m "refactor(ims/s3): rewrite setup_client on default AWS HTTP client

Drop the hyper 0.14 + hyper_socks2 SOCKS5 glue in ims::s3_client and
remove the socks5_proxy parameter from every public S3 helper
(s3_auth, s3_get_object_size, s3_download_object, s3_upload_object,
s3_remove_object, s3_multipart_upload_object). setup_client now hands
the AWS SDK a plain from_env config; the SDK's default rustls HTTP
client takes over. Preserves the previous TLS trust-anchor behavior:
the CSM root cert is still only used by reqwest for STS, and the AWS
SDK still relies on the platform trust store for S3 traffic.
"
```

---

## Task A4: Mechanical parameter sweep across the rest of the crate

**Files:** every remaining `.rs` file in `csm-rs/src/` that currently mentions `socks5_proxy` or `SOCKS5`. Enumerate at the start of the task with `rg -l "socks5_proxy" src/`. The expected list at time of writing:

- `src/lib.rs`
- `src/backend_connector/{authentication,cfs,console,group,migrate,mod,sat}.rs`
- `src/bos/wrapper/mod.rs`
- `src/bss/wrapper/mod.rs`
- `src/cfs/{cleanup,common}.rs`
- `src/cfs/component/utils.rs`
- `src/cfs/configuration/utils.rs`
- `src/cfs/configuration/http_client/v2/types/cfs_configuration_request.rs`
- `src/cfs/configuration/http_client/v3/types/cfs_configuration_request.rs`
- `src/cfs/session/{mod,utils}.rs`
- `src/cfs/wrapper/mod.rs`
- `src/commands/{apply_hw_cluster_pin/{command,utils}.rs, apply_session.rs, i_apply_sat_file/{command,utils/{configurations,images,session_templates}}.rs, migrate_backup.rs, migrate_restore.rs}`
- `src/common/{authentication,gitea,kubernetes,vault}.rs` (`common/http.rs` is already done in A2; skip it)
- `src/hsm/group/utils.rs`
- `src/hsm/wrapper/mod.rs`
- `src/ims/{image/utils,job/http_client,job/utils,mod}.rs`
- `src/node/{console,utils}.rs`
- `src/pcs/wrapper/mod.rs`

**Interfaces:**
- Consumes: A2's parameter-free `common::http` + `Client`, A3's parameter-free `ims::s3_client`.
- Produces: every function/method in the listed files loses its `socks5_proxy: Option<&str>` parameter and every call site drops the corresponding argument. No caller anywhere in the crate references the removed field or getter.

- [ ] **Step 1: Print the actual file list**

Run from `csm-rs/`:
```bash
rg -l "socks5_proxy" src/
```
Compare to the enumerated list above. Add or remove any files that drifted since planning. Note the count for step 3.

- [ ] **Step 2: Apply the sweep pattern to every listed file**

The transformation is entirely mechanical. For each file:

- Remove any parameter declared as `socks5_proxy: Option<&str>,` or `socks5_proxy: Option<String>,` from function signatures. Preserve every other parameter and its order.
- Remove `socks5_proxy` from struct/enum field declarations where it appears (e.g. `backend_connector/mod.rs` and any other spot that holds it as internal state).
- Replace every call-site argument of the form `socks5_proxy`, `socks5_proxy.as_deref()`, `client.socks5_proxy()`, `self.socks5_proxy.as_deref()`, or `self.client.socks5_proxy()` with **nothing**: delete the argument together with the trailing comma if it is not the last argument, or the leading comma if it is. Fix indentation only if the change would otherwise leave orphaned whitespace.
- Delete any comments that reference SOCKS5 in the modified files (block comments, doc comments, or trailing line comments). Leave neighboring paragraphs intact.

Do NOT alter any other behavior. Do not rename identifiers, reorder parameters, or attempt drive-by cleanup. If a file has a struct-literal like `Client::new(base, cert, socks5_proxy)`, it becomes `Client::new(base, cert)` — same rule.

- [ ] **Step 3: Verify no `socks5_proxy` identifier remains in `src/`**

Run from `csm-rs/`:
```bash
rg -c "socks5_proxy" src/ | wc -l
```
Expected: `0`.

If it prints a non-zero count, the printed lines report leftover references. Fix each one according to the rule in Step 2 and re-run.

- [ ] **Step 4: Run `cargo check --all-features`**

Run from `csm-rs/`:
```bash
cargo check --all-features
```
Expected: success. `Cargo.lock` will be regenerated silently.

If it fails, the compile errors point at the specific residual references. Repeat Step 2 for each until clean.

- [ ] **Step 5: Commit**

```bash
git add -A src/ Cargo.lock
git commit -m "refactor: drop socks5_proxy from every call site

Mechanical parameter sweep across the crate: every function that
threaded socks5_proxy: Option<&str> loses the parameter, every caller
drops the corresponding argument, and every struct that carried the
proxy string as internal state loses the field.

Cargo.lock regenerated with hyper 0.14 and hyper-socks2 trees removed.
"
```

---

## Task A5: Update `README.md`, doc comments, and remaining references

**Files:**
- Modify: `csm-rs/README.md`
- Modify: any doc comment left in `csm-rs/src/` that still mentions SOCKS5 (audit with ripgrep in Step 1)

**Interfaces:**
- Consumes: source tree with no `socks5_proxy` identifiers left (A4).
- Produces: README and doc comments consistent with the SOCKS-free API. Public examples do not show a proxy parameter.

- [ ] **Step 1: Audit remaining textual references**

Run from `csm-rs/`:
```bash
rg -i "socks|hyper_socks2|hyper::client::HttpConnector" README.md src/ Cargo.toml
```
Note every hit. Expected sources: README's examples/feature list/deployment paragraph, any lingering doc comment paragraphs referencing SOCKS5 in `backend_connector/mod.rs`, and any TODO in `src/` about SOCKS5.

- [ ] **Step 2: Update `README.md`**

For each SOCKS5 reference:
- If it appears inside a code example (e.g. `Client::new(...., Some("socks5://…"))`), drop the third argument.
- If it appears in a prose paragraph about "SOCKS5 tunnel from your workstation" or similar, delete the paragraph.
- If it appears in the feature list (bulleted table of feature flags), remove the SOCKS-related line.

Do not restructure surrounding sections. Only remove what was tied to SOCKS.

- [ ] **Step 3: Update in-crate doc comments**

For each doc comment hit surfaced in Step 1:
- Delete the sentence or bullet that mentions SOCKS5 or the proxy field.
- If the surrounding comment loses its point once that sentence is gone (e.g. it read "Holds the base URL, root certificate, optional SOCKS5 proxy, and a …"), rewrite it to omit the SOCKS clause naturally. Do not invent new prose — keep changes minimal.

- [ ] **Step 4: Verify no SOCKS or hyper-0.14 vestige remains**

Run from `csm-rs/`:
```bash
rg -i "socks|hyper_socks2|hyper::client::HttpConnector" README.md src/ Cargo.toml
```
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add README.md src/
git commit -m "docs: purge remaining SOCKS5 references from README and doc comments

Removes SOCKS5 examples from README.md, deletes the 'SOCKS5 tunnel
from your workstation' paragraph, and rewrites in-crate doc comments
that mentioned the (now-removed) proxy field. No functional change.
"
```

---

## Task A6: Full verification pass

**Files:** none modified in this task; verification only.

**Interfaces:**
- Consumes: everything from A1–A5.
- Produces: a green branch ready for release.

- [ ] **Step 1: Ripgrep gate — source tree, Cargo, README**

Run from `csm-rs/`:
```bash
rg -i "socks|hyper_socks2|hyper::client::HttpConnector" src/ Cargo.toml README.md
```
Expected: no output. If anything appears, cycle back to whichever task owns that file.

- [ ] **Step 2: `cargo check --all-features`**

Run from `csm-rs/`:
```bash
cargo check --all-features
```
Expected: success, no warnings related to unused imports left over from the sweep.

- [ ] **Step 3: `cargo test --all-features`**

Run from `csm-rs/`:
```bash
cargo test --all-features
```
Expected: all remaining tests pass. The three SOCKS-specific unit tests deleted in A2 and (any residual) elsewhere are gone; the count of tests decreases accordingly.

- [ ] **Step 4: `cargo clippy --all-features -- -D warnings`**

Run from `csm-rs/`:
```bash
cargo clippy --all-features -- -D warnings
```
Expected: success. If clippy flags an unused import that A2/A3/A4 missed, remove it now.

- [ ] **Step 5: Verify hyper 0.14 and hyper-socks2 are gone from the lockfile**

Run from `csm-rs/`:
```bash
rg -n '^name = "(hyper-socks2|hyper)"' Cargo.lock | rg -v '"hyper"$|version = "1\.'
```
Expected: no output. If `hyper-socks2` is still present, some transitive dep pulled it back — investigate; do not proceed until clean.

- [ ] **Step 6: Do not commit anything** — this task is verification only.

---

## Task A7: Release `csm-rs 1.0.0-beta.20`

**Files:**
- Modify: `csm-rs/Cargo.toml` (version field)

**Interfaces:**
- Consumes: green branch from A6.
- Produces: a release commit tagged/versioned for downstream consumption.

- [ ] **Step 1: Bump version in `Cargo.toml`**

Locate the `[package]` block:
```toml
version = "1.0.0-beta.19"
```
Change to:
```toml
version = "1.0.0-beta.20"
```

- [ ] **Step 2: Run `cargo check --all-features` once more to refresh `Cargo.lock`**

Run from `csm-rs/`:
```bash
cargo check --all-features
```

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: Release csm-rs version 1.0.0-beta.20"
```

- [ ] **Step 4: Handoff**

Merge/PR the `feat/remove-socks5` branch per the repo's usual process. The plan does not tag or push; that is left to the maintainer.

---

# Phase B — ochami-rs sweep

## Task B1: Cut branch and remove `reqwest`'s `socks` feature from `Cargo.toml`

**Files:**
- Modify: `ochami-rs/Cargo.toml`

**Interfaces:**
- Consumes: nothing.
- Produces: manifest with no SOCKS5 dep or feature.

- [ ] **Step 1: Create the branch**

Run from `ochami-rs/`:
```bash
git checkout main && git checkout -b feat/remove-socks5
```

- [ ] **Step 2: Edit `Cargo.toml` — strip `socks` from `reqwest`**

Locate (currently line 20):
```toml
reqwest = { version = "0.12.15", default-features = false, features = ["blocking", "json", "rustls-tls", "socks"] }
```
Change to:
```toml
reqwest = { version = "0.12.15", default-features = false, features = ["blocking", "json", "rustls-tls"] }
```

- [ ] **Step 3: Verify no SOCKS reference remains in `Cargo.toml`**

Run from `ochami-rs/`:
```bash
rg -i "socks" Cargo.toml
```
Expected: no output.

- [ ] **Step 4: Do NOT run `cargo check` yet** — source tree still references `socks5_proxy` throughout.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml
git commit -m "chore(deps): remove reqwest's socks feature

Workspace does not compile after this commit; source-code sweep lands
in follow-up commits on the same branch. See
docs/superpowers/specs/2026-07-12-remove-socks5-design.md.
"
```

---

## Task B2: Strip SOCKS5 from `src/http.rs`

**Files:**
- Modify: `ochami-rs/src/http.rs`

**Interfaces:**
- Consumes: SOCKS-free `Cargo.toml` from B1.
- Produces:
  - `build_client(root_cert: &[u8], /* other existing params */) -> Result<reqwest::Client, Error>`
  - Any sibling helper in `http.rs` loses its `socks5_proxy: Option<&str>` param.

- [ ] **Step 1: Remove `socks5_proxy` parameters from both builder functions and delete their `match` branches**

There are two functions carrying `socks5_proxy: Option<&str>` (lines 5 and 20). For each:
- Delete the `socks5_proxy: Option<&str>,` parameter line.
- Replace the `match socks5_proxy { … }` block that returned the client with a plain `let client = builder.build()?;` (preserving the surrounding `builder` construction and any `?` propagation).
- Delete `use reqwest::Proxy;` if the file imports it and it becomes orphaned.

- [ ] **Step 2: Confirm the file is SOCKS-clean**

Run from `ochami-rs/`:
```bash
rg -i "socks" src/http.rs
```
Expected: no output.

- [ ] **Step 3: Do NOT run `cargo check` yet** — every caller of these builders still passes `socks5_proxy`.

- [ ] **Step 4: Commit**

```bash
git add src/http.rs
git commit -m "refactor(http): drop socks5_proxy parameter from client builders

Both builder functions in src/http.rs lose their socks5_proxy:
Option<&str> parameter and the reqwest::Proxy match branches. Rest of
the crate still references the removed parameter and does not compile
until subsequent tasks in the sweep.
"
```

---

## Task B3: Strip SOCKS5 from `src/backend_connector.rs`

**Files:**
- Modify: `ochami-rs/src/backend_connector.rs`

**Interfaces:**
- Consumes: parameter-free `src/http.rs` from B2.
- Produces: the connector struct no longer holds `socks5_proxy: Option<String>`. Its constructor drops the `socks5_proxy: Option<&str>` parameter. All ~30 method bodies stop calling `self.socks5_proxy.as_deref()` when threading into the module helpers.

- [ ] **Step 1: Delete the `socks5_proxy` struct field**

Around line 57:
```rust
socks5_proxy: Option<String>,
```
Delete the line.

- [ ] **Step 2: Update the constructor**

Around lines 62–72. Delete the `socks5_proxy: Option<&str>,` parameter from the signature. In the struct-literal body, delete the `socks5_proxy: socks5_proxy.map(str::to_owned),` line (or similar shape).

- [ ] **Step 3: Delete every `self.socks5_proxy.as_deref()` argument**

The file has ~30 method bodies (call-site lines noted from the ripgrep census: 87, 112, 136, 157, 173, 189, 206, 227, 253, 270, 291, 313, 334, 353, 374, 402, 441, 463, 481, 546, 585, 601, 634, 699, 757, 780, 809, and a few more). At each call, drop `self.socks5_proxy.as_deref(),` from the argument list — preserve every other argument and its trailing comma.

- [ ] **Step 4: Verify the file is SOCKS-clean**

Run from `ochami-rs/`:
```bash
rg -i "socks" src/backend_connector.rs
```
Expected: no output.

- [ ] **Step 5: Do NOT run `cargo check` yet** — the module helpers still declare the parameter.

- [ ] **Step 6: Commit**

```bash
git add src/backend_connector.rs
git commit -m "refactor(backend_connector): drop socks5_proxy field and args

Removes the socks5_proxy: Option<String> field from the connector
struct, drops the corresponding constructor parameter, and rips
self.socks5_proxy.as_deref() out of every trait-impl call site.
Compile still broken until the http_client/utils modules stop taking
the parameter.
"
```

---

## Task B4: Mechanical sweep across `http_client.rs` files and utils

**Files:** every remaining file in `ochami-rs/src/` that mentions `socks5_proxy`. Expected list (~18 files):

- `src/bss/http_client.rs`
- `src/hsm/component/http_client.rs`
- `src/hsm/defaults/node_map/http_client.rs`
- `src/hsm/group/http_client.rs`
- `src/hsm/group/utils.rs`
- `src/hsm/inventory/ethernet_interfaces/http_client.rs`
- `src/hsm/inventory/hardware/http_client.rs`
- `src/hsm/inventory/hardware_by_fru/http_client.rs`
- `src/hsm/inventory/redfish_endpoint/http_client.rs`
- `src/hsm/memberships/http_client.rs`
- `src/hsm/node_map/http_client.rs`
- `src/hsm/partition/http_client.rs`
- `src/hsm/state/components/http_client.rs`
- `src/node/utils.rs`
- `src/pcs/power_cap/http_client.rs`
- `src/pcs/power_status/http_client.rs`
- `src/pcs/transitions/http_client.rs`

**Interfaces:**
- Consumes: parameter-free `src/http.rs` (B2) and connector struct (B3).
- Produces: every function in the listed files loses its `socks5_proxy: Option<&str>` parameter and every `build_client(root_cert, socks5_proxy)` call becomes `build_client(root_cert)`.

- [ ] **Step 1: Print the current file list**

Run from `ochami-rs/`:
```bash
rg -l "socks5_proxy" src/
```

- [ ] **Step 2: Apply the mechanical rule to every listed file**

- Delete `socks5_proxy: Option<&str>,` from every function signature. Preserve every other parameter.
- Replace every argument `socks5_proxy` at a call site with **nothing**: delete the argument together with its trailing (or leading, if last) comma.

Do not change anything else. No renames, no reordering.

- [ ] **Step 3: Verify no `socks5_proxy` identifier remains**

Run from `ochami-rs/`:
```bash
rg -c "socks5_proxy" src/ | wc -l
```
Expected: `0`.

- [ ] **Step 4: Run `cargo check`**

Run from `ochami-rs/`:
```bash
cargo check
```
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add -A src/ Cargo.lock
git commit -m "refactor: drop socks5_proxy from every call site

Mechanical parameter sweep across the http_client and utils modules.
Every function that threaded socks5_proxy: Option<&str> loses the
parameter and every caller drops the argument. Cargo.lock regenerated.
"
```

---

## Task B5: Update `README.md` and remaining doc comments

**Files:**
- Modify: `ochami-rs/README.md`
- Modify: any doc comment left in `ochami-rs/src/` still mentioning SOCKS5

**Interfaces:**
- Consumes: source tree with no `socks5_proxy` identifiers left (B4).
- Produces: docs consistent with the SOCKS-free API.

- [ ] **Step 1: Audit remaining textual references**

Run from `ochami-rs/`:
```bash
rg -i "socks" README.md src/ Cargo.toml
```
Note each hit.

- [ ] **Step 2: Edit each hit per the same rules as `csm-rs` Task A5**

Same rules: drop SOCKS-only paragraphs, drop the third argument from example code, rewrite mixed-topic sentences to omit the SOCKS clause without inventing new prose.

- [ ] **Step 3: Verify clean**

Run from `ochami-rs/`:
```bash
rg -i "socks" README.md src/ Cargo.toml
```
Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add README.md src/
git commit -m "docs: purge remaining SOCKS5 references from README and doc comments"
```

---

## Task B6: Full verification pass

**Files:** none modified; verification only.

- [ ] **Step 1: Ripgrep gate**

Run from `ochami-rs/`:
```bash
rg -i "socks" src/ Cargo.toml README.md
```
Expected: no output.

- [ ] **Step 2: `cargo check`, `cargo test`, `cargo clippy -- -D warnings`**

Run each from `ochami-rs/`. Each must succeed.

---

## Task B7: Release next ochami-rs beta

**Files:**
- Modify: `ochami-rs/Cargo.toml` (version field)

- [ ] **Step 1: Bump version**

Locate the `[package]` `version = "1.0.0-beta.<current>"` line. Increment the beta number by one (current tip is `1.0.0-beta.13` per the last release commit `8f46519`; if the tip has moved by execution time, increment whatever is there).

- [ ] **Step 2: Refresh `Cargo.lock`**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: Release ochami-rs version 1.0.0-beta.<N>"
```
Substitute `<N>` with the actual new number.

- [ ] **Step 4: Handoff**

Merge/PR the `feat/remove-socks5` branch per the repo's usual process.

---

# Phase C — manta-backend-dispatcher

The dispatcher's public traits do not carry `socks5_proxy`, so there is no source-code sweep in this phase — only the `Cargo.toml` feature flip and a release.

## Task C1: Cut branch, drop `reqwest`'s `socks` feature, verify, release

**Files:**
- Modify: `manta-backend-dispatcher/Cargo.toml`

- [ ] **Step 1: Create the branch**

Run from `manta-backend-dispatcher/`:
```bash
git checkout main && git checkout -b feat/remove-socks5
```

- [ ] **Step 2: Edit `Cargo.toml` — strip `socks` from `reqwest`**

Locate (currently line 12):
```toml
reqwest = { version = "0.12.15", default-features = false, features = ["blocking", "json", "rustls-tls", "socks"] }
```
Change to:
```toml
reqwest = { version = "0.12.15", default-features = false, features = ["blocking", "json", "rustls-tls"] }
```

- [ ] **Step 3: Verify no SOCKS reference remains**

Run from `manta-backend-dispatcher/`:
```bash
rg -i "socks" src/ Cargo.toml README.md 2>/dev/null
```
Expected: no output. (If there is SOCKS wording anywhere in the README, edit it out here.)

- [ ] **Step 4: Run `cargo check`, `cargo test`, `cargo clippy -- -D warnings`**

Run each from `manta-backend-dispatcher/`. Each must succeed.

- [ ] **Step 5: Bump version to `1.0.0-beta.14`**

Locate `[package]` `version = "1.0.0-beta.13"` and change to `version = "1.0.0-beta.14"`.

- [ ] **Step 6: Refresh `Cargo.lock`**

```bash
cargo check
```

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore(deps): remove reqwest's socks feature and release 1.0.0-beta.14

Aligns manta-backend-dispatcher with the SOCKS5 removal in csm-rs
and ochami-rs. Traits carry no socks5_proxy parameter today, so no
source changes are required. See
docs/superpowers/specs/2026-07-12-remove-socks5-design.md.
"
```

- [ ] **Step 8: Handoff**

Merge/PR the `feat/remove-socks5` branch per the repo's usual process.

---

# Phase D — downstream (manta CLI)

Out of scope for this plan. After the three releases above land on crates.io / registry, whoever owns the `manta` CLI bumps all three dependencies in one PR and drops any `--socks5-proxy` flag from the CLI surface. That work should be tracked in a separate spec/plan under the manta CLI repo.
