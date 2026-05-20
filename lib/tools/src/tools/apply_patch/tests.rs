use super::ApprovalRequirement;
use super::ApplyPatchAction;
use super::ApplyPatchError;
use super::ApplyPatchFileChange;
use super::FileChange;
use super::InternalApplyPatchInvocation;
use super::LocalApplyPatchExecutor;
use super::PatchSafetyChecker;
use super::SafetyCheck;
use super::apply_patch;
use super::convert_apply_patch_to_protocol;
use super::executor::ApplyPatchExecutor;
use super::parse_patch;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct StubSafetyChecker {
    result: SafetyCheck,
}

impl PatchSafetyChecker for StubSafetyChecker {
    fn assess(&self, _action: &ApplyPatchAction) -> SafetyCheck {
        self.result.clone()
    }
}

#[test]
fn convert_apply_patch_maps_add_variant() {
    let path = PathBuf::from("/tmp/a.txt");
    let action = ApplyPatchAction::new_add_for_test(&path, "hello");

    let got = convert_apply_patch_to_protocol(&action);

    assert_eq!(
        got.get(&path),
        Some(&FileChange::Add {
            content: "hello".to_string(),
        })
    );
}

#[test]
fn apply_patch_auto_approve_delegates_without_explicit_approval() {
    let checker = StubSafetyChecker {
        result: SafetyCheck::AutoApprove {
            user_explicitly_approved: false,
        },
    };
    let action = ApplyPatchAction::new("/tmp");

    let invocation = apply_patch(&checker, action);

    assert_eq!(
        invocation,
        InternalApplyPatchInvocation::DelegateToRuntime(super::ApplyPatchRuntimeInvocation {
            action: ApplyPatchAction::new("/tmp"),
            auto_approved: true,
            approval_requirement: ApprovalRequirement::Skip {
                bypass_sandbox: false,
            },
        })
    );
}

#[test]
fn apply_patch_reject_returns_output_error() {
    let checker = StubSafetyChecker {
        result: SafetyCheck::Reject {
            reason: "outside sandbox".to_string(),
        },
    };

    let invocation = apply_patch(&checker, ApplyPatchAction::new("/tmp"));

    match invocation {
        InternalApplyPatchInvocation::Output(Err(error)) => {
            assert_eq!(error.to_string(), "patch rejected: outside sandbox");
        }
        other => panic!("unexpected invocation: {other:?}"),
    }
}

#[test]
fn convert_apply_patch_maps_update_variant() {
    let path = PathBuf::from("/tmp/a.txt");
    let move_path = PathBuf::from("/tmp/b.txt");
    let action = ApplyPatchAction::new("/tmp").with_change(
        &path,
        ApplyPatchFileChange::Update {
            unified_diff: "@@\n-old\n+new\n".to_string(),
            move_path: Some(move_path.clone()),
            new_content: Some("new".to_string()),
        },
    );

    let got = convert_apply_patch_to_protocol(&action);

    assert_eq!(
        got.get(&path),
        Some(&FileChange::Update {
            unified_diff: "@@\n-old\n+new\n".to_string(),
            move_path: Some(move_path),
        })
    );
}

#[test]
fn parse_patch_reads_add_and_delete_hunks() {
    let patch = "\
*** Begin Patch
*** Add File: a.txt
+hello
+world
*** Delete File: b.txt
*** End Patch
";

    let action = parse_patch(patch, "/tmp").expect("patch should parse");

    assert_eq!(action.changes().len(), 2);
    assert_eq!(
        action.changes().get(&PathBuf::from("a.txt")),
        Some(&ApplyPatchFileChange::Add {
            content: "hello\nworld\n".to_string(),
        })
    );
    assert_eq!(
        action.changes().get(&PathBuf::from("b.txt")),
        Some(&ApplyPatchFileChange::Delete {
            content: String::new(),
        })
    );
}

#[test]
fn parse_patch_reads_update_and_move_hunks() {
    let patch = "\
*** Begin Patch
*** Update File: a.txt
*** Move to: b.txt
@@
-old
+new
*** End Patch
";

    let action = parse_patch(patch, "/tmp").expect("patch should parse");

    assert_eq!(
        action.changes().get(&PathBuf::from("a.txt")),
        Some(&ApplyPatchFileChange::Update {
            unified_diff: "@@\n-old\n+new\n".to_string(),
            move_path: Some(PathBuf::from("b.txt")),
            new_content: None,
        })
    );
}

