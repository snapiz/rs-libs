#[macro_use]
extern crate serde;

#[macro_use]
extern crate thiserror;

mod context;
mod error;
mod user;

pub use crate::context::{Context, ContextError, ContextResult};
pub use crate::error::{Error, Result};
pub use crate::user::{User, UserRole, UserState};
