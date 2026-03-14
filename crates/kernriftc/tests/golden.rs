use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
enum VerifyMutation {
    None,
    HashMismatch,
    SchemaInvalid,
    InvalidUtf8,
    SignatureMismatch,
    InvalidSigParse,
    InvalidPubkeyParse,
    ReportOverwriteRefusal,
}

struct GoldenCase {
    name: &'static str,
    policy_path: Option<&'static str>,
    verify_mutation: VerifyMutation,
    expected_verify_exit: i32,
    expected_report_result: Option<&'static str>,
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
            expected_report_result: Some("pass"),
        },
        GoldenCase {
            name: "policy_pass",
            policy_path: Some("tests/golden/cases/policy_pass.toml"),
            verify_mutation: VerifyMutation::None,
            expected_verify_exit: 0,
            expected_report_result: Some("pass"),
        },
        GoldenCase {
            name: "hash_mismatch",
            policy_path: None,
            verify_mutation: VerifyMutation::HashMismatch,
            expected_verify_exit: 1,
            expected_report_result: Some("deny"),
        },
        GoldenCase {
            name: "schema_invalid",
            policy_path: None,
            verify_mutation: VerifyMutation::SchemaInvalid,
            expected_verify_exit: 2,
            expected_report_result: Some("invalid_input"),
        },
        GoldenCase {
            name: "invalid_utf8",
            policy_path: None,
            verify_mutation: VerifyMutation::InvalidUtf8,
            expected_verify_exit: 2,
            expected_report_result: Some("invalid_input"),
        },
        GoldenCase {
            name: "signature_mismatch",
            policy_path: None,
            verify_mutation: VerifyMutation::SignatureMismatch,
            expected_verify_exit: 1,
            expected_report_result: Some("deny"),
        },
        GoldenCase {
            name: "invalid_sig_parse",
            policy_path: None,
            verify_mutation: VerifyMutation::InvalidSigParse,
            expected_verify_exit: 2,
            expected_report_result: Some("invalid_input"),
        },
        GoldenCase {
            name: "invalid_pubkey_parse",
            policy_path: None,
            verify_mutation: VerifyMutation::InvalidPubkeyParse,
            expected_verify_exit: 2,
            expected_report_result: Some("invalid_input"),
        },
        GoldenCase {
            name: "report_overwrite_refusal",
            policy_path: None,
            verify_mutation: VerifyMutation::ReportOverwriteRefusal,
            expected_verify_exit: 2,
            expected_report_result: None,
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
        let sig_path = tmp_dir.join("contracts.sig");
        let pubkey_path = tmp_dir.join("pubkey.hex");

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

        let mut verify_args = vec![
            "verify".to_string(),
            "--contracts".to_string(),
            contracts_path.display().to_string(),
            "--hash".to_string(),
            hash_path.display().to_string(),
            "--report".to_string(),
            verify_report_path.display().to_string(),
        ];

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
            VerifyMutation::InvalidUtf8 => {
                let bytes = [0xff_u8, 0xfe_u8, 0xfd_u8];
                fs::write(&contracts_path, bytes).expect("write invalid utf8 contracts");
                let hash = sha256_hex(&bytes);
                fs::write(&hash_path, format!("{hash}\n")).expect("write hash for invalid utf8");
            }
            VerifyMutation::SignatureMismatch => {
                let signing_key = test_signing_key();
                let sig = signing_key.sign(b"not-the-contract-bytes");
                fs::write(
                    &sig_path,
                    format!("{}\n", BASE64_STANDARD.encode(sig.to_bytes())),
                )
                .expect("write mismatched sig");
                fs::write(
                    &pubkey_path,
                    format!("{}\n", hex_encode(&signing_key.verifying_key().to_bytes())),
                )
                .expect("write pubkey");
                verify_args.push("--sig".to_string());
                verify_args.push(sig_path.display().to_string());
                verify_args.push("--pubkey".to_string());
                verify_args.push(pubkey_path.display().to_string());
            }
            VerifyMutation::InvalidSigParse => {
                let signing_key = test_signing_key();
                fs::write(&sig_path, "%%%not_base64%%%\n").expect("write invalid sig text");
                fs::write(
                    &pubkey_path,
                    format!("{}\n", hex_encode(&signing_key.verifying_key().to_bytes())),
                )
                .expect("write pubkey");
                verify_args.push("--sig".to_string());
                verify_args.push(sig_path.display().to_string());
                verify_args.push("--pubkey".to_string());
                verify_args.push(pubkey_path.display().to_string());
            }
            VerifyMutation::InvalidPubkeyParse => {
                let signing_key = test_signing_key();
                let sig = signing_key.sign(b"not-the-contract-bytes");
                fs::write(
                    &sig_path,
                    format!("{}\n", BASE64_STANDARD.encode(sig.to_bytes())),
                )
                .expect("write sig");
                fs::write(&pubkey_path, "zz\n").expect("write invalid pubkey");
                verify_args.push("--sig".to_string());
                verify_args.push(sig_path.display().to_string());
                verify_args.push("--pubkey".to_string());
                verify_args.push(pubkey_path.display().to_string());
            }
            VerifyMutation::ReportOverwriteRefusal => {
                fs::write(&verify_report_path, "sentinel\n").expect("write report sentinel");
            }
        }

        let verify = run_cmd(bin, &root, &verify_args);
        let verify_snapshot = normalize_command_snapshot(&verify, &root, &tmp_dir);
        let verify_expect = expect_dir.join(format!("{}.verify.stdout", case.name));
        assert_snapshot(&verify_expect, &verify_snapshot, update);
        assert_eq!(
            verify.code, case.expected_verify_exit,
            "verify exit mismatch for case '{}': stderr={}",
            case.name, verify.stderr
        );

        if case.expected_report_result.is_none() {
            let sentinel =
                fs::read_to_string(&verify_report_path).expect("read sentinel report file");
            assert_eq!(
                sentinel, "sentinel\n",
                "report file should not be clobbered"
            );
            let sentinel_expect = expect_dir.join(format!("{}.report_file.txt", case.name));
            assert_snapshot(&sentinel_expect, &sentinel, update);
        } else {
            let verify_report_json =
                fs::read_to_string(&verify_report_path).expect("read verify report snapshot");
            let verify_report_canon =
                canonical_json_text(&verify_report_json).expect("canonicalize verify report");
            let verify_report_expect = expect_dir.join(format!("{}.verify.report.json", case.name));
            assert_snapshot(&verify_report_expect, &verify_report_canon, update);

            let report_value: Value =
                serde_json::from_str(&verify_report_json).expect("parse verify report json");
            let got_result = report_value["result"].as_str().unwrap_or("<missing>");
            assert_eq!(
                Some(got_result),
                case.expected_report_result,
                "verify report result mismatch for '{}': {}",
                case.name,
                verify_report_json
            );
        }

        fs::remove_file(&contracts_path).ok();
        fs::remove_file(&hash_path).ok();
        fs::remove_file(&verify_report_path).ok();
        fs::remove_file(&sig_path).ok();
        fs::remove_file(&pubkey_path).ok();
        fs::remove_dir(&tmp_dir).ok();
    }
}

