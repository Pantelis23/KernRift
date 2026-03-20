use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use emit::{
    ContractsSchema, emit_caps_manifest_json, emit_contracts_json, emit_contracts_json_with_schema,
    emit_krir_json, emit_lockgraph_json, emit_report_json,
};
use kernriftc::{
    BackendArtifactKind, analyze, check_file, check_module, compile_file,
    emit_backend_artifact_file,
};
use krir::{
    BackendTargetContract, lower_current_krir_to_executable_krir,
    lower_executable_krir_to_x86_64_object,
};
use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

fn collect_kr_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = fs::read_dir(dir)
        .expect("read_dir")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("kr"))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn object_keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("json object")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn must_pass_suite() {
    let root = repo_root();
    let dir = root.join("tests").join("must_pass");
    for file in collect_kr_files(&dir) {
        check_file(&file).unwrap_or_else(|errs| {
            panic!("{} should return Ok(()), got {:?}", file.display(), errs)
        });
    }
}

#[test]
fn must_fail_suite() {
    let root = repo_root();
    let dir = root.join("tests").join("must_fail");
    for file in collect_kr_files(&dir) {
        let errs = check_file(&file).expect_err(&format!(
            "{} should return Err(_), got Ok(())",
            file.display()
        ));
        assert!(
            !errs.is_empty(),
            "expected errors for {}, got empty",
            file.display()
        );
    }
}

#[test]
fn krir_json_snapshot_basic() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let module = compile_file(&fixture).expect("compile basic.kr");
    let actual = emit_krir_json(&module).expect("serialize KRIR");

    let snapshot_path = root.join("tests").join("snapshots").join("basic.krir.json");
    let expected = fs::read_to_string(&snapshot_path).expect("read snapshot");

    let normalize = |s: &str| s.replace("\r\n", "\n");
    assert_eq!(
        normalize(actual.trim_end()),
        normalize(expected.trim_end()),
        "snapshot mismatch for {}",
        snapshot_path.display()
    );
}

#[test]
fn caps_manifest_contains_symbols() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let module = compile_file(&fixture).expect("compile basic.kr");
    check_module(&module).expect("checks should pass");

    let json = emit_caps_manifest_json(&module).expect("emit caps");
    let value: Value = serde_json::from_str(&json).expect("caps json");
    assert_eq!(
        object_keys(&value),
        BTreeSet::from(["module_caps".to_string(), "symbols".to_string()]),
        "caps manifest top-level schema drifted"
    );
    assert!(
        value["module_caps"].is_array(),
        "module_caps must be an array"
    );

    let symbols = value["symbols"].as_array().expect("symbols array");
    assert!(
        symbols.iter().all(|s| object_keys(s)
            == BTreeSet::from(["caps_req".to_string(), "name".to_string()])),
        "caps symbol schema must be exactly {{name,caps_req}}: {:?}",
        symbols
    );
    assert!(
        symbols
            .iter()
            .any(|s| s["name"] == "foo" && s["caps_req"].to_string().contains("PhysMap"))
    );
}

#[test]
fn lockgraph_and_report_emit_expected_fields() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");
    let module = compile_file(&fixture).expect("compile locks_ok.kr");
    check_module(&module).expect("checks should pass");

    let (report, errs) = analyze(&module);
    assert!(errs.is_empty(), "analysis errors: {:?}", errs);

    let lockgraph_json = emit_lockgraph_json(&report).expect("emit lockgraph");
    let lockgraph: Value = serde_json::from_str(&lockgraph_json).expect("lockgraph json");
    assert_eq!(
        object_keys(&lockgraph),
        BTreeSet::from(["edges".to_string(), "max_lock_depth".to_string()]),
        "lockgraph top-level schema drifted"
    );
    assert!(
        lockgraph["edges"].is_array(),
        "lockgraph edges must be array"
    );
    assert!(
        lockgraph["max_lock_depth"].is_u64(),
        "lockgraph max_lock_depth must be u64"
    );

    let report_json = emit_report_json(
        &report,
        &["max_lock_depth".to_string(), "no_yield_spans".to_string()],
    )
    .expect("emit report");
    let report_value: Value = serde_json::from_str(&report_json).expect("report json");
    assert_eq!(
        object_keys(&report_value),
        BTreeSet::from(["max_lock_depth".to_string(), "no_yield_spans".to_string()]),
        "report should only contain requested metrics"
    );
    assert!(
        report_value["max_lock_depth"].is_u64(),
        "report max_lock_depth must be u64"
    );
    assert!(
        report_value["no_yield_spans"].is_object(),
        "report no_yield_spans must be object"
    );
}

