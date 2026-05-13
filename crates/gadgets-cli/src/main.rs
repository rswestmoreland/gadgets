mod config;
mod init;
mod manifest_loader;
mod pack_trust;

use std::env;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use config::{
    ensure_supported_provider, load_project_config, valid_test_command_name,
    RemotePrConfig as CliRemotePrConfig,
};
use gadgets_approval::{
    approve_request, create_patch_approval_request, patch_approval_request_id, read_approval,
    read_request, verify_approval, PatchApprovalRequestInput,
};
use gadgets_evidence::{
    bundle_path_for_run, create_observe_bundle, default_runs_root, summarize_bundle,
    verify_bundle_hash, EvidenceTextArtifact, EvidenceWriteRequest, EvidenceWriteReport,
};
use gadgets_ledger::{
    append_event, default_ledger_path, new_audit_event, summarize_events, verify_ledger,
    with_target,
};
use gadgets_policy::RuntimeMode;
use gadgets_provider::{
    AnthropicProvider, MockProvider, ModelProvider, OpenAiProvider, ProviderError, ProviderRequest,
    ProviderResponseStatus,
};
use gadgets_tools::{
    run_filesystem_read, run_git_branch_create, run_git_commit_approved_patch, run_git_pr_body,
    run_git_remote_pr_create, run_git_status, run_patch_apply, run_patch_plan, run_test_command,
    FilesystemReadRequest, GitBranchCreateRequest, GitCommitRequest, GitPrBodyRequest,
    GitRemotePrRequest, GitStatusRequest, PatchApplyRequest, PatchPlanRequest,
    RemotePrProviderConfig, TestCommandSpec, TestRunRequest,
};
use init::init_project;
use pack_trust::{
    check_pack_trust, inspect_trust_roots, preview_pack_trust_policy,
    verify_pack_signature_metadata, PackSignatureMetadataReport, PackTrustPolicyPreviewReport,
    PackTrustReport, TrustRootsReport, TRUST_ROOT_RELATIVE_PATH as TRUST_ROOT_RELATIVE_PATH_FOR_AUDIT,
};
use manifest_loader::{
    ensure_pack_installed, gadget_manifest_available, load_gadget_manifest,
    load_installed_pack_manifests, load_pack_manifest, validate_installed_packs,
    validate_pack_tree, PackValidationReport, DEVELOPER_PACK, FILESYSTEM_READ_GADGET,
    GIT_PR_GADGET, PATCH_WRITER_GADGET, TEST_RUNNER_GADGET,
};

