use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode};

use kernriftc::{
    AdaptiveFeaturePromotionPlan, adaptive_feature_promotion_plan,
    adaptive_feature_promotion_readiness, adaptive_feature_proposal_summaries,
    validate_adaptive_feature_governance,
};
use serde_json::Value;

use super::args::ProposalsArgs;
use crate::EXIT_INVALID_INPUT;

#[derive(Debug)]
struct PromotionTargetFiles {
    hir_path: PathBuf,
    proposal_path: PathBuf,
}

#[derive(Debug)]
struct PromotionFileUpdate {
    path: PathBuf,
    original: String,
    updated: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PromotionFieldDiff {
    file: String,
    field: &'static str,
    before: String,
    after: String,
}

#[derive(Debug)]
struct CompiledPromotionState {
    feature_id: &'static str,
    proposal_id: &'static str,
    feature_status: &'static str,
    proposal_status: &'static str,
    canonical_replacement: &'static str,
}

#[derive(Debug)]
struct RepoFeatureState {
    feature_id: String,
    proposal_id: String,
    status: String,
    canonical_replacement: String,
}

#[derive(Debug)]
struct RepoProposalState {
    id: String,
    status: String,
    title: String,
    compatibility_risk: String,
    migration_plan: String,
}

#[derive(Debug)]
struct RepoPromotionState {
    feature: RepoFeatureState,
    proposal_hir: RepoProposalState,
    proposal_json: RepoProposalState,
}

pub(crate) fn run_proposals(args: &ProposalsArgs) -> ExitCode {
    if args.validate {
        let errors = validate_adaptive_feature_governance();
        if errors.is_empty() {
            println!("proposal-validation: OK");
            return ExitCode::SUCCESS;
        }
        for err in errors {
            println!("{}", err);
        }
        return ExitCode::from(1);
    }

    if let Some(feature_id) = args.promote_feature.as_deref() {
        match apply_adaptive_feature_promotion(Path::new("."), feature_id, args.dry_run, args.diff)
        {
            Ok(lines) => {
                for line in lines {
                    println!("{}", line);
                }
                return ExitCode::SUCCESS;
            }
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        }
    }

    if args.promotion_readiness {
        let readiness = adaptive_feature_promotion_readiness();
        println!("promotion-readiness: {}", readiness.len());
        for entry in readiness {
            println!("feature: {}", entry.feature_id);
            println!("current_status: {}", entry.current_status.as_str());
            println!("promotable_to_stable: {}", entry.promotable_to_stable);
            println!("reason: {}", entry.reason);
        }
        return ExitCode::SUCCESS;
    }

    let proposals = adaptive_feature_proposal_summaries();
    println!("proposals: {}", proposals.len());
    println!("features: {}", proposals.len());
    for summary in proposals {
        println!("feature: {}", summary.feature.id);
        println!("proposal_id: {}", summary.proposal.id);
        println!("status: {}", summary.feature.status.as_str());
        println!("surface_form: @{}", summary.feature.surface_form);
        println!("lowering_target: {}", summary.feature.lowering_target);
        println!(
            "canonical_replacement: {}",
            summary.feature.canonical_replacement
        );
    }
    ExitCode::SUCCESS
}

fn apply_adaptive_feature_promotion(
    repo_root: &Path,
    feature_id: &str,
    dry_run: bool,
    diff: bool,
) -> Result<Vec<String>, String> {
    validate_governance_repo_root(repo_root)?;
    ensure_clean_governance_worktree(repo_root)?;

    let plan = adaptive_feature_promotion_plan(feature_id)?;
    let compiled_state = compiled_promotion_state(feature_id)?;
    let repo_state = load_repo_promotion_state(repo_root, feature_id)?;
    validate_repo_promotion_state(&compiled_state, &repo_state)?;
    let paths = promotion_target_files(repo_root, &plan);
    let updates = build_promotion_target_updates(&paths, &plan)?;
    let mut lines = Vec::<String>::new();

    if diff {
        lines.extend(render_promotion_diff_preview(&plan, &updates)?);
    }

    if dry_run {
        lines.push(format!(
            "proposal-promotion: dry-run promotion for feature '{}' is valid",
            feature_id
        ));
        return Ok(lines);
    }

    write_files_atomically(&updates)?;
    if let Err(err) = validate_written_promotion_files(&updates, &plan) {
        rollback_written_promotion_files(&updates)?;
        return Err(err);
    }

    lines.push(format!(
        "proposal-promotion: promoted feature '{}' to stable",
        feature_id
    ));
    Ok(lines)
}

fn compiled_promotion_state(feature_id: &str) -> Result<CompiledPromotionState, String> {
    let summary = adaptive_feature_proposal_summaries()
        .into_iter()
        .find(|summary| summary.feature.id == feature_id)
        .ok_or_else(|| format!("proposal-promotion: unknown feature '{}'", feature_id))?;
    Ok(CompiledPromotionState {
        feature_id: summary.feature.id,
        proposal_id: summary.feature.proposal_id,
        feature_status: summary.feature.status.as_str(),
        proposal_status: summary.proposal.status.as_str(),
        canonical_replacement: summary.feature.canonical_replacement,
    })
}

fn validate_governance_repo_root(repo_root: &Path) -> Result<(), String> {
    let required = [
        repo_root.join(".git"),
        repo_root
            .join("crates")
            .join("hir")
            .join("src")
            .join("lib.rs"),
        repo_root.join("docs").join("design").join("examples"),
    ];
    if required.iter().all(|path| path.exists()) {
        Ok(())
    } else {
        Err("proposal-promotion: current directory is not a KernRift repo root".to_string())
    }
}

fn ensure_clean_governance_worktree(repo_root: &Path) -> Result<(), String> {
    let output = ProcessCommand::new("git")
        .arg("status")
        .arg("--porcelain=v1")
        .current_dir(repo_root)
        .output()
        .map_err(|err| format!("proposal-promotion: failed to run git status: {}", err))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "proposal-promotion: failed to run git status: {}",
            stderr.trim()
        ));
    }
    if !output.stdout.is_empty() {
        return Err("proposal-promotion: repository worktree is not clean".to_string());
    }
    Ok(())
}