#[test]
fn contracts_bundle_contains_governance_surfaces() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");
    let module = compile_file(&fixture).expect("compile locks_ok.kr");
    check_module(&module).expect("checks should pass");
    let (report, errs) = analyze(&module);
    assert!(errs.is_empty(), "analysis errors: {:?}", errs);

    let contracts_json = emit_contracts_json(&module, &report).expect("emit contracts");
    let contracts: Value = serde_json::from_str(&contracts_json).expect("contracts json");
    assert_eq!(
        object_keys(&contracts),
        BTreeSet::from([
            "capabilities".to_string(),
            "facts".to_string(),
            "lockgraph".to_string(),
            "report".to_string(),
            "schema_version".to_string(),
        ]),
        "contracts bundle top-level schema drifted"
    );
    assert_eq!(
        contracts["schema_version"],
        Value::String("kernrift_contracts_v1".to_string()),
        "contracts schema version drifted"
    );
    assert_eq!(
        object_keys(&contracts["facts"]),
        BTreeSet::from(["symbols".to_string()]),
        "facts top-level schema drifted"
    );
}

#[test]
fn contracts_v2_abi_shape_is_locked_for_kernel_semantics_fields() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let module =
        compile_file(&fixture).expect("compile policy_families_order_no_critical_alloc.kr");
    check_module(&module).expect("checks should pass");
    let (report, errs) = analyze(&module);
    assert!(errs.is_empty(), "analysis errors: {:?}", errs);

    let contracts_json = emit_contracts_json_with_schema(&module, &report, ContractsSchema::V2)
        .expect("emit contracts v2");
    let contracts: Value = serde_json::from_str(&contracts_json).expect("contracts json");

    assert_eq!(
        contracts["schema_version"],
        Value::String("kernrift_contracts_v2".to_string()),
        "contracts v2 schema_version drifted"
    );

    let symbols = contracts["facts"]["symbols"]
        .as_array()
        .expect("facts symbols");
    let entry = symbols
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("entry symbol");
    assert_eq!(
        object_keys(entry),
        BTreeSet::from([
            "attrs".to_string(),
            "caps_provenance".to_string(),
            "caps_req".to_string(),
            "caps_transitive".to_string(),
            "ctx_path_provenance".to_string(),
            "ctx_ok".to_string(),
            "ctx_provenance".to_string(),
            "ctx_reachable".to_string(),
            "eff_provenance".to_string(),
            "eff_transitive".to_string(),
            "eff_used".to_string(),
            "is_extern".to_string(),
            "name".to_string(),
            "raw_mmio_sites_count".to_string(),
            "raw_mmio_used".to_string(),
        ]),
        "v2 fact symbol keys drifted"
    );
    assert!(
        entry["ctx_reachable"].is_array(),
        "ctx_reachable must be array"
    );
    assert!(
        entry["ctx_provenance"].is_array(),
        "ctx_provenance must be array"
    );
    assert!(
        entry["ctx_path_provenance"].is_array(),
        "ctx_path_provenance must be array"
    );
    assert!(
        entry["eff_transitive"].is_array(),
        "eff_transitive must be array"
    );
    assert!(
        entry["eff_provenance"].is_array(),
        "eff_provenance must be array"
    );
    assert!(
        entry["caps_transitive"].is_array(),
        "caps_transitive must be array"
    );
    assert!(
        entry["caps_provenance"].is_array(),
        "caps_provenance must be array"
    );

    let eff_prov = entry["eff_provenance"]
        .as_array()
        .expect("eff_provenance array")
        .first()
        .expect("at least one eff provenance entry");
    assert_eq!(
        object_keys(eff_prov),
        BTreeSet::from(["effect".to_string(), "provenance".to_string()]),
        "eff_provenance entry keys drifted"
    );
    assert_eq!(
        object_keys(&eff_prov["provenance"]),
        BTreeSet::from([
            "direct".to_string(),
            "via_callee".to_string(),
            "via_extern".to_string(),
        ]),
        "eff provenance object keys drifted"
    );

    let cap_prov = entry["caps_provenance"]
        .as_array()
        .expect("caps_provenance array")
        .first()
        .expect("at least one cap provenance entry");
    assert_eq!(
        object_keys(cap_prov),
        BTreeSet::from(["capability".to_string(), "provenance".to_string()]),
        "caps_provenance entry keys drifted"
    );
    assert_eq!(
        object_keys(&cap_prov["provenance"]),
        BTreeSet::from([
            "direct".to_string(),
            "via_callee".to_string(),
            "via_extern".to_string(),
        ]),
        "caps provenance object keys drifted"
    );
    let ctx_prov = entry["ctx_provenance"]
        .as_array()
        .expect("ctx_provenance array")
        .first()
        .expect("at least one ctx provenance entry");
    assert_eq!(
        object_keys(ctx_prov),
        BTreeSet::from(["ctx".to_string(), "sources".to_string()]),
        "ctx_provenance entry keys drifted"
    );
    let ctx_path_prov = entry["ctx_path_provenance"]
        .as_array()
        .expect("ctx_path_provenance array")
        .first()
        .expect("at least one ctx path provenance entry");
    assert_eq!(
        object_keys(ctx_path_prov),
        BTreeSet::from(["ctx".to_string(), "path".to_string()]),
        "ctx_path_provenance entry keys drifted"
    );

    assert!(
        contracts["report"]["contexts"].is_null(),
        "report.contexts must not be present in v2"
    );
    assert!(
        contracts["report"]["critical"]["violations"].is_array(),
        "critical violations field must remain an array"
    );
    assert_eq!(
        object_keys(&contracts["report"]["effects"]),
        BTreeSet::from([
            "alloc_sites_count".to_string(),
            "block_sites_count".to_string(),
            "raw_mmio_sites_count".to_string(),
            "yield_sites_count".to_string(),
        ]),
        "v2 report.effects keys drifted"
    );
}

