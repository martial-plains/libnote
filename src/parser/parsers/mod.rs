//! Example parsers for common markup syntaxes

pub mod markdown;
pub mod org;
pub mod latex;

pub use markdown::MarkdownParser;
pub use org::OrgParser;
pub use latex::LaTeXParser;
