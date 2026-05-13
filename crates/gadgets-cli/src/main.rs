mod config;
mod init;
mod manifest_loader;

use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use gadgets_approval::{
    approve_request, create_patch_approval_request, patch_approval_request_id,
    read_approval, read_request, verify_approval, PatchApprovalRequestInput,
};
use gadgets_evidence::{
    bundle_path_for_run, create_observe_bundle, default_runs_root, summarize_bundle,
    verify_bundle_hash, EvidenceWriteRequest,
};
use gadgets_ledger::{append_event, default_ledger_path, new_audit_event, summarize_events, verify_ledger, with_target};
use gadgets_provider::{
    AnthropicProvider, MockProvider, ModelProvider, OpenAiProvider, ProviderError, ProviderRequest,
    ProviderResponseStatus,
};
use config::{ensure_supported_provider, load_project_config, valid_test_command_name, RemotePrConfig as CliRemotePrConfig};
use manifest_loader::{
    ensure_pack_installed, gadget_manifest_available, load_gadget_manifest,
    load_installed_pack_manifests, load_pack_manifest, validate_installed_packs,
    validate_pack_tree, PackValidationReport, DEVELOPER_PACK, FILESYSTEM_READ_GADGET, PATCH_WRITER_GADGET,
    TEST_RUNNER_GADGET, GIT_PR_GADGET,
};
use gadgets_tools::{
    run_filesystem_read, run_git_branch_create, run_git_commit_approved_patch, run_git_status,
    run_patch_apply, run_patch_plan, run_test_command, run_git_pr_body, run_git_remote_pr_create, FilesystemReadRequest,
    GitBranchCreateRequest, GitCommitRequest, GitPrBodyRequest, GitRemotePrRequest, GitStatusRequest,
    PatchApplyRequest, PatchPlanRequest, RemotePrProviderConfig, TestCommandSpec, TestRunRequest,
};
use init::init_project;

fn main() {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return;
    };

    match command.as_str() {
        "init" => {
            let target = args.next().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            match init_project(&target) {
                Ok(report) => {
                    println!("Initialized Gadgets project state at {}", report.gadgets_dir.display());
                    println!("Mode: safe");
                    println!("Provider: mock");
                    println!("File writes require approval: yes");
                    println!("Created directories: {}", report.created_dirs.len());
                    println!("Created files: {}", report.created_files.len());
                    if !report.created_anything() {
                        println!("No changes needed; project was already initialized.");
                    }
                    println!("Review {} before enabling additional packs or providers.", report.gadgets_dir.join("config.yaml").display());
                }
                Err(err) => {
                    eprintln!("failed to initialize Gadgets project state: {err}");
                    std::process::exit(1);
                }
            }
        }
        "ask" => handle_ask(args.collect()),
        "ledger" => handle_ledger(args.collect()),
        "evidence" => handle_evidence(args.collect()),
        "approval" => handle_approval(args.collect()),
        "patch" => handle_patch(args.collect()),
        "test" => handle_test(args.collect()),
        "git" => handle_git(args.collect()),
        "pack" => handle_pack(args.collect()),
        "help" | "--help" | "-h" => print_help(),
        "version" | "--version" | "-V" => println!("gadgets 0.1.0"),
        other => {
            eprintln!("unknown command: {other}");
            print_help();
            std::process::exit(2);
        }
    }
}



