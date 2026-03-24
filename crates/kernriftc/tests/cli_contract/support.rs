use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

static FIXTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// On Windows, `canonicalize()` returns an extended UNC path like `\\?\C:\...` and
/// backslashes are not JSON-safe when used directly in format strings. Normalize all
/// repo/temp paths to forward-slash form so that fixture paths embedded in expected
/// JSON strings match the program's output.
#[cfg(windows)]
fn normalize_to_forward_slashes(path: PathBuf) -> PathBuf {
    let s = path.display().to_string();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    PathBuf::from(s.replace('\\', "/"))
}
#[cfg(not(windows))]
fn normalize_to_forward_slashes(path: PathBuf) -> PathBuf {
    path
}

pub(super) fn repo_root() -> PathBuf {
    normalize_to_forward_slashes(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root"),
    )
}

// Contributor lock for future JSON-capable commands: reuse this helper from
// cli_contract coverage instead of creating command-specific transport rules.
pub(super) fn assert_json_transport(stdout: &str, stderr: &str, schema_version: &str) {
    assert!(
        stderr.is_empty(),
        "json mode must not write stderr: {}",
        stderr
    );
    assert!(
        stdout.ends_with('\n'),
        "json mode output must end with trailing newline: {:?}",
        stdout
    );
    let json: Value = serde_json::from_str(stdout).expect("json stdout");
    assert_eq!(
        json["schema_version"],
        json!(schema_version),
        "json mode must include stable schema_version"
    );
}

pub(super) fn object_keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("json object")
        .keys()
        .cloned()
        .collect()
}

pub(super) fn unique_temp_output_path(label: &str, ext: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    normalize_to_forward_slashes(
        std::env::temp_dir().join(format!("kernrift-{}-{}.{}", label, ts, ext)),
    )
}

pub(super) fn copy_fixture_to_temp(label: &str, fixture: &Path) -> PathBuf {
    let temp_path = unique_temp_output_path(label, "kr");
    fs::copy(fixture, &temp_path).expect("copy fixture to temp");
    temp_path
}

pub(super) fn write_temp_source_fixture(label: &str, src: &str) -> PathBuf {
    let temp_path = unique_temp_output_path(label, "kr");
    fs::write(&temp_path, src).expect("write temp source fixture");
    temp_path
}

pub(super) fn living_compiler_fixture(name: &str) -> PathBuf {
    normalize_to_forward_slashes(repo_root().join("tests").join("living_compiler").join(name))
}

pub(super) fn must_pass_fixture(name: &str) -> PathBuf {
    normalize_to_forward_slashes(repo_root().join("tests").join("must_pass").join(name))
}

pub(super) fn fixture_text(path: &Path) -> String {
    fs::read_to_string(path)
        .expect("read fixture")
        .replace("\r\n", "\n")
}

pub(super) fn stdout_string(assert: &Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
}

pub(super) fn stderr_string(assert: &Assert) -> String {
    String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8")
}

pub(super) fn stdout_json(assert: &Assert) -> Value {
    serde_json::from_slice(&assert.get_output().stdout).expect("json stdout")
}

pub(super) fn normalized_lines_without_file_label(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|line| !line.starts_with("file: "))
        .map(str::to_string)
        .collect()
}

pub(super) fn text_count_value(stdout: &str, label: &str) -> usize {
    let prefix = format!("{label}: ");
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .expect("count line present")
        .parse::<usize>()
        .expect("count line parses as usize")
}

pub(super) fn normalized_fix_preview_json(stdout: &str) -> Value {
    let mut value: Value = serde_json::from_str(stdout).expect("json stdout");
    value["file"] = json!("<normalized>");
    value
}

pub(super) fn normalized_canonical_findings_json(stdout: &str) -> Value {
    let mut value: Value = serde_json::from_str(stdout).expect("json stdout");
    value["file"] = json!("<normalized>");
    value
}

pub(super) fn normalized_canonical_edit_plan_json(stdout: &str) -> Value {
    let mut value: Value = serde_json::from_str(stdout).expect("json stdout");
    value["file"] = json!("<normalized>");
    value
}

pub(super) fn run_check_canonical_file(
    root: &Path,
    fixture: &Path,
    surface: Option<&str>,
    format: Option<&str>,
) -> Assert {
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root).arg("check").arg("--canonical");
    if let Some(surface) = surface {
        cmd.arg("--surface").arg(surface);
    }
    if let Some(format) = format {
        cmd.arg("--format").arg(format);
    }
    cmd.arg(fixture.as_os_str());
    cmd.assert()
}

pub(super) fn run_check_canonical_stdin(
    root: &Path,
    input: &str,
    surface: Option<&str>,
    format: Option<&str>,
) -> Assert {
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root)
        .arg("check")
        .arg("--canonical")
        .arg("--stdin");
    if let Some(surface) = surface {
        cmd.arg("--surface").arg(surface);
    }
    if let Some(format) = format {
        cmd.arg("--format").arg(format);
    }
    cmd.write_stdin(input).assert()
}

