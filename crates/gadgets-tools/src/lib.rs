//! Narrow tool/action providers for Gadgets Framework.
//!
//! Providers in this crate execute runtime-controlled actions after policy checks.
//! They do not expose provider SDK tool calls as security boundaries and they do
//! not accept arbitrary shell from model output.

pub mod filesystem_read;
pub mod patch_plan;
pub mod patch_apply;
mod git_status;
mod git_branch;
mod git_commit;
mod pr_body;
mod remote_pr;

pub use filesystem_read::{run_filesystem_read, FilesystemReadError, FilesystemReadReport, FilesystemReadRequest};
pub use patch_plan::{run_patch_plan, PatchPlanError, PatchPlanReport, PatchPlanRequest};
pub use patch_apply::{run_patch_apply, PatchApplyError, PatchApplyReport, PatchApplyRequest};
pub use git_status::{run_git_status, GitStatusError, GitStatusReport, GitStatusRequest};
pub use git_branch::{run_git_branch_create, GitBranchCreateReport, GitBranchCreateRequest, GitBranchError};
pub use git_commit::{run_git_commit_approved_patch, GitCommitError, GitCommitReport, GitCommitRequest};
pub use pr_body::{run_git_pr_body, GitPrBodyError, GitPrBodyReport, GitPrBodyRequest};
pub use remote_pr::{run_git_remote_pr_create, GitRemotePrError, GitRemotePrReport, GitRemotePrRequest, RemotePrProviderConfig};

mod test_runner;
pub use test_runner::{run_test_command, TestCommandSpec, TestRunError, TestRunReport, TestRunRequest};