fn main() {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return;
    };

    match command.as_str() {
        "init" => {
            let target = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            match init_project(&target) {
                Ok(report) => {
                    println!(
                        "Initialized Gadgets project state at {}",
                        report.gadgets_dir.display()
                    );
                    println!("Mode: safe");
                    println!("Provider: mock");
                    println!("File writes require approval: yes");
                    println!("Created directories: {}", report.created_dirs.len());
                    println!("Created files: {}", report.created_files.len());
                    if !report.created_anything() {
                        println!("No changes needed; project was already initialized.");
                    }
                    println!(
                        "Review {} before enabling additional packs or providers.",
                        report.gadgets_dir.join("config.yaml").display()
                    );
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
    let loaded_filesystem =
        match load_gadget_manifest(&project_root, &loaded_pack, FILESYSTEM_READ_GADGET) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("failed to load Filesystem Read Gadget manifest: {err}");
                std::process::exit(1);
            }
        };
    let loaded_patch_writer =
        match load_gadget_manifest(&project_root, &loaded_pack, PATCH_WRITER_GADGET) {
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
        handoff.to_gadget == loaded_filesystem.manifest.metadata.name
            && handoff.task_kind == "repo.inspect"
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
        println!(
            "Pack: {} ({})",
            loaded_pack.manifest.metadata.name,
            loaded_pack.source.label()
        );
        println!(
            "Gadget manifest source: {}",
            loaded_filesystem.source.label()
        );
        println!(
            "Selected handoff: {} -> {}",
            handoff.from_gadget, handoff.to_gadget
        );
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

        println!(
            "Gadgets {} Mode plan-only Patch Writer run",
            runtime_mode.as_str()
        );
        println!("Project root: {}", project_root.display());
        println!("Request: {prompt}");
        println!("Coordinator: {}", provider_response.text_summary);
        println!(
            "Provider profile: {} -> {} / {}",
            selected_profile.name, provider_response.provider, provider_response.model
        );
        println!(
            "Pack: {} ({})",
            loaded_pack.manifest.metadata.name,
            loaded_pack.source.label()
        );
        println!(
            "Gadget manifest source: {}",
            loaded_patch_writer.source.label()
        );
        println!(
            "Selected handoff: {} -> {}",
            handoff.from_gadget, handoff.to_gadget
        );
        for note in &provider_response.safety_notes {
            println!("Safety: {note}");
        }
        println!("Gadget: {}", loaded_patch_writer.manifest.metadata.name);
        println!(
            "No files will be modified. No patch will be applied. No commands will be executed."
        );
        println!();

        match run_patch_plan(&project_root, &loaded_patch_writer.manifest, request) {
            Ok(report) => {
                println!("Run: {}", report.run_id);
                println!("Policy decision: {:?}", report.policy_decision);
                println!("Audit events appended: {}", report.ledger_events_appended);
                println!("Evidence: {}", report.evidence_bundle_path.display());
                println!("Ledger: {}", report.ledger_path.display());
                println!(
                    "Proposed patch artifact: {}/proposed.patch",
                    report
                        .evidence_bundle_path
                        .parent()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| ".".to_string())
                );
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
                        println!(
                            "No installed packs configured at {}",
                            project_root.display()
                        );
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
            let (project_root, strict, requested_pack) =
                parse_pack_validate_args(args[1..].to_vec());
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
        "trust" => handle_pack_trust(args[1..].to_vec()),
        "help" | "--help" | "-h" => print_pack_help(),
        other => {
            eprintln!("unknown pack command: {other}");
            print_pack_help();
            std::process::exit(2);
        }
    }
}

fn handle_pack_trust(args: Vec<String>) {
    let Some(subcommand) = args.first() else {
        print_pack_help();
        std::process::exit(2);
    };

    match subcommand.as_str() {
        "check" => {
            let (project_root, pack_name) = parse_pack_trust_check_args(args[1..].to_vec());
            match check_pack_trust(&project_root, &pack_name) {
                Ok(report) => {
                    let run_id = format!("run_pack_trust_check_{}", unix_timestamp_millis());
                    let created_at = unix_timestamp_label();
                    let evidence = match write_pack_trust_check_evidence(
                        &project_root,
                        &run_id,
                        &created_at,
                        &report,
                    ) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to write pack trust evidence: {err}");
                            std::process::exit(1);
                        }
                    };
                    if let Err(err) = append_pack_trust_audit(
                        &project_root,
                        &run_id,
                        "pack.trust.checked",
                        "pack",
                        &report.pack_name,
                        &report.decision,
                        "Pack trust diagnostic check completed.",
                    ) {
                        eprintln!("failed to append pack trust audit event: {err}");
                        std::process::exit(1);
                    }
                    if let Err(err) = append_pack_trust_audit(
                        &project_root,
                        &run_id,
                        "evidence.created",
                        "evidence_bundle",
                        &format!("evb_{run_id}"),
                        "created",
                        "Pack trust diagnostic evidence bundle created.",
                    ) {
                        eprintln!("failed to append evidence audit event: {err}");
                        std::process::exit(1);
                    }
                    print_pack_trust_report(&report);
                    println!("Run: {run_id}");
                    println!("Evidence: {}", evidence.bundle_path.display());
                    println!("Ledger: {}", default_ledger_path(&project_root).display());
                }
                Err(err) => {
                    eprintln!("failed to inspect pack trust: {err}");
                    std::process::exit(1);
                }
            }
        }
        "preview" => {
            let (project_root, mode_override, pack_name) =
                parse_pack_trust_preview_args(args[1..].to_vec());
            let runtime_mode = match mode_override {
                Some(value) => match parse_pack_trust_preview_mode(&value) {
                    Ok(mode) => mode,
                    Err(err) => {
                        eprintln!("{err}");
                        std::process::exit(2);
                    }
                },
                None => {
                    let config = match load_project_config(&project_root) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to load Gadgets config: {err}");
                            std::process::exit(1);
                        }
                    };
                    match config.runtime_mode() {
                        Ok(mode) => mode,
                        Err(err) => {
                            eprintln!("invalid Gadgets runtime mode: {err}");
                            std::process::exit(1);
                        }
                    }
                }
            };
            match preview_pack_trust_policy(&project_root, &pack_name, runtime_mode) {
                Ok(report) => {
                    let run_id = format!("run_pack_trust_preview_{}", unix_timestamp_millis());
                    let created_at = unix_timestamp_label();
                    let evidence = match write_pack_trust_preview_evidence(
                        &project_root,
                        &run_id,
                        &created_at,
                        &report,
                    ) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to write pack trust preview evidence: {err}");
                            std::process::exit(1);
                        }
                    };
                    if let Err(err) = append_pack_trust_audit(
                        &project_root,
                        &run_id,
                        "pack.trust.policy.previewed",
                        "pack",
                        &report.pack.pack_name,
                        &report.preview_decision,
                        "Pack trust policy preview completed.",
                    ) {
                        eprintln!("failed to append pack trust preview audit event: {err}");
                        std::process::exit(1);
                    }
                    if let Err(err) = append_pack_trust_audit(
                        &project_root,
                        &run_id,
                        "evidence.created",
                        "evidence_bundle",
                        &format!("evb_{run_id}"),
                        "created",
                        "Pack trust policy preview evidence bundle created.",
                    ) {
                        eprintln!("failed to append evidence audit event: {err}");
                        std::process::exit(1);
                    }
                    print_pack_trust_preview_report(&report);
                    println!("Run: {run_id}");
                    println!("Evidence: {}", evidence.bundle_path.display());
                    println!("Ledger: {}", default_ledger_path(&project_root).display());
                }
                Err(err) => {
                    eprintln!("failed to preview pack trust policy: {err}");
                    std::process::exit(1);
                }
            }
        }
        "signature" => {
            let (project_root, pack_name) = parse_pack_trust_check_args(args[1..].to_vec());
            match verify_pack_signature_metadata(&project_root, &pack_name) {
                Ok(report) => {
                    let run_id = format!("run_pack_signature_{}", unix_timestamp_millis());
                    let created_at = unix_timestamp_label();
                    let evidence = match write_pack_signature_metadata_evidence(
                        &project_root,
                        &run_id,
                        &created_at,
                        &report,
                    ) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to write signature verification evidence: {err}");
                            std::process::exit(1);
                        }
                    };
                    if let Err(err) = append_pack_trust_audit(
                        &project_root,
                        &run_id,
                        "pack.signature.checked",
                        "pack",
                        &report.pack.pack_name,
                        &report.metadata_decision,
                        "Pack signature verification diagnostic check completed.",
                    ) {
                        eprintln!("failed to append signature metadata audit event: {err}");
                        std::process::exit(1);
                    }
                    if let Err(err) = append_pack_trust_audit(
                        &project_root,
                        &run_id,
                        "evidence.created",
                        "evidence_bundle",
                        &format!("evb_{run_id}"),
                        "created",
                        "Pack signature verification diagnostic evidence bundle created.",
                    ) {
                        eprintln!("failed to append evidence audit event: {err}");
                        std::process::exit(1);
                    }
                    print_pack_signature_metadata_report(&report);
                    println!("Run: {run_id}");
                    println!("Evidence: {}", evidence.bundle_path.display());
                    println!("Ledger: {}", default_ledger_path(&project_root).display());
                }
                Err(err) => {
                    eprintln!("failed to check pack signature: {err}");
                    std::process::exit(1);
                }
            }
        }
        "roots" => {
            let project_root = parse_pack_trust_roots_args(args[1..].to_vec());
            match inspect_trust_roots(&project_root) {
                Ok(report) => {
                    let run_id = format!("run_trust_roots_{}", unix_timestamp_millis());
                    let created_at = unix_timestamp_label();
                    let evidence = match write_trust_roots_evidence(
                        &project_root,
                        &run_id,
                        &created_at,
                        &report,
                    ) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to write trust root evidence: {err}");
                            std::process::exit(1);
                        }
                    };
                    let event_type = if report.exists {
                        "trust.root.loaded"
                    } else {
                        "trust.root.missing"
                    };
                    let decision = if report.exists { "loaded" } else { "missing" };
                    if let Err(err) = append_pack_trust_audit(
                        &project_root,
                        &run_id,
                        event_type,
                        "trust_root",
                        TRUST_ROOT_RELATIVE_PATH_FOR_AUDIT,
                        decision,
                        "Trust root diagnostic inspection completed.",
                    ) {
                        eprintln!("failed to append trust root audit event: {err}");
                        std::process::exit(1);
                    }
                    if let Err(err) = append_pack_trust_audit(
                        &project_root,
                        &run_id,
                        "evidence.created",
                        "evidence_bundle",
                        &format!("evb_{run_id}"),
                        "created",
                        "Trust root diagnostic evidence bundle created.",
                    ) {
                        eprintln!("failed to append evidence audit event: {err}");
                        std::process::exit(1);
                    }
                    print_trust_roots_report(&report);
                    println!("Run: {run_id}");
                    println!("Evidence: {}", evidence.bundle_path.display());
                    println!("Ledger: {}", default_ledger_path(&project_root).display());
                }
                Err(err) => {
                    eprintln!("failed to inspect trust roots: {err}");
                    std::process::exit(1);
                }
            }
        }
        "help" | "--help" | "-h" => print_pack_help(),
        other => {
            eprintln!("unknown pack trust command: {other}");
            print_pack_help();
            std::process::exit(2);
        }
    }
}


