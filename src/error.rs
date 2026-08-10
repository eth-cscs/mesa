//! Crate-wide error type.
//!
//! Every fallible call in csm-rs returns `Result<T, `[`Error`]`>`. The
//! variants fall roughly into three groups:
//!
//! - **Infrastructure errors** ([`Error::IoError`], [`Error::NetError`],
//!   [`Error::TokioError`], …) — propagated from the standard library or
//!   from third-party crates via `#[from]`.
//! - **HTTP/CSM errors** ([`Error::RequestError`], [`Error::CsmError`]) —
//!   raised when CSM returns a non-2xx response.
//! - **Domain errors** (e.g. [`Error::ConfigurationAlreadyExists`],
//!   [`Error::GroupNotFound`], [`Error::ImageNotFound`]) — distinguished
//!   variants for cases callers commonly want to branch on instead of
//!   string-matching against a generic [`Error::Message`].
//!
//! [`Error::Message`] is the catch-all "something went wrong, here's a
//! string" variant; prefer a specific variant when one exists so callers
//! can act on it programmatically.

use std::{env::VarError, io, num::ParseIntError, str::Utf8Error};

#[cfg(feature = "ims-s3")]
use aws_smithy_types::byte_stream;
use globset::Error as GlobsetError;
#[cfg(feature = "manta-dispatcher")]
use manta_backend_dispatcher::error::Error as MantaError;
use serde_json::Value;
use tokio::task::JoinError;

