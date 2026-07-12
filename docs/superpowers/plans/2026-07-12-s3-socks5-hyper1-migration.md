# S3 SOCKS5 hyper 1.x Migration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Retire the legacy hyper 0.14 transport used by the S3 SOCKS5 path so we can eventually remove `hyper 0.14`, `hyper-rustls 0.24`, and `h2 0.3` from the resolved dep tree.

**Architecture:** Bump `hyper-socks2` from 0.8 → 0.9.1 (which upstream now declares `hyper = "1"`), swap `aws-sdk-s3`'s `rustls` feature for `rustls-ring` (which routes through `aws-smithy-http-client`'s hyper-1.x `default-client`), rewrite the three-line connector plumbing in `src/ims/s3_client.rs::setup_client`, and delete the standalone `hyper = "0.14"` declaration.

**Tech Stack:** Rust; `aws-sdk-s3` / `aws-config` / `aws-smithy-http-client` (hyper 1.x default-client, rustls-ring); `hyper-socks2` 0.9.1; `hyper-util` (transitively via hyper-socks2 0.9.1).

**Spec:** `docs/superpowers/specs/2026-07-12-s3-socks5-hyper1-migration-design.md` (commit `31b2aab`).

## Global Constraints

- Migration is confined to the `ims-s3` feature; do not touch `k8s-console`, `commands-admin`, or any other feature gate.
- No public API changes to `csm-rs`. No re-exports leak. Callers see zero delta.
- Behavior of the SOCKS5-through-S3 flow must stay identical: same connect/read semantics, same `Error::S3Transport` surface, same `S3_OPERATION_TIMEOUT` (60 min).
- `aws-sdk-s3`, `aws-config`, `aws-smithy-runtime`, `aws-smithy-types` version bumps are **out of scope** — a separate PR.
- No new automated integration tests. Verification is manual against a real proxied CSM env (Task 3).
- Every task ends with a green `cargo check --features ims-s3`. If a task leaves the tree in a broken state, its steps have not all been completed.

---

### Task 1: Verify aws-smithy-http-client hyper-1.x API paths

**Purpose:** The spec identified two API shapes that need real-world confirmation before the migration is written: the exact module path for `HyperClientBuilder` in `aws-smithy-http-client 1.1.x`, and whether `hyper-util::client::legacy::connect::HttpConnector` is reachable from our current graph. Do this check now, and record findings inline so Task 2 uses the correct imports.

**Files:**
- Read-only: `~/.cargo/registry/src/index.crates.io-*/aws-smithy-http-client-1.1.13/src/`
- Read-only: `~/.cargo/registry/src/index.crates.io-*/hyper-util-0.1.*/src/client/legacy/connect/`

**Interfaces:**
- Consumes: nothing.
- Produces: two confirmed strings — the exact `use` path for `HyperClientBuilder`, and the exact `use` path for the hyper-1.x `HttpConnector`. These get pasted verbatim into Task 2's edits.

- [ ] **Step 1: Locate the aws-smithy-http-client 1.1.13 source in the cargo registry**

```bash
ls ~/.cargo/registry/src/index.crates.io-*/aws-smithy-http-client-1.1.13/src/
```

Expected: a listing that includes `lib.rs`, and probably a `hyper_1/` or `hyper/` module directory.

- [ ] **Step 2: Grep for `HyperClientBuilder` inside aws-smithy-http-client**

```bash
grep -rn "pub struct HyperClientBuilder\|pub use.*HyperClientBuilder" ~/.cargo/registry/src/index.crates.io-*/aws-smithy-http-client-1.1.13/src/
```

Expected: at least one hit that either defines `pub struct HyperClientBuilder` or re-exports it from a submodule. Note the module chain (e.g. `hyper_1::HyperClientBuilder` or `hyper::HyperClientBuilder`).

**Record here before proceeding to Step 3:**
```
Confirmed HyperClientBuilder path: aws_smithy_http_client::__________::HyperClientBuilder
```

- [ ] **Step 3: Grep for `HttpConnector` inside hyper-util**