fn promotion_target_files(
    repo_root: &Path,
    plan: &AdaptiveFeaturePromotionPlan,
) -> PromotionTargetFiles {
    PromotionTargetFiles {
        hir_path: repo_root
            .join("crates")
            .join("hir")
            .join("src")
            .join("lib.rs"),
        proposal_path: repo_root
            .join("docs")
            .join("design")
            .join("examples")
            .join(format!("{}.proposal.json", plan.proposal_id)),
    }
}

fn load_repo_promotion_state(
    repo_root: &Path,
    feature_id: &str,
) -> Result<RepoPromotionState, String> {
    let hir_path = repo_root
        .join("crates")
        .join("hir")
        .join("src")
        .join("lib.rs");
    let hir_src = fs::read_to_string(&hir_path).map_err(|err| {
        format!(
            "proposal-promotion: failed to read '{}': {}",
            hir_path.display(),
            err
        )
    })?;

    let feature_entry = extract_hir_entry(&hir_src, "const ADAPTIVE_SURFACE_FEATURES:", feature_id)
        .map_err(|_| {
            format!(
                "proposal-promotion: target repo missing feature '{}'",
                feature_id
            )
        })?;
    let feature = RepoFeatureState {
        feature_id: extract_rust_string_field(&feature_entry, "id").map_err(|_| {
            format!(
                "proposal-promotion: target repo missing feature '{}'",
                feature_id
            )
        })?,
        proposal_id: extract_rust_string_field(&feature_entry, "proposal_id").map_err(|_| {
            format!(
                "proposal-promotion: target repo missing feature '{}'",
                feature_id
            )
        })?,
        status: extract_rust_status_field(&feature_entry).map_err(|_| {
            format!(
                "proposal-promotion: target repo missing feature '{}'",
                feature_id
            )
        })?,
        canonical_replacement: extract_rust_string_field(&feature_entry, "canonical_replacement")
            .map_err(|_| {
            format!(
                "proposal-promotion: target repo missing feature '{}'",
                feature_id
            )
        })?,
    };

    let proposal_entry = extract_hir_entry(
        &hir_src,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        &feature.proposal_id,
    )
    .map_err(|_| {
        format!(
            "proposal-promotion: target repo missing proposal '{}'",
            feature.proposal_id
        )
    })?;
    let proposal_hir = RepoProposalState {
        id: extract_rust_string_field(&proposal_entry, "id").map_err(|_| {
            format!(
                "proposal-promotion: target repo missing proposal '{}'",
                feature.proposal_id
            )
        })?,
        status: extract_rust_status_field(&proposal_entry).map_err(|_| {
            format!(
                "proposal-promotion: target repo missing proposal '{}'",
                feature.proposal_id
            )
        })?,
        title: extract_rust_string_field(&proposal_entry, "title").map_err(|_| {
            format!(
                "proposal-promotion: target repo missing proposal '{}'",
                feature.proposal_id
            )
        })?,
        compatibility_risk: extract_rust_string_field(&proposal_entry, "compatibility_risk")
            .map_err(|_| {
                format!(
                    "proposal-promotion: target repo missing proposal '{}'",
                    feature.proposal_id
                )
            })?,
        migration_plan: extract_rust_string_field(&proposal_entry, "migration_plan").map_err(
            |_| {
                format!(
                    "proposal-promotion: target repo missing proposal '{}'",
                    feature.proposal_id
                )
            },
        )?,
    };

    let proposal_json_path = repo_root
        .join("docs")
        .join("design")
        .join("examples")
        .join(format!("{}.proposal.json", feature.proposal_id));
    let proposal_json_text = fs::read_to_string(&proposal_json_path).map_err(|_| {
        format!(
            "proposal-promotion: target repo missing proposal '{}'",
            feature.proposal_id
        )
    })?;
    let proposal_json_value: Value = serde_json::from_str(&proposal_json_text).map_err(|err| {
        format!(
            "proposal-promotion: failed to parse proposal JSON '{}': {}",
            proposal_json_path.display(),
            err
        )
    })?;
    let proposal_json_obj = proposal_json_value.as_object().ok_or_else(|| {
        format!(
            "proposal-promotion: failed to parse proposal JSON '{}': expected object",
            proposal_json_path.display()
        )
    })?;
    let proposal_json = RepoProposalState {
        id: proposal_json_obj
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!(
                    "proposal-promotion: target repo missing proposal '{}'",
                    feature.proposal_id
                )
            })?
            .to_string(),
        status: proposal_json_obj
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!(
                    "proposal-promotion: target repo missing proposal '{}'",
                    feature.proposal_id
                )
            })?
            .to_string(),
        title: proposal_json_obj
            .get("title")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!(
                    "proposal-promotion: target repo missing proposal '{}'",
                    feature.proposal_id
                )
            })?
            .to_string(),
        compatibility_risk: proposal_json_obj
            .get("compatibility_risk")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!(
                    "proposal-promotion: target repo missing proposal '{}'",
                    feature.proposal_id
                )
            })?
            .to_string(),
        migration_plan: proposal_json_obj
            .get("migration_plan")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!(
                    "proposal-promotion: target repo missing proposal '{}'",
                    feature.proposal_id
                )
            })?
            .to_string(),
    };

    Ok(RepoPromotionState {
        feature,
        proposal_hir,
        proposal_json,
    })
}

