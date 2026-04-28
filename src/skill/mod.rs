pub mod injection;
pub mod loader;
pub mod manager;
pub mod matching;
pub mod model;
pub mod parsing;
pub mod render;

pub use manager::SkillManager;
pub use model::{SkillLoadOutcome, SkillMetadata, SkillPromptContext, SkillRoot, SkillScope};
