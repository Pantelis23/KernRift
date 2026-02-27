use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
enum VerifyMutation {
    None,
    HashMismatch,
    SchemaInvalid,
}

struct GoldenCase {
    name: &'static str,
    policy_path: Option<&'static str>,
    verify_mutation: VerifyMutation,
    expected_verify_exit: i32,
}

struct CmdOut {
    code: i32,
    stdout: String,
    stderr: String,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn golden_check_and_verify_contracts() {
    let root = repo_root();
    let update = std::env::var("GOLDEN_UPDATE").ok().as_deref() == Some("1");
    let expect_dir = root.join("tests").join("golden").join("expect");
    let bin = assert_cmd::cargo::cargo_bin!("kernriftc");

    let cases = [
        GoldenCase {
            name: "basic_pass",
            policy_path: None,
            verify_mutation: VerifyMutation::None,
            expected_verify_exit: 0,
        },
        GoldenCase {
            name: "policy_pass",
            policy_path: Some("tests/golden/cases/policy_pass.toml"),
            verify_mutation: VerifyMutation::None,
            expected_verify_exit: 0,
        },
        GoldenCase {
            name: "hash_mismatch",
            policy_path: None,
            verify_mutation: VerifyMutation::HashMismatch,
            expected_verify_exit: 1,
        },
        GoldenCase {
            name: "schema_invalid",
            policy_path: None,
            verify_mutation: VerifyMutation::SchemaInvalid,
            expected_verify_exit: 2,
        },
    ];

    for case in cases {
        let case_file = root
            .join("tests")
            .join("golden")
            .join("cases")
            .join(format!("{}.kr", case.name));
        let tmp_dir = std::env::temp_dir().join(format!(
            "kernrift-golden-{}-{}",
            case.name,
            std::process::id()
        ));
        fs::create_dir_all(&tmp_dir).expect("create temp dir");

        let contracts_path = tmp_dir.join("contracts.json");
        let hash_path = tmp_dir.join("contracts.sha256");
        let verify_report_path = tmp_dir.join("verify.report.json");

        let mut check_args = vec!["check".to_string()];
        if let Some(policy_rel) = case.policy_path {
            check_args.push("--policy".to_string());
            check_args.push(root.join(policy_rel).display().to_string());
        }
        check_args.push("--contracts-out".to_string());
        check_args.push(contracts_path.display().to_string());
        check_args.push("--hash-out".to_string());
        check_args.push(hash_path.display().to_string());
        check_args.push(case_file.display().to_string());

        let check = run_cmd(bin, &root, &check_args);
        let check_snapshot = normalize_command_snapshot(&check, &root, &tmp_dir);
        let check_expect = expect_dir.join(format!("{}.check.stdout", case.name));
        assert_snapshot(&check_expect, &check_snapshot, update);
        assert_eq!(
            check.code, 0,
            "check should pass for case '{}': {}",
            case.name, check.stderr
        );

        let contracts_json = fs::read_to_string(&contracts_path).expect("read contracts");
        let contracts_canon = canonical_json_text(&contracts_json).expect("canonicalize contracts");
        let contracts_expect = expect_dir.join(format!("{}.contracts.json", case.name));
        assert_snapshot(&contracts_expect, &contracts_canon, update);

        match case.verify_mutation {
            VerifyMutation::None => {}
            VerifyMutation::HashMismatch => {
                fs::write(
                    &hash_path,
                    "0000000000000000000000000000000000000000000000000000000000000000\n",
                )
                .expect("tamper hash");
            }
            VerifyMutation::SchemaInvalid => {
                let tampered =
                    contracts_json.replace("kernrift_contracts_v1", "kernrift_contracts_v999");
                fs::write(&contracts_path, tampered.as_bytes()).expect("write tampered contracts");
                let hash = sha256_hex(tampered.as_bytes());
                fs::write(&hash_path, format!("{hash}\n")).expect("write tampered hash");
            }
        }

        let verify_args = vec![
            "verify".to_string(),
            "--contracts".to_string(),
            contracts_path.display().to_string(),
            "--hash".to_string(),
            hash_path.display().to_string(),
            "--report".to_string(),
            verify_report_path.display().to_string(),
        ];
        let verify = run_cmd(bin, &root, &verify_args);
        let verify_snapshot = normalize_command_snapshot(&verify, &root, &tmp_dir);
        let verify_expect = expect_dir.join(format!("{}.verify.stdout", case.name));
        assert_snapshot(&verify_expect, &verify_snapshot, update);
        assert_eq!(
            verify.code, case.expected_verify_exit,
            "verify exit mismatch for case '{}': stderr={}",
            case.name, verify.stderr
        );

        let verify_report_json =
            fs::read_to_string(&verify_report_path).expect("read verify report snapshot");
        let verify_report_canon =
            canonical_json_text(&verify_report_json).expect("canonicalize verify report");
        let verify_report_expect = expect_dir.join(format!("{}.verify.report.json", case.name));
        assert_snapshot(&verify_report_expect, &verify_report_canon, update);

        fs::remove_file(&contracts_path).ok();
        fs::remove_file(&hash_path).ok();
        fs::remove_file(&verify_report_path).ok();
        fs::remove_dir(&tmp_dir).ok();
    }
}

fn run_cmd(bin: &Path, cwd: &Path, args: &[String]) -> CmdOut {
    let output = Command::new(bin)
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run command");
    CmdOut {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

fn normalize_command_snapshot(out: &CmdOut, root: &Path, tmp_dir: &Path) -> String {
    let stdout = normalize_text(&out.stdout, root, tmp_dir);
    let stderr = normalize_text(&normalize_diagnostics(&out.stderr), root, tmp_dir);

    format!(
        "exit={}\nstdout:\n{}\nstderr:\n{}\n",
        out.code,
        trim_or_empty(&stdout),
        trim_or_empty(&stderr)
    )
}

fn trim_or_empty(text: &str) -> String {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        "<empty>".to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_text(text: &str, root: &Path, tmp_dir: &Path) -> String {
    let mut normalized = text.replace("\r\n", "\n").replace('\\', "/");

    let root_norm = root.display().to_string().replace('\\', "/");
    if !root_norm.is_empty() {
        normalized = normalized.replace(&root_norm, "<REPO>");
    }

    let tmp_norm = tmp_dir.display().to_string().replace('\\', "/");
    if !tmp_norm.is_empty() {
        normalized = normalized.replace(&tmp_norm, "<TMP>");
    }

    normalized
}

fn normalize_diagnostics(stderr: &str) -> String {
    let mut lines = stderr
        .replace("\r\n", "\n")
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect::<Vec<_>>();

    let non_empty = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();

    let sortable = !non_empty.is_empty()
        && non_empty.iter().all(|line| {
            line.starts_with("analysis:")
                || line.starts_with("cap-check:")
                || line.starts_with("ctx-check:")
                || line.starts_with("effect-check:")
                || line.starts_with("lockgraph:")
                || line.starts_with("policy:")
                || line.starts_with("verify:")
                || line.starts_with("failed to")
                || line.starts_with("invalid ")
                || line.starts_with("unsupported ")
        });

    if sortable {
        lines.sort();
    }

    lines.join("\n")
}

fn canonical_json_text(input: &str) -> Result<String, String> {
    let parsed: Value =
        serde_json::from_str(input).map_err(|e| format!("failed to parse json snapshot: {}", e))?;
    let canonical = canonicalize_value(&parsed);
    serde_json::to_string_pretty(&canonical)
        .map(|s| format!("{}\n", s))
        .map_err(|e| format!("failed to serialize canonical json: {}", e))
}

fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted = map
                .iter()
                .map(|(k, v)| (k.clone(), canonicalize_value(v)))
                .collect::<BTreeMap<_, _>>();
            let mut out = Map::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize_value).collect()),
        _ => value.clone(),
    }
}

fn assert_snapshot(path: &Path, actual: &str, update: bool) {
    if update {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create snapshot parent");
        }
        fs::write(path, actual).expect("write snapshot");
        return;
    }

    let expected = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read snapshot '{}': {}", path.display(), e));
    assert_eq!(
        expected,
        actual,
        "golden snapshot mismatch for '{}'. Re-run with GOLDEN_UPDATE=1 to refresh.",
        path.display()
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push(hex_nibble((b >> 4) & 0x0f));
        out.push(hex_nibble(b & 0x0f));
    }
    out
}

fn hex_nibble(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + (n - 10)) as char,
        _ => unreachable!(),
    }
}