fn validate_repo_promotion_state(
    compiled: &CompiledPromotionState,
    repo: &RepoPromotionState,
) -> Result<(), String> {
    if repo.feature.feature_id != compiled.feature_id {
        return Err(format!(
            "proposal-promotion: target repo missing feature '{}'",
            compiled.feature_id
        ));
    }
    if repo.feature.proposal_id != compiled.proposal_id {
        return Err(format!(
            "proposal-promotion: target repo feature '{}' proposal linkage mismatch",
            compiled.feature_id
        ));
    }
    if repo.feature.canonical_replacement != compiled.canonical_replacement {
        return Err(format!(
            "proposal-promotion: target repo feature '{}' canonical replacement mismatch",
            compiled.feature_id
        ));
    }
    if repo.proposal_hir.id != repo.proposal_json.id {
        return Err(format!(
            "proposal-promotion: target repo proposal '{}' id mismatch between HIR and JSON",
            compiled.proposal_id
        ));
    }
    if repo.proposal_hir.title != repo.proposal_json.title {
        return Err(format!(
            "proposal-promotion: target repo proposal '{}' title mismatch between HIR and JSON",
            compiled.proposal_id
        ));
    }
    if repo.proposal_hir.compatibility_risk != repo.proposal_json.compatibility_risk {
        return Err(format!(
            "proposal-promotion: target repo proposal '{}' compatibility text mismatch between HIR and JSON",
            compiled.proposal_id
        ));
    }
    if repo.proposal_hir.migration_plan != repo.proposal_json.migration_plan {
        return Err(format!(
            "proposal-promotion: target repo proposal '{}' migration text mismatch between HIR and JSON",
            compiled.proposal_id
        ));
    }
    if repo.proposal_hir.status != repo.proposal_json.status {
        return Err(format!(
            "proposal-promotion: target repo proposal '{}' status mismatch between HIR and JSON",
            compiled.proposal_id
        ));
    }
    if repo.feature.status != compiled.feature_status {
        return Err(format!(
            "proposal-promotion: binary/repo disagreement for feature '{}' current status",
            compiled.feature_id
        ));
    }
    if repo.proposal_hir.status != compiled.proposal_status {
        return Err(format!(
            "proposal-promotion: binary/repo disagreement for proposal '{}' current status",
            compiled.proposal_id
        ));
    }
    if repo.feature.status != "experimental" {
        return Err(format!(
            "proposal-promotion: target repo feature '{}' is not experimental",
            compiled.feature_id
        ));
    }
    if repo.proposal_hir.status != "experimental" {
        return Err(format!(
            "proposal-promotion: target repo proposal '{}' is not experimental",
            compiled.proposal_id
        ));
    }
    Ok(())
}