#[test]
fn executor_adds_updates_deletes_and_moves_files() {
    let root = create_temp_test_dir("apply_patch_exec");
    std::fs::write(root.join("before.txt"), "old\n").expect("write before");
    std::fs::write(root.join("delete.txt"), "trash\n").expect("write delete");

    let patch = "\
*** Begin Patch
*** Add File: added.txt
+hello
*** Update File: before.txt
*** Move to: after.txt
@@
-old
+new
*** Delete File: delete.txt
*** End Patch
";

    let action = parse_patch(patch, &root).expect("patch should parse");
    let summary = LocalApplyPatchExecutor::new()
        .execute(&action)
        .expect("patch should execute");

    assert_eq!(summary.changed_paths.len(), 3);
    assert_eq!(
        std::fs::read_to_string(root.join("added.txt")).expect("read added"),
        "hello\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("after.txt")).expect("read moved"),
        "new\n"
    );
    assert!(!root.join("before.txt").exists());
    assert!(!root.join("delete.txt").exists());
    cleanup_temp_test_dir(&root);
}

#[test]
fn executor_rejects_missing_delete_target() {
    let root = create_temp_test_dir("apply_patch_missing_delete");
    let action = ApplyPatchAction::new(&root).with_change(
        "missing.txt",
        ApplyPatchFileChange::Delete {
            content: String::new(),
        },
    );

    let error = LocalApplyPatchExecutor::new()
        .execute(&action)
        .expect_err("delete should fail");

    assert_eq!(
        error,
        ApplyPatchError::Conflict("file does not exist: missing.txt".to_string())
    );
    cleanup_temp_test_dir(&root);
}

#[test]
fn bdd_given_add_file_patch_when_parse_then_action_contains_add_change() {
    // Given: a patch text that adds one file.
    let patch = "\
*** Begin Patch
*** Add File: notes.txt
+line 1
+line 2
*** End Patch
";

    // When: the patch is parsed into an action.
    let action = parse_patch(patch, "/tmp").expect("patch should parse");

    // Then: the parsed action contains the expected add change.
    assert_eq!(
        action.changes().get(&PathBuf::from("notes.txt")),
        Some(&ApplyPatchFileChange::Add {
            content: "line 1\nline 2\n".to_string(),
        })
    );
}

#[test]
fn bdd_given_safe_patch_when_apply_patch_then_it_delegates_to_runtime() {
    // Given: a patch action and a checker that auto-approves it.
    let checker = StubSafetyChecker {
        result: SafetyCheck::AutoApprove {
            user_explicitly_approved: false,
        },
    };
    let action = ApplyPatchAction::new("/tmp").with_change(
        "safe.txt",
        ApplyPatchFileChange::Add {
            content: "ok\n".to_string(),
        },
    );

    // When: policy routing is evaluated.
    let invocation = apply_patch(&checker, action.clone());

    // Then: the patch is delegated for execution without manual approval.
    assert_eq!(
        invocation,
        InternalApplyPatchInvocation::DelegateToRuntime(super::ApplyPatchRuntimeInvocation {
            action,
            auto_approved: true,
            approval_requirement: ApprovalRequirement::Skip {
                bypass_sandbox: false,
            },
        })
    );
}

#[test]
fn bdd_given_patch_text_when_execute_then_files_are_changed_on_disk() {
    // Given: a working directory with an existing file and a patch text.
    let root = create_temp_test_dir("apply_patch_bdd_execute");
    std::fs::write(root.join("todo.txt"), "draft\n").expect("write seed file");
    let patch = "\
*** Begin Patch
*** Update File: todo.txt
@@
-draft
+done
*** Add File: summary.txt
+completed
*** End Patch
";

    // When: the patch is parsed and executed locally.
    let action = parse_patch(patch, &root).expect("patch should parse");
    let summary = LocalApplyPatchExecutor::new()
        .execute(&action)
        .expect("patch should execute");

    // Then: disk contents reflect the patch result.
    assert_eq!(summary.changed_paths.len(), 2);
    assert_eq!(
        std::fs::read_to_string(root.join("todo.txt")).expect("read updated file"),
        "done\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("summary.txt")).expect("read added file"),
        "completed\n"
    );
    cleanup_temp_test_dir(&root);
}

#[test]
fn bdd_given_invalid_patch_when_parse_then_it_returns_a_parse_error() {
    // Given: a malformed patch text without the begin marker.
    let patch = "\
*** Add File: broken.txt
+oops
*** End Patch
";

    // When: parsing is attempted.
    let error = parse_patch(patch, "/tmp").expect_err("patch should fail to parse");

    // Then: the caller gets a parse error describing the problem.
    assert_eq!(
        error,
        ApplyPatchError::Parse("missing `*** Begin Patch` header".to_string())
    );
}

fn create_temp_test_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}_{nanos}"));
    std::fs::create_dir_all(&path).expect("create temp test dir");
    path
}

fn cleanup_temp_test_dir(path: &PathBuf) {
    let _ = std::fs::remove_dir_all(path);
}