pub(super) fn run_fix_canonical_dry_run_file(
    root: &Path,
    fixture: &Path,
    surface: Option<&str>,
    format: Option<&str>,
) -> Assert {
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run");
    if let Some(surface) = surface {
        cmd.arg("--surface").arg(surface);
    }
    if let Some(format) = format {
        cmd.arg("--format").arg(format);
    }
    cmd.arg(fixture.as_os_str());
    cmd.assert()
}

pub(super) fn run_fix_canonical_dry_run_stdin(
    root: &Path,
    input: &str,
    surface: Option<&str>,
    format: Option<&str>,
) -> Assert {
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run")
        .arg("--stdin");
    if let Some(surface) = surface {
        cmd.arg("--surface").arg(surface);
    }
    if let Some(format) = format {
        cmd.arg("--format").arg(format);
    }
    cmd.write_stdin(input).assert()
}

pub(super) fn run_migrate_preview_canonical_edits_file(
    root: &Path,
    fixture: &Path,
    surface: &str,
    format: &str,
) -> Assert {
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root)
        .arg("migrate-preview")
        .arg("--canonical-edits")
        .arg("--format")
        .arg(format)
        .arg("--surface")
        .arg(surface)
        .arg(fixture.as_os_str());
    cmd.assert()
}

pub(super) fn run_migrate_preview_canonical_edits_stdin(
    root: &Path,
    input: &str,
    surface: &str,
    format: Option<&str>,
) -> Assert {
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root)
        .arg("migrate-preview")
        .arg("--canonical-edits")
        .arg("--stdin")
        .arg("--surface")
        .arg(surface);
    if let Some(format) = format {
        cmd.arg("--format").arg(format);
    }
    cmd.write_stdin(input).assert()
}

pub(super) fn run_migrate_preview_canonical_edits_file_default_surface(
    root: &Path,
    fixture: &Path,
    format: &str,
) -> Assert {
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root)
        .arg("migrate-preview")
        .arg("--canonical-edits")
        .arg("--format")
        .arg(format)
        .arg(fixture.as_os_str());
    cmd.assert()
}

pub(super) fn run_migrate_preview_canonical_edits_stdin_default_surface(
    root: &Path,
    input: &str,
    format: Option<&str>,
) -> Assert {
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root)
        .arg("migrate-preview")
        .arg("--canonical-edits")
        .arg("--stdin");
    if let Some(format) = format {
        cmd.arg("--format").arg(format);
    }
    cmd.write_stdin(input).assert()
}

pub(super) fn emit_backend_artifact_with_sidecar(
    root: &Path,
    kind: &str,
    fixture: &Path,
    artifact_path: &Path,
    meta_path: &Path,
    explicit_stable: bool,
) {
    fs::remove_file(artifact_path).ok();
    fs::remove_file(meta_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root);
    if explicit_stable {
        cmd.arg("--surface").arg("stable");
    }
    cmd.arg(format!("--emit={kind}"))
        .arg("-o")
        .arg(artifact_path.as_os_str())
        .arg("--meta-out")
        .arg(meta_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();
}

pub(super) fn emit_backend_artifact(
    root: &Path,
    kind: &str,
    fixture: &Path,
    artifact_path: &Path,
    explicit_stable: bool,
) {
    fs::remove_file(artifact_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root);
    if explicit_stable {
        cmd.arg("--surface").arg("stable");
    }
    cmd.arg(format!("--emit={kind}"))
        .arg("-o")
        .arg(artifact_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();
}

pub(super) fn inspect_artifact_output(
    root: &Path,
    artifact_path: &Path,
    format: Option<&str>,
) -> String {
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root)
        .arg("inspect-artifact")
        .arg(artifact_path.as_os_str());
    if let Some(format) = format {
        cmd.arg("--format").arg(format);
    }
    let assert = cmd.assert().success();
    String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
}

pub(super) fn write_v2_contracts_for_fixture(root: &Path, fixture: &Path, label: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-inspect-{}-{}.json", label, ts));
    fs::remove_file(&contracts_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    contracts_path
}

pub(super) fn write_temp_policy_file(label: &str, policy_text: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-{}-{}.toml", label, ts));
    fs::remove_file(&policy_path).ok();
    fs::write(&policy_path, policy_text).expect("write policy");
    policy_path
}

pub(super) fn write_temp_contracts_file(label: &str, contracts: &Value) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-contracts-{}-{}.json", label, ts));
    fs::remove_file(&contracts_path).ok();
    fs::write(
        &contracts_path,
        serde_json::to_vec_pretty(contracts).expect("serialize contracts"),
    )
    .expect("write contracts");
    contracts_path
}

pub(super) fn inject_single_critical_violation(
    contracts_json: &mut Value,
    function: &str,
    effect: &str,
    direct: bool,
    via_callee: &[&str],
    via_extern: &[&str],
) {
    contracts_json["report"]["critical"]["violations"] = json!([{
        "function": function,
        "effect": effect,
        "provenance": {
            "direct": direct,
            "via_callee": via_callee,
            "via_extern": via_extern
        }
    }]);
}

pub(super) fn write_verify_report_fixture(label: &str, report_json: &Value) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let seq = FIXTURE_SEQ.fetch_add(1, Ordering::Relaxed);
    let report_path = std::env::temp_dir().join(format!(
        "kernrift-inspect-report-{}-{}-{}.json",
        label, ts, seq
    ));
    fs::remove_file(&report_path).ok();
    fs::write(
        &report_path,
        serde_json::to_string_pretty(report_json).expect("report json"),
    )
    .expect("write report fixture");
    report_path
}