fn write_pack_trust_check_evidence(
    project_root: &Path,
    run_id: &str,
    created_at: &str,
    report: &PackTrustReport,
) -> Result<EvidenceWriteReport, gadgets_evidence::EvidenceError> {
    let mut request = EvidenceWriteRequest::observe(
        run_id,
        "pack.trust",
        created_at,
        format!(
            "Pack trust diagnostic check completed for {} with decision {}.",
            report.pack_name, report.decision
        ),
    );
    request.extra_artifacts = vec![
        EvidenceTextArtifact::new(
            "pack_trust_decision",
            "pack_trust_decision.txt",
            format!(
                "decision: {}\nenforcement_enabled: {}\nsource_kind: {}\n",
                report.decision,
                report.enforcement_enabled,
                report.source_kind.as_str()
            ),
        ),
        EvidenceTextArtifact::new("pack_identity", "pack_identity.yaml", pack_identity_yaml(report)),
        EvidenceTextArtifact::new(
            "pack_manifest_hash",
            "pack_manifest_hash.txt",
            format!("manifest_sha256: {}\n", report.manifest_sha256),
        ),
        EvidenceTextArtifact::new(
            "pack_contents_summary",
            "pack_contents_summary.txt",
            pack_contents_summary_text(report),
        ),
        EvidenceTextArtifact::new(
            "pack_signature_summary",
            "pack_signature_summary.yaml",
            pack_signature_summary_yaml(report),
        ),
        EvidenceTextArtifact::new(
            "trust_root_summary",
            "trust_root_summary.txt",
            format!(
                "trust_roots_present: {}\npath: {}\n",
                report.trust_roots_present, TRUST_ROOT_RELATIVE_PATH_FOR_AUDIT
            ),
        ),
        EvidenceTextArtifact::new(
            "trust_findings",
            "trust_findings.txt",
            pack_findings_text(&report.findings),
        ),
        EvidenceTextArtifact::new(
            "policy_mode",
            "policy_mode.txt",
            "diagnostic_only: true\nenforcement_enabled: false\n".to_string(),
        ),
    ];
    create_observe_bundle(&default_runs_root(project_root), request)
}

fn write_pack_trust_preview_evidence(
    project_root: &Path,
    run_id: &str,
    created_at: &str,
    report: &PackTrustPolicyPreviewReport,
) -> Result<EvidenceWriteReport, gadgets_evidence::EvidenceError> {
    let mut request = EvidenceWriteRequest::observe(
        run_id,
        "pack.trust",
        created_at,
        format!(
            "Pack trust policy preview completed for {} in {} mode with decision {}.",
            report.pack.pack_name,
            report.runtime_mode.as_str(),
            report.preview_decision
        ),
    );
    request.extra_artifacts = vec![
        EvidenceTextArtifact::new(
            "pack_trust_policy_preview",
            "pack_trust_policy_preview.txt",
            format!(
                "runtime_mode: {}\npreview_decision: {}\nwould_allow_load: {}\nwould_require_verified_signature: {}\nwould_require_trust_root: {}\nenforcement_active: {}\nsignature_metadata_decision: {}\nsignature_present: {}\ncryptographic_verification_performed: {}\ncryptographic_verification_valid: {}\ncontent_manifest_valid: {}\nsignature_expired: {}\ntrust_root_expired: {}\n",
                report.runtime_mode.as_str(),
                report.preview_decision,
                report.would_allow_load,
                report.would_require_verified_signature,
                report.would_require_trust_root,
                report.enforcement_active,
                report.signature_metadata_decision,
                report.signature_present,
                report.cryptographic_verification_performed,
                report.cryptographic_verification_valid,
                report.content_manifest_valid,
                report.signature_expired,
                report.trust_root_expired
            ),
        ),
        EvidenceTextArtifact::new(
            "pack_identity",
            "pack_identity.yaml",
            pack_identity_yaml(&report.pack),
        ),
        EvidenceTextArtifact::new(
            "pack_manifest_hash",
            "pack_manifest_hash.txt",
            format!("manifest_sha256: {}\n", report.pack.manifest_sha256),
        ),
        EvidenceTextArtifact::new(
            "pack_trust_decision",
            "pack_trust_decision.txt",
            format!(
                "diagnostic_decision: {}\npolicy_preview_decision: {}\nsignature_metadata_decision: {}\ncryptographic_verification_valid: {}\n",
                report.pack.decision,
                report.preview_decision,
                report.signature_metadata_decision,
                report.cryptographic_verification_valid
            ),
        ),
        EvidenceTextArtifact::new(
            "signature_policy_inputs",
            "signature_policy_inputs.txt",
            format!(
                "signature_present: {}\ncryptographic_verification_performed: {}\ncryptographic_verification_valid: {}\ncontent_manifest_valid: {}\nsignature_expired: {}\ntrust_root_expired: {}\n",
                report.signature_present,
                report.cryptographic_verification_performed,
                report.cryptographic_verification_valid,
                report.content_manifest_valid,
                report.signature_expired,
                report.trust_root_expired
            ),
        ),
        EvidenceTextArtifact::new(
            "trust_findings",
            "trust_findings.txt",
            pack_findings_text(&report.findings),
        ),
        EvidenceTextArtifact::new(
            "policy_mode",
            "policy_mode.txt",
            "diagnostic_only: true\nenforcement_active: false\n".to_string(),
        ),
    ];
    create_observe_bundle(&default_runs_root(project_root), request)
}