```bash
grep -rn "pub struct HttpConnector\|pub use.*HttpConnector" ~/.cargo/registry/src/index.crates.io-*/hyper-util-0.1.*/src/
```

Expected: a hit in `hyper-util-0.1.*/src/client/legacy/connect/` (or similar). The spec's guess is `hyper_util::client::legacy::connect::HttpConnector`.

**Record here before proceeding:**
```
Confirmed HttpConnector path: hyper_util::__________::HttpConnector
```

- [ ] **Step 4: Check whether hyper-util is currently in the graph**

```bash
cargo tree -i hyper-util 2>&1 | head -20
```

Expected: it IS in the graph (transitively via `aws-smithy-http-client`, `hyper-rustls`, etc.). If `hyper-util` does NOT appear, Task 2 must add it as a direct optional dep gated by `ims-s3`. Note the result:

```
hyper-util in current graph: YES / NO
```

- [ ] **Step 5: No commit — investigation only**

This task produces no code changes. The two recorded strings feed Task 2. If either finding contradicts the spec, update Task 2's code samples in this file before proceeding.

---

### Task 2: Migrate dependencies and rewrite the S3 SOCKS5 connector

**Purpose:** Apply all Cargo.toml + `s3_client.rs` edits in one atomic change. The migration is not decomposable — dep changes without code changes won't compile, and vice versa.

**Files:**
- Modify: `Cargo.toml` (features block + dependencies block, ~5 edits per spec Section 1)
- Modify: `src/ims/s3_client.rs` (line 4 import + lines 96–131 inside `setup_client`)

**Interfaces:**
- Consumes: the two confirmed paths from Task 1.
- Produces: a compiling crate with `hyper 0.14` removed from the resolved dep tree and the S3 SOCKS5 flow running on hyper 1.x transport. No new public API.

- [ ] **Step 1: Edit `Cargo.toml` — update the `ims-s3` feature list**

Locate the `ims-s3` feature at Cargo.toml:55–59 and replace it with:

```toml
ims-s3 = [
    "dep:aws-sdk-s3", "dep:aws-config", "dep:aws-smithy-runtime",
    "dep:aws-smithy-types", "dep:aws-smithy-http-client",
    "dep:hyper-socks2", "dep:indicatif",
]
```

Two changes vs. current: added `"dep:aws-smithy-http-client"`; removed `"dep:hyper"`.

- [ ] **Step 2: Edit `Cargo.toml` — swap `aws-sdk-s3`'s rustls feature**

Locate the `aws-sdk-s3` line at Cargo.toml:112 and change `features = ["rustls"]` to `features = ["rustls-ring"]`:

```toml
aws-sdk-s3 = { version = "1.135", features = ["rustls-ring"], default-features = false, optional = true }
```

Rationale: `"rustls"` pulls the legacy `hyper-rustls 0.24` / hyper 0.14 path; `"rustls-ring"` routes through `aws-smithy-http-client`'s hyper-1.x `default-client`.

- [ ] **Step 3: Edit `Cargo.toml` — bump `hyper-socks2` and rewrite its rationale comment**

Locate the current comment block at Cargo.toml:98–104 (starts "Needed by `ims::s3_client::setup_client`") plus the `hyper-socks2` line at Cargo.toml:105. Replace all of it (the comment block + the two lines for hyper-socks2 and hyper 0.14) with:

```toml
# SOCKS5 for the AWS SDK's hyper 1.x HTTP transport. `hyper_socks2::SocksConnector`
# wraps a `hyper_util` HttpConnector and is handed to the SDK via
# `aws_smithy_http_client::hyper_1::HyperClientBuilder`. Only used by `ims::s3_client`.
hyper-socks2 = { version = "0.9.1", default-features = false, optional = true }
```

The `hyper = { version = "0.14", optional = true }` line is deleted entirely — no other module uses hyper directly.

- [ ] **Step 4: Edit `Cargo.toml` — add `aws-smithy-http-client` as a direct optional dep**

Immediately below the `hyper-socks2` line from Step 3, add:

```toml
aws-smithy-http-client = { version = "1.1", default-features = false, features = ["default-client", "rustls-ring"], optional = true }
```

