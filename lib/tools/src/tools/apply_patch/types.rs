use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyPatchError {
    Rejected(String),
    Parse(String),
    Conflict(String),
    Io(String),
}

impl fmt::Display for ApplyPatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(reason) => write!(f, "patch rejected: {reason}"),
            Self::Parse(reason) => write!(f, "failed to parse apply_patch: {reason}"),
            Self::Conflict(reason) => write!(f, "failed to apply patch: {reason}"),
            Self::Io(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for ApplyPatchError {}

impl From<io::Error> for ApplyPatchError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPatchAction {
    pub cwd: PathBuf,
    changes: BTreeMap<PathBuf, ApplyPatchFileChange>,
}

impl ApplyPatchAction {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            changes: BTreeMap::new(),
        }
    }

    pub fn new_add_for_test(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        let path = path.into();
        let cwd = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        Self::new(cwd).with_change(path, ApplyPatchFileChange::Add {
            content: content.into(),
        })
    }

    pub fn with_change(mut self, path: impl Into<PathBuf>, change: ApplyPatchFileChange) -> Self {
        self.changes.insert(path.into(), change);
        self
    }

    pub fn insert_change(&mut self, path: impl Into<PathBuf>, change: ApplyPatchFileChange) {
        self.changes.insert(path.into(), change);
    }

    pub fn changes(&self) -> &BTreeMap<PathBuf, ApplyPatchFileChange> {
        &self.changes
    }

    pub fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyPatchFileChange {
    Add {
        content: String,
    },
    Delete {
        content: String,
    },
    Update {
        unified_diff: String,
        move_path: Option<PathBuf>,
        new_content: Option<String>,
    },
}