fn write_pack_signature_metadata_evidence(
    project_root: &Path,
    run_id: &str,
    created_at: &str,
    report: &PackSignatureMetadataReport,
) -> Result<EvidenceWriteReport, gadgets_evidence::EvidenceError> {
    let mut request = EvidenceWriteRequest::observe(
        run_id,
        "pack.trust",
        created_at,
        format!(
            "Pack signature verification diagnostic completed for {} with decision {}.",
            report.pack.pack_name, report.metadata_decision
        ),
    );
    request.extra_artifacts = vec![
        EvidenceTextArtifact::new(
            "signature_metadata_check",
            "signature_metadata_check.txt",
            format!(
                "metadata_decision: {}\nsignature_present: {}\nmetadata_valid: {}\npublisher_reference_found: {}\npack_allowed_by_trust_root: {}\ncontent_manifest_valid: {}\ncryptographic_verification_performed: {}\ncryptographic_verification_valid: {}\nsignature_expired: {}\ntrust_root_expired: {}\n",
                report.metadata_decision,
                report.signature_present,
                report.metadata_valid,
                report.publisher_reference_found,
                report.pack_allowed_by_trust_root,
                report.content_manifest_valid,
                report.cryptographic_verification_performed,
                report.cryptographic_verification_valid,
                report.signature_expired,
                report.trust_root_expired
            ),
        ),
        EvidenceTextArtifact::new(
            "signature_verification_result",
            "signature_verification_result.txt",
            format!(
                "metadata_decision: {}\ncryptographic_verification_performed: {}\ncryptographic_verification_valid: {}\ncontent_manifest_valid: {}\n",
                report.metadata_decision,
                report.cryptographic_verification_performed,
                report.cryptographic_verification_valid,
                report.content_manifest_valid
            ),
        ),
        EvidenceTextArtifact::new(
            "signature_payload_v1",
            "signature_payload_v1.txt",
            report
                .signature_payload_v1
                .clone()
                .unwrap_or_else(|| "signature_payload_v1: unavailable\n".to_string()),
        ),
        EvidenceTextArtifact::new(
            "pack_identity",
            "pack_identity.yaml",
            pack_identity_yaml(&report.pack),
        ),
        EvidenceTextArtifact::new(
            "pack_manifest_hash",
            "pack_manifest_hash.txt",
            format!("manifest_sha256: {}\n", report.pack.manifest_sha256),
        ),
        EvidenceTextArtifact::new(
            "pack_signature_summary",
            "pack_signature_summary.yaml",
            pack_signature_summary_yaml(&report.pack),
        ),
        EvidenceTextArtifact::new(
            "trust_root_summary",
            "trust_root_summary.yaml",
            trust_root_summary_yaml(&report.trust_roots),
        ),
        EvidenceTextArtifact::new(
            "signature_metadata_findings",
            "signature_metadata_findings.txt",
            pack_findings_text(&report.findings),
        ),
        EvidenceTextArtifact::new(
            "policy_mode",
            "policy_mode.txt",
            format!(
                "diagnostic_only: true\nenforcement_enabled: false\ncryptographic_verification_performed: {}\ncryptographic_verification_valid: {}\n",
                report.cryptographic_verification_performed,
                report.cryptographic_verification_valid
            ),
        ),
    ];
    create_observe_bundle(&default_runs_root(project_root), request)
}

fn write_trust_roots_evidence(
    project_root: &Path,
    run_id: &str,
    created_at: &str,
    report: &TrustRootsReport,
) -> Result<EvidenceWriteReport, gadgets_evidence::EvidenceError> {
    let mut request = EvidenceWriteRequest::observe(
        run_id,
        "pack.trust",
        created_at,
        "Trust root diagnostic inspection completed.",
    );
    request.extra_artifacts = vec![
        EvidenceTextArtifact::new(
            "trust_root_path",
            "trust_root_path.txt",
            format!("{}\n", report.path.display()),
        ),
        EvidenceTextArtifact::new(
            "trust_root_summary",
            "trust_root_summary.yaml",
            trust_root_summary_yaml(report),
        ),
        EvidenceTextArtifact::new(
            "trusted_publishers_summary",
            "trusted_publishers_summary.txt",
            trusted_publishers_summary_text(report),
        ),
        EvidenceTextArtifact::new(
            "trust_root_findings",
            "trust_root_findings.txt",
            pack_findings_text(&report.findings),
        ),
    ];
    create_observe_bundle(&default_runs_root(project_root), request)
}

fn append_pack_trust_audit(
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
        "pack.trust",
        run_id,
        decision,
        summary,
    );
    let event = with_target(event, target_kind, target_id);
    append_event(&ledger_path, event)?;
    Ok(())
}

fn pack_identity_yaml(report: &PackTrustReport) -> String {
    format!(
        "pack_name: {}\npack_version: {}\nsource: {}\nsource_kind: {}\ndecision: {}\n",
        report.pack_name,
        report.pack_version,
        report.source,
        report.source_kind.as_str(),
        report.decision
    )
}

fn pack_contents_summary_text(report: &PackTrustReport) -> String {
    match &report.contents_manifest {
        Some(contents) => format!(
            "path: {}\nsha256: {}\nfile_count: {}\n",
            contents.path.display(),
            contents.sha256,
            contents.file_count
        ),
        None => "contents_manifest_present: false\n".to_string(),
    }
}

fn pack_signature_summary_yaml(report: &PackTrustReport) -> String {
    match &report.signature {
        Some(signature) => {
            let mut out = String::new();
            let _ = writeln!(out, "signature_file_present: true");
            let _ = writeln!(out, "path: {}", signature.path.display());
            let _ = writeln!(out, "sha256: {}", signature.sha256);
            let _ = writeln!(
                out,
                "version: {}",
                signature.version.as_deref().unwrap_or("unknown")
            );
            let _ = writeln!(
                out,
                "algorithm: {}",
                signature.algorithm.as_deref().unwrap_or("unknown")
            );
            let _ = writeln!(
                out,
                "publisher: {}",
                signature.publisher.as_deref().unwrap_or("unknown")
            );
            let _ = writeln!(
                out,
                "key_id: {}",
                signature.key_id.as_deref().unwrap_or("unknown")
            );
            if let Some(value) = signature.pack_id.as_deref() {
                let _ = writeln!(out, "pack_id: {value}");
            }
            if let Some(value) = signature.pack_version.as_deref() {
                let _ = writeln!(out, "pack_version: {value}");
            }
            if let Some(value) = signature.manifest_sha256.as_deref() {
                let _ = writeln!(out, "manifest_sha256: {value}");
            }
            if let Some(value) = signature.contents_sha256.as_deref() {
                let _ = writeln!(out, "contents_sha256: {value}");
            }
            if let Some(value) = signature.created_at.as_deref() {
                let _ = writeln!(out, "created_at: {value}");
            }
            if let Some(value) = signature.expires_at.as_deref() {
                let _ = writeln!(out, "expires_at: {value}");
            }
            let _ = writeln!(out, "signature_present: {}", signature.signature_present);
            out
        }
        None => "signature_present: false\n".to_string(),
    }
}

fn trust_root_summary_yaml(report: &TrustRootsReport) -> String {
    format!(
        "path: {}\nexists: {}\nparsed: {}\nversion: {}\npublisher_count: {}\n",
        report.path.display(),
        report.exists,
        report.parsed,
        report.version.as_deref().unwrap_or("unknown"),
        report.publisher_count
    )
}

