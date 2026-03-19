#[test]
fn structured_output_conventions_spec_locks_future_json_command_transport_tests() {
    assert!(
        KRIR_SPEC_TEXT
            .contains("new JSON-capable commands must add `cli_contract` transport assertions"),
        "structured output conventions spec must require cli_contract transport assertions"
    );
    assert!(
        KRIR_SPEC_TEXT.contains("prefer reusing `assert_json_transport`"),
        "structured output conventions spec must point contributors at assert_json_transport"
    );
    assert!(
        KRIR_SPEC_TEXT.contains("stdout` only") || KRIR_SPEC_TEXT.contains("`stdout` only"),
        "structured output conventions spec must lock stdout-only JSON transport"
    );
    assert!(
        KRIR_SPEC_TEXT.contains("empty `stderr`"),
        "structured output conventions spec must lock empty stderr in JSON mode"
    );
    assert!(
        KRIR_SPEC_TEXT.contains("trailing\n  newline termination")
            || KRIR_SPEC_TEXT.contains("trailing newline"),
        "structured output conventions spec must lock trailing-newline JSON termination"
    );
}

#[test]
fn structured_output_command_matrix_spec_lists_current_json_capable_commands() {
    for surface in [
        "kernriftc inspect-report --report <verify-report.json> --format json",
        "kernriftc inspect-artifact <artifact> --format json",
        "kernriftc verify-artifact-meta --format json <artifact> <meta.json>",
        "kernriftc policy --format json --policy <policy.toml> --contracts <contracts.json>",
        "kernriftc check --format json --policy <policy.toml> <file.kr>",
        "kernriftc check --canonical --format json <file.kr>",
        "kernriftc migrate-preview --canonical-edits --format json --surface stable <file.kr>",
        "kernriftc fix --canonical --write --format json <file.kr>",
        "kernriftc fix --canonical --dry-run --format json <file.kr>",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(surface),
            "structured output command matrix must list '{}'",
            surface
        );
    }
    for schema in [
        "kernrift_inspect_report_v1",
        "kernrift_inspect_artifact_v2",
        "kernrift_verify_artifact_meta_v2",
        "kernrift_policy_violations_v1",
        "kernrift_canonical_findings_v2",
        "kernrift_canonical_edit_plan_v2",
        "kernrift_canonical_fix_result_v1",
        "kernrift_canonical_fix_preview_v1",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(schema),
            "structured output command matrix must mention schema '{}'",
            schema
        );
    }
}

#[test]
fn structured_output_test_coverage_matrix_spec_lists_current_json_capable_commands() {
    assert!(
        KRIR_SPEC_TEXT.contains("### Structured Output Test Coverage Matrix"),
        "structured output spec must include the test coverage matrix section"
    );
    for surface in [
        "kernriftc inspect-report --report <verify-report.json> --format json",
        "kernriftc inspect-artifact <artifact> --format json",
        "kernriftc verify-artifact-meta --format json <artifact> <meta.json>",
        "kernriftc policy --format json --policy <policy.toml> --contracts <contracts.json>",
        "kernriftc check --format json --policy <policy.toml> <file.kr>",
        "kernriftc check --canonical --format json <file.kr>",
        "kernriftc migrate-preview --canonical-edits --format json --surface stable <file.kr>",
        "kernriftc fix --canonical --write --format json <file.kr>",
        "kernriftc fix --canonical --dry-run --format json <file.kr>",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(surface),
            "structured output test coverage matrix must list '{}'",
            surface
        );
    }
}

#[test]
fn structured_output_new_json_command_checklist_is_present() {
    assert!(
        KRIR_SPEC_TEXT.contains("### New JSON Command Checklist"),
        "structured output spec must include the new JSON command checklist"
    );
    for line in [
        "document the command surface in the structured-output command matrix",
        "document its coverage in the structured-output test coverage matrix",
        "add `cli_contract` transport assertions",
        "reuse `assert_json_transport` where applicable",
        "add or reference a schema when the payload is versioned",
        "preserve `stdout`-only, empty-`stderr`, trailing-newline transport behavior",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(line),
            "structured output checklist must include '{}'",
            line
        );
    }
}

#[test]
fn readme_points_to_local_safe_and_full_serial_validation_wrappers() {
    assert!(
        README_TEXT.contains("bash tools/validation/local_safe.sh"),
        "README must point contributors at the repo-owned local-safe validation wrapper"
    );
    assert!(
        README_TEXT.contains("default repo-owned local-safe validation path")
            && README_TEXT.contains("32 GB class machines"),
        "README must describe local_safe.sh as the default path on this machine class"
    );
    assert!(
        README_TEXT.contains("bash tools/validation/full_serial.sh"),
        "README must still point contributors at the heavier full_serial wrapper"
    );
    assert!(
        README_TEXT.contains("heavier local `hir` coverage")
            && README_TEXT.contains("closer to the CI-style path"),
        "README must distinguish full_serial as the heavier local path"
    );
}