fn handle_ask(args: Vec<String>) {
    let (project_root, prompt_parts) = parse_ask_args(args);
    if prompt_parts.is_empty() {
        print_help();
        std::process::exit(2);
    }

    let prompt = prompt_parts.join(" ");
    let config = match load_project_config(&project_root) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("failed to load Gadgets config: {err}");
            std::process::exit(1);
        }
    };
    let runtime_mode = match config.runtime_mode() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("invalid Gadgets runtime mode: {err}");
            std::process::exit(1);
        }
    };
    let selected_profile = match config.selected_model_profile() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("failed to select provider profile: {err}");
            std::process::exit(1);
        }
    };
    if let Err(err) = ensure_supported_provider(&selected_profile.profile.provider) {
        eprintln!("{err}");
        std::process::exit(1);
    }

    if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK) {
        eprintln!("cannot run Developer Pack workflow: {err}");
        std::process::exit(1);
    }

    let loaded_pack = match load_pack_manifest(&project_root, DEVELOPER_PACK) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("failed to load Developer Pack manifest: {err}");
            std::process::exit(1);
        }
    };
    let loaded_filesystem = match load_gadget_manifest(&project_root, &loaded_pack, FILESYSTEM_READ_GADGET) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("failed to load Filesystem Read Gadget manifest: {err}");
            std::process::exit(1);
        }
    };
    let loaded_patch_writer = match load_gadget_manifest(&project_root, &loaded_pack, PATCH_WRITER_GADGET) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("failed to load Patch Writer Gadget manifest: {err}");
            std::process::exit(1);
        }
    };

    let run_id = format!("run_{}", unix_timestamp_millis());
    let created_at = unix_timestamp_label();
    let provider = match build_model_provider(
        &selected_profile.profile.provider,
        &selected_profile.profile.model,
        selected_profile.profile.api_key_env.as_deref(),
        selected_profile.profile.endpoint.as_deref(),
    ) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("failed to initialize provider: {err}");
            std::process::exit(1);
        }
    };
    let allowed_targets = vec![
        loaded_filesystem.manifest.metadata.name.clone(),
        loaded_patch_writer.manifest.metadata.name.clone(),
    ];
    let provider_request = ProviderRequest::coordinator_request_with_profile(
        format!("mdl_{run_id}"),
        run_id.clone(),
        selected_profile.name.to_string(),
        prompt.clone(),
        allowed_targets,
    );
    let provider_response = match provider.complete(&provider_request) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("provider failed to create Coordinator handoff: {err}");
            std::process::exit(1);
        }
    };

    if provider_response.status != ProviderResponseStatus::Completed {
        eprintln!("provider did not complete the Coordinator request");
        std::process::exit(1);
    }

    if let Some(handoff) = provider_response.handoff_requests.iter().find(|handoff| {
        handoff.to_gadget == loaded_filesystem.manifest.metadata.name && handoff.task_kind == "repo.inspect"
    }) {
        let request = FilesystemReadRequest::observe_repo(run_id, created_at, prompt.clone())
            .with_runtime_mode(runtime_mode)
            .with_coordinator_handoff(
                provider_response.text_summary.clone(),
                handoff.handoff_id.clone(),
                handoff.reason.clone(),
                provider_response.provider.clone(),
            );

        println!("Gadgets {} Mode observe-only run", runtime_mode.as_str());
        println!("Project root: {}", project_root.display());
        println!("Request: {prompt}");
        println!("Coordinator: {}", provider_response.text_summary);
        println!(
            "Provider profile: {} -> {} / {}",
            selected_profile.name, provider_response.provider, provider_response.model
        );
        println!("Pack: {} ({})", loaded_pack.manifest.metadata.name, loaded_pack.source.label());
        println!("Gadget manifest source: {}", loaded_filesystem.source.label());
        println!("Selected handoff: {} -> {}", handoff.from_gadget, handoff.to_gadget);
        for note in &provider_response.safety_notes {
            println!("Safety: {note}");
        }
        println!("Gadget: {}", loaded_filesystem.manifest.metadata.name);
        println!("No files will be modified. No commands will be executed. Provider output is still policy-checked by the runtime.");
        println!();

        match run_filesystem_read(&project_root, &loaded_filesystem.manifest, request) {
            Ok(report) => {
                println!("Run: {}", report.run_id);
                println!("Directories considered: {}", report.directories_considered);
                println!("Files considered: {}", report.files_considered);
                println!("Files read: {}", report.files_read);
                println!("Denied paths/actions: {}", report.denied_paths.len());
                println!("Skipped paths: {}", report.skipped_paths.len());
                println!("Audit events appended: {}", report.ledger_events_appended);
                println!("Evidence: {}", report.evidence_bundle_path.display());
                println!("Ledger: {}", report.ledger_path.display());
            }
            Err(err) => {
                eprintln!("filesystem observe run failed: {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(handoff) = provider_response.handoff_requests.iter().find(|handoff| {
        handoff.to_gadget == loaded_patch_writer.manifest.metadata.name
            && handoff.task_kind == "repo.patch.plan"
    }) {
        let request = PatchPlanRequest::plan_patch(run_id, created_at, prompt.clone())
            .with_runtime_mode(runtime_mode)
            .with_coordinator_handoff(
                provider_response.text_summary.clone(),
                handoff.handoff_id.clone(),
                handoff.reason.clone(),
                provider_response.provider.clone(),
            );

        println!("Gadgets {} Mode plan-only Patch Writer run", runtime_mode.as_str());
        println!("Project root: {}", project_root.display());
        println!("Request: {prompt}");
        println!("Coordinator: {}", provider_response.text_summary);
        println!(
            "Provider profile: {} -> {} / {}",
            selected_profile.name, provider_response.provider, provider_response.model
        );
        println!("Pack: {} ({})", loaded_pack.manifest.metadata.name, loaded_pack.source.label());
        println!("Gadget manifest source: {}", loaded_patch_writer.source.label());
        println!("Selected handoff: {} -> {}", handoff.from_gadget, handoff.to_gadget);
        for note in &provider_response.safety_notes {
            println!("Safety: {note}");
        }
        println!("Gadget: {}", loaded_patch_writer.manifest.metadata.name);
        println!("No files will be modified. No patch will be applied. No commands will be executed.");
        println!();

        match run_patch_plan(&project_root, &loaded_patch_writer.manifest, request) {
            Ok(report) => {
                println!("Run: {}", report.run_id);
                println!("Policy decision: {:?}", report.policy_decision);
                println!("Audit events appended: {}", report.ledger_events_appended);
                println!("Evidence: {}", report.evidence_bundle_path.display());
                println!("Ledger: {}", report.ledger_path.display());
                println!("Proposed patch artifact: {}/proposed.patch", report.evidence_bundle_path.parent().map(|path| path.display().to_string()).unwrap_or_else(|| ".".to_string()));
            }
            Err(err) => {
                eprintln!("patch plan run failed: {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    eprintln!("provider did not produce an allowed Developer Pack handoff");
    std::process::exit(1);
}

fn build_model_provider(
    provider_name: &str,
    model_name: &str,
    api_key_env: Option<&str>,
    endpoint: Option<&str>,
) -> Result<Box<dyn ModelProvider>, ProviderError> {
    match provider_name {
        "mock" => Ok(Box::new(MockProvider::new(provider_name, model_name))),
        "openai" => Ok(Box::new(OpenAiProvider::from_env(
            model_name,
            api_key_env,
            endpoint,
        )?)),
        "anthropic" => Ok(Box::new(AnthropicProvider::from_env(
            model_name,
            api_key_env,
            endpoint,
        )?)),
        other => Err(ProviderError::InvalidResponse(format!(
            "unsupported provider reached runtime dispatch: {other}"
        ))),
    }
}

fn parse_ask_args(args: Vec<String>) -> (PathBuf, Vec<String>) {
    let mut project_root = PathBuf::from(".");
    let mut prompt_parts = Vec::new();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--project" | "--root" => {
                let Some(path) = iter.next() else {
                    eprintln!("{arg} requires a path");
                    std::process::exit(2);
                };
                project_root = PathBuf::from(path);
            }
            "--" => {
                prompt_parts.extend(iter);
                break;
            }
            _ => prompt_parts.push(arg),
        }
    }

    (project_root, prompt_parts)
}

fn handle_pack(args: Vec<String>) {
    let Some(subcommand) = args.first() else {
        print_pack_help();
        std::process::exit(2);
    };

    match subcommand.as_str() {
        "list" => {
            let (project_root, rest) = parse_project_option(args[1..].to_vec());
            if !rest.is_empty() {
                eprintln!("unexpected pack list arguments: {}", rest.join(" "));
                print_pack_help();
                std::process::exit(2);
            }
            let config = match load_project_config(&project_root) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Gadgets config: {err}");
                    std::process::exit(1);
                }
            };
            match load_installed_pack_manifests(&project_root, &config.installed_packs) {
                Ok(packs) => {
                    if packs.is_empty() {
                        println!("No installed packs configured at {}", project_root.display());
                        return;
                    }
                    for pack in packs {
                        println!(
                            "{} | {} | {} | {}",
                            pack.manifest.metadata.name,
                            pack.manifest.metadata.version,
                            pack.source.label(),
                            pack.manifest.metadata.description
                        );
                    }
                }
                Err(err) => {
                    eprintln!("failed to load installed packs: {err}");
                    std::process::exit(1);
                }
            }
        }
        "show" => {
            let (project_root, rest) = parse_project_option(args[1..].to_vec());
            let Some(pack_name) = rest.first() else {
                print_pack_help();
                std::process::exit(2);
            };
            let config = match load_project_config(&project_root) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Gadgets config: {err}");
                    std::process::exit(1);
                }
            };
            if let Err(err) = ensure_pack_installed(&config.installed_packs, pack_name) {
                eprintln!("failed to load pack: {err}");
                std::process::exit(1);
            }
            let pack = match load_pack_manifest(&project_root, pack_name) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load pack manifest: {err}");
                    std::process::exit(1);
                }
            };

            println!("Pack: {}", pack.manifest.metadata.name);
            println!("Version: {}", pack.manifest.metadata.version);
            println!("Display: {}", pack.manifest.metadata.display_name);
            println!("Description: {}", pack.manifest.metadata.description);
            println!("Source: {}", pack.source.label());
            if let Some(mode) = &pack.manifest.default_mode {
                println!("Default mode: {mode}");
            }
            println!();
            println!("Declared Gadgets:");
            for gadget in &pack.manifest.gadgets {
                let status = if gadget_manifest_available(&project_root, &pack, gadget) {
                    "available"
                } else {
                    "manifest pending"
                };
                println!("  - {gadget} [{status}]");
            }
        }
        "validate" => {
            let (project_root, strict, requested_pack) = parse_pack_validate_args(args[1..].to_vec());
            let reports = if let Some(pack_name) = requested_pack {
                match validate_pack_tree(&project_root, &pack_name, strict) {
                    Ok(report) => vec![report],
                    Err(err) => {
                        eprintln!("failed to validate pack: {err}");
                        std::process::exit(1);
                    }
                }
            } else {
                let config = match load_project_config(&project_root) {
                    Ok(value) => value,
                    Err(err) => {
                        eprintln!("failed to load Gadgets config: {err}");
                        std::process::exit(1);
                    }
                };
                match validate_installed_packs(&project_root, &config.installed_packs, strict) {
                    Ok(value) => value,
                    Err(err) => {
                        eprintln!("failed to validate installed packs: {err}");
                        std::process::exit(1);
                    }
                }
            };

            let mut all_valid = true;
            for report in reports {
                print_pack_validation_report(&report);
                all_valid = all_valid && report.is_valid();
            }

            if !all_valid {
                std::process::exit(1);
            }
        }
        "help" | "--help" | "-h" => print_pack_help(),
        other => {
            eprintln!("unknown pack command: {other}");
            print_pack_help();
            std::process::exit(2);
        }
    }
}

fn parse_pack_validate_args(args: Vec<String>) -> (PathBuf, bool, Option<String>) {
    let mut project_root = PathBuf::from(".");
    let mut strict = false;
    let mut pack_name = None;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--project" | "--root" => {
                let Some(path) = iter.next() else {
                    eprintln!("{arg} requires a path");
                    std::process::exit(2);
                };
                project_root = PathBuf::from(path);
            }
            "--strict" => strict = true,
            "--" => {
                if let Some(value) = iter.next() {
                    if pack_name.replace(value).is_some() {
                        eprintln!("pack validate accepts at most one pack name");
                        std::process::exit(2);
                    }
                }
                if iter.next().is_some() {
                    eprintln!("pack validate accepts at most one pack name");
                    std::process::exit(2);
                }
                break;
            }
            _ => {
                if pack_name.replace(arg).is_some() {
                    eprintln!("pack validate accepts at most one pack name");
                    std::process::exit(2);
                }
            }
        }
    }

    (project_root, strict, pack_name)
}

