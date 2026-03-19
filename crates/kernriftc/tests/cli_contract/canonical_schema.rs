#[test]
fn canonical_findings_json_schema_accepts_empty_and_nonempty_reports() {
    let compiled = compile_canonical_findings_schema();
    for instance in [
        json!({
            "schema_version": "kernrift_canonical_findings_v1",
            "surface": "stable",
            "canonical_findings": 0,
            "findings": []
        }),
        json!({
            "schema_version": "kernrift_canonical_findings_v1",
            "surface": "experimental",
            "canonical_findings": 1,
            "findings": [{
                "function": "helper",
                "classification": "compatibility_alias",
                "surface_form": "@irq_handler",
                "canonical_replacement": "@ctx(irq)",
                "migration_safe": true
            }]
        }),
    ] {
        if let Err(errors) = compiled.validate(&instance) {
            let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
            panic!(
                "canonical findings JSON must validate against canonical findings v1 schema: {}",
                details.join(" | ")
            );
        }
    }
}


#[test]
fn canonical_edit_plan_json_schema_accepts_empty_and_nonempty_reports() {
    let compiled = compile_canonical_edit_plan_schema();
    for instance in [
        json!({
            "schema_version": "kernrift_canonical_edit_plan_v1",
            "surface": "stable",
            "edits_count": 0,
            "edits": []
        }),
        json!({
            "schema_version": "kernrift_canonical_edit_plan_v1",
            "surface": "experimental",
            "edits_count": 1,
            "edits": [{
                "function": "worker",
                "classification": "compatibility_alias",
                "surface_form": "@thread_entry",
                "canonical_replacement": "@ctx(thread)",
                "migration_safe": true,
                "rewrite_intent": "Replace the attribute token `@thread_entry` with `@ctx(thread)`."
            }]
        }),
    ] {
        if let Err(errors) = compiled.validate(&instance) {
            let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
            panic!(
                "canonical edit plan JSON must validate against canonical edit plan v1 schema: {}",
                details.join(" | ")
            );
        }
    }
}

#[test]
fn canonical_fix_result_json_schema_accepts_empty_and_nonempty_reports() {
    let compiled = compile_canonical_fix_result_schema();
    for instance in [
        json!({
            "schema_version": "kernrift_canonical_fix_result_v1",
            "surface": "stable",
            "file": "basic.kr",
            "rewrites_applied": 0,
            "changed": false,
            "rewrites": []
        }),
        json!({
            "schema_version": "kernrift_canonical_fix_result_v1",
            "surface": "experimental",
            "file": "aliases.kr",
            "rewrites_applied": 1,
            "changed": true,
            "rewrites": [{
                "function": "helper",
                "classification": "compatibility_alias",
                "surface_form": "@irq_handler",
                "canonical_replacement": "@ctx(irq)",
                "migration_safe": true
            }]
        }),
    ] {
        if let Err(errors) = compiled.validate(&instance) {
            let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
            panic!(
                "canonical fix result JSON must validate against canonical fix result v1 schema: {}",
                details.join(" | ")
            );
        }
    }
}

#[test]
fn canonical_fix_preview_json_schema_accepts_empty_and_nonempty_reports() {
    let compiled = compile_canonical_fix_preview_schema();
    for instance in [
        json!({
            "schema_version": "kernrift_canonical_fix_preview_v1",
            "surface": "stable",
            "file": "fixture.kr",
            "rewrites_planned": 0,
            "would_change": false,
            "rewrites": []
        }),
        json!({
            "schema_version": "kernrift_canonical_fix_preview_v1",
            "surface": "experimental",
            "file": "fixture.kr",
            "rewrites_planned": 1,
            "would_change": true,
            "rewrites": [{
                "function": "helper",
                "classification": "compatibility_alias",
                "surface_form": "@irq_handler",
                "canonical_replacement": "@ctx(irq)",
                "migration_safe": true
            }]
        }),
    ] {
        if let Err(errors) = compiled.validate(&instance) {
            let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
            panic!(
                "canonical fix preview JSON must validate against canonical fix preview v1 schema: {}",
                details.join(" | ")
            );
        }
    }
}