#[test]
fn readme_points_to_full_serial_low_memory_validation_wrapper() {
    assert!(
        README_TEXT.contains("bash tools/validation/full_serial.sh"),
        "README must point contributors at the repo-owned full serialized validation wrapper"
    );
    assert!(
        README_TEXT.contains("per-crate serialized test steps")
            && README_TEXT.contains("heavier local `hir` coverage"),
        "README must describe the full_serial wrapper as the heavier serialized local path"
    );
}

#[test]
fn full_serial_wrapper_avoids_workspace_test_aggregation() {
    assert!(
        !FULL_SERIAL_WRAPPER_TEXT.contains("cargo test --workspace -- --test-threads=1"),
        "full_serial wrapper must not use the workspace-wide test aggregation step locally"
    );
    for step in [
        "run_step cargo test -p parser -- --test-threads=1",
        "run_step cargo test -p hir -- --test-threads=1",
        "run_step cargo test -p krir -- --test-threads=1",
        "run_step cargo test -p kernriftc --test cli_contract -- --test-threads=1",
        "run_step cargo test -p kernriftc --test golden -- --test-threads=1",
        "run_step cargo test -p kernriftc -- --test-threads=1",
    ] {
        assert!(
            FULL_SERIAL_WRAPPER_TEXT.contains(step),
            "full_serial wrapper must include '{}'",
            step
        );
    }
}

#[test]
fn local_safe_wrapper_stays_explicit_and_avoids_dangerous_local_steps() {
    assert!(
        !LOCAL_SAFE_WRAPPER_TEXT.contains("cargo test --workspace"),
        "local_safe wrapper must not use workspace-wide test aggregation"
    );
    assert!(
        !LOCAL_SAFE_WRAPPER_TEXT.contains("cargo test -p hir -- --test-threads=1"),
        "local_safe wrapper must not include the heavy hir randomized coverage locally"
    );
    assert!(
        !LOCAL_SAFE_WRAPPER_TEXT.contains("./tools/acceptance/all.sh"),
        "local_safe wrapper must avoid the heavier all.sh acceptance aggregation"
    );
    for step in [
        "run_step cargo fmt --all",
        "run_step cargo test -p kernriftc --test cli_contract -- --test-threads=1",
        "run_step cargo test -p kernriftc --test golden -- --test-threads=1",
        "run_step cargo test -p kernriftc -- --test-threads=1",
        "run_step cargo clippy -p kernriftc --all-targets -- -D warnings",
        "run_step ./tools/acceptance/krir_v0_1.sh",
        "run_step ./tools/acceptance/kernriftc_artifact_exports.sh",
        "run_step cargo run -q -p kernriftc -- --selftest",
    ] {
        assert!(
            LOCAL_SAFE_WRAPPER_TEXT.contains(step),
            "local_safe wrapper must include '{}'",
            step
        );
    }
}

#[test]
fn kr0_frontend_spec_declares_canonical_spellings_and_alias_policy() {
    assert!(
        KRIR_SPEC_TEXT.contains("### Canonical KR0 Frontend Spellings"),
        "kr0 frontend spec must declare a canonical spelling section"
    );
    for spelling in [
        "`@ctx(...)`",
        "`@eff(...)`",
        "`@caps(...)`",
        "`@module_caps(...)`",
        "`critical { ... }`",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(spelling),
            "kr0 frontend spec must list canonical spelling '{}'",
            spelling
        );
    }
    for alias in [
        "`@thread_entry`",
        "`@irq_handler`",
        "`@may_block`",
        "`@irq_legacy`",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(alias),
            "kr0 frontend spec must classify compatibility alias '{}'",
            alias
        );
    }
    assert!(
        KRIR_SPEC_TEXT.contains("Compatibility fixtures under `tests/living_compiler/*alias*.kr`"),
        "kr0 frontend spec must explain that alias fixtures are compatibility locks"
    );
    assert!(
        KRIR_SPEC_TEXT.contains(
            "classify non-canonical spellings as `compatibility aliases` or `deprecated aliases`"
        ),
        "kr0 frontend spec must describe alias classification guidance"
    );
    for legacy in [
        "`@irq` -> `@ctx(irq)`",
        "`@noirq` -> `@ctx(thread, boot)`",
        "`@alloc` -> `@eff(alloc)`",
        "`@block` -> `@eff(block)`",
        "`@preempt_off` -> `@eff(preempt_off)`",
        "`@yieldpoint` -> `yieldpoint()`",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(legacy),
            "kr0 frontend spec must document legacy-to-canonical mapping '{}'",
            legacy
        );
    }
}

