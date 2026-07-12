# S3 SOCKS5 path: hyper 0.14 → hyper 1.x migration

**Status:** design
**Date:** 2026-07-12
**Scope:** `src/ims/s3_client.rs` and the `ims-s3` feature's dependency set

## Problem

`src/ims/s3_client.rs::setup_client` routes AWS S3 traffic through a SOCKS5 proxy for CSM environments that require it. The current implementation is stuck on hyper 0.14:

- `hyper_socks2 = "0.8.0"` (which pins hyper 0.14)
- `hyper = { version = "0.14", optional = true }` (declared solely for the connector plumbing at `s3_client.rs:120`)
- `aws-sdk-s3` uses the `"rustls"` feature, which drags in `hyper-rustls 0.24` → hyper 0.14
- The AWS SDK's hyper-0.14 shim is reached via `aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder`

`aws-smithy-http-client 1.1.13` (already in the dep graph transitively) has since migrated to hyper 1.x as its `default-client`. The 0.14 path is now a legacy opt-in kept only for backward compatibility. Nothing in this crate needs to stay on 0.14 — the pin is inertia, not a real constraint.

Migrating unblocks eventually removing the `hyper 0.14`, `hyper-rustls 0.24`, and `h2 0.3` crates from the build.

## Goals

- Eliminate `hyper 0.14`, `hyper-rustls 0.24`, and `h2 0.3` from the resolved dep tree.
- Keep the SOCKS5-through-S3 flow behaviorally identical: same connection semantics, same `Error::S3Transport` surface, same per-op timeout.
- No public API change to `csm-rs`.
- Migration is confined to the `ims-s3` feature; other features are unaffected.

## Non-goals

- Version bumps of `aws-sdk-s3`, `aws-config`, `aws-smithy-runtime`, `aws-smithy-types` beyond what this migration requires. Those minor/patch bumps ship as a separate follow-up PR.
- Any changes to the `k8s-console` / kube SOCKS5 path — it already runs on hyper 1.x via `http::Uri`.
- Adding automated integration tests for the proxied S3 flow. Verification is manual against a real CSM env (see Verification).

## Design

### Dependency changes (`Cargo.toml`)

Four coordinated edits inside `[dependencies]` and `[features]`:

1. **`ims-s3` feature list** — drop `"dep:hyper"`, add `"dep:aws-smithy-http-client"`:

   ```toml
   ims-s3 = [
       "dep:aws-sdk-s3", "dep:aws-config", "dep:aws-smithy-runtime",
       "dep:aws-smithy-types", "dep:aws-smithy-http-client",
       "dep:hyper-socks2", "dep:indicatif",
   ]
   ```

2. **`aws-sdk-s3` features** — swap `"rustls"` for `"rustls-ring"`, which routes through `aws-smithy-http-client`'s hyper-1.x `default-client` instead of the legacy `hyper-014` path:

   ```toml
   aws-sdk-s3 = { version = "1.135", features = ["rustls-ring"], default-features = false, optional = true }
   ```

3. **`hyper-socks2`** — bump `0.8.0` → `0.9.1` (declares `hyper = "1"` upstream). Keep `default-features = false` so `hyper-tls` isn't pulled in; TLS is handled by the AWS SDK itself, not at the SOCKS layer:

   ```toml
   # SOCKS5 for the AWS SDK's hyper 1.x HTTP transport. `hyper_socks2::SocksConnector`
   # wraps a `hyper_util` HttpConnector and is handed to the SDK via
   # `aws_smithy_http_client::hyper_1::HyperClientBuilder`. Only used by `ims::s3_client`.
   hyper-socks2 = { version = "0.9.1", default-features = false, optional = true }
   ```

4. **New direct dep: `aws-smithy-http-client`** — needed to name `HyperClientBuilder` from the hyper-1.x path. Already resolved transitively; no new crate downloaded.

   ```toml
   aws-smithy-http-client = { version = "1.1", default-features = false, features = ["default-client", "rustls-ring"], optional = true }
   ```

5. **Delete the `hyper = { version = "0.14", optional = true }` line entirely** (currently `Cargo.toml:106`). No other module in this crate uses hyper directly.

6. **Rewrite the rationale comment at `Cargo.toml:98–104`.** The old text ("Drop these when aws-smithy-http-client upgrades to hyper 1.x") is now stale. Replace with the two-line comment shown above the `hyper-socks2` line in point 3.

### Code changes (`src/ims/s3_client.rs`)

Three localized edits; the surrounding structure of `setup_client` is untouched.

