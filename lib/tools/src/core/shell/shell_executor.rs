use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::{fmt, io};

pub use crate::core::shell::types::ShellCommand;
pub use crate::core::shell::types::ShellProgram;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRequest {
    pub command: String,
    pub workdir: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub login: bool,
    pub shell: Option<ShellProgram>,
}

impl ShellRequest {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            workdir: None,
            env: BTreeMap::new(),
            login: false,
            shell: None,
        }
    }

    pub fn with_workdir(mut self, workdir: impl Into<PathBuf>) -> Self {
        self.workdir = Some(workdir.into());
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_login(mut self, login: bool) -> Self {
        self.login = login;
        self
    }

    pub fn with_shell(mut self, shell: ShellProgram) -> Self {
        self.shell = Some(shell);
        self
    }

    pub fn resolved_shell(&self) -> ShellProgram {
        self.shell.clone().unwrap_or_else(ShellProgram::detect_default)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellOutput {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl ShellOutput {
    pub fn success(&self) -> bool {
        self.status_code == Some(0)
    }
}

#[derive(Debug)]
pub enum ShellError {
    EmptyCommand,
    InvalidWorkdir(PathBuf),
    Io(io::Error),
}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCommand => write!(f, "shell command cannot be empty"),
            Self::InvalidWorkdir(path) => {
                write!(f, "shell working directory does not exist: {}", path.display())
            }
            Self::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ShellError {}

impl From<io::Error> for ShellError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub trait ShellExecutor {
    fn execute(&self, request: &ShellRequest) -> Result<ShellOutput, ShellError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalShellExecutor;

impl LocalShellExecutor {
    pub fn new() -> Self {
        Self
    }

    fn build_command(&self, request: &ShellRequest) -> Result<Command, ShellError> {
        if request.command.trim().is_empty() {
            return Err(ShellError::EmptyCommand);
        }

        if let Some(workdir) = &request.workdir {
            if !workdir.exists() {
                return Err(ShellError::InvalidWorkdir(workdir.clone()));
            }
        }

        let shell = request.resolved_shell();
        let shell_command = shell.command_for(&request.command, request.login);
        let mut command = Command::new(&shell_command.program);
        command.args(&shell_command.args);

        if let Some(workdir) = &request.workdir {
            command.current_dir(workdir);
        }

        if !request.env.is_empty() {
            command.envs(&request.env);
        }

        Ok(command)
    }
}

impl ShellExecutor for LocalShellExecutor {
    fn execute(&self, request: &ShellRequest) -> Result<ShellOutput, ShellError> {
        let output = self.build_command(request)?.output()?;

        Ok(ShellOutput {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::core::shell::types::ShellProgram;
    use super::LocalShellExecutor;
    use super::ShellExecutor;
    use super::ShellRequest;

    #[test]
    fn default_shell_is_available() {
        let shell = ShellRequest::new("echo hi").resolved_shell();
        let command = shell.command_for("echo hi", false);

        assert!(!command.program.is_empty());
        assert!(!command.args.is_empty());
    }

    #[test]
    fn executor_runs_command() {
        let output = LocalShellExecutor::new()
            .execute(&ShellRequest::new("echo shell-core").with_shell(ShellProgram::detect_default()))
            .expect("shell command should run");

        assert!(output.success());
        assert!(output.stdout.contains("shell-core"));
    }

    #[test]
    fn executor_lists_current_directory_and_prints_output() {
        let output = LocalShellExecutor::new()
            .execute(
                &ShellRequest::new("ls -1")
                    .with_workdir(".")
                    .with_shell(ShellProgram::detect_default()),
            )
            .expect("directory listing should run");

        assert!(output.success());
        assert!(output.stdout.contains("Cargo.toml"));
        println!("{}", output.stdout);
    }

    #[test]
    fn empty_command_is_rejected() {
        let err = LocalShellExecutor::new()
            .execute(&ShellRequest::new("   "))
            .expect_err("empty command should fail");

        assert_eq!(err.to_string(), "shell command cannot be empty");
    }
}
