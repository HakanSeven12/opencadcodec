//! Versioned storage of captured typed CAD records.
//!
//! Enable `typed-storage` to capture, inspect and store owned records without
//! retaining the live source document. Raw source records and private AcDs
//! storage are excluded. Capture is not a qualification for CAD writeback.
//!
//! ```
//! use acadrust::{CadDocument, storage::{StorageNamespace, TypedRecordContainer}};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let source = CadDocument::new();
//! let captured = TypedRecordContainer::capture_document(
//!     &source, StorageNamespace::new("source-A".into())?,
//! )?;
//! let bytes = captured.container.to_bytes()?;
//! drop(source);
//! let stored = TypedRecordContainer::from_bytes(&bytes)?;
//! assert_eq!(stored.metadata().namespace.as_str(), "source-A");
//! assert!(!stored.records().layers().is_empty());
//! assert_eq!(stored.capture_report(), &captured.report);
//! # Ok(())
//! # }
//! ```

mod capture;
mod envelope;
pub(crate) mod records;
mod strict;
mod wire;

use crate::CadDocument;
use records::RecordBody;
pub use records::{CapturedExtendedData, CapturedXDataValue, RecordViews, StorageDocumentContext};
use serde::{Deserialize, Serialize};

/// Caller-supplied opaque identity of the source namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageNamespace(String);

impl StorageNamespace {
    pub fn new(value: String) -> Result<Self, StorageError> {
        if value.is_empty() {
            return Err(StorageError::new(
                StorageErrorKind::InvalidNamespace,
                "namespace",
                "namespace must not be empty",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageMetadata {
    pub format_version: u32,
    pub kind: String,
    pub namespace: StorageNamespace,
    pub writer_codec_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentCoverage {
    CompleteTyped,
    PartialTyped,
    ShellOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureEvidence {
    pub location: String,
    pub coverage: ContentCoverage,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureReport {
    pub entries: Vec<CaptureEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageErrorKind {
    InvalidNamespace,
    UnsupportedVersion,
    UnsupportedKind,
    MalformedInput,
    DuplicateIdentity,
    RelationConflict,
    UnrepresentableContent,
    HandleSpaceExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageError {
    pub kind: StorageErrorKind,
    pub location: String,
    pub message: String,
}
impl StorageError {
    pub(crate) fn new(
        kind: StorageErrorKind,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            location: location.into(),
            message: message.into(),
        }
    }
    pub(crate) fn malformed(location: &str, error: impl std::fmt::Display) -> Self {
        Self::new(
            StorageErrorKind::MalformedInput,
            location,
            error.to_string(),
        )
    }
}
impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}
impl std::error::Error for StorageError {}

#[derive(Debug, Clone)]
pub struct TypedRecordContainer {
    metadata: StorageMetadata,
    report: CaptureReport,
    body: RecordBody,
}

#[must_use]
#[derive(Debug, Clone)]
pub struct CapturedRecords {
    pub container: TypedRecordContainer,
    pub report: CaptureReport,
}

impl TypedRecordContainer {
    pub fn capture_document(
        source: &CadDocument,
        namespace: StorageNamespace,
    ) -> Result<CapturedRecords, StorageError> {
        let mut body = source.storage_records();
        let report = capture::prepare(source, &mut body);
        // A source-free roundtrip also clears the model's explicitly skipped
        // storage fields on every nested record, without mutating the source.
        let body = envelope::decode_body(&envelope::encode_body(&body)?)?;
        let container = Self {
            metadata: StorageMetadata {
                format_version: 1,
                kind: "typed-records".into(),
                namespace,
                writer_codec_version: env!("CARGO_PKG_VERSION").into(),
            },
            report: report.clone(),
            body,
        };
        Ok(CapturedRecords { container, report })
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, StorageError> {
        envelope::encode(self)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StorageError> {
        envelope::decode(bytes)
    }
    pub fn metadata(&self) -> &StorageMetadata {
        &self.metadata
    }
    pub fn capture_report(&self) -> &CaptureReport {
        &self.report
    }
    pub fn records(&self) -> RecordViews<'_> {
        RecordViews { body: &self.body }
    }
}
