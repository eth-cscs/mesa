# Remove `indicatif` and `humansize` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the `indicatif` and `humansize` dependencies from `csm-rs` while preserving byte-size and transfer-progress feedback via `log::info!` lines and a small in-crate `format_bytes` helper.

**Architecture:** Introduce `pub(crate) fn format_bytes(u64) -> String` in a new `src/fmt.rs`; migrate the two `humansize` call sites and the three `indicatif` progress-bar call sites to `log::info!` lines emitting `"<verb> {done}/{total} bytes"`; then drop both crates from `Cargo.toml`.

**Tech Stack:** Rust 2024, `cargo`, `log` (already a dep at 0.4.32).

## Global Constraints

- Language: Rust 2024.
- No new dependencies. Only the `log` crate (already present) is used to replace behaviour.
- No changes to public function signatures. Only internal reporting behaviour changes.
- Every task ends with a green `cargo check --all-features` and, for touched Rust files, a green `cargo clippy --all-features -- -D warnings`.
- Follow existing formatting: 2-space indent, `pub(crate)` for internal helpers, `///` docstrings on any new item (crate has `#![warn(missing_docs)]` and `#![warn(clippy::pedantic)]`).
- Commit messages follow the crate's Conventional Commits style: `chore(deps): …`, `refactor(…): …`, `test(…): …`, etc. (see `git log --oneline`).

---

### Task 1: Add `format_bytes` helper with unit tests

**Files:**
- Create: `src/fmt.rs`
- Modify: `src/lib.rs` (add `pub(crate) mod fmt;` to the module declarations block near line 116-132)

**Interfaces:**
- Produces: `pub(crate) fn format_bytes(bytes: u64) -> String` — returns a decimal-unit size string ("0 B", "999 B", "1.0 kB", "1.5 kB", "1.5 MB", "1.5 GB", "1.5 TB"). Used by Task 2.

- [ ] **Step 1: Write the failing test**

Create the whole file `src/fmt.rs` with the test skeleton first (implementation stub returns `String::new()` so the test compiles but fails):

```rust
//! Byte-size formatting helpers used by the migrate commands and the
//! S3 client. Replaces the previous dependency on the `humansize` crate.

/// Format `bytes` as a decimal-unit size string.
///
/// Uses SI units (`B`, `kB`, `MB`, `GB`, `TB`) with one decimal place
/// for any unit above bytes. Values below `1_000` are rendered as
/// integer bytes (e.g. `999` → `"999 B"`).
#[must_use]
pub(crate) fn format_bytes(_bytes: u64) -> String {
  String::new()
}

#[cfg(test)]
mod tests {
  use super::format_bytes;

  #[test]
  fn matches_decimal_shape() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(999), "999 B");
    assert_eq!(format_bytes(1_000), "1.0 kB");
    assert_eq!(format_bytes(1_500), "1.5 kB");
    assert_eq!(format_bytes(1_500_000), "1.5 MB");
    assert_eq!(format_bytes(1_500_000_000), "1.5 GB");
    assert_eq!(format_bytes(1_500_000_000_000), "1.5 TB");
  }
}
```

Then wire the module into `src/lib.rs`. Add this line next to the other module declarations (right after `pub(crate) mod common;` around line 127):

```rust
pub(crate) mod fmt;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib fmt::tests::matches_decimal_shape`
Expected: FAIL (`assertion \`left == right\` failed` — actual is `""`).

- [ ] **Step 3: Implement `format_bytes`**

Replace the body of `format_bytes` in `src/fmt.rs`:

```rust
#[must_use]
pub(crate) fn format_bytes(bytes: u64) -> String {
  const UNITS: [&str; 4] = ["kB", "MB", "GB", "TB"];
  if bytes < 1_000 {
    return format!("{bytes} B");
  }
  let mut value = bytes as f64 / 1_000.0;
  let mut unit_index = 0;
  while value >= 1_000.0 && unit_index < UNITS.len() - 1 {
    value /= 1_000.0;
    unit_index += 1;
  }
  format!("{value:.1} {}", UNITS[unit_index])
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib fmt::tests::matches_decimal_shape`
Expected: PASS.

Also run the full check to ensure no clippy regressions:
Run: `cargo clippy --all-features --lib -- -D warnings`
Expected: no warnings on the new module.