#[test]
fn lockgraph_json_top_level_keys_are_exact() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let module = compile_file(&fixture).expect("compile callee_acquires_lock.kr");
    let (report, errs) = analyze(&module);
    assert!(errs.is_empty(), "analysis errors: {:?}", errs);

    let lockgraph_json = emit_lockgraph_json(&report).expect("emit lockgraph");
    let lockgraph: Value = serde_json::from_str(&lockgraph_json).expect("lockgraph json");
    assert_eq!(
        object_keys(&lockgraph),
        BTreeSet::from(["edges".to_string(), "max_lock_depth".to_string()]),
        "lockgraph JSON must have only edges/max_lock_depth at top level"
    );
}

#[test]
fn interprocedural_lock_edge_is_emitted() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let module = compile_file(&fixture).expect("compile callee_acquires_lock.kr");
    check_module(&module).expect("checks should pass");

    let (report, errs) = analyze(&module);
    assert!(errs.is_empty(), "analysis errors: {:?}", errs);
    let lockgraph_json = emit_lockgraph_json(&report).expect("emit lockgraph");
    let lockgraph: Value = serde_json::from_str(&lockgraph_json).expect("lockgraph json");
    assert_eq!(
        object_keys(&lockgraph),
        BTreeSet::from(["edges".to_string(), "max_lock_depth".to_string()]),
        "lockgraph top-level keys must remain exactly edges/max_lock_depth"
    );
    assert_eq!(lockgraph["max_lock_depth"], Value::Number(2_u64.into()));

    let raw_edges = lockgraph["edges"].as_array().expect("edges array");
    assert_eq!(
        raw_edges.len(),
        1,
        "expected exactly one lockgraph edge for callee_acquires_lock.kr"
    );
    for edge in raw_edges {
        let obj = edge.as_object().expect("edge object");
        assert_eq!(obj.len(), 2, "edge schema must only contain from/to keys");
        assert!(
            obj.get("from").and_then(|v| v.as_str()).is_some(),
            "edge.from string"
        );
        assert!(
            obj.get("to").and_then(|v| v.as_str()).is_some(),
            "edge.to string"
        );
    }

    let edges = raw_edges
        .iter()
        .map(|e| {
            (
                e["from"].as_str().expect("edge.from string").to_string(),
                e["to"].as_str().expect("edge.to string").to_string(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        edges,
        BTreeSet::from([("ConsoleLock".into(), "SchedLock".into())]),
        "expected exact lockgraph edge set for callee_acquires_lock.kr"
    );
}

#[test]
fn yield_hidden_two_levels_reports_expected_lockgraph_error() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("yield_hidden_two_levels.kr");
    let errs = check_file(&fixture).expect_err("yield_hidden_two_levels should fail");
    let needle = "calls yielding callee 'mid' under lock(s): SchedLock";
    assert!(
        errs.iter()
            .any(|e| e.starts_with("lockgraph: ") && e.contains(needle)),
        "expected the yielding-callee-under-lock diagnostic to be lockgraph-labeled, got {:?}",
        errs
    );
    let hits = errs.iter().filter(|e| e.contains(needle)).count();
    assert_eq!(
        hits, 1,
        "expected exactly one yielding-callee-under-lock callsite error, got {:?}",
        errs
    );

    let lockgraph_msgs = errs
        .iter()
        .filter(|e| e.starts_with("lockgraph:"))
        .cloned()
        .collect::<Vec<_>>();
    let lockgraph_errs = lockgraph_msgs.len();
    assert_eq!(
        lockgraph_errs, 1,
        "expected exactly one lockgraph error (no secondary noise), got {:?}",
        errs
    );
    let expected_lockgraph =
        "lockgraph: function 'outer' calls yielding callee 'mid' under lock(s): SchedLock";
    assert_eq!(
        lockgraph_msgs,
        vec![expected_lockgraph.to_string()],
        "expected no additional lockgraph messages beyond the callsite diagnostic"
    );

    let analysis_errs = errs.iter().filter(|e| e.starts_with("analysis:")).count();
    assert_eq!(
        analysis_errs, 0,
        "expected no analysis diagnostics for this fixture, got {:?}",
        errs
    );
}

#[test]
fn yield_hidden_in_leaf_wrapper_reports_expected_lockgraph_error() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("yield_hidden_in_leaf_wrapper.kr");
    let errs = check_file(&fixture).expect_err("yield_hidden_in_leaf_wrapper should fail");
    assert_eq!(
        errs,
        vec![
            "lockgraph: function 'outer' calls yielding callee 'wrapper' under lock(s): SchedLock"
                .to_string()
        ],
        "expected exact yielding-callee-under-lock diagnostic through @leaf wrapper"
    );
}

