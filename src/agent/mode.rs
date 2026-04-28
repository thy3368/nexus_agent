#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMode {
    Execute,
    Plan,
}

impl AgentMode {
    pub fn from_config_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "plan" | "design" | "design-plan" => Self::Plan,
            _ => Self::Execute,
        }
    }

    pub fn is_plan(self) -> bool {
        matches!(self, Self::Plan)
    }
}