**Top of file (line 4):** remove `use hyper::client::HttpConnector;`

**Inside `setup_client`, the current lines 96–131 block:**

```rust
// OLD (line 96):    use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;
// NEW:              use aws_smithy_http_client::hyper_1::HyperClientBuilder;

if let Some(socks5_env) = socks5_proxy {
    log::debug!("SOCKS5 enabled");

    // OLD: let mut http_connector: HttpConnector = hyper::client::HttpConnector::new();
    // NEW: hyper 1.x split the connector into hyper-util.
    let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
    http_connector.enforce_http(false);

    let socks_http_connector = hyper_socks2::SocksConnector {
        // OLD: proxy_addr: hyper::Uri::try_from(socks5_env)...
        // NEW: hyper 1.x's Uri IS http::Uri (re-exported); use the http crate we already depend on.
        proxy_addr: http::Uri::try_from(socks5_env)
            .map_err(|e| Error::S3Transport(e.to_string()))?,
        auth: None,
        connector: http_connector.clone(),
    };

    let http_client = HyperClientBuilder::new().build(socks_http_connector);
    loader = loader.http_client(http_client);
}
```

The `enforce_http(false)`, `auth: None`, and `loader.http_client(...)` semantics are all preserved — the runtime behavior is identical to today; only the underlying type graph changes.

### Two shapes to verify at implementation time

Not guessed — these are the two API surfaces that could legitimately have moved and need a `cargo doc` check before we commit:

1. **`aws_smithy_http_client::hyper_1::HyperClientBuilder`** — the exact module path. Might be `aws_smithy_http_client::hyper_1::` (as written) or `aws_smithy_http_client::` at the crate root, depending on the 1.1.x layout. First move at impl time is `cargo doc --features ims-s3 --open -p aws-smithy-http-client` and confirm — the `--features ims-s3` is required because `aws-smithy-http-client` is an optional dep gated by that feature.
2. **`hyper_util::client::legacy::connect::HttpConnector`** — the canonical location for hyper 1.x's raw TCP connector. `hyper-util` reaches the graph transitively via `hyper-socks2 0.9.1`; if the compiler rejects the naked path, add a direct dep:

   ```toml
   hyper-util = { version = "0.1", default-features = false, features = ["client-legacy"], optional = true }
   ```

   and reference it from `ims-s3`.

### Feature-gate scope

- `ims-s3` stays default-on. No behavioral change for existing embedders.
- `k8s-console` and `commands-admin` are untouched.
- No public API surface changes; no re-exports leak.
- Callers of `csm-rs` (including `manta`) see no delta.

## Error handling

The single fallible line (`http::Uri::try_from(socks5_env)`) maps its error to `Error::S3Transport(...)`, matching the current behavior. No new error variants. Downstream connect/read errors continue to bubble through the AWS SDK's own error types — unchanged.

## Verification

### Local checks (before taking to CSM)

1. `cargo check --features ims-s3` — passes with no warnings about unused deps.
2. `cargo tree -i hyper@0.14` — returns nothing.
3. `cargo tree -i hyper-rustls@0.24` — returns nothing.
4. `cargo tree -i hyper@1` — shows `aws-smithy-http-client` and `hyper-socks2` as parents.
5. `cargo test --features ims-s3` — module compiles, existing unit tests link.
6. `cargo clippy --features ims-s3 --all-targets -- -D warnings` — catches any lint the type changes introduce.

### Manual verification (real CSM env)

7. Build a manta binary with these changes.
8. Point it at a CSM cluster over a working SOCKS5 tunnel.
9. Exercise both directions:
   - **Download**: IMS image pull through `setup_client(socks5_proxy = Some(...))`.
   - **Upload**: IMS image push (the multi-GB path the 60-min `S3_OPERATION_TIMEOUT` at `s3_client.rs:25` exists for).
10. Failure-mode sanity check: stop the SOCKS5 proxy mid-transfer. Failure must surface as `Error::S3Transport`, not a panic or a hang past the operation timeout.

### Rollback trigger

If verification steps 7–10 fail and the fix isn't obvious within a working afternoon, revert the branch. The current hyper 0.14 path is known-good and there's no external pressure forcing the move.

## Follow-up (out of scope, separate PR)

- Mechanical version bumps: `aws-sdk-s3 1.135 → 1.138`, `aws-config 1.8 → 1.9`, and any patch updates to `aws-smithy-runtime` / `aws-smithy-types`. Low-risk, but keeping them out of this PR isolates the risk of the transport migration.