fn trusted_publishers_summary_text(report: &TrustRootsReport) -> String {
    if report.publishers.is_empty() {
        return "trusted_publishers: none\n".to_string();
    }

    let mut out = String::new();
    for publisher in &report.publishers {
        let _ = writeln!(
            out,
            "publisher: {}",
            publisher.publisher.as_deref().unwrap_or("unknown")
        );
        let _ = writeln!(
            out,
            "key_id: {}",
            publisher.key_id.as_deref().unwrap_or("unknown")
        );
        let _ = writeln!(
            out,
            "algorithm: {}",
            publisher.algorithm.as_deref().unwrap_or("unknown")
        );
        let _ = writeln!(out, "public_key_present: {}", publisher.public_key_present);
        let _ = writeln!(out, "allowed_pack_count: {}", publisher.allowed_pack_count);
        if !publisher.allowed_pack_ids.is_empty() {
            let _ = writeln!(out, "allowed_pack_ids: {}", publisher.allowed_pack_ids.join(","));
        }
        if let Some(value) = publisher.expires_at.as_deref() {
            let _ = writeln!(out, "expires_at: {value}");
        }
        let _ = writeln!(out);
    }
    out
}

fn pack_findings_text(findings: &[pack_trust::PackTrustFinding]) -> String {
    if findings.is_empty() {
        return "none\n".to_string();
    }

    let mut out = String::new();
    for finding in findings {
        let _ = writeln!(out, "{}: {}", finding.severity.as_str(), finding.message);
    }
    out
}

fn parse_pack_trust_roots_args(args: Vec<String>) -> PathBuf {
    let mut project_root = PathBuf::from(".");
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
            _ => {
                eprintln!("unexpected pack trust roots argument: {arg}");
                print_pack_help();
                std::process::exit(2);
            }
        }
    }

    project_root
}

fn parse_pack_trust_check_args(args: Vec<String>) -> (PathBuf, String) {
    let mut project_root = PathBuf::from(".");
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
            "--" => {
                if let Some(value) = iter.next() {
                    if pack_name.replace(value).is_some() {
                        eprintln!("pack trust check accepts exactly one pack name");
                        std::process::exit(2);
                    }
                }
                if iter.next().is_some() {
                    eprintln!("pack trust check accepts exactly one pack name");
                    std::process::exit(2);
                }
                break;
            }
            _ => {
                if pack_name.replace(arg).is_some() {
                    eprintln!("pack trust check accepts exactly one pack name");
                    std::process::exit(2);
                }
            }
        }
    }

    let Some(pack_name) = pack_name else {
        print_pack_help();
        std::process::exit(2);
    };

    (project_root, pack_name)
}

fn parse_pack_trust_preview_args(args: Vec<String>) -> (PathBuf, Option<String>, String) {
    let mut project_root = PathBuf::from(".");
    let mut mode_override = None;
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
            "--mode" => {
                let Some(value) = iter.next() else {
                    eprintln!("--mode requires safe, team, or production");
                    std::process::exit(2);
                };
                mode_override = Some(value);
            }
            "--" => {
                if let Some(value) = iter.next() {
                    if pack_name.replace(value).is_some() {
                        eprintln!("pack trust preview accepts exactly one pack name");
                        std::process::exit(2);
                    }
                }
                if iter.next().is_some() {
                    eprintln!("pack trust preview accepts exactly one pack name");
                    std::process::exit(2);
                }
                break;
            }
            _ => {
                if pack_name.replace(arg).is_some() {
                    eprintln!("pack trust preview accepts exactly one pack name");
                    std::process::exit(2);
                }
            }
        }
    }

    let Some(pack_name) = pack_name else {
        print_pack_help();
        std::process::exit(2);
    };

    (project_root, mode_override, pack_name)
}

fn parse_pack_trust_preview_mode(input: &str) -> Result<RuntimeMode, String> {
    match input {
        "safe" => Ok(RuntimeMode::Safe),
        "team" => Ok(RuntimeMode::Team),
        "production" => Ok(RuntimeMode::Production),
        other => Err(format!(
            "invalid pack trust preview mode `{other}`; expected safe, team, or production"
        )),
    }
}

fn print_pack_trust_preview_report(report: &PackTrustPolicyPreviewReport) {
    println!("Pack trust policy preview: {}", report.pack.pack_name);
    println!("Version: {}", report.pack.pack_version);
    println!("Runtime mode: {}", report.runtime_mode.as_str());
    println!("Source: {}", report.pack.source);
    println!("Source kind: {}", report.pack.source_kind.as_str());
    println!("Diagnostic decision: {}", report.pack.decision);
    println!("Policy preview decision: {}", report.preview_decision);
    println!("Signature metadata decision: {}", report.signature_metadata_decision);
    println!("Signature present: {}", report.signature_present);
    println!(
        "Cryptographic verification performed: {}",
        report.cryptographic_verification_performed
    );
    println!(
        "Cryptographic verification valid: {}",
        report.cryptographic_verification_valid
    );
    println!("Content manifest valid: {}", report.content_manifest_valid);
    println!("Signature expired: {}", report.signature_expired);
    println!("Trust root expired: {}", report.trust_root_expired);
    println!("Would allow load: {}", report.would_allow_load);
    println!(
        "Would require verified signature: {}",
        report.would_require_verified_signature
    );
    println!("Would require trust root: {}", report.would_require_trust_root);
    println!("Enforcement active: {}", report.enforcement_active);
    println!("Manifest SHA-256: {}", report.pack.manifest_sha256);

    if !report.findings.is_empty() {
        println!();
        println!("Findings:");
        for finding in &report.findings {
            println!("  - {}: {}", finding.severity.as_str(), finding.message);
        }
    }

    println!();
    println!("This command is diagnostic only. It consumes signature verification diagnostics to preview future pack trust policy outcomes, but it does not enforce signatures, mutate trust roots, install packs, download packs, or execute Gadgets.");
}