#[test]
fn report_rejects_unknown_metric() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");
    let module = compile_file(&fixture).expect("compile locks_ok.kr");
    check_module(&module).expect("checks should pass");
    let (report, errs) = analyze(&module);
    assert!(errs.is_empty(), "analysis errors: {:?}", errs);

    let err = emit_report_json(&report, &["unknown_metric".to_string()])
        .expect_err("unknown metric should fail");
    assert!(err.contains("unsupported report metric"));
}

#[test]
fn unresolved_callee_reports_hir_error() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("unresolved_callee.kr");
    let errs = check_file(&fixture).expect_err("unresolved_callee should fail");
    assert!(
        errs.iter()
            .any(|e| e.contains("undefined symbol 'no_such_symbol'")),
        "expected HIR undefined symbol error, got {:?}",
        errs
    );
}

#[test]
fn extern_missing_eff_reports_hir_error() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("extern_missing_eff.kr");
    let errs = check_file(&fixture).expect_err("extern_missing_eff should fail");
    assert_eq!(
        errs,
        vec![
            "extern 'sleep' must declare @eff(...) facts explicitly at 2:1\n  2 | extern @ctx(thread) @caps() fn sleep();\n  = help: use the canonical extern skeleton: extern @ctx(...) @eff(...) @caps() fn sleep();".to_string()
        ],
        "expected exact HIR extern missing @eff error"
    );
}