fn print_pack_validation_report(report: &PackValidationReport) {
    let status = if report.is_valid() { "VALID" } else { "INVALID" };
    println!("Pack: {} [{}]", report.pack_name, status);
    println!("Source: {}", report.pack_source);
    println!("Strict mode: {}", if report.strict { "yes" } else { "no" });
    println!("Gadgets checked: {}", report.gadgets_checked);
    println!("Gadgets valid: {}", report.gadgets_valid);
    println!("Gadgets missing: {}", report.gadgets_missing);
    println!("Declared Gadgets:");
    for row in &report.gadget_rows {
        match &row.source {
            Some(source) => println!("  - {} | {} | {}", row.name, row.status, source),
            None => println!("  - {} | {}", row.name, row.status),
        }
    }

    if !report.issues.is_empty() {
        println!("Issues:");
        for issue in &report.issues {
            println!("  - {}: {}", issue.severity.as_str(), issue.message);
        }
    }
    println!();
}

fn parse_project_option(args: Vec<String>) -> (PathBuf, Vec<String>) {
    let mut project_root = PathBuf::from(".");
    let mut rest = Vec::new();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--project" | "--root" => {
                let Some(path) = iter.next() else {
                    eprintln!("{arg} requires a path");
                    std::process::exit(2);
                };
                project_root = PathBuf::from(path);
            }
            "--" => {
                rest.extend(iter);
                break;
            }
            _ => rest.push(arg),
        }
    }

    (project_root, rest)
}

fn parse_git_commit_approved_patch_args(args: Vec<String>) -> (String, String) {
    let mut approval_request_id: Option<String> = None;
    let mut commit_message: Option<String> = None;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--message" | "-m" => {
                let Some(value) = iter.next() else {
                    eprintln!("{arg} requires a commit message");
                    std::process::exit(2);
                };
                if commit_message.replace(value).is_some() {
                    eprintln!("commit message may be specified only once");
                    std::process::exit(2);
                }
            }
            "--" => {
                for value in iter {
                    if approval_request_id.replace(value).is_some() {
                        eprintln!("git commit approved-patch accepts exactly one approval request id");
                        std::process::exit(2);
                    }
                }
                break;
            }
            _ => {
                if approval_request_id.replace(arg).is_some() {
                    eprintln!("git commit approved-patch accepts exactly one approval request id");
                    std::process::exit(2);
                }
            }
        }
    }

    let Some(approval_request_id) = approval_request_id else {
        print_git_help();
        std::process::exit(2);
    };
    let commit_message = commit_message.unwrap_or_else(|| {
        format!("Apply approved Gadgets patch {approval_request_id}")
    });
    (approval_request_id, commit_message)
}

fn parse_git_pr_body_args(args: Vec<String>) -> (String, Option<String>, Option<String>, Option<String>) {
    let mut approval_request_id: Option<String> = None;
    let mut test_run_id: Option<String> = None;
    let mut commit_run_id: Option<String> = None;
    let mut title: Option<String> = None;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--test-run" => {
                let Some(value) = iter.next() else {
                    eprintln!("{arg} requires a run id");
                    std::process::exit(2);
                };
                if test_run_id.replace(value).is_some() {
                    eprintln!("--test-run may be specified only once");
                    std::process::exit(2);
                }
            }
            "--commit-run" => {
                let Some(value) = iter.next() else {
                    eprintln!("{arg} requires a run id");
                    std::process::exit(2);
                };
                if commit_run_id.replace(value).is_some() {
                    eprintln!("--commit-run may be specified only once");
                    std::process::exit(2);
                }
            }
            "--title" => {
                let Some(value) = iter.next() else {
                    eprintln!("{arg} requires a title");
                    std::process::exit(2);
                };
                if title.replace(value).is_some() {
                    eprintln!("--title may be specified only once");
                    std::process::exit(2);
                }
            }
            "--" => {
                for value in iter {
                    if approval_request_id.replace(value).is_some() {
                        eprintln!("git pr body accepts exactly one approval request id");
                        std::process::exit(2);
                    }
                }
                break;
            }
            _ => {
                if approval_request_id.replace(arg).is_some() {
                    eprintln!("git pr body accepts exactly one approval request id");
                    std::process::exit(2);
                }
            }
        }
    }

    let Some(approval_request_id) = approval_request_id else {
        print_git_help();
        std::process::exit(2);
    };
    (approval_request_id, test_run_id, commit_run_id, title)
}


fn parse_git_pr_create_args(args: Vec<String>, default_base_branch: &str) -> (String, String, String, String, Option<String>) {
    let mut approval_request_id: Option<String> = None;
    let mut pr_body_run_id: Option<String> = None;
    let mut head_branch: Option<String> = None;
    let mut base_branch: Option<String> = Some(default_base_branch.to_string());
    let mut title: Option<String> = None;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--body-run" => {
                let Some(value) = iter.next() else {
                    eprintln!("{arg} requires a PR body run id");
                    std::process::exit(2);
                };
                if pr_body_run_id.replace(value).is_some() {
                    eprintln!("--body-run may be specified only once");
                    std::process::exit(2);
                }
            }
            "--head" => {
                let Some(value) = iter.next() else {
                    eprintln!("{arg} requires a branch name");
                    std::process::exit(2);
                };
                if head_branch.replace(value).is_some() {
                    eprintln!("--head may be specified only once");
                    std::process::exit(2);
                }
            }
            "--base" => {
                let Some(value) = iter.next() else {
                    eprintln!("{arg} requires a branch name");
                    std::process::exit(2);
                };
                if base_branch.replace(value).is_some() {
                    eprintln!("--base may be specified only once");
                    std::process::exit(2);
                }
            }
            "--title" => {
                let Some(value) = iter.next() else {
                    eprintln!("{arg} requires a title");
                    std::process::exit(2);
                };
                if title.replace(value).is_some() {
                    eprintln!("--title may be specified only once");
                    std::process::exit(2);
                }
            }
            "--" => {
                for value in iter {
                    if approval_request_id.replace(value).is_some() {
                        eprintln!("git pr create accepts exactly one approval request id");
                        std::process::exit(2);
                    }
                }
                break;
            }
            _ => {
                if approval_request_id.replace(arg).is_some() {
                    eprintln!("git pr create accepts exactly one approval request id");
                    std::process::exit(2);
                }
            }
        }
    }

    let Some(approval_request_id) = approval_request_id else {
        print_git_help();
        std::process::exit(2);
    };
    let Some(pr_body_run_id) = pr_body_run_id else {
        eprintln!("git pr create requires --body-run <run-id>");
        print_git_help();
        std::process::exit(2);
    };
    let Some(head_branch) = head_branch else {
        eprintln!("git pr create requires --head <branch>");
        print_git_help();
        std::process::exit(2);
    };
    let Some(base_branch) = base_branch else {
        print_git_help();
        std::process::exit(2);
    };
    (approval_request_id, pr_body_run_id, head_branch, base_branch, title)
}

fn remote_pr_provider_config(config: &CliRemotePrConfig) -> RemotePrProviderConfig {
    RemotePrProviderConfig {
        enabled: config.enabled,
        provider: config.provider.clone(),
        owner: config.owner.clone(),
        repo: config.repo.clone(),
        api_base: config.api_base.clone(),
        token_env: config.token_env.clone(),
        default_base_branch: config.default_base_branch.clone(),
    }
}

fn handle_ledger(args: Vec<String>) {
    let Some(subcommand) = args.first() else {
        print_ledger_help();
        std::process::exit(2);
    };

    match subcommand.as_str() {
        "show" => {
            let ledger_path = ledger_path_from_arg(args.get(1));
            match summarize_events(&ledger_path) {
                Ok(rows) => {
                    if rows.is_empty() {
                        println!("No audit events found at {}", ledger_path.display());
                        return;
                    }

                    for row in rows {
                        println!(
                            "{} | {} | {} | {} | {} | {}",
                            row.timestamp,
                            row.event_id,
                            row.event_type,
                            row.actor,
                            row.decision,
                            row.summary
                        );
                    }
                }
                Err(err) => {
                    eprintln!("failed to read audit ledger: {err}");
                    std::process::exit(1);
                }
            }
        }
        "verify" => {
            let ledger_path = ledger_path_from_arg(args.get(1));
            match verify_ledger(&ledger_path) {
                Ok(report) if report.valid => {
                    println!(
                        "Ledger verified: {} events checked at {}",
                        report.events_checked,
                        report.ledger_path.display()
                    );
                }
                Ok(report) => {
                    eprintln!(
                        "Ledger verification failed: {} events checked at {}",
                        report.events_checked,
                        report.ledger_path.display()
                    );
                    for err in report.errors {
                        eprintln!("- {err}");
                    }
                    std::process::exit(1);
                }
                Err(err) => {
                    eprintln!("failed to verify audit ledger: {err}");
                    std::process::exit(1);
                }
            }
        }
        "help" | "--help" | "-h" => print_ledger_help(),
        other => {
            eprintln!("unknown ledger command: {other}");
            print_ledger_help();
            std::process::exit(2);
        }
    }
}

