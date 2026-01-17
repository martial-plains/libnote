//! Error types for the libnote library
//!
//! This module provides centralized error handling using `thiserror` across all components

use thiserror::Error;

/// Parse-related errors
#[derive(Debug, Clone, Error, uniffi::Error)]
pub enum ParseError {
    /// Syntax error at a specific line
    #[error("Syntax error at line {line}: {message}")]
    SyntaxError { line: u64, message: String },

    /// Unsupported syntax type
    #[error("Unsupported syntax: {0}")]
    UnsupportedSyntax(String),

    /// Block detection failed
    #[error("Block detection failed: {0}")]
    DetectionFailed(String),

    /// Other parsing errors
    #[error("Parse error: {0}")]
    Other(String),
}

impl ParseError {
    /// Create a syntax error at a specific line
    pub fn syntax_error(line: u64, message: impl Into<String>) -> Self {
        Self::SyntaxError {
            line,
            message: message.into(),
        }
    }

    /// Create an unsupported syntax error
    pub fn unsupported_syntax(syntax: impl Into<String>) -> Self {
        Self::UnsupportedSyntax(syntax.into())
    }

    /// Create a detection failed error
    pub fn detection_failed(reason: impl Into<String>) -> Self {
        Self::DetectionFailed(reason.into())
    }

    /// Create a generic parse error
    pub fn other(reason: impl Into<String>) -> Self {
        Self::Other(reason.into())
    }
}

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Document-related errors
#[derive(Debug, Error, uniffi::Error)]
pub enum DocumentError {
    /// Invalid document format
    #[error("Invalid document format: {0}")]
    InvalidFormat(String),

    /// Failed to parse document
    #[error("Failed to parse document: {0}")]
    ParseFailed(#[from] ParseError),

    /// Block index out of range
    #[error("Block index out of range")]
    InvalidIndex,

    /// Failed to render document
    #[error("Failed to render document: {0}")]
    RenderFailed(String),

    /// Missing or invalid syntax handler
    #[error("No parser available for syntax: {0}")]
    MissingParser(String),

    /// General document error
    #[error("Document error: {0}")]
    Other(String),
}

impl DocumentError {
    /// Create an invalid format error
    pub fn invalid_format(reason: impl Into<String>) -> Self {
        Self::InvalidFormat(reason.into())
    }

    /// Create a render failed error
    pub fn render_failed(reason: impl Into<String>) -> Self {
        Self::RenderFailed(reason.into())
    }

    /// Create a missing parser error
    pub fn missing_parser(syntax: impl Into<String>) -> Self {
        Self::MissingParser(syntax.into())
    }

    /// Create a generic document error
    pub fn other(reason: impl Into<String>) -> Self {
        Self::Other(reason.into())
    }
}

/// Result type for document operations
pub type DocumentResult<T> = Result<T, DocumentError>;

/// Serialization-related errors
#[derive(Debug, Error, uniffi::Error)]
pub enum SerializationError {
    /// Invalid UTF-8 encoding
    #[error("Invalid UTF-8 encoding")]
    InvalidUtf8,

    /// Unsupported format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Serialization failed
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    /// Deserialization failed
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

impl SerializationError {
    /// Create an unsupported format error
    pub fn unsupported_format(format: impl Into<String>) -> Self {
        Self::UnsupportedFormat(format.into())
    }

    /// Create a serialization failed error
    pub fn serialization_failed(reason: impl Into<String>) -> Self {
        Self::SerializationFailed(reason.into())
    }

    /// Create a deserialization failed error
    pub fn deserialization_failed(reason: impl Into<String>) -> Self {
        Self::DeserializationFailed(reason.into())
    }
}

/// Result type for serialization operations
pub type SerializationResult<T> = Result<T, SerializationError>;

/// Repository-related errors
#[derive(Debug, Error, uniffi::Error)]
pub enum RepositoryError {
    /// Note not found
    #[error("Note not found: {0}")]
    NotFound(String),

    /// Note already exists
    #[error("Note already exists: {0}")]
    AlreadyExists(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// Invalid note ID
    #[error("Invalid note ID: {0}")]
    InvalidId(String),

    /// General repository error
    #[error("Repository error: {0}")]
    Other(String),
}

impl RepositoryError {
    /// Create a not found error
    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound(id.into())
    }

    /// Create an already exists error
    pub fn already_exists(id: impl Into<String>) -> Self {
        Self::AlreadyExists(id.into())
    }

    /// Create an I/O error
    pub fn io_error(reason: impl Into<String>) -> Self {
        Self::IoError(reason.into())
    }

    /// Create an invalid ID error
    pub fn invalid_id(id: impl Into<String>) -> Self {
        Self::InvalidId(id.into())
    }

    /// Create a generic repository error
    pub fn other(reason: impl Into<String>) -> Self {
        Self::Other(reason.into())
    }
}

/// Result type for repository operations
pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Main unified error type that can represent any libnote error
#[derive(Debug, Error, uniffi::Error)]
pub enum LibnoteError {
    /// Parsing error
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// Document error
    #[error(transparent)]
    Document(#[from] DocumentError),

    /// Serialization error
    #[error(transparent)]
    Serialization(#[from] SerializationError),

    /// Repository error
    #[error(transparent)]
    Repository(#[from] RepositoryError),

    /// Generic error with custom message
    #[error("{0}")]
    Other(String),
}

impl LibnoteError {
    /// Create a generic error
    pub fn other(reason: impl Into<String>) -> Self {
        Self::Other(reason.into())
    }
}

/// Result type for libnote operations
pub type LibnoteResult<T> = Result<T, LibnoteError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_syntax() {
        let err = ParseError::syntax_error(42, "unexpected token");
        assert!(err.to_string().contains("line 42"));
        assert!(err.to_string().contains("unexpected token"));
    }

    #[test]
    fn test_parse_error_unsupported_syntax() {
        let err = ParseError::unsupported_syntax("ReStructuredText");
        assert!(err.to_string().contains("ReStructuredText"));
    }

    #[test]
    fn test_document_error_from_parse_error() {
        let parse_err = ParseError::other("test error");
        let doc_err: DocumentError = parse_err.into();
        assert!(doc_err.to_string().contains("test error"));
    }

    #[test]
    fn test_document_error_invalid_index() {
        let doc_err = DocumentError::InvalidIndex;
        assert!(doc_err.to_string().contains("index"));
    }

    #[test]
    fn test_libnote_error_from_document_error() {
        let doc_err = DocumentError::InvalidIndex;
        let lib_err: LibnoteError = doc_err.into();
        assert!(lib_err.to_string().contains("index"));
    }

    #[test]
    fn test_serialization_error_utf8() {
        let err = SerializationError::InvalidUtf8;
        assert!(err.to_string().contains("UTF-8"));
    }

    #[test]
    fn test_repository_error_not_found() {
        let err = RepositoryError::not_found("note-123");
        assert!(err.to_string().contains("note-123"));
    }
}