- [ ] **Step 5: Conditional — if Task 1 Step 4 reported hyper-util NOT in the graph, add it as a direct optional dep**

Only do this step if Task 1 Step 4 recorded `NO`. Add below the `aws-smithy-http-client` line:

```toml
hyper-util = { version = "0.1", default-features = false, features = ["client-legacy"], optional = true }
```

And add `"dep:hyper-util"` to the `ims-s3` feature list from Step 1.

Skip this step if Task 1 confirmed hyper-util is already in the graph transitively.

- [ ] **Step 6: Sanity-check the Cargo.toml delta compiles at the manifest level**

```bash
cargo check --features ims-s3 2>&1 | head -40
```

Expected: the build will fail at `s3_client.rs` because the source still uses `hyper::client::HttpConnector` and the hyper-0.14 import path. This is expected — Steps 7–9 fix it. What must NOT happen at this step: a Cargo.toml parse error or "unresolved dep" error (e.g. "no matching package named `aws-smithy-http-client`" or "feature `rustls-ring` does not exist"). If you see either, re-read Steps 1–5.

- [ ] **Step 7: Edit `src/ims/s3_client.rs` — remove the top-of-file hyper import**

Delete line 4:

```rust
use hyper::client::HttpConnector;   // REMOVE
```

- [ ] **Step 8: Edit `src/ims/s3_client.rs` — rewrite the SOCKS5 block inside `setup_client`**

At `src/ims/s3_client.rs:96`, replace:

```rust
  use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;
```