fn handle_evidence(args: Vec<String>) {
    let Some(subcommand) = args.first() else {
        print_evidence_help();
        std::process::exit(2);
    };

    match subcommand.as_str() {
        "show" => {
            let Some(run_id) = args.get(1) else {
                print_evidence_help();
                std::process::exit(2);
            };
            let project_root = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let runs_root = default_runs_root(&project_root);
            let bundle_path = match bundle_path_for_run(&runs_root, run_id) {
                Ok(path) => path,
                Err(err) => {
                    eprintln!("invalid evidence path: {err}");
                    std::process::exit(1);
                }
            };
            match summarize_bundle(&bundle_path) {
                Ok(summary) => {
                    println!("Evidence bundle: {}", summary.evidence_bundle_id);
                    println!("Run: {}", summary.run_id);
                    println!("Gadget: {}", summary.gadget);
                    println!("Status: {}", summary.status);
                    println!("Artifacts: {}", summary.artifact_count);
                    println!("Denied actions: {}", summary.denied_action_count);
                    println!("Assumptions: {}", summary.assumption_count);
                    if let Some(hash) = summary.bundle_hash {
                        println!("Bundle hash: {hash}");
                    }
                    println!();
                    println!("{}", summary.summary);
                }
                Err(err) => {
                    eprintln!("failed to read evidence bundle: {err}");
                    std::process::exit(1);
                }
            }
        }
        "verify" => {
            let Some(run_id) = args.get(1) else {
                print_evidence_help();
                std::process::exit(2);
            };
            let project_root = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let runs_root = default_runs_root(&project_root);
            let bundle_path = match bundle_path_for_run(&runs_root, run_id) {
                Ok(path) => path,
                Err(err) => {
                    eprintln!("invalid evidence path: {err}");
                    std::process::exit(1);
                }
            };
            match verify_bundle_hash(&bundle_path) {
                Ok(true) => println!("Evidence bundle verified at {}", bundle_path.display()),
                Ok(false) => {
                    eprintln!("Evidence bundle verification failed at {}", bundle_path.display());
                    std::process::exit(1);
                }
                Err(err) => {
                    eprintln!("failed to verify evidence bundle: {err}");
                    std::process::exit(1);
                }
            }
        }
        "create-observe" => {
            let Some(run_id) = args.get(1) else {
                print_evidence_help();
                std::process::exit(2);
            };
            let Some(gadget) = args.get(2) else {
                print_evidence_help();
                std::process::exit(2);
            };
            if args.len() < 4 {
                print_evidence_help();
                std::process::exit(2);
            }
            let summary = args[3..].join(" ");
            let created_at = unix_timestamp_label();
            let request = EvidenceWriteRequest::observe(run_id.as_str(), gadget.as_str(), created_at, summary);
            let runs_root = default_runs_root(Path::new("."));
            match create_observe_bundle(&runs_root, request) {
                Ok(report) => {
                    println!("Evidence bundle created for run {}", report.run_id);
                    println!("Bundle: {}", report.bundle_path.display());
                    println!("Summary: {}", report.summary_path.display());
                    println!("Hash: {}", report.bundle_hash);
                }
                Err(err) => {
                    eprintln!("failed to create evidence bundle: {err}");
                    std::process::exit(1);
                }
            }
        }
        "help" | "--help" | "-h" => print_evidence_help(),
        other => {
            eprintln!("unknown evidence command: {other}");
            print_evidence_help();
            std::process::exit(2);
        }
    }
}



