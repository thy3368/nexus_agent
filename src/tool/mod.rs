//! Tool execution interface

use crate::tool::traits::tool_definition::ToolMeta;
use serde::{Deserialize, Serialize};

pub mod adapter;
pub mod traits;

pub mod tool_registry;