#[test]
fn extern_missing_ctx_reports_hir_error() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("extern_missing_ctx.kr");
    let errs = check_file(&fixture).expect_err("extern_missing_ctx should fail");
    assert_eq!(
        errs,
        vec![
            "extern 'sleep' must declare @ctx(...) facts explicitly at 1:1\n  1 | extern @eff(block) @caps() fn sleep();\n  = help: use the canonical extern skeleton: extern @ctx(...) @eff(...) @caps() fn sleep();".to_string()
        ],
        "expected exact HIR extern missing @ctx error"
    );
}

#[test]
fn extern_missing_caps_reports_hir_error() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("extern_missing_caps.kr");
    let errs = check_file(&fixture).expect_err("extern_missing_caps should fail");
    assert_eq!(
        errs,
        vec![
            "EXTERN_CAPS_CONTRACT_REQUIRED: extern 'sleep' must declare @caps(...) facts explicitly at 1:1\n  1 | extern @ctx(thread) @eff(block) fn sleep();\n  = help: use the canonical extern skeleton: extern @ctx(...) @eff(...) @caps() fn sleep();"
                .to_string()
        ],
        "expected exact HIR extern missing @caps error"
    );
}

#[test]
fn release_mismatch_nested_reports_exact_lockgraph_message() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("release_mismatch_nested.kr");
    let errs = check_file(&fixture).expect_err("release_mismatch_nested should fail");
    let expected = "lockgraph: function 'nested_release_mismatch' release mismatch: expected 'SchedLock' on top, found 'ConsoleLock'";
    assert_eq!(
        errs,
        vec![expected.to_string()],
        "expected exact single lockgraph mismatch error"
    );
    assert!(
        !errs.iter().any(|e| e.starts_with("analysis:")),
        "expected no analysis diagnostics, got {:?}",
        errs
    );
}

#[test]
fn thread_no_yield_span_reports_unbounded() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("thread_no_yield_unbounded.kr");
    let module = compile_file(&fixture).expect("compile thread_no_yield_unbounded.kr");
    check_module(&module).expect("checks should pass");
    let (report, errs) = analyze(&module);
    assert!(errs.is_empty(), "analysis errors: {:?}", errs);

    let report_json =
        emit_report_json(&report, &["no_yield_spans".to_string()]).expect("emit report");
    let value: Value = serde_json::from_str(&report_json).expect("report json");
    assert_eq!(
        value["no_yield_spans"]["worker"],
        Value::String("unbounded".to_string()),
        "worker no_yield_spans should be unbounded"
    );
    assert_eq!(
        value["no_yield_spans"]["helper"],
        Value::String("unbounded".to_string()),
        "helper no_yield_spans should be unbounded"
    );
}

