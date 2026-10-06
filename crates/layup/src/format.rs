//! Syntax-only formatter for the shared document language.
use crate::Error;
pub fn format(source: &str) -> Result<String, Error> {
    crate::document::format(source)
}
