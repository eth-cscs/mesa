# Remove `indicatif` and `humansize` dependencies from `csm-rs`

**Date:** 2026-07-12
**Status:** Approved — ready for implementation plan
**Owner:** Manuel Sopena Ballesteros

## 1. Goal & scope

Drop the `indicatif` and `humansize` dependencies from the `csm-rs` crate. Preserve *some* transfer/progress and human-size feedback by emitting `log::info!` lines (the crate already uses the `log` facade throughout its command and HTTP modules).

Public function signatures do not change. Only the internal reporting behaviour of the S3 upload/download paths and the migrate-backup/migrate-restore commands changes.

**Out of scope**

- Swapping `log` → `tracing`.
- Any change to S3 chunking, retry, or auth behaviour.
- Any change to the migrate-backup / migrate-restore control flow, error handling, or on-disk layout.
- Introducing a callback- or channel-based progress API for callers.

## 2. Replacement helper

One in-crate helper is introduced.

- **`pub(crate) fn format_bytes(n: u64) -> String`**
  - Replaces `humansize::format_size(n, humansize::DECIMAL)`.
  - Decimal units: `B`, `kB`, `MB`, `GB`, `TB`.
  - One decimal place, matching `humansize`'s `DECIMAL` output shape closely enough that existing log messages stay readable (e.g. `"1.5 GB"`).
  - Location: new file `src/fmt.rs`. Registered in `src/lib.rs` as `pub(crate) mod fmt;`.

No progress-logger struct is introduced. Every site that today calls `ProgressBar::inc(chunk_len)` becomes a plain `log::info!` line on each chunk update (see §3). No throttling.

## 3. Per-call-site changes

### 3.1 `src/ims/s3_client.rs`

- Remove `use indicatif::{ProgressBar, ProgressStyle};` (l.12).
- Remove the `BAR_FORMAT` const and its `/// indicatif …` doc comment (l.47-…).
- **Upload path** (currently around l.218):
  - Delete `let bar = ProgressBar::new(bar_size as u64);` and the following `bar.set_style(...)` block.
  - Maintain a local `let mut uploaded: u64 = 0;` accumulator.
  - On each chunk completion, replace `bar.inc(chunk_len)` with:
    ```rust
    uploaded += chunk_len;
    log::info!("uploaded {}/{} bytes", uploaded, total);
    ```
    where `total` is the `bar_size as u64` value the previous code passed to `ProgressBar::new`.
  - Any final `bar.finish_with_message(...)` (if present) is replaced with a single trailing `log::info!("upload complete: {}/{} bytes", uploaded, total);`.
- **Download path** (currently around l.395):
  - Same treatment as upload, with wording `"downloaded {}/{} bytes"` and `"download complete: {}/{} bytes"`.

### 3.2 `src/commands/migrate_restore.rs`

- Remove `use humansize::DECIMAL;` (l.11) and `use indicatif::{ProgressBar, ProgressStyle};` (l.12).
- **l.545:** `humansize::format_size(file_metadata.len(), DECIMAL)` → `crate::fmt::format_bytes(file_metadata.len())`.
- **l.765:** same replacement as l.545.
- **Restore loop** (currently around l.727):
  - Delete `let bar = ProgressBar::new(len);` and the `bar.set_style(ProgressStyle::with_template(BAR_FORMAT)…)` block.
  - Maintain a local `let mut restored: u64 = 0;` accumulator.
  - Each iteration that previously called `bar.inc(step)` becomes:
    ```rust
    restored += step;
    log::info!("restored {}/{} bytes", restored, len);
    ```

### 3.3 `src/commands/migrate_backup.rs`

- Remove `use humansize::DECIMAL;` (l.6).
- **l.239:** `humansize::format_size(object_size as u64, DECIMAL)` → `crate::fmt::format_bytes(object_size as u64)`.

### 3.4 `Cargo.toml`

- Remove `indicatif = { version = "0.18", default-features = false, optional = true }` (l.95).
- Remove the `"dep:indicatif"` entry from the S3 feature's dep list (l.56).
- Update the feature-block doc comment above (l.52) to drop the `indicatif` mention.
- Remove `humansize = "2.1.3"` (l.106).

### 3.5 `src/lib.rs`

- Add `pub(crate) mod fmt;` alongside the other module declarations.

## 4. Testing & verification

- `cargo check --no-default-features` — confirms the removals don't leave broken imports on the minimal build.
- `cargo check --all-features` — confirms S3 feature still compiles without `indicatif`.
- `cargo clippy --all-features -- -D warnings` — no new warnings.
- One unit test on `format_bytes` covering boundary values: `0`, `999`, `1_000`, `1_500`, `1_500_000`, `1_500_000_000`, `1_500_000_000_000`. The test asserts the exact string shape (`"0 B"`, `"999 B"`, `"1.0 kB"`, `"1.5 kB"`, `"1.5 MB"`, `"1.5 GB"`, `"1.5 TB"`) so future changes don't silently drift the log-line format.
- Manual verification: run a migrate-backup and a migrate-restore against a real CSM (or scoped-down dry-run) and inspect the resulting log output. Confirm:
  - No leftover ANSI escape codes or carriage-return artifacts.
  - `uploaded N/M bytes` / `downloaded N/M bytes` / `restored N/M bytes` lines appear as expected.
  - Size strings from `format_bytes` are formatted consistently with the pre-removal `humansize::DECIMAL` output.

No test is written for the per-chunk `log::info!` calls themselves — they are plain, unconditional log lines with no branching to cover.

## 5. Rollback

Both crates are pure additions: the removal is a single-commit revert if the log-based reporting proves too noisy in practice. If a follow-up decision emerges to add throttling, it can be layered onto `log::info!` without further dependency changes.
