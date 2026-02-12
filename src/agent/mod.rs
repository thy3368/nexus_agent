//! Agent orchestration and ReACT loop
//!
//! Architecture:
//! - `domain/`: Pure business logic (domain layer)
//! - `actor.rs`: Actix Actor adapter (interface/adapter layer)
//! - `agent_int.rs`: Agent interface trait

pub mod domain;

// Re-export domain types for public API