fn handle_git(args: Vec<String>) {
    let Some(subcommand) = args.first() else {
        print_git_help();
        std::process::exit(2);
    };

    match subcommand.as_str() {
        "status" => {
            let (project_root, rest) = parse_project_option(args[1..].to_vec());
            if !rest.is_empty() {
                eprintln!("unexpected git status arguments: {}", rest.join(" "));
                print_git_help();
                std::process::exit(2);
            }

            let config = match load_project_config(&project_root) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Gadgets config: {err}");
                    std::process::exit(1);
                }
            };
            let runtime_mode = match config.runtime_mode() {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("invalid Gadgets runtime mode: {err}");
                    std::process::exit(1);
                }
            };
            if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK) {
                eprintln!("cannot run Developer Pack Git workflow: {err}");
                std::process::exit(1);
            }

            let loaded_pack = match load_pack_manifest(&project_root, DEVELOPER_PACK) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Developer Pack manifest: {err}");
                    std::process::exit(1);
                }
            };
            let loaded_git = match load_gadget_manifest(&project_root, &loaded_pack, GIT_PR_GADGET) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Git Gadget manifest: {err}");
                    std::process::exit(1);
                }
            };

            let run_id = format!("run_git_status_{}", unix_timestamp_millis());
            let request = GitStatusRequest::local_status(run_id, unix_timestamp_label())
                .with_runtime_mode(runtime_mode);

            println!("Gadgets {} Mode local Git status", runtime_mode.as_str());
            println!("Project root: {}", project_root.display());
            println!("Pack: {} ({})", loaded_pack.manifest.metadata.name, loaded_pack.source.label());
            println!("Gadget manifest source: {}", loaded_git.source.label());
            println!("Safety: this runs a fixed local git status command selected by the runtime.");
            println!("No branch, commit, push, pull, fetch, PR, provider, patch, shell, Linux admin, database, cloud, or deployment action will run.");
            println!();

            match run_git_status(&project_root, &loaded_git.manifest, request) {
                Ok(report) => {
                    println!("Git status run: {}", report.run_id);
                    println!("Passed: {}", report.passed);
                    println!("Exit code: {}", report.exit_code.map(|code| code.to_string()).unwrap_or_else(|| "none".to_string()));
                    println!("Branch: {}", report.branch.unwrap_or_else(|| "unknown".to_string()));
                    println!("Changed entries: {}", report.changed_entries);
                    println!("Duration ms: {}", report.duration_ms);
                    println!("Audit events appended: {}", report.ledger_events_appended);
                    println!("Evidence: {}", report.evidence_bundle_path.display());
                    println!("Ledger: {}", report.ledger_path.display());
                    if !report.passed {
                        std::process::exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("git status failed: {err}");
                    std::process::exit(1);
                }
            }
        }
        "branch" => {
            let Some(branch_subcommand) = args.get(1) else {
                print_git_help();
                std::process::exit(2);
            };
            match branch_subcommand.as_str() {
                "create" => {
                    let (project_root, rest) = parse_project_option(args[2..].to_vec());
                    if rest.len() != 1 {
                        print_git_help();
                        std::process::exit(2);
                    }
                    let branch_name = rest[0].clone();

                    let config = match load_project_config(&project_root) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Gadgets config: {err}");
                            std::process::exit(1);
                        }
                    };
                    let runtime_mode = match config.runtime_mode() {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("invalid Gadgets runtime mode: {err}");
                            std::process::exit(1);
                        }
                    };
                    if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK) {
                        eprintln!("cannot run Developer Pack Git workflow: {err}");
                        std::process::exit(1);
                    }

                    let loaded_pack = match load_pack_manifest(&project_root, DEVELOPER_PACK) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Developer Pack manifest: {err}");
                            std::process::exit(1);
                        }
                    };
                    let loaded_git = match load_gadget_manifest(&project_root, &loaded_pack, GIT_PR_GADGET) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Git Gadget manifest: {err}");
                            std::process::exit(1);
                        }
                    };

                    let run_id = format!("run_git_branch_{}", unix_timestamp_millis());
                    let request = GitBranchCreateRequest::create_branch(
                        run_id,
                        unix_timestamp_label(),
                        branch_name.clone(),
                        config.git.protected_branches.clone(),
                    )
                    .with_runtime_mode(runtime_mode);

                    println!("Gadgets {} Mode protected local Git branch creation", runtime_mode.as_str());
                    println!("Project root: {}", project_root.display());
                    println!("Branch name: {branch_name}");
                    println!("Pack: {} ({})", loaded_pack.manifest.metadata.name, loaded_pack.source.label());
                    println!("Gadget manifest source: {}", loaded_git.source.label());
                    println!("Safety: this runs fixed local git branch creation after runtime branch-name and protected-branch checks.");
                    println!("No checkout, switch, stage, commit, push, pull, fetch, merge, PR, provider, patch, shell, Linux admin, database, cloud, or deployment action will run.");
                    println!();

                    match run_git_branch_create(&project_root, &loaded_git.manifest, request) {
                        Ok(report) => {
                            println!("Git branch run: {}", report.run_id);
                            println!("Branch: {}", report.branch_name);
                            println!("Passed: {}", report.passed);
                            println!("Exit code: {}", report.exit_code.map(|code| code.to_string()).unwrap_or_else(|| "none".to_string()));
                            println!("Duration ms: {}", report.duration_ms);
                            println!("Audit events appended: {}", report.ledger_events_appended);
                            println!("Evidence: {}", report.evidence_bundle_path.display());
                            println!("Ledger: {}", report.ledger_path.display());
                            if !report.passed {
                                std::process::exit(1);
                            }
                        }
                        Err(err) => {
                            eprintln!("git branch create failed: {err}");
                            std::process::exit(1);
                        }
                    }
                }
                "help" | "--help" | "-h" => print_git_help(),
                other => {
                    eprintln!("unknown git branch command: {other}");
                    print_git_help();
                    std::process::exit(2);
                }
            }
        }
        "commit" => {
            let Some(commit_subcommand) = args.get(1) else {
                print_git_help();
                std::process::exit(2);
            };
            match commit_subcommand.as_str() {
                "approved-patch" => {
                    let (project_root, rest) = parse_project_option(args[2..].to_vec());
                    let (approval_request_id, commit_message) = parse_git_commit_approved_patch_args(rest);

                    let config = match load_project_config(&project_root) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Gadgets config: {err}");
                            std::process::exit(1);
                        }
                    };
                    let runtime_mode = match config.runtime_mode() {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("invalid Gadgets runtime mode: {err}");
                            std::process::exit(1);
                        }
                    };
                    if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK) {
                        eprintln!("cannot run Developer Pack Git workflow: {err}");
                        std::process::exit(1);
                    }

                    let loaded_pack = match load_pack_manifest(&project_root, DEVELOPER_PACK) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Developer Pack manifest: {err}");
                            std::process::exit(1);
                        }
                    };
                    let loaded_git = match load_gadget_manifest(&project_root, &loaded_pack, GIT_PR_GADGET) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Git Gadget manifest: {err}");
                            std::process::exit(1);
                        }
                    };

                    let run_id = format!("run_git_commit_{}", unix_timestamp_millis());
                    let request = GitCommitRequest::approved_patch_commit(
                        run_id,
                        unix_timestamp_label(),
                        approval_request_id.clone(),
                        commit_message.clone(),
                        config.git.protected_branches.clone(),
                    )
                    .with_runtime_mode(runtime_mode);

                    println!("Gadgets {} Mode approved local Git commit", runtime_mode.as_str());
                    println!("Project root: {}", project_root.display());
                    println!("Approval request: {approval_request_id}");
                    println!("Commit message: {commit_message}");
                    println!("Pack: {} ({})", loaded_pack.manifest.metadata.name, loaded_pack.source.label());
                    println!("Gadget manifest source: {}", loaded_git.source.label());
                    println!("Safety: this verifies approval, rejects protected current branches, stages only approved patch files, and creates one local commit.");
                    println!("No checkout, switch, push, pull, fetch, merge, rebase, PR, provider, patch apply, shell, tests, Linux admin, database, cloud, or deployment action will run.");
                    println!();

                    match run_git_commit_approved_patch(&project_root, &loaded_git.manifest, request) {
                        Ok(report) => {
                            println!("Git commit run: {}", report.run_id);
                            println!("Approval request: {}", report.approval_request_id);
                            println!("Branch: {}", report.branch_name);
                            println!("Passed: {}", report.passed);
                            println!("Exit code: {}", report.exit_code.map(|code| code.to_string()).unwrap_or_else(|| "none".to_string()));
                            println!("Commit hash: {}", report.commit_hash.clone().unwrap_or_else(|| "none".to_string()));
                            println!("Approved files: {}", report.approved_files.len());
                            println!("Staged files: {}", report.staged_files.len());
                            println!("Duration ms: {}", report.duration_ms);
                            println!("Audit events appended: {}", report.ledger_events_appended);
                            println!("Evidence: {}", report.evidence_bundle_path.display());
                            println!("Ledger: {}", report.ledger_path.display());
                            if !report.passed {
                                std::process::exit(1);
                            }
                        }
                        Err(err) => {
                            eprintln!("git approved-patch commit failed: {err}");
                            std::process::exit(1);
                        }
                    }
                }
                "help" | "--help" | "-h" => print_git_help(),
                other => {
                    eprintln!("unknown git commit command: {other}");
                    print_git_help();
                    std::process::exit(2);
                }
            }
        }
        "pr" => {
            let Some(pr_subcommand) = args.get(1) else {
                print_git_help();
                std::process::exit(2);
            };
            match pr_subcommand.as_str() {
                "body" => {
                    let (project_root, rest) = parse_project_option(args[2..].to_vec());
                    let (approval_request_id, test_run_id, commit_run_id, title) = parse_git_pr_body_args(rest);

                    let config = match load_project_config(&project_root) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Gadgets config: {err}");
                            std::process::exit(1);
                        }
                    };
                    let runtime_mode = match config.runtime_mode() {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("invalid Gadgets runtime mode: {err}");
                            std::process::exit(1);
                        }
                    };
                    if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK) {
                        eprintln!("cannot run Developer Pack Git workflow: {err}");
                        std::process::exit(1);
                    }

                    let loaded_pack = match load_pack_manifest(&project_root, DEVELOPER_PACK) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Developer Pack manifest: {err}");
                            std::process::exit(1);
                        }
                    };
                    let loaded_git = match load_gadget_manifest(&project_root, &loaded_pack, GIT_PR_GADGET) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Git Gadget manifest: {err}");
                            std::process::exit(1);
                        }
                    };

                    let run_id = format!("run_git_pr_body_{}", unix_timestamp_millis());
                    let request = GitPrBodyRequest::local_body(
                        run_id,
                        unix_timestamp_label(),
                        approval_request_id.clone(),
                    )
                    .with_runtime_mode(runtime_mode)
                    .with_test_run_id(test_run_id.clone())
                    .with_commit_run_id(commit_run_id.clone())
                    .with_title_override(title.clone());

                    println!("Gadgets {} Mode local PR body generation", runtime_mode.as_str());
                    println!("Project root: {}", project_root.display());
                    println!("Approval request: {approval_request_id}");
                    if let Some(value) = test_run_id.as_deref() {
                        println!("Test evidence run: {value}");
                    }
                    if let Some(value) = commit_run_id.as_deref() {
                        println!("Commit evidence run: {value}");
                    }
                    println!("Pack: {} ({})", loaded_pack.manifest.metadata.name, loaded_pack.source.label());
                    println!("Gadget manifest source: {}", loaded_git.source.label());
                    println!("Safety: this generates local PR Markdown evidence only from verified approval and optional evidence references.");
                    println!("No remote PR, GitHub/GitLab API, push, pull, fetch, merge, rebase, provider, patch apply, test run, shell, Linux admin, database, cloud, or deployment action will run.");
                    println!();

                    match run_git_pr_body(&project_root, &loaded_git.manifest, request) {
                        Ok(report) => {
                            println!("PR body run: {}", report.run_id);
                            println!("Approval request: {}", report.approval_request_id);
                            println!("Title: {}", report.title);
                            println!("PR body: {}", report.body_artifact_path.display());
                            println!("Audit events appended: {}", report.ledger_events_appended);
                            println!("Evidence: {}", report.evidence_bundle_path.display());
                            println!("Ledger: {}", report.ledger_path.display());
                        }
                        Err(err) => {
                            eprintln!("git PR body generation failed: {err}");
                            std::process::exit(1);
                        }
                    }
                }
                "create" => {
                    let (project_root, rest) = parse_project_option(args[2..].to_vec());
                    let config = match load_project_config(&project_root) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Gadgets config: {err}");
                            std::process::exit(1);
                        }
                    };
                    let (approval_request_id, pr_body_run_id, head_branch, base_branch, title) =
                        parse_git_pr_create_args(rest, &config.git.remote_pr.default_base_branch);
                    let runtime_mode = match config.runtime_mode() {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("invalid Gadgets runtime mode: {err}");
                            std::process::exit(1);
                        }
                    };
                    if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK) {
                        eprintln!("cannot run Developer Pack Git workflow: {err}");
                        std::process::exit(1);
                    }

                    let loaded_pack = match load_pack_manifest(&project_root, DEVELOPER_PACK) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Developer Pack manifest: {err}");
                            std::process::exit(1);
                        }
                    };
                    let loaded_git = match load_gadget_manifest(&project_root, &loaded_pack, GIT_PR_GADGET) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Git Gadget manifest: {err}");
                            std::process::exit(1);
                        }
                    };

                    let run_id = format!("run_git_pr_create_{}", unix_timestamp_millis());
                    let remote_config = remote_pr_provider_config(&config.git.remote_pr);
                    let request = GitRemotePrRequest::create_remote_pr(
                        run_id,
                        unix_timestamp_label(),
                        approval_request_id.clone(),
                        pr_body_run_id.clone(),
                        head_branch.clone(),
                        base_branch.clone(),
                        remote_config,
                    )
                    .with_runtime_mode(runtime_mode)
                    .with_title_override(title.clone());

                    println!("Gadgets {} Mode guarded remote PR creation", runtime_mode.as_str());
                    println!("Project root: {}", project_root.display());
                    println!("Approval request: {approval_request_id}");
                    println!("PR body run: {pr_body_run_id}");
                    println!("Head branch: {head_branch}");
                    println!("Base branch: {base_branch}");
                    println!("Pack: {} ({})", loaded_pack.manifest.metadata.name, loaded_pack.source.label());
                    println!("Gadget manifest source: {}", loaded_git.source.label());
                    println!("Safety: this requires explicit remote_pr.enabled config, verified approval, and local PR body evidence before one GitHub API call.");
                    println!("No Git push, pull, fetch, merge, rebase, checkout, switch, shell, provider tool, patch apply, test run, Linux admin, database, cloud, or deployment action will run.");
                    println!();

                    match run_git_remote_pr_create(&project_root, &loaded_git.manifest, request) {
                        Ok(report) => {
                            println!("Remote PR run: {}", report.run_id);
                            println!("Repository: {}", report.repository);
                            println!("Title: {}", report.title);
                            println!("Passed: {}", report.passed);
                            println!("HTTP status: {}", report.http_status.map(|code| code.to_string()).unwrap_or_else(|| "none".to_string()));
                            println!("PR number: {}", report.pr_number.map(|value| value.to_string()).unwrap_or_else(|| "none".to_string()));
                            println!("PR URL: {}", report.pr_url.clone().unwrap_or_else(|| "none".to_string()));
                            println!("Duration ms: {}", report.duration_ms);
                            println!("Audit events appended: {}", report.ledger_events_appended);
                            println!("Evidence: {}", report.evidence_bundle_path.display());
                            println!("Ledger: {}", report.ledger_path.display());
                            if !report.passed {
                                std::process::exit(1);
                            }
                        }
                        Err(err) => {
                            eprintln!("git remote PR creation failed: {err}");
                            std::process::exit(1);
                        }
                    }
                }
                "help" | "--help" | "-h" => print_git_help(),
                other => {
                    eprintln!("unknown git pr command: {other}");
                    print_git_help();
                    std::process::exit(2);
                }
            }
        }
        "help" | "--help" | "-h" => print_git_help(),
        other => {
            eprintln!("unknown git command: {other}");
            print_git_help();
            std::process::exit(2);
        }
    }
}

