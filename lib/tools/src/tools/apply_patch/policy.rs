use crate::tools::apply_patch::ApplyPatchAction;
use crate::tools::apply_patch::ApplyPatchError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalRequirement {
    Skip {
        bypass_sandbox: bool,
    },
    NeedsApproval {
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyCheck {
    AutoApprove {
        user_explicitly_approved: bool,
    },
    AskUser,
    Reject {
        reason: String,
    },
}

pub trait PatchSafetyChecker {
    fn assess(&self, action: &ApplyPatchAction) -> SafetyCheck;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InternalApplyPatchInvocation {
    Output(Result<String, ApplyPatchError>),
    DelegateToRuntime(ApplyPatchRuntimeInvocation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPatchRuntimeInvocation {
    pub action: ApplyPatchAction,
    pub auto_approved: bool,
    pub approval_requirement: ApprovalRequirement,
}

pub fn apply_patch(
    checker: &impl PatchSafetyChecker,
    action: ApplyPatchAction,
) -> InternalApplyPatchInvocation {
    match checker.assess(&action) {
        SafetyCheck::AutoApprove {
            user_explicitly_approved,
        } => InternalApplyPatchInvocation::DelegateToRuntime(ApplyPatchRuntimeInvocation {
            action,
            auto_approved: !user_explicitly_approved,
            approval_requirement: ApprovalRequirement::Skip {
                bypass_sandbox: false,
            },
        }),
        SafetyCheck::AskUser => {
            InternalApplyPatchInvocation::DelegateToRuntime(ApplyPatchRuntimeInvocation {
                action,
                auto_approved: false,
                approval_requirement: ApprovalRequirement::NeedsApproval { reason: None },
            })
        }
        SafetyCheck::Reject { reason } => {
            InternalApplyPatchInvocation::Output(Err(ApplyPatchError::Rejected(reason)))
        }
    }
}