/// Errors returned by any csm-rs call.
///
/// See the [module docs][self] for a high-level grouping of variants.
/// The `#[error("…")]` message on each variant is the canonical
/// description; per-variant rustdoc is intentionally omitted.
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum Error {
  #[error("CSM-RS > Generic error: {0}")]
  Message(String),
  #[error("CSM-RS > Environment variable: {0}")]
  EnvVarError(#[from] VarError),
  #[error("CSM-RS > IO: {0}")]
  IoError(#[from] io::Error),
  #[error("CSM-RS > Serde JSON: {0}")]
  SerdeJsonError(#[from] serde_json::Error),
  #[error("CSM-RS > Serde YAML: {0}")]
  SerdeYamlError(#[from] serde_yaml::Error),
  #[error("CSM-RS > Net: {0}")]
  NetError(#[from] reqwest::Error),
  #[error("CSM-RS > Tokio: {0}")]
  TokioError(#[from] JoinError),
  #[error("CSM-RS > Error converting from UTF8 to String: {0}")]
  UtfError(#[from] Utf8Error),
  #[error("CSM-RS > Glob error: {0}")]
  GlobError(#[from] GlobsetError),
  #[error("CSM-RS > Parse int error: {0}")]
  ParseStrIntError(#[from] ParseIntError),
  #[cfg(feature = "ims-s3")]
  #[error("CSM-RS > URL parse error: {0}")]
  SmithyDataStreamError(#[from] byte_stream::error::Error),
  #[error(
    "http request:\nurl: {url}\nresponse: {response}\npayload: {payload}"
  )]
  RequestError {
    response: reqwest::Error,
    /// URL that returned the error. Captured for log-correlation so an
    /// operator seeing a 401 can grep production logs for the endpoint
    /// that rejected the token. Same role as `CsmError::{method, url}`.
    url: String,
    payload: String, // NOTE: CSM/OCHAMI Apis either returns plain text or a json therefore, we
                     // will just return a String
  },
  /// Structured error payload returned by CSM/HSM endpoints when an
  /// HTTP request fails. `method` and `url` carry the request context
  /// (so operators grepping a production log can correlate the error
  /// with the CSM endpoint that returned it). `status` is the HTTP
  /// status code, `detail` is the human-readable message extracted
  /// from the RFC 7807 `Problem7807` body (`detail` field, falling
  /// back to `title`), and `body` retains the raw JSON so callers
  /// needing extension fields can still reach them without
  /// string-parsing the Display output.
  #[error("CSM-RS > CSM: {method} {url} -> status={status} {detail}")]
  CsmError {
    method: String,
    url: String,
    status: u16,
    detail: String,
    body: Option<Value>,
  },
  #[error("CSM-RS > Console: {0}")]
  ConsoleError(String),
  #[error("CSM-RS > K8s: {0}")]
  K8sError(String),
  #[error("CSM-RS > K8s: field '{0}' missing in k8s credentials")]
  K8sCredentialMissingError(String),
  #[error("CSM-RS > K8s: '{0}' value not a string")]
  K8sCredentialNotStringError(String),
  #[cfg(feature = "k8s-console")]
  #[error("CSM-RS > K8s: {0}")]
  K8sExecError(#[from] kube::Error),
  #[error("CSM-RS > Image '{0}' not found")]
  ImageNotFound(String),
  #[error("CSM-RS > Group '{0}' not found")]
  GroupNotFound(String),
  #[error("CSM-RS > No derivatives found for CFS Configuration: {0}")]
  ConfigurationDerivativesNotFound(String),
  #[error("CSM-RS > Configuration '{0}' does not have a name defined")]
  ConfigurationNameNotDefined(String),
  #[error("CSM-RS > CFS Configuration already exists: {0}")]
  ConfigurationAlreadyExists(String),
  #[error(
    "CSM-RS > CFS Configuration used as a runtime configuration for a cluster and/or used to build an image used to boot node(s)"
  )]
  ConfigurationUsedAsRuntimeConfigurationOrUsedToBuildBootImageUsed,
  #[error("CSM-RS > Session '{0}' not found")]
  SessionNotFound(String),
  #[error("CSM-RS > Session '{0}' does not have a name defined")]
  SessionNameNotDefined(String),
  #[error("CSM-RS > Session '{0}' does not have a configuration defined")]
  SessionConfigurationNotDefined(String),
  #[error("CSM-RS > IMS key '{0}' not found")]
  ImsKeyNotFound(String),
  #[error("CSM-RS > HSM component '{0}' not found")]
  HsmComponentNotFound(String),
  #[error("CSM-RS > HSM component '{0}' does not have a ID defined")]
  HsmComponentIdNotDefined(String),
  #[error("CSM-RS > HSM component '{0}' does not have a NID defined")]
  HsmComponentNidNotDefined(String),
  #[error("CSM-RS > HSM component '{0}' does not have a power state defined")]
  HsmComponentPowerStateNotDefined(String),
  #[error("CSM-RS > HSM component '{0}' does not have a '{1}' defined")]
  HsmComponentFieldNotDefined(String, String),
  #[error("CSM-RS > CFS component field '{0}' not defined")]
  CfsComponentFieldNotDefined(String),
  #[error("CSM-RS > CFS component does not have a 'name' defined")]
  CfsComponentNameFieldNotDefined(),
  #[error("CSM-RS > CFS component does not have a 'desired_conf' defined")]
  CfsComponentDesiredConfFieldNotDefined(),
  /// YAML payload didn't match the expected shape (e.g. a SAT-file
  /// field is missing or has the wrong type). The string is a
  /// human-readable description naming the offending field.
  #[error("CSM-RS > YAML shape: {0}")]
  YamlShape(String),
  /// An HSM hardware-inventory response didn't decode into the
  /// expected typed shape. The string names the field or context.
  #[error("CSM-RS > HSM hardware inventory: {0}")]
  HsmInventoryShape(String),
  /// S3 transport-level error encountered by the IMS S3 client
  /// (auth, upload, download, `ETag` retrieval, ...). The string
  /// describes the failing operation; underlying SDK errors are
  /// folded in as context. Gated by the `ims-s3` Cargo feature on
  /// the `SmithyDataStreamError` chain, but the variant itself is
  /// always present so callers don't need feature-aware matches.
  #[error("CSM-RS > S3: {0}")]
  S3Transport(String),
  /// Cray product-catalog lookup failed (entry missing, multiple
  /// entries when one was expected, or a required field absent).
  /// The string carries the lookup context (product name, image
  /// name, version, etc.).
  #[error("CSM-RS > Cray product catalog: {0}")]
  CrayProductCatalog(String),
  /// SAT-file processing error: validation failure, unrecognised
  /// section shape, referenced image/configuration missing, or an
  /// HSM-group access check rejecting the file. The string
  /// describes what's wrong; the caller's UX surface is "tell the
  /// user to fix their SAT file".
  #[error("CSM-RS > SAT file: {0}")]
  SatFile(String),
  /// Error encountered by the migrate-backup / migrate-restore
  /// workflows under the `commands-admin` feature: BOS template
  /// shape, IMS bundle parsing, local file I/O, missing CLI
  /// argument, etc. The string carries the operation context.
  #[error("CSM-RS > Migrate: {0}")]
  MigrateOp(String),
  /// A Gitea / git-repo API response didn't decode into the
  /// expected shape. Used when extracting `ref`, `commit/sha`,
  /// `object/type`, tag name/url, etc. from a Gitea response and
  /// the field isn't present or has the wrong type.
  #[error("CSM-RS > Git repo shape: {0}")]
  GitRepoShape(String),
  /// Caller-input validation failed: required field missing,
  /// argument outside the expected shape, no NID/XName found in a
  /// component lookup, etc. The string is a static description of
  /// what the caller got wrong.
  #[error("CSM-RS > Validation failed: {0}")]
  ValidationFailed(&'static str),
  /// Failure attaching to or executing on a node serial console pod
  /// via Kubernetes (`cray-console-node`). Carries the pod name and
  /// the underlying error message so operators can correlate the
  /// failure with kube-side logs.
  #[error("CSM-RS > Console attach failed for pod {pod}: {cause}")]
  ConsoleAttach { pod: String, cause: String },
  /// Workflow-level failure inside the `apply_session` command (no
  /// HSM group provided, target nodes already busy, CFS session name
  /// missing, etc.). The string names the specific workflow-state
  /// violation.
  #[error("CSM-RS > Apply session: {0}")]
  ApplySession(String),
  /// Non-JSON CSM error payload (some CSM endpoints return plain text
  /// on non-2xx). Mirror of [`Error::CsmError`] for endpoints whose
  /// failure bodies don't follow RFC 7807; carries the same
  /// `{method, url, status}` context plus the raw text.
  #[error(
    "CSM-RS > CSM (text): {method} {url} -> status={status} {payload}"
  )]
  CsmText {
    method: String,
    url: String,
    status: u16,
    payload: String,
  },
  /// JWT decoding or claim-shape failure: base64 decode failed, the
  /// payload isn't UTF-8 / valid JSON, or an expected claim is
  /// missing or has the wrong type.
  #[error("CSM-RS > JWT: {0}")]
  JwtShape(&'static str),
}

impl Error {
  /// Build a [`CsmError`](Error::CsmError) from request context and a
  /// non-success HTTP response. Extracts the RFC 7807 `detail` field
  /// (falling back to `title`, then empty) and keeps the raw payload
  /// available via `body`. `method` and `url` are stored so the
  /// resulting `Display` output names the endpoint that failed.
  pub(crate) fn csm_from_response(
    method: &str,
    url: &str,
    status: u16,
    payload: Value,
  ) -> Self {
    let detail = payload
      .get("detail")
      .and_then(Value::as_str)
      .or_else(|| payload.get("title").and_then(Value::as_str))
      .map(str::to_string)
      .unwrap_or_default();
    Error::CsmError {
      method: method.to_string(),
      url: url.to_string(),
      status,
      detail,
      body: Some(payload),
    }
  }

  /// Build a [`CsmText`](Error::CsmText) from request context and a
  /// non-success, non-JSON response payload. Used where CSM returns
  /// plain text on error (some HSM endpoints).
  pub(crate) fn csm_text_from_response(
    method: &str,
    url: &str,
    status: u16,
    payload: String,
  ) -> Self {
    Error::CsmText {
      method: method.to_string(),
      url: url.to_string(),
      status,
      payload,
    }
  }
}

// Convert Error to manta_backend_dispatcher::error::Error.
//
// This match is intentionally exhaustive (no `_` arm) so that adding a
// new csm-rs Error variant forces an explicit decision about how it
// surfaces across the dispatcher boundary, rather than silently
// collapsing to MantaError::Message.
#[cfg(feature = "manta-dispatcher")]
impl From<crate::error::Error> for MantaError {
  fn from(val: crate::error::Error) -> Self {
    match val {
      // Pass-through infrastructure errors with direct dispatcher equivalents.
      Error::IoError(e) => MantaError::IoError(e),
      Error::SerdeJsonError(e) => MantaError::SerdeError(e),
      Error::SerdeYamlError(e) => MantaError::YamlError(e),
      Error::NetError(e) => MantaError::NetError(e),
      Error::RequestError {
        response,
        url,
        payload,
      } => MantaError::RequestError {
        response,
        // The dispatcher's `RequestError` variant only carries
        // `{response, payload}`; fold our `url` into the start of the
        // payload so it survives the boundary. Lift to a dedicated
        // field when manta-backend-dispatcher gains one.
        payload: format!("url: {url}\npayload: {payload}"),
      },
      Error::CsmError {
        method,
        url,
        status,
        detail,
        body,
      } => MantaError::CsmError {
        status,
        // Fold method+url into the dispatcher-side detail so the
        // endpoint that failed is still visible across the boundary
        // (manta-backend-dispatcher's CsmError variant currently only
        // carries {status, detail, body}; lift this if the dispatcher
        // gains structured fields).
        detail: format!("{method} {url} -> {detail}"),
        body,
      },

      // Direct 1:1 dispatcher variants.
      Error::Message(s) => MantaError::Message(s),
      Error::ConsoleError(s) => MantaError::ConsoleError(s),
      Error::ConfigurationAlreadyExists(s) => {
        MantaError::ConfigurationAlreadyExistsError(s)
      }
      Error::SessionNotFound(_) => MantaError::SessionNotFound,
      Error::ConfigurationUsedAsRuntimeConfigurationOrUsedToBuildBootImageUsed => {
        MantaError::Conflict(
          Error::ConfigurationUsedAsRuntimeConfigurationOrUsedToBuildBootImageUsed
            .to_string(),
        )
      }

      // Not-found variants — fold into the generic NotFound carrying a
      // human-readable subject so dispatcher callers can branch on
      // NotFound vs. other failure classes.
      Error::ImageNotFound(s) => MantaError::NotFound(format!("Image '{s}'")),
      Error::GroupNotFound(s) => MantaError::NotFound(format!("Group '{s}'")),
      Error::HsmComponentNotFound(s) => {
        MantaError::NotFound(format!("HSM component '{s}'"))
      }
      Error::ImsKeyNotFound(s) => MantaError::NotFound(format!("IMS key '{s}'")),
      Error::ConfigurationDerivativesNotFound(s) => {
        MantaError::NotFound(format!("No derivatives for CFS configuration '{s}'"))
      }

      // Missing-field variants — fold into MissingField so dispatcher
      // callers can distinguish "structural" failures (a required field
      // wasn't present) from network/IO/not-found.
      Error::ConfigurationNameNotDefined(s) => {
        MantaError::MissingField(format!("CFS configuration '{s}' name"))
      }
      Error::SessionNameNotDefined(s) => {
        MantaError::MissingField(format!("session '{s}' name"))
      }
      Error::SessionConfigurationNotDefined(s) => {
        MantaError::MissingField(format!("session '{s}' configuration"))
      }
      Error::HsmComponentIdNotDefined(s) => {
        MantaError::MissingField(format!("HSM component '{s}' ID"))
      }
      Error::HsmComponentNidNotDefined(s) => {
        MantaError::MissingField(format!("HSM component '{s}' NID"))
      }
      Error::HsmComponentPowerStateNotDefined(s) => {
        MantaError::MissingField(format!("HSM component '{s}' power state"))
      }
      Error::HsmComponentFieldNotDefined(c, f) => {
        MantaError::MissingField(format!("HSM component '{c}' field '{f}'"))
      }
      Error::CfsComponentFieldNotDefined(s) => {
        MantaError::MissingField(format!("CFS component field '{s}'"))
      }
      Error::CfsComponentNameFieldNotDefined() => {
        MantaError::MissingField("CFS component 'name'".to_string())
      }
      Error::CfsComponentDesiredConfFieldNotDefined() => {
        MantaError::MissingField("CFS component 'desired_conf'".to_string())
      }

      // K8s variants — fold into the dispatcher's single K8sError with
      // a human-readable subject preserved.
      Error::K8sError(s) => MantaError::K8sError(s),
      Error::K8sCredentialMissingError(s) => {
        MantaError::K8sError(format!("field '{s}' missing in k8s credentials"))
      }
      Error::K8sCredentialNotStringError(s) => {
        MantaError::K8sError(format!("'{s}' value not a string"))
      }
      #[cfg(feature = "k8s-console")]
      Error::K8sExecError(e) => MantaError::K8sError(e.to_string()),

      // Infrastructure / third-party errors with no dispatcher
      // equivalent. Preserve the csm-rs Display output (which carries
      // the "CSM-RS > …" context prefix) rather than just the inner
      // error's message.
      e @ Error::EnvVarError(_) => MantaError::Message(e.to_string()),
      e @ Error::TokioError(_) => MantaError::Message(e.to_string()),
      e @ Error::UtfError(_) => MantaError::Message(e.to_string()),
      e @ Error::GlobError(_) => MantaError::Message(e.to_string()),
      e @ Error::ParseStrIntError(_) => MantaError::Message(e.to_string()),
      #[cfg(feature = "ims-s3")]
      e @ Error::SmithyDataStreamError(_) => MantaError::Message(e.to_string()),

      // New shape/format errors fold into MissingField since the dispatcher
      // doesn't yet have richer structural variants for them.
      Error::YamlShape(s) => MantaError::MissingField(format!("YAML: {s}")),
      Error::HsmInventoryShape(s) => {
        MantaError::MissingField(format!("HSM inventory: {s}"))
      }
      Error::S3Transport(s) => MantaError::Message(format!("S3: {s}")),
      Error::CrayProductCatalog(s) => {
        MantaError::NotFound(format!("Cray product catalog: {s}"))
      }
      Error::SatFile(s) => MantaError::Message(format!("SAT file: {s}")),
      Error::MigrateOp(s) => MantaError::Message(format!("Migrate: {s}")),
      Error::GitRepoShape(s) => {
        MantaError::MissingField(format!("git repo: {s}"))
      }
      Error::ValidationFailed(s) => {
        MantaError::Message(format!("Validation: {s}"))
      }
      Error::ConsoleAttach { pod, cause } => MantaError::ConsoleError(format!(
        "console attach failed for pod {pod}: {cause}"
      )),
      Error::ApplySession(s) => {
        MantaError::Message(format!("Apply session: {s}"))
      }
      Error::CsmText {
        method,
        url,
        status,
        payload,
      } => MantaError::CsmError {
        status,
        // Fold method+url+payload into the dispatcher's `detail`
        // since MantaError::CsmError carries `{status, detail, body}`.
        // `body` stays None — there's no JSON to surface.
        detail: format!("{method} {url} -> {payload}"),
        body: None,
      },
      Error::JwtShape(s) => MantaError::Message(format!("JWT: {s}")),
    }
  }
}