fn handle_test(args: Vec<String>) {
    let Some(subcommand) = args.first() else {
        print_test_help();
        std::process::exit(2);
    };

    match subcommand.as_str() {
        "run" => {
            let (project_root, rest) = parse_project_option(args[1..].to_vec());
            if rest.len() != 1 {
                print_test_help();
                std::process::exit(2);
            }
            let command_name = &rest[0];
            if !valid_test_command_name(command_name) {
                eprintln!("invalid test command name: {command_name}");
                std::process::exit(2);
            }

            let config = match load_project_config(&project_root) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Gadgets config: {err}");
                    std::process::exit(1);
                }
            };
            let runtime_mode = match config.runtime_mode() {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("invalid Gadgets runtime mode: {err}");
                    std::process::exit(1);
                }
            };
            if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK) {
                eprintln!("cannot run Developer Pack test workflow: {err}");
                std::process::exit(1);
            }
            let Some(configured_command) = config
                .test_commands
                .iter()
                .find(|item| item.name == *command_name)
            else {
                eprintln!("unknown test command `{command_name}`; add it to .gadgets/config.yaml test_commands first");
                std::process::exit(2);
            };

            let loaded_pack = match load_pack_manifest(&project_root, DEVELOPER_PACK) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Developer Pack manifest: {err}");
                    std::process::exit(1);
                }
            };
            let loaded_test_runner = match load_gadget_manifest(&project_root, &loaded_pack, TEST_RUNNER_GADGET) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Test Runner Gadget manifest: {err}");
                    std::process::exit(1);
                }
            };

            let test_run_id = format!("run_test_{}", unix_timestamp_millis());
            let command_spec = TestCommandSpec {
                name: configured_command.name.clone(),
                command: configured_command.command.clone(),
                working_dir: configured_command.working_dir.clone(),
                timeout_seconds: configured_command.timeout_seconds,
            };
            let request = TestRunRequest::named_command(
                test_run_id,
                unix_timestamp_label(),
                command_spec,
            )
            .with_runtime_mode(runtime_mode);

            println!("Gadgets {} Mode allowlisted test run", runtime_mode.as_str());
            println!("Project root: {}", project_root.display());
            println!("Command name: {command_name}");
            println!("Pack: {} ({})", loaded_pack.manifest.metadata.name, loaded_pack.source.label());
            println!("Gadget manifest source: {}", loaded_test_runner.source.label());
            println!("Safety: the command string is loaded from .gadgets/config.yaml by name only.");
            println!("No model provider, patch apply, Git, PR, Linux admin, database, cloud, or deployment action will run.");
            println!();

            match run_test_command(&project_root, &loaded_test_runner.manifest, request) {
                Ok(report) => {
                    println!("Test run: {}", report.run_id);
                    println!("Command: {}", report.command_name);
                    println!("Passed: {}", report.passed);
                    println!("Timed out: {}", report.timed_out);
                    println!("Exit code: {}", report.exit_code.map(|code| code.to_string()).unwrap_or_else(|| "none".to_string()));
                    println!("Duration ms: {}", report.duration_ms);
                    println!("Audit events appended: {}", report.ledger_events_appended);
                    println!("Evidence: {}", report.evidence_bundle_path.display());
                    println!("Ledger: {}", report.ledger_path.display());
                    if !report.passed {
                        std::process::exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("test run failed: {err}");
                    std::process::exit(1);
                }
            }
        }
        "help" | "--help" | "-h" => print_test_help(),
        other => {
            eprintln!("unknown test command: {other}");
            print_test_help();
            std::process::exit(2);
        }
    }
}