- [ ] **Step 5: Commit**

```bash
git add src/fmt.rs src/lib.rs
git commit -m "feat(fmt): add in-crate format_bytes helper

Replaces the humansize::format_size(_, DECIMAL) call sites that will be
migrated in the following commits."
```

---

### Task 2: Migrate `humansize` call sites to `format_bytes`

**Files:**
- Modify: `src/commands/migrate_backup.rs` (l.6 and l.239)
- Modify: `src/commands/migrate_restore.rs` (l.11, l.545, l.765)

**Interfaces:**
- Consumes: `crate::fmt::format_bytes` from Task 1.
- Produces: nothing new. `humansize` is unused in source after this task; the `Cargo.toml` line is dropped in Task 5.

- [ ] **Step 1: Update `src/commands/migrate_backup.rs`**

At line 6, remove:

```rust
use humansize::DECIMAL;
```

At line 239, change:

```rust
humansize::format_size(object_size as u64, DECIMAL),
```

to:

```rust
crate::fmt::format_bytes(object_size as u64),
```

- [ ] **Step 2: Update `src/commands/migrate_restore.rs`**

At line 11, remove:

```rust
use humansize::DECIMAL;
```

At line 545, change:

```rust
Ok(file_metadata) => humansize::format_size(file_metadata.len(), DECIMAL),
```

to:

```rust
Ok(file_metadata) => crate::fmt::format_bytes(file_metadata.len()),
```

At line 765, change:

```rust
Ok(file_metadata) => humansize::format_size(file_metadata.len(), DECIMAL),
```

to:

```rust
Ok(file_metadata) => crate::fmt::format_bytes(file_metadata.len()),
```

Leave the two `indicatif` imports and the `BAR_FORMAT` import in `migrate_restore.rs` alone for now — Task 3 handles them.

- [ ] **Step 3: Verify the crate still compiles**

Run: `cargo check --all-features`
Expected: clean build.

Run: `cargo clippy --all-features -- -D warnings`
Expected: no new warnings. If clippy complains that `humansize` in `Cargo.toml` is now an "unused dependency", that is expected and will be resolved in Task 5.

- [ ] **Step 4: Commit**

```bash
git add src/commands/migrate_backup.rs src/commands/migrate_restore.rs
git commit -m "refactor(commands): swap humansize::format_size for crate::fmt::format_bytes"
```

---

### Task 3: Replace `indicatif` progress bar in `migrate_restore.rs`

**Files:**
- Modify: `src/commands/migrate_restore.rs` (l.8, l.12, l.727-731, l.750, l.753)

**Interfaces:**
- Consumes: `log::info!` (via existing crate-level `log` dep).
- Produces: nothing new. `indicatif` still lives in `s3_client.rs` after this task; Task 4 finishes the migration.

Context: the affected function is `file_md5sum`. It reads a file in chunks to compute an MD5, and previously drew a byte-progress bar over the file length.

- [ ] **Step 1: Remove the two indicatif-related imports**

At the top of `src/commands/migrate_restore.rs`:

Delete line 8 (`use crate::ims::s3_client::BAR_FORMAT;`).

Delete line 12 (`use indicatif::{ProgressBar, ProgressStyle};`).

- [ ] **Step 2: Rewrite the progress-bar block inside `file_md5sum`**

Locate the block starting at line 727:

```rust
  let bar = ProgressBar::new(len);
  // BAR_FORMAT is a compile-time constant — template parse is infallible.
  bar.set_style(
    ProgressStyle::with_template(BAR_FORMAT).expect("BAR_FORMAT is valid"),
  );
```

Replace it with:

```rust
  let mut restored: u64 = 0;
```

Locate `bar.inc(part_len as u64);` (line 750) and replace it with:

```rust
    restored += part_len as u64;
    log::info!("restored {restored}/{len} bytes");
```

Locate `bar.finish();` (line 753) and delete that line entirely. (The final `log::info!` line already emitted in the loop at `restored == len` covers the completion signal.)

- [ ] **Step 3: Verify the crate still compiles**

Run: `cargo check --all-features`
Expected: clean build.