#[test]
fn kr0_authoring_reference_covers_canonical_templates() {
    assert!(
        KRIR_SPEC_TEXT.contains("docs/spec/kr0-canonical-authoring-reference.md"),
        "krir spec must point readers at the canonical KR0 authoring reference"
    );
    for section in [
        "## Canonical Function Forms",
        "## Canonical Extern Form",
        "## Canonical Context, Effect, and Capability Facts",
        "## Canonical MMIO Declaration and Use",
        "## Canonical Critical and Yield Usage",
        "## Common Mistakes -> Canonical Replacement",
        "## Alias Fixture Note",
    ] {
        assert!(
            KR0_AUTHORING_REFERENCE_TEXT.contains(section),
            "kr0 authoring reference must contain '{}'",
            section
        );
    }
    for template in [
        "fn entry() { }",
        "extern @ctx(thread, boot) @eff(block) @caps() fn sleep();",
        "@module_caps(PhysMap);",
        "@ctx(thread, boot,)",
        "@module_caps(PhysMap,)",
        "kernriftc check --canonical <file.kr>",
        "mmio UART0 = 0x1000;",
        "mmio_reg UART0.DR = 0x00 : u32 rw;",
        "mmio_read<u32>(UART0 + 0x04);",
        "raw_mmio_write<u32>(0x1014, x);",
        "critical {",
        "yieldpoint();",
        "| `@irq` | `@ctx(irq)` |",
        "| `@noirq` | `@ctx(thread, boot)` |",
        "| `@alloc` | `@eff(alloc)` |",
        "| `@block` | `@eff(block)` |",
        "| `@preempt_off` | `@eff(preempt_off)` |",
    ] {
        assert!(
            KR0_AUTHORING_REFERENCE_TEXT.contains(template),
            "kr0 authoring reference must contain template '{}'",
            template
        );
    }
    assert!(
        KR0_AUTHORING_REFERENCE_TEXT.contains("tests/living_compiler/*alias*.kr"),
        "kr0 authoring reference must explain that alias fixtures are compatibility locks"
    );
}

#[test]
fn kr0_spec_and_authoring_reference_document_trailing_comma_fact_lists() {
    assert!(
        KRIR_SPEC_TEXT.contains("CsvIdent    := Ident { \",\" Ident } [ \",\" ]"),
        "krir spec must document optional trailing commas in canonical fact lists"
    );
    for example in [
        "@ctx(thread, boot,)",
        "@eff(block,)",
        "@caps(PhysMap,)",
        "@module_caps(PhysMap,)",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(example) || KR0_AUTHORING_REFERENCE_TEXT.contains(example),
            "docs must mention trailing-comma example '{}'",
            example
        );
    }
}

#[test]
fn kr0_general_teaching_docs_prefer_canonical_frontend_surface() {
    let teaching_docs = [
        ARCHITECTURE_DOC_TEXT,
        KR0_KR3_PLAN_TEXT,
        ADAPTIVE_OS_CONTEXT_TEXT,
        KERNEL_PROFILE_SPEC_TEXT,
        KERNEL_PROFILE_NOTES_TEXT,
    ];

    for legacy in [
        "@irq",
        "@noirq",
        "@alloc",
        "@block",
        "@preempt_off",
        "@yieldpoint",
        "mmio<T>",
        "volatile_load",
        "volatile_store",
    ] {
        for doc in teaching_docs {
            assert!(
                !doc.contains(legacy),
                "general teaching docs must not drift back to legacy/non-canonical surface '{}'",
                legacy
            );
        }
    }

    assert!(
        ARCHITECTURE_DOC_TEXT.contains("@ctx(...)")
            && ARCHITECTURE_DOC_TEXT.contains("@eff(...)")
            && ARCHITECTURE_DOC_TEXT.contains("@caps(...)"),
        "architecture docs must teach canonical fact spellings"
    );
    assert!(
        KR0_KR3_PLAN_TEXT.contains("yieldpoint()"),
        "KR0/KR3 plan must teach the canonical yieldpoint() form"
    );
    assert!(
        ADAPTIVE_OS_CONTEXT_TEXT.contains("mmio_read<T>(addr)")
            && ADAPTIVE_OS_CONTEXT_TEXT.contains("mmio_write<T>(addr, value)"),
        "adaptive OS context doc must teach current typed MMIO call forms"
    );
}
