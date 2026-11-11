//! Scalia blueprints: vehicles as data.

pub mod draw;
pub mod layout;
pub mod mesh;
pub mod shipped;
pub mod types;
pub mod validate;

pub use validate::{triangle_budget, triangle_count, validate};

use std::fmt;
use types::Blueprint;

/// A problem with a blueprint file, naming the field at fault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlueprintError {
    pub field: String,
    pub message: String,
}

impl fmt::Display for BlueprintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for BlueprintError {}

/// Parse RON text into a `Blueprint`.
pub fn parse(text: &str) -> Result<Blueprint, BlueprintError> {
    let b = ron::from_str::<Blueprint>(text).map_err(|e| {
        let message = format!("{}:{}: {}", e.position.line, e.position.col, e.code);
        let field = match &e.code {
            ron::Error::NoSuchStructField { found, .. } => found.clone(),
            _ => "<syntax>".to_string(),
        };
        BlueprintError { field, message }
    })?;
    validate(&b)?;
    Ok(b)
}
