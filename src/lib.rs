#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

uniffi::setup_scaffolding!();

pub mod document;
pub mod error;
pub mod formats;
pub mod managers;
pub mod models;
pub mod parser;
pub mod repo;
pub mod vault;

// Re-export common error types for convenience
pub use error::{
    DocumentError, DocumentResult, LibnoteError, LibnoteResult, ParseError, ParseResult,
    RepositoryError, RepositoryResult, SerializationError, SerializationResult,
};