fn print_pack_signature_metadata_report(report: &PackSignatureMetadataReport) {
    println!("Pack signature verification check: {}", report.pack.pack_name);
    println!("Version: {}", report.pack.pack_version);
    println!("Source: {}", report.pack.source);
    println!("Source kind: {}", report.pack.source_kind.as_str());
    println!("Metadata decision: {}", report.metadata_decision);
    println!("Signature present: {}", report.signature_present);
    println!("Metadata valid: {}", report.metadata_valid);
    println!(
        "Publisher reference found: {}",
        report.publisher_reference_found
    );
    println!(
        "Pack allowed by trust root: {}",
        report.pack_allowed_by_trust_root
    );
    println!(
        "Cryptographic verification performed: {}",
        report.cryptographic_verification_performed
    );
    println!(
        "Cryptographic verification valid: {}",
        report.cryptographic_verification_valid
    );
    println!("Content manifest valid: {}", report.content_manifest_valid);
    println!("Signature expired: {}", report.signature_expired);
    println!("Trust root expired: {}", report.trust_root_expired);
    println!("Manifest SHA-256: {}", report.pack.manifest_sha256);

    match &report.pack.signature {
        Some(signature) => {
            println!("Signature file: {}", signature.path.display());
            println!("Signature file SHA-256: {}", signature.sha256);
            println!(
                "Signature version: {}",
                signature.version.as_deref().unwrap_or("unknown")
            );
            println!(
                "Signature algorithm: {}",
                signature.algorithm.as_deref().unwrap_or("unknown")
            );
            println!(
                "Signature publisher: {}",
                signature.publisher.as_deref().unwrap_or("unknown")
            );
            println!(
                "Signature key id: {}",
                signature.key_id.as_deref().unwrap_or("unknown")
            );
            println!("Signature value present: {}", signature.signature_present);
        }
        None => println!("Signature file: none"),
    }

    println!("Trust roots present: {}", report.trust_roots.exists);
    println!("Trusted publishers: {}", report.trust_roots.publisher_count);

    if !report.findings.is_empty() {
        println!();
        println!("Findings:");
        for finding in &report.findings {
            println!("  - {}: {}", finding.severity.as_str(), finding.message);
        }
    }

    println!();
    println!("This command is diagnostic only. It validates signature metadata, verifies Ed25519 signatures when signed metadata and trust roots are available, and does not enforce signatures, mutate trust roots, install packs, download packs, or execute Gadgets.");
}

fn print_trust_roots_report(report: &TrustRootsReport) {
    println!("Pack trust roots check");
    println!("Path: {}", report.path.display());
    println!("Exists: {}", report.exists);
    println!("Parsed: {}", report.parsed);
    println!(
        "Version: {}",
        report.version.as_deref().unwrap_or("unknown")
    );
    println!("Trusted publishers: {}", report.publisher_count);

    if !report.publishers.is_empty() {
        println!();
        println!("Publishers:");
        for publisher in &report.publishers {
            println!(
                "  - publisher: {}",
                publisher.publisher.as_deref().unwrap_or("unknown")
            );
            println!(
                "    key id: {}",
                publisher.key_id.as_deref().unwrap_or("unknown")
            );
            println!(
                "    algorithm: {}",
                publisher.algorithm.as_deref().unwrap_or("unknown")
            );
            println!("    public key present: {}", publisher.public_key_present);
            println!("    allowed pack ids: {}", publisher.allowed_pack_count);
            if let Some(expires_at) = publisher.expires_at.as_deref() {
                println!("    expires at: {expires_at}");
            }
        }
    }

    if !report.findings.is_empty() {
        println!();
        println!("Findings:");
        for finding in &report.findings {
            println!("  - {}: {}", finding.severity.as_str(), finding.message);
        }
    }

    println!();
    println!("This command is diagnostic only. It does not verify signatures, enforce trust, mutate trust roots, install packs, download packs, or execute Gadgets.");
}

fn print_pack_trust_report(report: &PackTrustReport) {
    println!("Pack trust check: {}", report.pack_name);
    println!("Version: {}", report.pack_version);
    println!("Source: {}", report.source);
    println!("Source kind: {}", report.source_kind.as_str());
    println!("Decision: {}", report.decision);
    println!("Enforcement enabled: {}", report.enforcement_enabled);
    println!("Manifest SHA-256: {}", report.manifest_sha256);
    println!("Trust roots present: {}", report.trust_roots_present);

    match &report.contents_manifest {
        Some(contents) => {
            println!("Contents manifest: {}", contents.path.display());
            println!("Contents manifest SHA-256: {}", contents.sha256);
            println!("Contents file entries: {}", contents.file_count);
        }
        None => println!("Contents manifest: none"),
    }

    match &report.signature {
        Some(signature) => {
            println!("Signature file: {}", signature.path.display());
            println!("Signature file SHA-256: {}", signature.sha256);
            println!(
                "Signature algorithm: {}",
                signature.algorithm.as_deref().unwrap_or("unknown")
            );
            println!(
                "Signature publisher: {}",
                signature.publisher.as_deref().unwrap_or("unknown")
            );
            println!(
                "Signature key id: {}",
                signature.key_id.as_deref().unwrap_or("unknown")
            );
            if let Some(value) = signature.pack_id.as_deref() {
                println!("Signature pack id: {value}");
            }
            if let Some(value) = signature.pack_version.as_deref() {
                println!("Signature pack version: {value}");
            }
            if let Some(value) = signature.created_at.as_deref() {
                println!("Signature created at: {value}");
            }
            if let Some(value) = signature.expires_at.as_deref() {
                println!("Signature expires at: {value}");
            }
        }
        None => println!("Signature file: none"),
    }

    if !report.findings.is_empty() {
        println!();
        println!("Findings:");
        for finding in &report.findings {
            println!("  - {}: {}", finding.severity.as_str(), finding.message);
        }
    }

    println!();
    println!("This command is diagnostic only. It does not enforce signatures, mutate trust roots, install packs, download packs, or execute Gadgets.");
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
    let status = if report.is_valid() {
        "VALID"
    } else {
        "INVALID"
    };
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
                        eprintln!(
                            "git commit approved-patch accepts exactly one approval request id"
                        );
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
    let commit_message = commit_message
        .unwrap_or_else(|| format!("Apply approved Gadgets patch {approval_request_id}"));
    (approval_request_id, commit_message)
}