fn build_promotion_target_updates(
    paths: &PromotionTargetFiles,
    plan: &AdaptiveFeaturePromotionPlan,
) -> Result<Vec<PromotionFileUpdate>, String> {
    let hir_original = fs::read_to_string(&paths.hir_path).map_err(|err| {
        format!(
            "proposal-promotion: failed to read '{}': {}",
            paths.hir_path.display(),
            err
        )
    })?;
    let proposal_original = fs::read_to_string(&paths.proposal_path).map_err(|err| {
        format!(
            "proposal-promotion: failed to read '{}': {}",
            paths.proposal_path.display(),
            err
        )
    })?;

    let hir_updated = promote_status_in_hir_source(&hir_original, plan)?;
    let proposal_updated = promote_proposal_example_json(&proposal_original, plan)?;

    Ok(vec![
        PromotionFileUpdate {
            path: paths.hir_path.clone(),
            original: hir_original,
            updated: hir_updated,
        },
        PromotionFileUpdate {
            path: paths.proposal_path.clone(),
            original: proposal_original,
            updated: proposal_updated,
        },
    ])
}

fn render_promotion_diff_preview(
    plan: &AdaptiveFeaturePromotionPlan,
    updates: &[PromotionFileUpdate],
) -> Result<Vec<String>, String> {
    let diffs = build_promotion_field_diffs(plan, updates)?;
    let mut lines = Vec::with_capacity(3 + diffs.len() * 4);
    lines.push(format!("promotion-diff: {}", diffs.len()));
    lines.push(format!("feature: {}", plan.feature_id));
    lines.push(format!("proposal_id: {}", plan.proposal_id));
    for diff in diffs {
        lines.push(format!("file: {}", diff.file));
        lines.push(format!("field: {}", diff.field));
        lines.push(format!("before: {}", diff.before));
        lines.push(format!("after: {}", diff.after));
    }
    Ok(lines)
}

fn build_promotion_field_diffs(
    plan: &AdaptiveFeaturePromotionPlan,
    updates: &[PromotionFileUpdate],
) -> Result<Vec<PromotionFieldDiff>, String> {
    let hir_update = updates
        .iter()
        .find(|update| update.path.ends_with("crates/hir/src/lib.rs"))
        .ok_or_else(|| "proposal-promotion: missing HIR update target".to_string())?;
    let proposal_update = updates
        .iter()
        .find(|update| {
            update
                .path
                .ends_with(format!("{}.proposal.json", plan.proposal_id))
        })
        .ok_or_else(|| "proposal-promotion: missing proposal update target".to_string())?;

    let original_feature_entry = extract_hir_entry(
        &hir_update.original,
        "const ADAPTIVE_SURFACE_FEATURES:",
        plan.feature_id,
    )?;
    let updated_feature_entry = extract_hir_entry(
        &hir_update.updated,
        "const ADAPTIVE_SURFACE_FEATURES:",
        plan.feature_id,
    )?;
    let original_proposal_entry = extract_hir_entry(
        &hir_update.original,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        plan.proposal_id,
    )?;
    let updated_proposal_entry = extract_hir_entry(
        &hir_update.updated,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        plan.proposal_id,
    )?;

    let mut diffs = vec![
        PromotionFieldDiff {
            file: "crates/hir/src/lib.rs".to_string(),
            field: "feature.status",
            before: extract_rust_status_field(&original_feature_entry)?,
            after: extract_rust_status_field(&updated_feature_entry)?,
        },
        PromotionFieldDiff {
            file: "crates/hir/src/lib.rs".to_string(),
            field: "proposal.status",
            before: extract_rust_status_field(&original_proposal_entry)?,
            after: extract_rust_status_field(&updated_proposal_entry)?,
        },
        PromotionFieldDiff {
            file: "crates/hir/src/lib.rs".to_string(),
            field: "proposal.title",
            before: extract_rust_string_field(&original_proposal_entry, "title")?,
            after: extract_rust_string_field(&updated_proposal_entry, "title")?,
        },
        PromotionFieldDiff {
            file: "crates/hir/src/lib.rs".to_string(),
            field: "proposal.compatibility_risk",
            before: extract_rust_string_field(&original_proposal_entry, "compatibility_risk")?,
            after: extract_rust_string_field(&updated_proposal_entry, "compatibility_risk")?,
        },
        PromotionFieldDiff {
            file: "crates/hir/src/lib.rs".to_string(),
            field: "proposal.migration_plan",
            before: extract_rust_string_field(&original_proposal_entry, "migration_plan")?,
            after: extract_rust_string_field(&updated_proposal_entry, "migration_plan")?,
        },
    ];

    let original_json = parse_proposal_json_fields(&proposal_update.original)?;
    let updated_json = parse_proposal_json_fields(&proposal_update.updated)?;
    let proposal_path = format!("docs/design/examples/{}.proposal.json", plan.proposal_id);
    diffs.extend([
        PromotionFieldDiff {
            file: proposal_path.clone(),
            field: "proposal.status",
            before: original_json.status,
            after: updated_json.status,
        },
        PromotionFieldDiff {
            file: proposal_path.clone(),
            field: "proposal.title",
            before: original_json.title,
            after: updated_json.title,
        },
        PromotionFieldDiff {
            file: proposal_path.clone(),
            field: "proposal.compatibility_risk",
            before: original_json.compatibility_risk,
            after: updated_json.compatibility_risk,
        },
        PromotionFieldDiff {
            file: proposal_path,
            field: "proposal.migration_plan",
            before: original_json.migration_plan,
            after: updated_json.migration_plan,
        },
    ]);

    diffs.sort_by(|a, b| a.file.cmp(&b.file).then(a.field.cmp(b.field)));
    Ok(diffs)
}