with the confirmed path from Task 1 Step 2 (spec's guess is):

```rust
  use aws_smithy_http_client::hyper_1::HyperClientBuilder;
```

Then, inside the `if let Some(socks5_env) = socks5_proxy {` block (currently lines 117–132), replace the body with:

```rust
    log::debug!("SOCKS5 enabled");

    let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
    http_connector.enforce_http(false);

    let socks_http_connector = hyper_socks2::SocksConnector {
      proxy_addr: http::Uri::try_from(socks5_env)
        .map_err(|e| Error::S3Transport(e.to_string()))?, // scheme is required by HttpConnector
      auth: None,
      connector: http_connector.clone(),
    };

    let http_client = HyperClientBuilder::new().build(socks_http_connector);
    loader = loader.http_client(http_client);
```

Substitute the `hyper_util::...HttpConnector` path with Task 1 Step 3's confirmed value if it differed.

Three substantive changes vs. current:
- `hyper::client::HttpConnector::new()` → `hyper_util::client::legacy::connect::HttpConnector::new()` (or Task 1's confirmed path)
- `hyper::Uri::try_from(socks5_env)` → `http::Uri::try_from(socks5_env)` (hyper 1.x's `Uri` IS `http::Uri`; use the `http` crate we already depend on)
- The `HyperClientBuilder` import path (Step 8's `use` line above)

The `enforce_http(false)`, `auth: None`, `connector: http_connector.clone()`, and `loader.http_client(...)` semantics are all preserved verbatim.

- [ ] **Step 9: Compile the crate**

```bash
cargo check --features ims-s3 2>&1 | tail -30
```

Expected: clean compile with no errors. If the compiler rejects a module path in the `use` statements, it means Task 1's grep missed the real path — go back to Task 1, correct the finding, then re-edit `s3_client.rs`.

- [ ] **Step 10: Verify hyper 0.14 is out of the resolved dep tree**

```bash
cargo tree -i hyper@0.14 --features ims-s3 2>&1
```

Expected: `error: no crates matched 'hyper@0.14'` (or empty output). If it still appears, some feature is still pulling it — most commonly the `rustls` feature wasn't fully swapped. Re-check Step 2.

- [ ] **Step 11: Verify hyper-rustls 0.24 is out of the tree**

```bash
cargo tree -i hyper-rustls@0.24 --features ims-s3 2>&1
```

Expected: `error: no crates matched 'hyper-rustls@0.24'`.

- [ ] **Step 12: Verify hyper 1.x has the expected parents**

```bash
cargo tree -i hyper@1 --features ims-s3 2>&1 | head -30
```

Expected: `aws-smithy-http-client` and `hyper-socks2` both appear as direct parents. If either is missing, the migration is incomplete.

- [ ] **Step 13: Run the crate's tests**

```bash
cargo test --features ims-s3 2>&1 | tail -30
```

Expected: all tests pass. There are no tests exercising the proxied S3 flow (that's Task 3), but the module must still link and existing unit tests must still pass.

- [ ] **Step 14: Run clippy with warnings-as-errors**

```bash
cargo clippy --features ims-s3 --all-targets -- -D warnings 2>&1 | tail -30
```

Expected: clean, no warnings. The type changes may introduce a clippy hint about redundant `.clone()` or an unused import — fix inline if so.

- [ ] **Step 15: Commit**

```bash
git add Cargo.toml Cargo.lock src/ims/s3_client.rs
git commit -m "feat(ims-s3): migrate S3 SOCKS5 path to hyper 1.x

Bump hyper-socks2 to 0.9.1 (upstream now declares hyper = \"1\"),
swap aws-sdk-s3's rustls feature for rustls-ring, and rewrite the
connector plumbing in setup_client to use aws-smithy-http-client's
hyper-1.x default-client. Removes hyper 0.14, hyper-rustls 0.24,
and h2 0.3 from the resolved dep tree.

Behavior of the SOCKS5-through-S3 flow is unchanged — same
Error::S3Transport surface, same S3_OPERATION_TIMEOUT.

Manual verification against a real proxied CSM env still required
before merge (see plan Task 3).

Design: docs/superpowers/specs/2026-07-12-s3-socks5-hyper1-migration-design.md"
```

---

### Task 3: Manual verification against a real proxied CSM environment

**Purpose:** The spec explicitly deferred automated testing; the migration's runtime correctness must be confirmed manually before the branch is merged. This task is the executor handing back to the human operator with a specific checklist.

**Files:** none (verification only).

**Interfaces:**
- Consumes: the committed migration from Task 2.
- Produces: a go/no-go signal on merge.

- [ ] **Step 1: Build a manta binary with these changes**

If this repo is being consumed by manta, in the manta workspace:

```bash
cargo build --release
```

Ensure the `csm-rs` dep points at the branch/commit that contains Task 2's commit.

- [ ] **Step 2: Configure a working SOCKS5 tunnel to a CSM cluster**

Whatever mechanism the operator normally uses — an SSH `-D` tunnel, a corporate proxy, etc. Confirm the tunnel is up before Step 3.

- [ ] **Step 3: Exercise the S3 download path**

Trigger an IMS image pull that goes through `csm-rs::ims::s3_client::setup_client` with `socks5_proxy = Some(...)`. Confirm the object arrives intact.

- [ ] **Step 4: Exercise the S3 upload path**

Trigger an IMS image push (the multi-GB path the 60-min `S3_OPERATION_TIMEOUT` at `src/ims/s3_client.rs:25` exists for). Confirm the upload completes without a spurious timeout or connector error.

- [ ] **Step 5: Failure-mode sanity check**

With a transfer in progress, stop the SOCKS5 tunnel. The transfer must fail as `Error::S3Transport(...)` — not panic, not hang past the operation timeout. Restart the tunnel before continuing.

- [ ] **Step 6: Sign off or roll back**

- **All of Steps 3–5 pass:** the branch is safe to merge.
- **Any of Steps 3–5 fail and the fix isn't obvious within a working afternoon:** revert the branch (`git revert <commit-from-Task-2>`). The current hyper 0.14 path is known-good and there is no external pressure forcing this migration.

---

## Post-migration follow-ups (separate PRs, not in this plan)

- Mechanical version bumps: `aws-sdk-s3 1.135 → 1.138`, `aws-config 1.8 → 1.9`, patch updates to `aws-smithy-runtime` / `aws-smithy-types`.
- Removing the direct `http = "1.3.1"` dep is *not* on the table — see the earlier investigation: `kube::Config::proxy_url` requires `http::Uri`, and no other in-scope crate re-exports it.