#[test]
fn golden_mmio_typed_slice_checks_are_stable() {
    let root = repo_root();
    let bin = assert_cmd::cargo::cargo_bin!("kernriftc");

    let pass_fixture = root.join("tests").join("must_pass").join("mmio_typed.kr");
    let pass = run_cmd(
        bin,
        &root,
        &["check".to_string(), pass_fixture.display().to_string()],
    );
    assert_eq!(
        pass.code, 0,
        "typed mmio pass fixture should succeed, stderr={}",
        pass.stderr
    );

    let fail_fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_invalid_type.kr");
    let fail = run_cmd(
        bin,
        &root,
        &["check".to_string(), fail_fixture.display().to_string()],
    );
    assert_eq!(
        fail.code, 1,
        "invalid typed mmio fixture should fail, stderr={}",
        fail.stderr
    );
    let first = fail.stderr.lines().next().unwrap_or_default();
    assert!(
        first.starts_with(
            "unsupported mmio element type 'u128'; expected one of: u8, u16, u32, u64 at byte "
        ),
        "unexpected invalid typed mmio diagnostic: {}",
        fail.stderr
    );

    let arity_fail_fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_invalid_arity.kr");
    let arity_fail = run_cmd(
        bin,
        &root,
        &[
            "check".to_string(),
            arity_fail_fixture.display().to_string(),
        ],
    );
    assert_eq!(
        arity_fail.code, 1,
        "invalid typed mmio arity fixture should fail, stderr={}",
        arity_fail.stderr
    );
    let first = arity_fail.stderr.lines().next().unwrap_or_default();
    assert!(
        first.starts_with(
            "mmio_write<T>(addr, value) requires exactly two arguments: address and value at byte "
        ),
        "unexpected invalid typed mmio arity diagnostic: {}",
        arity_fail.stderr
    );

    let operand_fail_fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_invalid_operand.kr");
    let operand_fail = run_cmd(
        bin,
        &root,
        &[
            "check".to_string(),
            operand_fail_fixture.display().to_string(),
        ],
    );
    assert_eq!(
        operand_fail.code, 1,
        "invalid typed mmio operand fixture should fail, stderr={}",
        operand_fail.stderr
    );
    let first = operand_fail.stderr.lines().next().unwrap_or_default();
    assert!(
        first.starts_with(
            "unsupported mmio address operand 'a + b'; expected identifier, integer literal, or identifier + integer literal at byte "
        ),
        "unexpected invalid typed mmio operand diagnostic: {}",
        operand_fail.stderr
    );

    let declared_base_fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_declared_base.kr");
    let declared_base = run_cmd(
        bin,
        &root,
        &[
            "check".to_string(),
            declared_base_fixture.display().to_string(),
        ],
    );
    assert_eq!(
        declared_base.code, 0,
        "declared mmio base fixture should pass, stderr={}",
        declared_base.stderr
    );

    let undeclared_base_fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_undeclared_base.kr");
    let undeclared_base = run_cmd(
        bin,
        &root,
        &[
            "check".to_string(),
            undeclared_base_fixture.display().to_string(),
        ],
    );
    assert_eq!(
        undeclared_base.code, 1,
        "undeclared mmio base fixture should fail, stderr={}",
        undeclared_base.stderr
    );
    assert_eq!(
        undeclared_base.stderr.lines().next(),
        Some("undeclared mmio base 'UART0' used in mmio_read<u32>(UART0)")
    );

    let invalid_decl_fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_invalid_declaration.kr");
    let invalid_decl = run_cmd(
        bin,
        &root,
        &[
            "check".to_string(),
            invalid_decl_fixture.display().to_string(),
        ],
    );
    assert_eq!(
        invalid_decl.code, 1,
        "invalid mmio declaration fixture should fail, stderr={}",
        invalid_decl.stderr
    );
    let first = invalid_decl.stderr.lines().next().unwrap_or_default();
    assert!(
        first.starts_with(
            "invalid mmio base declaration for 'UART0': expected integer literal at byte "
        ),
        "unexpected invalid mmio declaration diagnostic: {}",
        invalid_decl.stderr
    );

    let reg_pass_fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_register_declared.kr");
    let reg_pass = run_cmd(
        bin,
        &root,
        &["check".to_string(), reg_pass_fixture.display().to_string()],
    );
    assert_eq!(
        reg_pass.code, 0,
        "declared mmio register fixture should pass, stderr={}",
        reg_pass.stderr
    );

    let reg_offset_fail_fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_undeclared_offset.kr");
    let reg_offset_fail = run_cmd(
        bin,
        &root,
        &[
            "check".to_string(),
            reg_offset_fail_fixture.display().to_string(),
        ],
    );
    assert_eq!(
        reg_offset_fail.code, 1,
        "undeclared mmio register offset fixture should fail, stderr={}",
        reg_offset_fail.stderr
    );
    assert_eq!(
        reg_offset_fail.stderr.lines().next(),
        Some("undeclared mmio register offset '0x44' for base 'UART0'")
    );

    let reg_access_fail_fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_access_mismatch.kr");
    let reg_access_fail = run_cmd(
        bin,
        &root,
        &[
            "check".to_string(),
            reg_access_fail_fixture.display().to_string(),
        ],
    );
    assert_eq!(
        reg_access_fail.code, 1,
        "mmio register access mismatch fixture should fail, stderr={}",
        reg_access_fail.stderr
    );
    assert_eq!(
        reg_access_fail.stderr.lines().next(),
        Some("mmio_write<u32>(UART0 + 0x04, x) violates register access: 'UART0.SR' is read-only")
    );

    let reg_width_fail_fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_width_mismatch.kr");
    let reg_width_fail = run_cmd(
        bin,
        &root,
        &[
            "check".to_string(),
            reg_width_fail_fixture.display().to_string(),
        ],
    );
    assert_eq!(
        reg_width_fail.code, 1,
        "mmio register width mismatch fixture should fail, stderr={}",
        reg_width_fail.stderr
    );
    assert_eq!(
        reg_width_fail.stderr.lines().next(),
        Some("mmio_write<u32>(UART0 + 0x08, x) width mismatch: register 'UART0.CR' is u16")
    );

    let reg_base_fail_fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_undeclared_base.kr");
    let reg_base_fail = run_cmd(
        bin,
        &root,
        &[
            "check".to_string(),
            reg_base_fail_fixture.display().to_string(),
        ],
    );
    assert_eq!(
        reg_base_fail.code, 1,
        "undeclared mmio register base fixture should fail, stderr={}",
        reg_base_fail.stderr
    );
    assert_eq!(
        reg_base_fail.stderr.lines().next(),
        Some("undeclared mmio base 'UART0' in register declaration 'UART0.DR'")
    );

    let reg_invalid_decl_fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_invalid_declaration.kr");
    let reg_invalid_decl = run_cmd(
        bin,
        &root,
        &[
            "check".to_string(),
            reg_invalid_decl_fixture.display().to_string(),
        ],
    );
    assert_eq!(
        reg_invalid_decl.code, 1,
        "invalid mmio register declaration fixture should fail, stderr={}",
        reg_invalid_decl.stderr
    );
    let first = reg_invalid_decl.stderr.lines().next().unwrap_or_default();
    assert!(
        first.starts_with(
            "invalid mmio register declaration for 'UART0.DR': expected integer literal offset at byte "
        ),
        "unexpected invalid mmio register declaration diagnostic: {}",
        reg_invalid_decl.stderr
    );
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
                || line.starts_with("refusing to overwrite existing output")
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
    let expected = expected.replace("\r\n", "\n");
    let actual = actual.replace("\r\n", "\n");
    assert_eq!(
        expected,
        actual,
        "golden snapshot mismatch for '{}'. Re-run with GOLDEN_UPDATE=1 to refresh.",
        path.display()
    );
}

fn test_signing_key() -> SigningKey {
    let seed = std::array::from_fn::<u8, 32, _>(|i| (i as u8).wrapping_add(1));
    SigningKey::from_bytes(&seed)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(hex_nibble((b >> 4) & 0x0f));
        out.push(hex_nibble(b & 0x0f));
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_encode(&digest)
}

fn hex_nibble(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + (n - 10)) as char,
        _ => unreachable!(),
    }
}