#[test]
fn error_ordering_is_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("error_ordering.kr");

    let errs_first = check_file(&fixture).expect_err("error_ordering should fail");
    let errs_second = check_file(&fixture).expect_err("error_ordering should fail");
    assert_eq!(
        errs_first, errs_second,
        "error ordering must be deterministic across runs"
    );

    assert_eq!(
        errs_first,
        vec![
            "cap-check: function 'cap_hungry' requires unavailable caps: MissingCap".to_string(),
            "ctx-check: function 'irq_yielder' contains yieldpoint but is allowed in irq/nmi context".to_string(),
        ],
        "error ordering should remain stable and sorted by pass/message"
    );
}

#[test]
fn recursion_is_reported_as_analysis_error() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_fail").join("recursion.kr");
    let errs = check_file(&fixture).expect_err("recursion should fail");

    assert!(
        errs.iter()
            .any(|e| e.starts_with("analysis: recursion unsupported in KR0.1:")),
        "expected analysis-labeled recursion error, got {:?}",
        errs
    );
    assert!(
        !errs.iter().any(|e| e.starts_with("lockgraph: recursion")),
        "recursion must not be labeled as lockgraph, got {:?}",
        errs
    );
}

#[test]
fn branch_op_symbol_layout_is_contiguous_in_x86_64_elf_object() {
    // Regression: branch ops were counted as test_bytes+16 but emit test_bytes+21 bytes.
    // The missing 5 bytes (the jmp-over-else instruction) caused every function symbol
    // following a branch-containing function to have an offset 5 bytes too low, making
    // internal call displacements land 5 bytes before the actual target.
    let root = repo_root();
    let fixture = root.join("examples").join("uart_console_branch_mask.kr");
    let module = compile_file(&fixture).expect("compile uart_console_branch_mask.kr");
    let executable =
        lower_current_krir_to_executable_krir(&module).expect("lower to executable krir");
    let target = BackendTargetContract::x86_64_sysv();
    let object = lower_executable_krir_to_x86_64_object(&executable, &target)
        .expect("lower to x86_64 elf object");

    let mut by_offset: Vec<_> = object.function_symbols.iter().collect();
    by_offset.sort_by_key(|s| s.offset);

    for window in by_offset.windows(2) {
        let (prev, next) = (&window[0], &window[1]);
        assert_eq!(
            prev.offset + prev.size,
            next.offset,
            "symbol '{}' (offset={}, size={}) must end exactly where '{}' (offset={}) begins; \
             branch op size mismatch causes gaps/overlaps",
            prev.name,
            prev.offset,
            prev.size,
            next.name,
            next.offset,
        );
    }

    let last = by_offset.last().expect("at least one symbol");
    assert_eq!(
        last.offset + last.size,
        object.text_bytes.len() as u64,
        "last symbol '{}' (offset={}, size={}) must end at text boundary ({})",
        last.name,
        last.offset,
        last.size,
        object.text_bytes.len(),
    );
}

#[test]
fn staticlib_emit_produces_ar_archive_with_global_symbols() {
    // KR0 exit criterion: "freestanding static library callable from C"
    // --emit staticlib must produce a valid GNU ar archive containing the ELF object.
    // Skipped when ar (binutils) is not available on the host.
    if std::process::Command::new("ar")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping staticlib test: ar not available on this host");
        return;
    }

    let root = repo_root();
    let fixture = root.join("examples").join("uart_freestanding_lib.kr");
    let archive_bytes = emit_backend_artifact_file(&fixture, BackendArtifactKind::StaticLib)
        .expect("compile uart_freestanding_lib.kr to staticlib");

    // GNU ar archive must start with the magic string.
    assert!(
        archive_bytes.starts_with(b"!<arch>\n"),
        "staticlib output must start with ar magic '!<arch>\\n'; \
         got {:?}",
        &archive_bytes[..8.min(archive_bytes.len())]
    );

    // Must contain at least the symbol table + one object member header (60 bytes each).
    assert!(
        archive_bytes.len() > 8 + 60,
        "staticlib output is too short to contain an archive member (len={})",
        archive_bytes.len()
    );
}