fn handle_patch(args: Vec<String>) {
    let Some(subcommand) = args.first() else {
        print_patch_help();
        std::process::exit(2);
    };

    match subcommand.as_str() {
        "apply" => {
            let (project_root, rest) = parse_project_option(args[1..].to_vec());
            let Some(approval_request_id) = rest.first() else {
                print_patch_help();
                std::process::exit(2);
            };

            let config = match load_project_config(&project_root) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Gadgets config: {err}");
                    std::process::exit(1);
                }
            };
            let runtime_mode = match config.runtime_mode() {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("invalid Gadgets runtime mode: {err}");
                    std::process::exit(1);
                }
            };
            if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK) {
                eprintln!("cannot run Developer Pack patch workflow: {err}");
                std::process::exit(1);
            }
            let loaded_pack = match load_pack_manifest(&project_root, DEVELOPER_PACK) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Developer Pack manifest: {err}");
                    std::process::exit(1);
                }
            };
            let loaded_patch_writer = match load_gadget_manifest(&project_root, &loaded_pack, PATCH_WRITER_GADGET) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("failed to load Patch Writer Gadget manifest: {err}");
                    std::process::exit(1);
                }
            };

            let apply_run_id = format!("run_apply_{}", unix_timestamp_millis());
            let request = PatchApplyRequest::local_apply(
                approval_request_id.to_string(),
                apply_run_id,
                unix_timestamp_label(),
            )
            .with_runtime_mode(runtime_mode);

            println!("Gadgets {} Mode approved local patch apply", runtime_mode.as_str());
            println!("Project root: {}", project_root.display());
            println!("Approval request: {approval_request_id}");
            println!("Pack: {} ({})", loaded_pack.manifest.metadata.name, loaded_pack.source.label());
            println!("Gadget manifest source: {}", loaded_patch_writer.source.label());
            println!("Safety: approval record, scope hash, patch hash, and path policy must all verify before any write.");
            println!("No shell commands, tests, Git actions, provider tools, PR actions, or admin actions will run.");
            println!();

            match run_patch_apply(&project_root, &loaded_patch_writer.manifest, request) {
                Ok(report) => {
                    println!("Apply run: {}", report.apply_run_id);
                    println!("Source plan run: {}", report.source_plan_run_id);
                    println!("Approval request: {}", report.approval_request_id);
                    println!("Files changed: {}", report.files_changed.len());
                    for file in &report.files_changed {
                        println!("  - {file}");
                    }
                    println!("Audit events appended: {}", report.ledger_events_appended);
                    println!("Evidence: {}", report.evidence_bundle_path.display());
                    println!("Ledger: {}", report.ledger_path.display());
                }
                Err(err) => {
                    eprintln!("patch apply failed: {err}");
                    std::process::exit(1);
                }
            }
        }
        "help" | "--help" | "-h" => print_patch_help(),
        other => {
            eprintln!("unknown patch command: {other}");
            print_patch_help();
            std::process::exit(2);
        }
    }
}

fn handle_approval(args: Vec<String>) {
    let Some(subcommand) = args.first() else {
        print_approval_help();
        std::process::exit(2);
    };

    match subcommand.as_str() {
        "request-patch" => {
            let (project_root, rest) = parse_project_option(args[1..].to_vec());
            let Some(run_id) = rest.first() else {
                print_approval_help();
                std::process::exit(2);
            };
            let expires_at = parse_optional_named_value(&rest[1..], "--expires-at");
            let mut input = PatchApprovalRequestInput::local_patch_apply(
                run_id.as_str(),
                unix_timestamp_label(),
            );
            if let Some(value) = expires_at {
                input = input.with_expires_at(value);
            }
            match create_patch_approval_request(&project_root, input) {
                Ok(report) => {
                    if let Err(err) = append_approval_audit(
                        &project_root,
                        run_id,
                        "approval.requested",
                        "approval",
                        &report.approval_request_id,
                        "allowed",
                        "Patch apply approval request created and bound to proposed patch hash.",
                    ) {
                        eprintln!("approval request was created, but audit append failed: {err}");
                        std::process::exit(1);
                    }
                    println!("Approval request created: {}", report.approval_request_id);
                    println!("Request file: {}", report.request_path.display());
                    println!("Patch artifact: {}", report.patch_path.display());
                    println!("Patch SHA-256: {}", report.patch_sha256);
                    println!("Scope hash: {}", report.scope_hash);
                    println!("This does not apply the patch. A future apply step must still pass policy checks.");
                }
                Err(err) => {
                    eprintln!("failed to create approval request: {err}");
                    std::process::exit(1);
                }
            }
        }
        "approve" => {
            let (project_root, rest) = parse_project_option(args[1..].to_vec());
            let Some(approval_request_id) = rest.first() else {
                print_approval_help();
                std::process::exit(2);
            };
            let Some(approver) = rest.get(1) else {
                print_approval_help();
                std::process::exit(2);
            };
            match approve_request(&project_root, approval_request_id, approver, unix_timestamp_label()) {
                Ok(report) => {
                    let run_id = approval_request_id
                        .strip_prefix("apr_")
                        .and_then(|value| value.strip_suffix("_patch"))
                        .unwrap_or(approval_request_id);
                    if let Err(err) = append_approval_audit(
                        &project_root,
                        run_id,
                        "approval.approved",
                        "approval",
                        approval_request_id,
                        "allowed",
                        "Patch apply approval record created for exact scope hash.",
                    ) {
                        eprintln!("approval record was created, but audit append failed: {err}");
                        std::process::exit(1);
                    }
                    println!("Approval recorded: {}", report.approval_id);
                    println!("Request: {}", report.approval_request_id);
                    println!("Approval file: {}", report.approval_path.display());
                    println!("Scope hash: {}", report.scope_hash);
                    println!("This does not apply the patch. It only records a scoped approval.");
                }
                Err(err) => {
                    eprintln!("failed to approve request: {err}");
                    std::process::exit(1);
                }
            }
        }
        "show" => {
            let (project_root, rest) = parse_project_option(args[1..].to_vec());
            let Some(approval_request_id) = rest.first() else {
                print_approval_help();
                std::process::exit(2);
            };
            match read_request(&project_root, approval_request_id) {
                Ok(request) => {
                    println!("Approval request: {}", request.approval_request_id);
                    println!("Status: {}", request.status);
                    println!("Action: {}", request.action_kind);
                    println!("Executor: {}", request.executor_gadget);
                    println!("Zone: {}", request.target.zone);
                    println!("Run: {}", request.target.run_id);
                    println!("Patch SHA-256: {}", request.target.patch_sha256);
                    println!("Scope hash: {}", request.scope_hash);
                    if let Some(expires_at) = request.expires_at.as_deref() {
                        println!("Expires: {expires_at}");
                    }
                    match read_approval(&project_root, approval_request_id) {
                        Ok(Some(record)) => {
                            println!();
                            println!("Approval record: {}", record.approval_id);
                            println!("Approved by: {}", record.approved_by);
                            println!("Approved at: {}", record.approved_at);
                            println!("Status: {}", record.status);
                            println!("Scope hash: {}", record.scope_hash);
                        }
                        Ok(None) => println!("Approval record: not present"),
                        Err(err) => {
                            eprintln!("failed to read approval record: {err}");
                            std::process::exit(1);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("failed to read approval request: {err}");
                    std::process::exit(1);
                }
            }
        }
        "verify" => {
            let (project_root, rest) = parse_project_option(args[1..].to_vec());
            let Some(approval_request_id) = rest.first() else {
                print_approval_help();
                std::process::exit(2);
            };
            match verify_approval(&project_root, approval_request_id) {
                Ok(verification) if verification.valid => {
                    println!("Approval record verified: {}", verification.approval_request_id);
                    println!("Request: {}", verification.request_path.display());
                    if let Some(path) = verification.approval_path {
                        println!("Approval: {}", path.display());
                    } else {
                        println!("Approval: not present; verification should not be valid for apply.");
                    }
                    println!("Patch artifact: {}", verification.patch_path.display());
                }
                Ok(verification) => {
                    eprintln!("Approval verification failed: {}", verification.approval_request_id);
                    for err in verification.errors {
                        eprintln!("  - {err}");
                    }
                    std::process::exit(1);
                }
                Err(err) => {
                    eprintln!("failed to verify approval: {err}");
                    std::process::exit(1);
                }
            }
        }
        "id-for-run" => {
            let Some(run_id) = args.get(1) else {
                print_approval_help();
                std::process::exit(2);
            };
            println!("{}", patch_approval_request_id(run_id));
        }
        "help" | "--help" | "-h" => print_approval_help(),
        other => {
            eprintln!("unknown approval command: {other}");
            print_approval_help();
            std::process::exit(2);
        }
    }
}

fn parse_optional_named_value(args: &[String], name: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == name {
            let Some(value) = iter.next() else {
                eprintln!("{name} requires a value");
                std::process::exit(2);
            };
            return Some(value.clone());
        }
    }
    None
}

fn append_approval_audit(
    project_root: &Path,
    run_id: &str,
    event_type: &str,
    target_kind: &str,
    target_id: &str,
    decision: &str,
    summary: &str,
) -> Result<(), gadgets_ledger::LedgerError> {
    let ledger_path = default_ledger_path(project_root);
    let event = new_audit_event(
        format!("aud_{}_{}", run_id, unix_timestamp_millis()),
        unix_timestamp_label(),
        event_type,
        "gadget",
        "approval",
        run_id,
        decision,
        summary,
    );
    let event = with_target(event, target_kind, target_id);
    append_event(&ledger_path, event)?;
    Ok(())
}

fn ledger_path_from_arg(arg: Option<&String>) -> PathBuf {
    match arg {
        Some(value) => {
            let path = PathBuf::from(value);
            if path.is_dir() {
                default_ledger_path(&path)
            } else {
                path
            }
        }
        None => default_ledger_path(Path::new(".")),
    }
}

fn unix_timestamp_label() -> String {
    format!("unix_ms:{}", unix_timestamp_millis())
}

fn unix_timestamp_millis() -> u128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_millis(),
        Err(_) => 0,
    }
}