Run: `cargo clippy --all-features -- -D warnings`
Expected: no new warnings. Note: `pub const BAR_FORMAT` in `s3_client.rs` is now unused by the rest of the workspace but still `pub`, so clippy will not warn on it. Task 4 removes it.

- [ ] **Step 4: Commit**

```bash
git add src/commands/migrate_restore.rs
git commit -m "refactor(migrate_restore): replace indicatif progress bar with log::info! lines"
```

---

### Task 4: Replace `indicatif` progress bars in `s3_client.rs`

**Files:**
- Modify: `src/ims/s3_client.rs` (l.12, l.47-49, l.146, l.214-236, l.325-486)

**Interfaces:**
- Consumes: `log::info!`.
- Produces: nothing new. After this task all `indicatif` references are gone from the crate source; Task 5 removes the `Cargo.toml` line.

- [ ] **Step 1: Remove the indicatif import and the `BAR_FORMAT` constant**

At the top of `src/ims/s3_client.rs`:

Delete line 12 (`use indicatif::{ProgressBar, ProgressStyle};`).

Delete lines 47-49 (both the doc comment and the `pub const BAR_FORMAT` declaration):

```rust
/// `indicatif` progress bar template used by the S3 upload/download
/// helpers in this module.
pub const BAR_FORMAT: &str = "[{elapsed_precise}] {bar:40.cyan/blue} ({bytes_per_sec}) {bytes:>7}/{total_bytes:7} {msg} [ETA {eta}]";
```

- [ ] **Step 2: Update the download path (function containing lines 214-236)**

Locate the block:

```rust
  let bar_size = object.content_length().ok_or_else(|| {
    Error::S3Transport("could not get S3 object size.".to_string())
  })?;

  let bar = ProgressBar::new(bar_size as u64);
  bar.set_style(ProgressStyle::with_template(BAR_FORMAT).map_err(|e| {
    Error::S3Transport(format!(
      "ERROR - Could not create progress bar.\nReason:\n{e}"
    ))
  })?);

  while let Some(bytes) = object.body.try_next().await.map_err(|e| {
    Error::S3Transport(format!(
      "ERROR - Could not finish s3 object download.\nReason:\n{e}"
    ))
  })? {
    let bytes = file.write(&bytes)?;
    bar.inc(bytes as u64);
  }

  bar.finish();
```

Replace it with:

```rust
  let bar_size = object.content_length().ok_or_else(|| {
    Error::S3Transport("could not get S3 object size.".to_string())
  })?;
  let total = bar_size as u64;
  let mut downloaded: u64 = 0;

  while let Some(bytes) = object.body.try_next().await.map_err(|e| {
    Error::S3Transport(format!(
      "ERROR - Could not finish s3 object download.\nReason:\n{e}"
    ))
  })? {
    let bytes = file.write(&bytes)?;
    downloaded += bytes as u64;
    log::info!("downloaded {downloaded}/{total} bytes");
  }
```

- [ ] **Step 3: Update the multipart-upload path (function containing lines 395-481)**

Locate the block at line 395:

```rust
  let bar = ProgressBar::new(file_size);
  bar.set_style(ProgressStyle::with_template(BAR_FORMAT).map_err(|e| {
    Error::S3Transport(format!(
      "ERROR - Could not create progress bar.\nReason:\n{e}"
    ))
  })?);
```

Replace it with:

```rust
  let mut uploaded: u64 = 0;
```

Locate line 461 (`bar.inc(this_chunk);`) inside the for-loop and replace it with:

```rust
    uploaded += this_chunk;
    log::info!("uploaded {uploaded}/{file_size} bytes");
```

Locate line 481 (`bar.finish();`) after the `complete_multipart_upload` block, and delete that line entirely.

- [ ] **Step 4: Update the two doc comments that mention "progress bar"**

At line 146 (the doc comment on `s3_download_object`) change:

```rust
/// Streams the object body to disk with a progress bar. Returns the
```

to:

```rust
/// Streams the object body to disk, logging byte progress. Returns the
```

At line 327 (the doc comment on `s3_multipart_upload_object`) change:

```rust
/// Splits `file_path` into chunks and uploads them with a progress bar.
```

to:

```rust
/// Splits `file_path` into chunks and uploads them, logging byte progress.
```

- [ ] **Step 5: Verify the crate still compiles**