fn parse_proposal_json_fields(src: &str) -> Result<RepoProposalState, String> {
    let proposal_json_value: Value = serde_json::from_str(src)
        .map_err(|err| format!("proposal-promotion: failed to parse proposal JSON: {}", err))?;
    let proposal_json_obj = proposal_json_value
        .as_object()
        .ok_or_else(|| "proposal-promotion: proposal JSON must be an object".to_string())?;
    Ok(RepoProposalState {
        id: proposal_json_obj
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "proposal-promotion: missing proposal JSON field 'id'".to_string())?
            .to_string(),
        status: proposal_json_obj
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| "proposal-promotion: missing proposal JSON field 'status'".to_string())?
            .to_string(),
        title: proposal_json_obj
            .get("title")
            .and_then(Value::as_str)
            .ok_or_else(|| "proposal-promotion: missing proposal JSON field 'title'".to_string())?
            .to_string(),
        compatibility_risk: proposal_json_obj
            .get("compatibility_risk")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                "proposal-promotion: missing proposal JSON field 'compatibility_risk'".to_string()
            })?
            .to_string(),
        migration_plan: proposal_json_obj
            .get("migration_plan")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                "proposal-promotion: missing proposal JSON field 'migration_plan'".to_string()
            })?
            .to_string(),
    })
}

fn promote_status_in_hir_source(
    src: &str,
    plan: &AdaptiveFeaturePromotionPlan,
) -> Result<String, String> {
    let src = promote_status_in_rust_entry(
        src,
        "const ADAPTIVE_SURFACE_FEATURES:",
        plan.feature_id,
        plan.feature_id,
    )?;
    let src = promote_status_in_rust_entry(
        &src,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        plan.proposal_id,
        plan.feature_id,
    )?;
    normalize_proposal_text_in_hir_source(&src, plan)
}

fn promote_status_in_rust_entry(
    src: &str,
    section_marker: &str,
    entry_id: &str,
    feature_id: &str,
) -> Result<String, String> {
    let section_start = src.find(section_marker).ok_or_else(|| {
        format!(
            "proposal-promotion: failed to locate '{}' in crates/hir/src/lib.rs",
            section_marker
        )
    })?;
    let id_marker = format!("        id: \"{}\",", entry_id);
    let relative_entry_start = src[section_start..]
        .find(&id_marker)
        .ok_or_else(|| format!("proposal-promotion: failed to locate entry '{}'", entry_id))?;
    let entry_start = section_start + relative_entry_start;
    let relative_entry_end = src[entry_start..].find("    },").ok_or_else(|| {
        format!(
            "proposal-promotion: failed to locate end of entry '{}'",
            entry_id
        )
    })?;
    let entry_end = entry_start + relative_entry_end;
    let entry = &src[entry_start..entry_end];
    let experimental = "status: AdaptiveFeatureStatus::Experimental,";
    let stable = "status: AdaptiveFeatureStatus::Stable,";
    if !entry.contains(experimental) {
        return Err(format!(
            "proposal-promotion: feature '{}' is not promotable: expected experimental status in '{}'",
            feature_id, entry_id
        ));
    }
    let replaced = entry.replacen(experimental, stable, 1);
    let mut out = String::with_capacity(src.len() - entry.len() + replaced.len());
    out.push_str(&src[..entry_start]);
    out.push_str(&replaced);
    out.push_str(&src[entry_end..]);
    Ok(out)
}