fn print_help() {
    println!("Gadgets Framework CLI");
    println!();
    println!("Usage:");
    println!("  gadgets init [path]");
    println!("  gadgets ask [--project <path>] <request>");
    println!("  gadgets ledger show [project-root-or-ledger-path]");
    println!("  gadgets ledger verify [project-root-or-ledger-path]");
    println!("  gadgets evidence show <run-id> [project-root]");
    println!("  gadgets evidence verify <run-id> [project-root]");
    println!("  gadgets evidence create-observe <run-id> <gadget> <summary>");
    println!("  gadgets approval request-patch [--project <path>] <run-id> [--expires-at <RFC3339-UTC>]");
    println!("  gadgets approval approve [--project <path>] <approval-request-id> <approver>");
    println!("  gadgets approval show [--project <path>] <approval-request-id>");
    println!("  gadgets approval verify [--project <path>] <approval-request-id>");
    println!("  gadgets patch apply [--project <path>] <approval-request-id>");
    println!("  gadgets test run [--project <path>] <test-command-name>");
    println!("  gadgets git status [--project <path>]");
    println!("  gadgets git branch create [--project <path>] <branch-name>");
    println!("  gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]");
    println!("  gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]");
    println!("  gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]");
    println!("  gadgets pack list [--project <path>]");
    println!("  gadgets pack show [--project <path>] <pack>");
    println!("  gadgets pack validate [--project <path>] [--strict] [pack]");
    println!("  gadgets help");
    println!("  gadgets version");
    println!();
    println!("Current implemented commands:");
    println!("  init      Create local .gadgets project state using Safe Mode defaults.");
    println!("  ask       Run configured provider Coordinator, then execute an allowed observe or plan-only Developer Pack handoff.");
    println!("  ledger    Show or verify the local append-only audit ledger.");
    println!("  evidence  Create, show, or verify observe-only evidence bundles.");
    println!("  approval  Create, show, approve, or verify scoped approval records.");
    println!("  patch     Apply an approved local patch after exact approval and policy verification.");
    println!("  test      Run a named allowlisted test command from .gadgets/config.yaml.");
    println!("  git       Read local Git status, create protected local branches, commit approved patches, generate PR bodies, and create guarded remote PRs through fixed commands.");
    println!("  pack      List, show, or validate Gadget pack manifests.");
}

fn print_test_help() {
    println!("Gadgets test commands");
    println!();
    println!("Usage:");
    println!("  gadgets test run [--project <path>] <test-command-name>");
    println!();
    println!("Test commands must be named entries in .gadgets/config.yaml test_commands.");
    println!("The model and user prompt cannot supply raw command strings.");
    println!("The Test Runner does not apply patches, run Git or PR actions, or perform admin actions.");
}

fn print_git_help() {
    println!("Gadgets Git commands");
    println!();
    println!("Usage:");
    println!("  gadgets git status [--project <path>]");
    println!("  gadgets git branch create [--project <path>] <branch-name>");
    println!("  gadgets git commit approved-patch [--project <path>] <approval-request-id> [--message <message>]");
    println!("  gadgets git pr body [--project <path>] <approval-request-id> [--test-run <run-id>] [--commit-run <run-id>] [--title <title>]");
    println!("  gadgets git pr create [--project <path>] <approval-request-id> --body-run <run-id> --head <branch> [--base <branch>] [--title <title>]");
    println!();
    println!("status runs a fixed local git status command selected by the runtime.");
    println!("branch create runs fixed local git branch creation after branch-name and protected-branch checks.");
    println!("commit approved-patch verifies a scoped approval, rejects protected current branches, stages only approved files, and creates one local commit.");
    println!("pr body generates local Markdown evidence only; it does not create a remote PR.");
    println!("pr create creates one remote GitHub pull request only when remote_pr.enabled is true and approval plus PR-body evidence are verified.");
    println!("Git commands do not checkout, switch, push, pull, fetch, merge, rebase, or run shell commands.");
}

fn print_pack_help() {
    println!("Gadgets pack commands");
    println!();
    println!("Usage:");
    println!("  gadgets pack list [--project <path>]");
    println!("  gadgets pack show [--project <path>] <pack>");
    println!("  gadgets pack validate [--project <path>] [--strict] [pack]");
    println!();
    println!("Packs are loaded from .gadgets/packs/<pack>/pack.yaml when present, then built-in pack manifests.");
    println!("validate checks pack metadata plus declared Gadget manifests; --strict treats missing Gadget manifests as errors.");
}

fn print_ledger_help() {
    println!("Gadgets audit ledger commands");
    println!();
    println!("Usage:");
    println!("  gadgets ledger show [project-root-or-ledger-path]");
    println!("  gadgets ledger verify [project-root-or-ledger-path]");
    println!();
    println!("If no path is provided, ./.gadgets/ledger/events.jsonl is used.");
}

fn print_evidence_help() {
    println!("Gadgets evidence commands");
    println!();
    println!("Usage:");
    println!("  gadgets evidence show <run-id> [project-root]");
    println!("  gadgets evidence verify <run-id> [project-root]");
    println!("  gadgets evidence create-observe <run-id> <gadget> <summary>");
    println!();
    println!("Evidence is stored under .gadgets/runs/<run-id>/evidence/ by default.");
    println!("create-observe is a development helper and does not inspect files or call models.");
}


fn print_approval_help() {
    println!("Gadgets approval commands");
    println!();
    println!("Usage:");
    println!("  gadgets approval request-patch [--project <path>] <run-id> [--expires-at <RFC3339-UTC>]");
    println!("  gadgets approval approve [--project <path>] <approval-request-id> <approver>");
    println!("  gadgets approval show [--project <path>] <approval-request-id>");
    println!("  gadgets approval verify [--project <path>] <approval-request-id>");
    println!("  gadgets approval id-for-run <run-id>");
    println!();
    println!("request-patch binds an approval request to .gadgets/runs/<run-id>/evidence/proposed.patch.");
    println!("approve records approval for the exact scope hash. It does not apply the patch.");
    println!("expires-at must use strict UTC RFC3339 without fractional seconds, for example 2999-01-01T00:00:00Z.");
    println!("verify requires both request.yaml and approval.yaml, checks the exact proposed.patch hash, and rejects expired approvals.");
}

fn print_patch_help() {
    println!("Gadgets patch commands");
    println!();
    println!("Usage:");
    println!("  gadgets patch apply [--project <path>] <approval-request-id>");
    println!();
    println!("apply verifies approval request, approval record, patch hash, scope hash, and path policy before writing files.");
    println!("It does not run shell commands, tests, Git commands, provider tools, PR actions, or admin actions.");
}