fn parse_git_pr_body_args(
    args: Vec<String>,
) -> (String, Option<String>, Option<String>, Option<String>) {
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

fn parse_git_pr_create_args(
    args: Vec<String>,
    default_base_branch: &str,
) -> (String, String, String, String, Option<String>) {
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
    (
        approval_request_id,
        pr_body_run_id,
        head_branch,
        base_branch,
        title,
    )
}

fn remote_pr_provider_config(config: &CliRemotePrConfig) -> RemotePrProviderConfig {
    RemotePrProviderConfig {
        enabled: config.enabled,
        dry_run: config.dry_run,
        provider: config.provider.clone(),
        owner: config.owner.clone(),
        repo: config.repo.clone(),
        api_base: config.api_base.clone(),
        token_env: config.token_env.clone(),
        default_base_branch: config.default_base_branch.clone(),
        allowed_base_branches: config.allowed_base_branches.clone(),
        allowed_head_prefixes: config.allowed_head_prefixes.clone(),
        duplicate_strategy: config.duplicate_strategy.clone(),
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
            let project_root = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
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
            let project_root = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
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
                    eprintln!(
                        "Evidence bundle verification failed at {}",
                        bundle_path.display()
                    );
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
            let request = EvidenceWriteRequest::observe(
                run_id.as_str(),
                gadget.as_str(),
                created_at,
                summary,
            );
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
            let loaded_git = match load_gadget_manifest(&project_root, &loaded_pack, GIT_PR_GADGET)
            {
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
            println!(
                "Pack: {} ({})",
                loaded_pack.manifest.metadata.name,
                loaded_pack.source.label()
            );
            println!("Gadget manifest source: {}", loaded_git.source.label());
            println!("Safety: this runs a fixed local git status command selected by the runtime.");
            println!("No branch, commit, push, pull, fetch, PR, provider, patch, shell, Linux admin, database, cloud, or deployment action will run.");
            println!();

            match run_git_status(&project_root, &loaded_git.manifest, request) {
                Ok(report) => {
                    println!("Git status run: {}", report.run_id);
                    println!("Passed: {}", report.passed);
                    println!(
                        "Exit code: {}",
                        report
                            .exit_code
                            .map(|code| code.to_string())
                            .unwrap_or_else(|| "none".to_string())
                    );
                    println!(
                        "Branch: {}",
                        report.branch.unwrap_or_else(|| "unknown".to_string())
                    );
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
                    if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK)
                    {
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
                    let loaded_git =
                        match load_gadget_manifest(&project_root, &loaded_pack, GIT_PR_GADGET) {
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

                    println!(
                        "Gadgets {} Mode protected local Git branch creation",
                        runtime_mode.as_str()
                    );
                    println!("Project root: {}", project_root.display());
                    println!("Branch name: {branch_name}");
                    println!(
                        "Pack: {} ({})",
                        loaded_pack.manifest.metadata.name,
                        loaded_pack.source.label()
                    );
                    println!("Gadget manifest source: {}", loaded_git.source.label());
                    println!("Safety: this runs fixed local git branch creation after runtime branch-name and protected-branch checks.");
                    println!("No checkout, switch, stage, commit, push, pull, fetch, merge, PR, provider, patch, shell, Linux admin, database, cloud, or deployment action will run.");
                    println!();

                    match run_git_branch_create(&project_root, &loaded_git.manifest, request) {
                        Ok(report) => {
                            println!("Git branch run: {}", report.run_id);
                            println!("Branch: {}", report.branch_name);
                            println!("Passed: {}", report.passed);
                            println!("Dry run: {}", report.dry_run);
                            println!("Duplicate PR found: {}", report.duplicate_found);
                            println!(
                                "Exit code: {}",
                                report
                                    .exit_code
                                    .map(|code| code.to_string())
                                    .unwrap_or_else(|| "none".to_string())
                            );
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
                    let (approval_request_id, commit_message) =
                        parse_git_commit_approved_patch_args(rest);

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
                    if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK)
                    {
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
                    let loaded_git =
                        match load_gadget_manifest(&project_root, &loaded_pack, GIT_PR_GADGET) {
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

                    println!(
                        "Gadgets {} Mode approved local Git commit",
                        runtime_mode.as_str()
                    );
                    println!("Project root: {}", project_root.display());
                    println!("Approval request: {approval_request_id}");
                    println!("Commit message: {commit_message}");
                    println!(
                        "Pack: {} ({})",
                        loaded_pack.manifest.metadata.name,
                        loaded_pack.source.label()
                    );
                    println!("Gadget manifest source: {}", loaded_git.source.label());
                    println!("Safety: this verifies approval, rejects protected current branches, stages only approved patch files, and creates one local commit.");
                    println!("No checkout, switch, push, pull, fetch, merge, rebase, PR, provider, patch apply, shell, tests, Linux admin, database, cloud, or deployment action will run.");
                    println!();

                    match run_git_commit_approved_patch(
                        &project_root,
                        &loaded_git.manifest,
                        request,
                    ) {
                        Ok(report) => {
                            println!("Git commit run: {}", report.run_id);
                            println!("Approval request: {}", report.approval_request_id);
                            println!("Branch: {}", report.branch_name);
                            println!("Passed: {}", report.passed);
                            println!("Dry run: {}", report.dry_run);
                            println!("Duplicate PR found: {}", report.duplicate_found);
                            println!(
                                "Exit code: {}",
                                report
                                    .exit_code
                                    .map(|code| code.to_string())
                                    .unwrap_or_else(|| "none".to_string())
                            );
                            println!(
                                "Commit hash: {}",
                                report
                                    .commit_hash
                                    .clone()
                                    .unwrap_or_else(|| "none".to_string())
                            );
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
                    let (approval_request_id, test_run_id, commit_run_id, title) =
                        parse_git_pr_body_args(rest);

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
                    if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK)
                    {
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
                    let loaded_git =
                        match load_gadget_manifest(&project_root, &loaded_pack, GIT_PR_GADGET) {
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

                    println!(
                        "Gadgets {} Mode local PR body generation",
                        runtime_mode.as_str()
                    );
                    println!("Project root: {}", project_root.display());
                    println!("Approval request: {approval_request_id}");
                    if let Some(value) = test_run_id.as_deref() {
                        println!("Test evidence run: {value}");
                    }
                    if let Some(value) = commit_run_id.as_deref() {
                        println!("Commit evidence run: {value}");
                    }
                    println!(
                        "Pack: {} ({})",
                        loaded_pack.manifest.metadata.name,
                        loaded_pack.source.label()
                    );
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
                    if let Err(err) = ensure_pack_installed(&config.installed_packs, DEVELOPER_PACK)
                    {
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
                    let loaded_git =
                        match load_gadget_manifest(&project_root, &loaded_pack, GIT_PR_GADGET) {
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

                    println!(
                        "Gadgets {} Mode guarded remote PR creation",
                        runtime_mode.as_str()
                    );
                    println!("Project root: {}", project_root.display());
                    println!("Approval request: {approval_request_id}");
                    println!("PR body run: {pr_body_run_id}");
                    println!("Head branch: {head_branch}");
                    println!("Base branch: {base_branch}");
                    println!(
                        "Pack: {} ({})",
                        loaded_pack.manifest.metadata.name,
                        loaded_pack.source.label()
                    );
                    println!("Gadget manifest source: {}", loaded_git.source.label());
                    println!("Dry run: {}", config.git.remote_pr.dry_run);
                    println!("Safety: this requires explicit remote_pr.enabled config, verified approval, local PR body evidence, allowed branch rules, and duplicate-open-PR checks before any GitHub API mutation.");
                    println!("No Git push, pull, fetch, merge, rebase, checkout, switch, shell, provider tool, patch apply, test run, Linux admin, database, cloud, or deployment action will run.");
                    println!();

                    match run_git_remote_pr_create(&project_root, &loaded_git.manifest, request) {
                        Ok(report) => {
                            println!("Remote PR run: {}", report.run_id);
                            println!("Repository: {}", report.repository);
                            println!("Title: {}", report.title);
                            println!("Passed: {}", report.passed);
                            println!("Dry run: {}", report.dry_run);
                            println!("Duplicate PR found: {}", report.duplicate_found);
                            println!(
                                "HTTP status: {}",
                                report
                                    .http_status
                                    .map(|code| code.to_string())
                                    .unwrap_or_else(|| "none".to_string())
                            );
                            println!(
                                "PR number: {}",
                                report
                                    .pr_number
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "none".to_string())
                            );
                            println!(
                                "PR URL: {}",
                                report.pr_url.clone().unwrap_or_else(|| "none".to_string())
                            );
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
            let loaded_test_runner =
                match load_gadget_manifest(&project_root, &loaded_pack, TEST_RUNNER_GADGET) {
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
            let request =
                TestRunRequest::named_command(test_run_id, unix_timestamp_label(), command_spec)
                    .with_runtime_mode(runtime_mode);

            println!(
                "Gadgets {} Mode allowlisted test run",
                runtime_mode.as_str()
            );
            println!("Project root: {}", project_root.display());
            println!("Command name: {command_name}");
            println!(
                "Pack: {} ({})",
                loaded_pack.manifest.metadata.name,
                loaded_pack.source.label()
            );
            println!(
                "Gadget manifest source: {}",
                loaded_test_runner.source.label()
            );
            println!(
                "Safety: the command string is loaded from .gadgets/config.yaml by name only."
            );
            println!("No model provider, patch apply, Git, PR, Linux admin, database, cloud, or deployment action will run.");
            println!();

            match run_test_command(&project_root, &loaded_test_runner.manifest, request) {
                Ok(report) => {
                    println!("Test run: {}", report.run_id);
                    println!("Command: {}", report.command_name);
                    println!("Passed: {}", report.passed);
                    println!("Timed out: {}", report.timed_out);
                    println!(
                        "Exit code: {}",
                        report
                            .exit_code
                            .map(|code| code.to_string())
                            .unwrap_or_else(|| "none".to_string())
                    );
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
            let loaded_patch_writer =
                match load_gadget_manifest(&project_root, &loaded_pack, PATCH_WRITER_GADGET) {
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

            println!(
                "Gadgets {} Mode approved local patch apply",
                runtime_mode.as_str()
            );
            println!("Project root: {}", project_root.display());
            println!("Approval request: {approval_request_id}");
            println!(
                "Pack: {} ({})",
                loaded_pack.manifest.metadata.name,
                loaded_pack.source.label()
            );
            println!(
                "Gadget manifest source: {}",
                loaded_patch_writer.source.label()
            );
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
            match approve_request(
                &project_root,
                approval_request_id,
                approver,
                unix_timestamp_label(),
            ) {
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
                    println!(
                        "Approval record verified: {}",
                        verification.approval_request_id
                    );
                    println!("Request: {}", verification.request_path.display());
                    if let Some(path) = verification.approval_path {
                        println!("Approval: {}", path.display());
                    } else {
                        println!(
                            "Approval: not present; verification should not be valid for apply."
                        );
                    }
                    println!("Patch artifact: {}", verification.patch_path.display());
                }
                Ok(verification) => {
                    eprintln!(
                        "Approval verification failed: {}",
                        verification.approval_request_id
                    );
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
    println!(
        "  gadgets approval request-patch [--project <path>] <run-id> [--expires-at <RFC3339-UTC>]"
    );
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
    println!("  gadgets pack trust check [--project <path>] <pack>");
    println!("  gadgets pack trust roots [--project <path>]");
    println!("  gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>");
    println!("  gadgets pack trust signature [--project <path>] <pack>");
    println!("  gadgets help");
    println!("  gadgets version");
    println!();
    println!("Current implemented commands:");
    println!("  init      Create local .gadgets project state using Safe Mode defaults.");
    println!("  ask       Run configured provider Coordinator, then execute an allowed observe or plan-only Developer Pack handoff.");
    println!("  ledger    Show or verify the local append-only audit ledger.");
    println!("  evidence  Create, show, or verify observe-only evidence bundles.");
    println!("  approval  Create, show, approve, or verify scoped approval records.");
    println!(
        "  patch     Apply an approved local patch after exact approval and policy verification."
    );
    println!("  test      Run a named allowlisted test command from .gadgets/config.yaml.");
    println!("  git       Read local Git status, create protected local branches, commit approved patches, generate PR bodies, and create guarded remote PRs through fixed commands.");
    println!("  pack      List, show, validate, or inspect trust status for Gadget pack manifests.");
}

fn print_test_help() {
    println!("Gadgets test commands");
    println!();
    println!("Usage:");
    println!("  gadgets test run [--project <path>] <test-command-name>");
    println!();
    println!("Test commands must be named entries in .gadgets/config.yaml test_commands.");
    println!("The model and user prompt cannot supply raw command strings.");
    println!(
        "The Test Runner does not apply patches, run Git or PR actions, or perform admin actions."
    );
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
    println!("pr create creates one remote GitHub pull request only when remote_pr.enabled is true, dry_run is false, approval plus PR-body evidence are verified, branch rules pass, and duplicate checks pass.");
    println!("Git commands do not checkout, switch, push, pull, fetch, merge, rebase, or run shell commands.");
}

fn print_pack_help() {
    println!("Gadgets pack commands");
    println!();
    println!("Usage:");
    println!("  gadgets pack list [--project <path>]");
    println!("  gadgets pack show [--project <path>] <pack>");
    println!("  gadgets pack validate [--project <path>] [--strict] [pack]");
    println!("  gadgets pack trust check [--project <path>] <pack>");
    println!("  gadgets pack trust roots [--project <path>]");
    println!("  gadgets pack trust preview [--project <path>] [--mode safe|team|production] <pack>");
    println!("  gadgets pack trust signature [--project <path>] <pack>");
    println!();
    println!("Packs are loaded from .gadgets/packs/<pack>/pack.yaml when present, then built-in pack manifests.");
    println!("validate checks pack metadata plus declared Gadget manifests; --strict treats missing Gadget manifests as errors.");
    println!("trust check reports built-in/local/signature metadata status and writes diagnostic evidence/audit; it does not enforce signatures or mutate trust roots.");
    println!("trust roots reports configured local trust-root metadata and writes diagnostic evidence/audit; it does not verify signatures or mutate trust roots.");
    println!("trust preview consumes signature diagnostics to report future Safe/Team/Production policy outcomes and writes evidence/audit; it does not enforce signatures or block pack loading.");
    println!("trust signature validates metadata, verifies Ed25519 signatures when signed metadata and trust roots are available, and writes diagnostic evidence/audit without enforcing pack loading.");
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
    println!(
        "  gadgets approval request-patch [--project <path>] <run-id> [--expires-at <RFC3339-UTC>]"
    );
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