fn normalize_proposal_text_in_hir_source(
    src: &str,
    plan: &AdaptiveFeaturePromotionPlan,
) -> Result<String, String> {
    let section_marker = "const ADAPTIVE_FEATURE_PROPOSALS:";
    let section_start = src.find(section_marker).ok_or_else(|| {
        format!(
            "proposal-promotion: failed to locate '{}' in crates/hir/src/lib.rs",
            section_marker
        )
    })?;
    let id_marker = format!("        id: \"{}\",", plan.proposal_id);
    let relative_entry_start = src[section_start..].find(&id_marker).ok_or_else(|| {
        format!(
            "proposal-promotion: failed to locate entry '{}'",
            plan.proposal_id
        )
    })?;
    let entry_start = section_start + relative_entry_start;
    let relative_entry_end = src[entry_start..].find("    },").ok_or_else(|| {
        format!(
            "proposal-promotion: failed to locate end of entry '{}'",
            plan.proposal_id
        )
    })?;
    let entry_end = entry_start + relative_entry_end;
    let entry = &src[entry_start..entry_end];

    let normalized = replace_rust_string_field(entry, "title", &plan.normalized_proposal_title)?;
    let normalized = replace_rust_string_field(
        &normalized,
        "compatibility_risk",
        &plan.normalized_compatibility_risk,
    )?;
    let normalized = replace_rust_string_field(
        &normalized,
        "migration_plan",
        &plan.normalized_migration_plan,
    )?;

    let mut out = String::with_capacity(src.len() - entry.len() + normalized.len());
    out.push_str(&src[..entry_start]);
    out.push_str(&normalized);
    out.push_str(&src[entry_end..]);
    Ok(out)
}

fn replace_rust_string_field(
    src: &str,
    field_name: &str,
    new_value: &str,
) -> Result<String, String> {
    let field_marker = format!("        {}: \"", field_name);
    let field_start = src.find(&field_marker).ok_or_else(|| {
        format!(
            "proposal-promotion: failed to locate field '{}' in proposal entry",
            field_name
        )
    })?;
    let value_start = field_start + field_marker.len();
    let value_end = rust_string_literal_end(src, value_start).ok_or_else(|| {
        format!(
            "proposal-promotion: failed to locate end of field '{}' in proposal entry",
            field_name
        )
    })?;
    let escaped_value = escape_rust_string_literal(new_value);
    let mut out =
        String::with_capacity(src.len() - (value_end - value_start) + escaped_value.len());
    out.push_str(&src[..value_start]);
    out.push_str(&escaped_value);
    out.push_str(&src[value_end..]);
    Ok(out)
}

fn extract_rust_string_field(src: &str, field_name: &str) -> Result<String, String> {
    let field_marker = format!("        {}: \"", field_name);
    let field_start = src
        .find(&field_marker)
        .ok_or_else(|| format!("missing rust string field '{}'", field_name))?;
    let value_start = field_start + field_marker.len();
    let value_end = rust_string_literal_end(src, value_start)
        .ok_or_else(|| format!("missing rust string field end '{}'", field_name))?;
    unescape_rust_string_literal(&src[value_start..value_end])
}

fn rust_string_literal_end(src: &str, value_start: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut idx = value_start;
    let mut escaped = false;
    while idx < bytes.len() {
        let byte = bytes[idx];
        if escaped {
            escaped = false;
            idx += 1;
            continue;
        }
        match byte {
            b'\\' => {
                escaped = true;
                idx += 1;
            }
            b'"' => return Some(idx),
            _ => idx += 1,
        }
    }
    None
}

