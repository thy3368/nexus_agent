#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl ShellCommand {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellProgram {
    Sh,
    Bash,
    Zsh,
    PowerShell,
    Cmd,
    Custom(String),
}

impl ShellProgram {
    pub fn detect_default() -> Self {
        if cfg!(windows) {
            return std::env::var("COMSPEC")
                .ok()
                .and_then(|value| Self::from_path_hint(&value))
                .unwrap_or(Self::Cmd);
        }

        std::env::var("SHELL")
            .ok()
            .and_then(|value| Self::from_path_hint(&value))
            .unwrap_or(Self::Sh)
    }

    pub fn command_for(&self, command: &str, login: bool) -> ShellCommand {
        match self {
            Self::Sh => ShellCommand::new("sh", vec!["-c".to_string(), command.to_string()]),
            Self::Bash => ShellCommand::new(
                "bash",
                if login {
                    vec!["-lc".to_string(), command.to_string()]
                } else {
                    vec!["-c".to_string(), command.to_string()]
                },
            ),
            Self::Zsh => ShellCommand::new(
                "zsh",
                if login {
                    vec!["-lc".to_string(), command.to_string()]
                } else {
                    vec!["-c".to_string(), command.to_string()]
                },
            ),
            Self::PowerShell => ShellCommand::new(
                "powershell",
                vec!["-Command".to_string(), command.to_string()],
            ),
            Self::Cmd => {
                ShellCommand::new("cmd", vec!["/C".to_string(), command.to_string()])
            }
            Self::Custom(program) => {
                ShellCommand::new(program.clone(), vec!["-c".to_string(), command.to_string()])
            }
        }
    }

    fn from_path_hint(path: &str) -> Option<Self> {
        let lowercase = path.rsplit(['/', '\\']).next()?.to_ascii_lowercase();
        match lowercase.as_str() {
            "sh" => Some(Self::Sh),
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" => Some(Self::PowerShell),
            "cmd" | "cmd.exe" => Some(Self::Cmd),
            other if !other.is_empty() => Some(Self::Custom(other.to_string())),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ShellProgram;

    #[test]
    fn bash_login_command_uses_login_flag() {
        let command = ShellProgram::Bash.command_for("echo hi", true);

        assert_eq!(command.program, "bash");
        assert_eq!(command.args, vec!["-lc", "echo hi"]);
    }

    #[test]
    fn cmd_command_uses_windows_flag() {
        let command = ShellProgram::Cmd.command_for("echo hi", false);

        assert_eq!(command.program, "cmd");
        assert_eq!(command.args, vec!["/C", "echo hi"]);
    }

    #[test]
    fn zsh_login_command_uses_login_flag() {
        let command = ShellProgram::Zsh.command_for("echo hi", true);

        assert_eq!(command.program, "zsh");
        assert_eq!(command.args, vec!["-lc", "echo hi"]);
    }
}