pub(super) fn write_promotion_repo_fixture(feature_id: &str) -> PathBuf {
    let root = repo_root();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let seq = FIXTURE_SEQ.fetch_add(1, Ordering::Relaxed);
    let repo_dir =
        std::env::temp_dir().join(format!("kernrift-promotion-{}-{}-{}", feature_id, ts, seq));
    let hir_dir = repo_dir.join("crates").join("hir").join("src");
    let proposal_dir = repo_dir.join("docs").join("design").join("examples");
    fs::create_dir_all(&hir_dir).expect("create hir dir");
    fs::create_dir_all(&proposal_dir).expect("create proposal dir");
    fs::copy(
        root.join("crates").join("hir").join("src").join("lib.rs"),
        hir_dir.join("lib.rs"),
    )
    .expect("copy hir lib");
    for entry in
        fs::read_dir(root.join("docs").join("design").join("examples")).expect("read proposal dir")
    {
        let entry = entry.expect("proposal dir entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            fs::copy(
                &path,
                proposal_dir.join(path.file_name().expect("proposal file name")),
            )
            .expect("copy proposal example");
        }
    }
    // GIT_TEMPLATE_DIR="" prevents Homebrew-git from copying templates (avoids
    // "File exists" on macOS CI).  GIT_CONFIG_NOSYSTEM=1 skips /etc/gitconfig.
    // We set user.name, user.email, and commit.gpgsign locally so the global
    // ~/.gitconfig (which may have commit.gpgsign=true on CI) is irrelevant.
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(&repo_dir)
            .env("GIT_TEMPLATE_DIR", "")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(args)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout: {}\nstderr: {}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.name", "KernRift Test"]);
    git(&["config", "user.email", "kernrift@example.test"]);
    git(&["config", "commit.gpgsign", "false"]);
    git(&["add", "."]);
    git(&["commit", "-m", "baseline"]);
    repo_dir
}

pub(super) fn git_commit_all(repo_dir: &Path, message: &str) {
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(repo_dir)
            .args(args)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout: {}\nstderr: {}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["add", "."]);
    git(&["commit", "-m", message]);
}

pub(super) fn replace_once_in_file(path: &Path, from: &str, to: &str) {
    let src = fs::read_to_string(path).expect("read file");
    assert!(src.contains(from), "missing pattern '{}'", from);
    let updated = src.replacen(from, to, 1);
    fs::write(path, updated).expect("write file");
}

pub(super) fn replace_json_string_field(path: &Path, field: &str, value: &str) {
    let src = fs::read_to_string(path).expect("read json file");
    let mut json: serde_json::Value = serde_json::from_str(&src).expect("parse json");
    let obj = json.as_object_mut().expect("json object");
    obj.insert(
        field.to_string(),
        serde_json::Value::String(value.to_string()),
    );
    let mut text = serde_json::to_string_pretty(&json).expect("serialize json");
    text.push('\n');
    fs::write(path, text).expect("write json file");
}

pub(super) fn hir_entry_slice(src: &str, section_marker: &str, entry_id: &str) -> String {
    let section_start = src.find(section_marker).expect("section marker");
    let id_marker = format!("        id: \"{}\",", entry_id);
    let entry_start = section_start + src[section_start..].find(&id_marker).expect("entry");
    let entry_end = entry_start + src[entry_start..].find("    },").expect("entry end");
    src[entry_start..entry_end].to_string()
}

pub(super) fn replace_in_hir_entry(
    path: &Path,
    section_marker: &str,
    entry_id: &str,
    from: &str,
    to: &str,
) {
    let src = fs::read_to_string(path).expect("read hir file");
    let section_start = src.find(section_marker).expect("section marker");
    let id_marker = format!("        id: \"{}\",", entry_id);
    let relative_entry_start = src[section_start..].find(&id_marker).expect("entry");
    let entry_start = section_start + relative_entry_start;
    let relative_entry_end = src[entry_start..].find("    },").expect("entry end");
    let entry_end = entry_start + relative_entry_end;
    let entry = &src[entry_start..entry_end];
    assert!(entry.contains(from), "missing pattern '{}'", from);
    let replaced = entry.replacen(from, to, 1);
    let mut out = String::with_capacity(src.len() - entry.len() + replaced.len());
    out.push_str(&src[..entry_start]);
    out.push_str(&replaced);
    out.push_str(&src[entry_end..]);
    fs::write(path, out).expect("write hir file");
}