fn escape_rust_string_literal(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\0' => escaped.push_str("\\0"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn unescape_rust_string_literal(value: &str) -> Result<String, String> {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(next) = chars.next() else {
            return Err("unterminated rust string escape".to_string());
        };
        match next {
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            'x' => {
                let hi = chars
                    .next()
                    .ok_or_else(|| "invalid rust hex escape".to_string())?;
                let lo = chars
                    .next()
                    .ok_or_else(|| "invalid rust hex escape".to_string())?;
                let hex = [hi, lo].iter().collect::<String>();
                let byte = u8::from_str_radix(&hex, 16)
                    .map_err(|_| "invalid rust hex escape".to_string())?;
                out.push(byte as char);
            }
            'u' => {
                if chars.next() != Some('{') {
                    return Err("invalid rust unicode escape".to_string());
                }
                let mut hex = String::new();
                loop {
                    let ch = chars
                        .next()
                        .ok_or_else(|| "invalid rust unicode escape".to_string())?;
                    if ch == '}' {
                        break;
                    }
                    hex.push(ch);
                }
                let code = u32::from_str_radix(&hex, 16)
                    .map_err(|_| "invalid rust unicode escape".to_string())?;
                let scalar = char::from_u32(code)
                    .ok_or_else(|| "invalid rust unicode scalar".to_string())?;
                out.push(scalar);
            }
            other => {
                return Err(format!("unsupported rust string escape '{}'", other));
            }
        }
    }
    Ok(out)
}

fn extract_rust_status_field(src: &str) -> Result<String, String> {
    let marker = "status: AdaptiveFeatureStatus::";
    let start = src
        .find(marker)
        .ok_or_else(|| "missing rust status field".to_string())?
        + marker.len();
    let end = src[start..]
        .find(',')
        .map(|idx| start + idx)
        .ok_or_else(|| "missing rust status field end".to_string())?;
    match &src[start..end] {
        "Experimental" => Ok("experimental".to_string()),
        "Stable" => Ok("stable".to_string()),
        "Deprecated" => Ok("deprecated".to_string()),
        other => Err(format!("unknown rust status '{}'", other)),
    }
}

fn promote_proposal_example_json(
    src: &str,
    plan: &AdaptiveFeaturePromotionPlan,
) -> Result<String, String> {
    let mut value: Value = serde_json::from_str(src)
        .map_err(|err| format!("proposal-promotion: failed to parse proposal JSON: {}", err))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "proposal-promotion: proposal JSON must be an object".to_string())?;
    object.insert("status".to_string(), Value::String("stable".to_string()));
    object.insert(
        "title".to_string(),
        Value::String(plan.normalized_proposal_title.clone()),
    );
    object.insert(
        "compatibility_risk".to_string(),
        Value::String(plan.normalized_compatibility_risk.clone()),
    );
    object.insert(
        "migration_plan".to_string(),
        Value::String(plan.normalized_migration_plan.clone()),
    );
    let mut text = serde_json::to_string_pretty(&value).map_err(|err| {
        format!(
            "proposal-promotion: failed to serialize proposal JSON: {}",
            err
        )
    })?;
    text.push('\n');
    Ok(text)
}

fn write_files_atomically(updates: &[PromotionFileUpdate]) -> Result<(), String> {
    let mut temp_paths = Vec::<PathBuf>::new();
    for (idx, update) in updates.iter().enumerate() {
        let tmp = update.path.with_extension(format!(
            "{}.kernriftc-promote-{}.tmp",
            update
                .path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("file"),
            idx
        ));
        fs::write(&tmp, &update.updated).map_err(|err| {
            let _ = remove_temp_files(&temp_paths);
            format!(
                "proposal-promotion: failed to stage '{}': {}",
                update.path.display(),
                err
            )
        })?;
        temp_paths.push(tmp);
    }

    let mut renamed = Vec::<(PathBuf, String)>::new();
    for (update, tmp) in updates.iter().zip(temp_paths.iter()) {
        if let Err(err) = fs::rename(tmp, &update.path) {
            let _ = rollback_renamed_files(&renamed);
            let _ = remove_temp_files(&temp_paths);
            return Err(format!(
                "proposal-promotion: failed to commit '{}': {}",
                update.path.display(),
                err
            ));
        }
        renamed.push((update.path.clone(), update.original.clone()));
    }

    Ok(())
}

