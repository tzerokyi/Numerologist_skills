//! Deterministic, headless Chinese divination chart engines.
//!
//! The algorithms describe *traditional conventions*, not scientific predictions.
//! Public request/response structs are serializable for CLI, automation and
//! a future optional MCP adapter. No interpreter, GUI or network server is used.

pub mod bazi;
pub mod calendar;
pub mod qimen;
pub mod ziwei;

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumerologistError(pub String);

impl Display for NumerologistError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for NumerologistError {}

pub type Result<T> = std::result::Result<T, NumerologistError>;

pub(crate) fn invalid(message: impl Into<String>) -> NumerologistError {
    NumerologistError(message.into())
}