Run: `cargo check --all-features`
Expected: clean build.

Run: `cargo check --no-default-features`
Expected: clean build (the S3 feature is off, so the `s3_client` module is compiled out).

Run: `cargo clippy --all-features -- -D warnings`
Expected: no new warnings.

- [ ] **Step 6: Commit**

```bash
git add src/ims/s3_client.rs
git commit -m "refactor(s3_client): replace indicatif progress bars with log::info! lines

Drops the pub const BAR_FORMAT template and both ProgressBar usages
(download and multipart upload)."
```

---

### Task 5: Drop `indicatif` and `humansize` from `Cargo.toml`

**Files:**
- Modify: `Cargo.toml` (l.52, l.54-57, l.95, l.106)

**Interfaces:** none — this is a build-config change.

- [ ] **Step 1: Update the `ims-s3` feature block**

In `Cargo.toml`, locate lines 51-57:

```toml
# IMS S3 image transport (`ims::s3_client`). Pulls in `aws-sdk-s3`,
# `aws-config`, `aws-smithy-*`, `indicatif`. Uses the AWS SDK's default
# rustls HTTP client. Default-on for backwards compatibility.
ims-s3 = [
    "dep:aws-sdk-s3", "dep:aws-config", "dep:aws-smithy-runtime",
    "dep:aws-smithy-types", "dep:indicatif",
]
```

Replace with:

```toml
# IMS S3 image transport (`ims::s3_client`). Pulls in `aws-sdk-s3`,
# `aws-config`, `aws-smithy-*`. Uses the AWS SDK's default
# rustls HTTP client. Default-on for backwards compatibility.
ims-s3 = [
    "dep:aws-sdk-s3", "dep:aws-config", "dep:aws-smithy-runtime",
    "dep:aws-smithy-types",
]
```

- [ ] **Step 2: Remove the `indicatif` dependency line**

Delete line 95:

```toml
indicatif = { version = "0.18", default-features = false, optional = true }
```

- [ ] **Step 3: Remove the `humansize` dependency line**

Delete line 106:

```toml
humansize = "2.1.3"
```

- [ ] **Step 4: Update `Cargo.lock` and verify the workspace still builds cleanly**

Run: `cargo check --all-features`
Expected: clean build. `Cargo.lock` is updated to drop `indicatif` and `humansize` (and any of their transitive-only deps).

Run: `cargo check --no-default-features`
Expected: clean build.

Run: `cargo clippy --all-features -- -D warnings`
Expected: no warnings.

Run: `cargo test --lib fmt::tests::matches_decimal_shape`
Expected: PASS (regression guard on `format_bytes` output shape).

- [ ] **Step 5: Confirm the removal is complete**

Run: `grep -rn "indicatif\|humansize" src Cargo.toml Cargo.lock | grep -v "^Cargo.lock:"`
Expected: no matches. (The `grep -v Cargo.lock` filter is only to make the output easy to read; `cargo check` in Step 4 already updated the lockfile.)

If any match remains, revert to the corresponding task and finish the cleanup.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore(deps): drop indicatif and humansize

All progress-bar and byte-formatting call sites now use log::info! lines
and the in-crate fmt::format_bytes helper."
```

---

## Self-Review

**Spec coverage** — every §3 sub-section is covered:
- §3.1 (`s3_client.rs`): Task 4 (indicatif import, `BAR_FORMAT`, download, multipart upload).
- §3.2 (`migrate_restore.rs`): Task 2 (humansize sites) + Task 3 (indicatif progress bar + `BAR_FORMAT` import).
- §3.3 (`migrate_backup.rs`): Task 2 (humansize site).
- §3.4 (`Cargo.toml`): Task 5.
- §3.5 (`src/lib.rs`): Task 1.
- §4 (verification): Task 1 Step 4, Task 2 Step 3, Task 3 Step 3, Task 4 Step 5, Task 5 Step 4.

**Placeholder scan** — no TBD/TODO/"handle edge cases" text; every code step contains the actual code; every command step has an expected outcome.

**Type consistency** — `crate::fmt::format_bytes` is used identically in Task 1 (definition), Task 2 (call sites), and unchanged in later tasks. All accumulator identifiers (`downloaded`, `uploaded`, `restored`) are local to their function and never referenced across tasks.