fn validate_written_promotion_files(
    updates: &[PromotionFileUpdate],
    plan: &AdaptiveFeaturePromotionPlan,
) -> Result<(), String> {
    for update in updates {
        let current = fs::read_to_string(&update.path).map_err(|err| {
            format!(
                "proposal-promotion: failed to read '{}' after write: {}",
                update.path.display(),
                err
            )
        })?;
        if current != update.updated {
            return Err(format!(
                "proposal-promotion: validation failed for '{}'",
                update.path.display()
            ));
        }
    }

    let hir_update = updates
        .iter()
        .find(|update| update.path.ends_with("crates/hir/src/lib.rs"))
        .ok_or_else(|| "proposal-promotion: missing HIR update target".to_string())?;
    let proposal_update = updates
        .iter()
        .find(|update| {
            update
                .path
                .ends_with(format!("{}.proposal.json", plan.proposal_id))
        })
        .ok_or_else(|| "proposal-promotion: missing proposal update target".to_string())?;

    let feature_entry = extract_hir_entry(
        &hir_update.updated,
        "const ADAPTIVE_SURFACE_FEATURES:",
        plan.feature_id,
    )?;
    if !feature_entry.contains("status: AdaptiveFeatureStatus::Stable,") {
        return Err(format!(
            "proposal-promotion: validation failed for feature '{}'",
            plan.feature_id
        ));
    }

    let proposal_entry = extract_hir_entry(
        &hir_update.updated,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        plan.proposal_id,
    )?;
    if !proposal_entry.contains("status: AdaptiveFeatureStatus::Stable,")
        || !proposal_entry.contains(&format!("title: \"{}\",", plan.normalized_proposal_title))
        || !proposal_entry.contains(&format!(
            "compatibility_risk: \"{}\",",
            plan.normalized_compatibility_risk
        ))
        || !proposal_entry.contains(&format!(
            "migration_plan: \"{}\",",
            plan.normalized_migration_plan
        ))
    {
        return Err(format!(
            "proposal-promotion: validation failed for proposal '{}'",
            plan.proposal_id
        ));
    }

    let proposal_json: Value = serde_json::from_str(&proposal_update.updated).map_err(|err| {
        format!(
            "proposal-promotion: validation failed for proposal '{}': {}",
            plan.proposal_id, err
        )
    })?;
    let obj = proposal_json.as_object().ok_or_else(|| {
        "proposal-promotion: validation failed for proposal JSON object".to_string()
    })?;
    let status = obj
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| "proposal-promotion: validation failed for proposal status".to_string())?;
    let title = obj
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| "proposal-promotion: validation failed for proposal title".to_string())?;
    let compatibility = obj
        .get("compatibility_risk")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "proposal-promotion: validation failed for proposal compatibility text".to_string()
        })?;
    let migration = obj
        .get("migration_plan")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "proposal-promotion: validation failed for proposal migration text".to_string()
        })?;
    if status != "stable"
        || title != plan.normalized_proposal_title
        || compatibility != plan.normalized_compatibility_risk
        || migration != plan.normalized_migration_plan
    {
        return Err(format!(
            "proposal-promotion: validation failed for proposal '{}'",
            plan.proposal_id
        ));
    }

    Ok(())
}

fn extract_hir_entry(src: &str, section_marker: &str, entry_id: &str) -> Result<String, String> {
    let section_start = src.find(section_marker).ok_or_else(|| {
        format!(
            "proposal-promotion: failed to locate '{}' in crates/hir/src/lib.rs",
            section_marker
        )
    })?;
    let id_marker = format!("        id: \"{}\",", entry_id);
    let relative_entry_start = src[section_start..]
        .find(&id_marker)
        .ok_or_else(|| format!("proposal-promotion: failed to locate entry '{}'", entry_id))?;
    let entry_start = section_start + relative_entry_start;
    let relative_entry_end = src[entry_start..].find("    },").ok_or_else(|| {
        format!(
            "proposal-promotion: failed to locate end of entry '{}'",
            entry_id
        )
    })?;
    let entry_end = entry_start + relative_entry_end;
    Ok(src[entry_start..entry_end].to_string())
}

fn rollback_written_promotion_files(updates: &[PromotionFileUpdate]) -> Result<(), String> {
    let mut errs = Vec::<String>::new();
    for update in updates {
        if let Err(err) = fs::write(&update.path, &update.original) {
            errs.push(format!("{}: {}", update.path.display(), err));
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "proposal-promotion: rollback failed for {}",
            errs.join(", ")
        ))
    }
}

fn rollback_renamed_files(renamed: &[(PathBuf, String)]) -> Result<(), String> {
    let mut errs = Vec::<String>::new();
    for (path, original) in renamed.iter().rev() {
        if let Err(err) = fs::write(path, original) {
            errs.push(format!("{}: {}", path.display(), err));
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "proposal-promotion: rollback failed for {}",
            errs.join(", ")
        ))
    }
}

fn remove_temp_files(temp_paths: &[PathBuf]) -> Result<(), String> {
    let mut errs = Vec::<String>::new();
    for path in temp_paths {
        if let Err(err) = fs::remove_file(path)
            && path.exists()
        {
            errs.push(format!("{}: {}", path.display(), err));
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "proposal-promotion: temp cleanup failed for {}",
            errs.join(", ")
        ))
    }
}
