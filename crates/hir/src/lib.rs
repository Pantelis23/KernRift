use std::collections::BTreeSet;

use krir::{CallEdge, Ctx, Eff, Function, FunctionAttrs, KrirModule, KrirOp};
use parser::{FnAst, ModuleAst, RawAttr, Stmt, split_csv};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceProfile {
    #[default]
    Stable,
    Experimental,
}

impl SurfaceProfile {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "stable" => Ok(Self::Stable),
            "experimental" => Ok(Self::Experimental),
            other => Err(format!(
                "invalid surface mode '{}', expected 'stable' or 'experimental'",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdaptiveFeatureStatus {
    Experimental,
    Stable,
    Deprecated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AdaptiveSurfaceFeature {
    pub id: &'static str,
    pub proposal_id: &'static str,
    pub surface_form: &'static str,
    pub status: AdaptiveFeatureStatus,
    pub lowering_target: &'static str,
    pub safety_notes: &'static str,
    pub migration_supported: bool,
    pub migration_note: &'static str,
    pub surface_profile_gate: SurfaceProfile,
    #[serde(skip_serializing)]
    lowering_rule: AdaptiveLoweringRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AdaptiveFeatureProposal {
    pub id: &'static str,
    pub title: &'static str,
    pub motivation: &'static str,
    pub syntax_before: &'static str,
    pub syntax_after: &'static str,
    pub lowering_description: &'static str,
    pub compatibility_risk: &'static str,
    pub migration_plan: &'static str,
    pub status: AdaptiveFeatureStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdaptiveLoweringRule {
    ContextAlias(&'static [Ctx]),
    EffectAlias(&'static [Eff]),
}

const ADAPTIVE_SURFACE_FEATURES: [AdaptiveSurfaceFeature; 4] = [
    AdaptiveSurfaceFeature {
        id: "irq_handler_alias",
        proposal_id: "irq_handler_alias",
        surface_form: "irq_handler",
        status: AdaptiveFeatureStatus::Experimental,
        lowering_target: "@ctx(irq)",
        safety_notes: "Pure surface alias; lowers to the existing irq context declaration.",
        migration_supported: true,
        migration_note: "Replace with @ctx(irq) when pinning code back to stable.",
        surface_profile_gate: SurfaceProfile::Experimental,
        lowering_rule: AdaptiveLoweringRule::ContextAlias(&[Ctx::Irq]),
    },
    AdaptiveSurfaceFeature {
        id: "thread_entry_alias",
        proposal_id: "thread_entry_alias",
        surface_form: "thread_entry",
        status: AdaptiveFeatureStatus::Experimental,
        lowering_target: "@ctx(thread)",
        safety_notes: "Pure surface alias; lowers to the existing thread context declaration.",
        migration_supported: true,
        migration_note: "Replace with @ctx(thread) when pinning code back to stable.",
        surface_profile_gate: SurfaceProfile::Experimental,
        lowering_rule: AdaptiveLoweringRule::ContextAlias(&[Ctx::Thread]),
    },
    AdaptiveSurfaceFeature {
        id: "may_block_alias",
        proposal_id: "may_block_alias",
        surface_form: "may_block",
        status: AdaptiveFeatureStatus::Experimental,
        lowering_target: "@eff(block)",
        safety_notes: "Pure surface alias; lowers to the existing block effect declaration.",
        migration_supported: true,
        migration_note: "Replace with @eff(block) when pinning code back to stable.",
        surface_profile_gate: SurfaceProfile::Experimental,
        lowering_rule: AdaptiveLoweringRule::EffectAlias(&[Eff::Block]),
    },
    AdaptiveSurfaceFeature {
        id: "irq_legacy_alias",
        proposal_id: "irq_legacy_alias",
        surface_form: "irq_legacy",
        status: AdaptiveFeatureStatus::Deprecated,
        lowering_target: "@ctx(irq)",
        safety_notes: "Deprecated surface alias kept only to prove centralized lifecycle gating.",
        migration_supported: true,
        migration_note: "Replace with @ctx(irq) or @irq_handler depending on the chosen profile policy.",
        surface_profile_gate: SurfaceProfile::Stable,
        lowering_rule: AdaptiveLoweringRule::ContextAlias(&[Ctx::Irq]),
    },
];

pub fn adaptive_surface_features() -> &'static [AdaptiveSurfaceFeature] {
    &ADAPTIVE_SURFACE_FEATURES
}

const ADAPTIVE_FEATURE_PROPOSALS: [AdaptiveFeatureProposal; 4] = [
    AdaptiveFeatureProposal {
        id: "irq_handler_alias",
        title: "Experimental @irq_handler surface alias",
        motivation: "Provide a governed surface-only shorthand for irq-context entry points.",
        syntax_before: "@ctx(irq) fn isr() { }",
        syntax_after: "@irq_handler fn isr() { }",
        lowering_description: "Lower @irq_handler to the existing canonical @ctx(irq) representation during HIR lowering.",
        compatibility_risk: "Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
        migration_plan: "Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
        status: AdaptiveFeatureStatus::Experimental,
    },
    AdaptiveFeatureProposal {
        id: "thread_entry_alias",
        title: "Experimental @thread_entry surface alias",
        motivation: "Provide a governed surface-only shorthand for thread-only entry points.",
        syntax_before: "@ctx(thread) fn worker() { }",
        syntax_after: "@thread_entry fn worker() { }",
        lowering_description: "Lower @thread_entry to the existing canonical @ctx(thread) representation during HIR lowering.",
        compatibility_risk: "Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
        migration_plan: "Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
        status: AdaptiveFeatureStatus::Experimental,
    },
    AdaptiveFeatureProposal {
        id: "may_block_alias",
        title: "Experimental @may_block surface alias",
        motivation: "Provide a governed surface-only shorthand for declaring block effects.",
        syntax_before: "@eff(block) fn worker() { }",
        syntax_after: "@may_block fn worker() { }",
        lowering_description: "Lower @may_block to the existing canonical @eff(block) representation during HIR lowering.",
        compatibility_risk: "Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
        migration_plan: "Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
        status: AdaptiveFeatureStatus::Experimental,
    },
    AdaptiveFeatureProposal {
        id: "irq_legacy_alias",
        title: "Deprecated @irq_legacy surface alias",
        motivation: "Preserve a historical alias only long enough to exercise deterministic lifecycle gating.",
        syntax_before: "@ctx(irq) fn legacy_isr() { }",
        syntax_after: "@irq_legacy fn legacy_isr() { }",
        lowering_description: "Would lower to the existing canonical @ctx(irq) representation if lifecycle policy allowed it.",
        compatibility_risk: "Medium; the alias is deprecated and intentionally unavailable under current surface profiles.",
        migration_plan: "Replace with @ctx(irq) or @irq_handler depending on the chosen profile policy.",
        status: AdaptiveFeatureStatus::Deprecated,
    },
];

pub fn adaptive_feature_proposals() -> &'static [AdaptiveFeatureProposal] {
    &ADAPTIVE_FEATURE_PROPOSALS
}

pub fn adaptive_feature_proposal(feature_id: &str) -> Option<&'static AdaptiveFeatureProposal> {
    let feature = ADAPTIVE_SURFACE_FEATURES
        .iter()
        .find(|feature| feature.id == feature_id)?;
    ADAPTIVE_FEATURE_PROPOSALS
        .iter()
        .find(|proposal| proposal.id == feature.proposal_id)
}

fn adaptive_surface_feature(attr_name: &str) -> Option<&'static AdaptiveSurfaceFeature> {
    ADAPTIVE_SURFACE_FEATURES
        .iter()
        .find(|feature| feature.surface_form == attr_name)
}

fn surface_profile_enables_feature(
    surface_profile: SurfaceProfile,
    feature: &AdaptiveSurfaceFeature,
) -> bool {
    match surface_profile {
        SurfaceProfile::Stable => matches!(feature.status, AdaptiveFeatureStatus::Stable),
        SurfaceProfile::Experimental => matches!(
            feature.status,
            AdaptiveFeatureStatus::Stable | AdaptiveFeatureStatus::Experimental
        ),
    }
}

fn feature_unavailability_error(
    surface_profile: SurfaceProfile,
    feature: &AdaptiveSurfaceFeature,
    function_name: &str,
) -> String {
    match feature.status {
        AdaptiveFeatureStatus::Experimental => format!(
            "surface feature '@{}' requires --surface experimental for '{}'",
            feature.surface_form, function_name
        ),
        AdaptiveFeatureStatus::Deprecated => format!(
            "surface feature '@{}' is deprecated and unavailable under --surface {} for '{}'",
            feature.surface_form,
            match surface_profile {
                SurfaceProfile::Stable => "stable",
                SurfaceProfile::Experimental => "experimental",
            },
            function_name
        ),
        AdaptiveFeatureStatus::Stable => format!(
            "surface feature '@{}' is unavailable under --surface {} for '{}'",
            feature.surface_form,
            match surface_profile {
                SurfaceProfile::Stable => "stable",
                SurfaceProfile::Experimental => "experimental",
            },
            function_name
        ),
    }
}

pub fn irq_handler_alias_proposal() -> AdaptiveFeatureProposal {
    *adaptive_feature_proposal("irq_handler_alias").expect("irq_handler_alias proposal")
}

pub fn lower_to_krir(ast: &ModuleAst) -> Result<KrirModule, Vec<String>> {
    lower_to_krir_with_surface(ast, SurfaceProfile::Stable)
}

pub fn lower_to_krir_with_surface(
    ast: &ModuleAst,
    surface_profile: SurfaceProfile,
) -> Result<KrirModule, Vec<String>> {
    let mut errors = Vec::new();
    let mut functions = Vec::new();
    let mut names = BTreeSet::new();

    for item in &ast.items {
        if !names.insert(item.name.clone()) {
            errors.push(format!("duplicate symbol '{}'", item.name));
        }
    }

    for item in &ast.items {
        match lower_function(item, surface_profile) {
            Ok(function) => functions.push(function),
            Err(errs) => errors.extend(errs),
        }
    }

    let mut call_edges = Vec::new();

    for function in &functions {
        if function.is_extern {
            continue;
        }
        for op in &function.ops {
            if let KrirOp::Call { callee } = op {
                if !names.contains(callee) {
                    errors.push(format!(
                        "undefined symbol '{}': add extern declaration with facts (@ctx/@eff/@caps)",
                        callee
                    ));
                    continue;
                }
                call_edges.push(CallEdge {
                    caller: function.name.clone(),
                    callee: callee.clone(),
                });
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut module = KrirModule {
        module_caps: ast.module_caps.clone(),
        functions,
        call_edges,
    };
    module.canonicalize();
    Ok(module)
}

fn lower_function(item: &FnAst, surface_profile: SurfaceProfile) -> Result<Function, Vec<String>> {
    let mut errors = Vec::new();
    let mut ctx_ok = BTreeSet::new();
    let mut eff_used = BTreeSet::new();
    let mut caps_req = BTreeSet::new();
    let mut attrs = FunctionAttrs::default();
    let mut saw_ctx = false;
    let mut saw_eff = false;
    let mut saw_caps = false;

    for attr in &item.attrs {
        let name = attr.name.to_ascii_lowercase();
        match name.as_str() {
            "ctx" => {
                saw_ctx = true;
                match parse_ctx_attr(attr) {
                    Ok(values) => ctx_ok.extend(values),
                    Err(msg) => errors.push(format!("{} for '{}'", msg, item.name)),
                }
            }
            "eff" => {
                saw_eff = true;
                match parse_eff_attr(attr) {
                    Ok(values) => eff_used.extend(values),
                    Err(msg) => errors.push(format!("{} for '{}'", msg, item.name)),
                }
            }
            "caps" => {
                saw_caps = true;
                if let Some(raw) = &attr.args {
                    caps_req.extend(split_csv(raw));
                }
            }
            "irq" => {
                saw_ctx = true;
                ctx_ok.clear();
                ctx_ok.insert(Ctx::Irq);
            }
            "noirq" => {
                saw_ctx = true;
                ctx_ok.clear();
                ctx_ok.insert(Ctx::Boot);
                ctx_ok.insert(Ctx::Thread);
            }
            "alloc" => {
                saw_eff = true;
                eff_used.insert(Eff::Alloc);
            }
            "block" => {
                saw_eff = true;
                eff_used.insert(Eff::Block);
            }
            "preempt_off" => {
                saw_eff = true;
                eff_used.insert(Eff::PreemptOff);
            }
            "noyield" => attrs.noyield = true,
            "critical" => attrs.critical = true,
            "leaf" => attrs.leaf = true,
            "hotpath" => attrs.hotpath = true,
            "lock_budget" => match parse_lock_budget(attr) {
                Ok(v) => attrs.lock_budget = Some(v),
                Err(msg) => errors.push(format!("{} for '{}'", msg, item.name)),
            },
            "module_caps" => {}
            other => {
                if let Some(feature) = adaptive_surface_feature(other) {
                    if !surface_profile_enables_feature(surface_profile, feature) {
                        errors.push(feature_unavailability_error(
                            surface_profile,
                            feature,
                            &item.name,
                        ));
                        continue;
                    }
                    match feature.lowering_rule {
                        AdaptiveLoweringRule::ContextAlias(ctxs) => {
                            saw_ctx = true;
                            ctx_ok.clear();
                            ctx_ok.extend(ctxs.iter().copied());
                        }
                        AdaptiveLoweringRule::EffectAlias(effs) => {
                            saw_eff = true;
                            eff_used.extend(effs.iter().copied());
                        }
                    }
                } else {
                    errors.push(format!(
                        "unknown attribute '@{}' on function '{}'",
                        other, item.name
                    ));
                }
            }
        }
    }

    if item.is_extern {
        if !saw_ctx {
            errors.push(format!(
                "extern '{}' must declare @ctx(...) facts explicitly",
                item.name
            ));
        }
        if !saw_eff {
            errors.push(format!(
                "extern '{}' must declare @eff(...) facts explicitly",
                item.name
            ));
        }
        if !saw_caps {
            errors.push(format!(
                "EXTERN_CAPS_CONTRACT_REQUIRED: extern '{}' must declare @caps(...) facts explicitly",
                item.name
            ));
        }
    } else {
        if !saw_ctx {
            // Default is conservative and excludes IRQ/NMI.
            ctx_ok.insert(Ctx::Boot);
            ctx_ok.insert(Ctx::Thread);
        }
        if !saw_eff {
            // Default is no declared effects.
        }
        if !saw_caps {
            // Default is no required capabilities.
        }
    }

    let mut ops = Vec::new();
    for stmt in &item.body {
        lower_stmt(stmt, &mut ops, &mut eff_used);
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(Function {
        name: item.name.clone(),
        is_extern: item.is_extern,
        ctx_ok: ctx_ok.into_iter().collect(),
        eff_used: eff_used.into_iter().collect(),
        caps_req: caps_req.into_iter().collect(),
        attrs,
        ops,
    })
}

fn lower_stmt(stmt: &Stmt, ops: &mut Vec<KrirOp>, eff_used: &mut BTreeSet<Eff>) {
    match stmt {
        Stmt::Call(callee) => ops.push(KrirOp::Call {
            callee: callee.clone(),
        }),
        Stmt::Critical(inner) => {
            ops.push(KrirOp::CriticalEnter);
            for stmt in inner {
                lower_stmt(stmt, ops, eff_used);
            }
            ops.push(KrirOp::CriticalExit);
        }
        Stmt::YieldPoint => {
            ops.push(KrirOp::YieldPoint);
            eff_used.insert(Eff::Yield);
        }
        Stmt::AllocPoint => {
            ops.push(KrirOp::AllocPoint);
            eff_used.insert(Eff::Alloc);
        }
        Stmt::BlockPoint => {
            ops.push(KrirOp::BlockPoint);
            eff_used.insert(Eff::Block);
        }
        Stmt::Acquire(lock_class) => ops.push(KrirOp::Acquire {
            lock_class: lock_class.clone(),
        }),
        Stmt::Release(lock_class) => ops.push(KrirOp::Release {
            lock_class: lock_class.clone(),
        }),
        Stmt::MmioRead => {
            ops.push(KrirOp::MmioRead);
            eff_used.insert(Eff::Mmio);
        }
        Stmt::MmioWrite => {
            ops.push(KrirOp::MmioWrite);
            eff_used.insert(Eff::Mmio);
        }
    }
}

fn parse_ctx_attr(attr: &RawAttr) -> Result<Vec<Ctx>, String> {
    let Some(args) = attr.args.as_deref() else {
        return Err("@ctx(...) requires a context list".to_string());
    };

    let mut out = Vec::new();
    for token in split_csv(args) {
        let ctx = match token.trim().to_ascii_lowercase().as_str() {
            "boot" => Ctx::Boot,
            "thread" => Ctx::Thread,
            "irq" => Ctx::Irq,
            "nmi" => Ctx::Nmi,
            _ => return Err(format!("unknown context '{}'", token)),
        };
        out.push(ctx);
    }
    Ok(out)
}

fn parse_eff_attr(attr: &RawAttr) -> Result<Vec<Eff>, String> {
    let Some(args) = attr.args.as_deref() else {
        return Err("@eff(...) requires an effect list".to_string());
    };

    let mut out = Vec::new();
    for token in split_csv(args) {
        let eff = match token.trim().to_ascii_lowercase().as_str() {
            "alloc" => Eff::Alloc,
            "block" => Eff::Block,
            "preempt_off" => Eff::PreemptOff,
            "ioport" => Eff::Ioport,
            "mmio" => Eff::Mmio,
            "dma_map" => Eff::DmaMap,
            "yield" => Eff::Yield,
            _ => return Err(format!("unknown effect '{}'", token)),
        };
        out.push(eff);
    }
    Ok(out)
}

fn parse_lock_budget(attr: &RawAttr) -> Result<u64, String> {
    let Some(args) = attr.args.as_deref() else {
        return Err("@lock_budget(N) requires a number".to_string());
    };
    let trimmed = args.trim();
    trimmed
        .parse::<u64>()
        .map_err(|_| format!("invalid lock budget '{}'", trimmed))
}

#[cfg(test)]
mod tests {
    use super::{
        AdaptiveFeatureStatus, SurfaceProfile, adaptive_feature_proposal,
        adaptive_feature_proposals, adaptive_surface_features, lower_to_krir,
        lower_to_krir_with_surface, surface_profile_enables_feature,
    };
    use parser::parse_module;
    use proptest::prelude::*;
    use serde_json::json;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn lower_to_krir_never_panics_on_random_parseable_input(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            let input = String::from_utf8_lossy(&bytes).to_string();
            let parsed = std::panic::catch_unwind(|| parse_module(&input));
            prop_assert!(parsed.is_ok());

            let parse_outcome = match parsed {
                Ok(value) => value,
                Err(_) => return Ok(()),
            };
            if let Ok(ast) = parse_outcome {
                let lowered = std::panic::catch_unwind(|| lower_to_krir(&ast));
                prop_assert!(lowered.is_ok());
            }
        }
    }

    #[test]
    fn irq_handler_alias_is_rejected_in_stable_surface() {
        let ast = parse_module("@irq_handler fn isr() { }").expect("parse");
        let errs = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable)
            .expect_err("stable surface must reject irq_handler");
        assert_eq!(
            errs,
            vec!["surface feature '@irq_handler' requires --surface experimental for 'isr'"]
        );
    }

    #[test]
    fn adaptive_surface_aliases_lower_identically_to_canonical_forms() {
        let cases = [
            ("@irq_handler fn isr() { }", "@ctx(irq) fn isr() { }"),
            (
                "@thread_entry fn worker() { }",
                "@ctx(thread) fn worker() { }",
            ),
            ("@may_block fn worker() { }", "@eff(block) fn worker() { }"),
        ];

        for (alias_src, canonical_src) in cases {
            let alias_ast = parse_module(alias_src).expect("parse alias");
            let canonical_ast = parse_module(canonical_src).expect("parse canonical");
            let alias = lower_to_krir_with_surface(&alias_ast, SurfaceProfile::Experimental)
                .expect("experimental alias lowering");
            let canonical =
                lower_to_krir_with_surface(&canonical_ast, SurfaceProfile::Stable).expect("lower");
            assert_eq!(alias, canonical, "alias '{}' drifted", alias_src);
        }
    }

    #[test]
    fn additional_adaptive_aliases_are_rejected_in_stable_surface() {
        let cases = [
            (
                "@thread_entry fn worker() { }",
                "surface feature '@thread_entry' requires --surface experimental for 'worker'",
            ),
            (
                "@may_block fn worker() { }",
                "surface feature '@may_block' requires --surface experimental for 'worker'",
            ),
        ];

        for (src, expected) in cases {
            let ast = parse_module(src).expect("parse");
            let errs = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable)
                .expect_err("stable surface must reject alias");
            assert_eq!(
                errs,
                vec![expected],
                "stable rejection drifted for '{}'",
                src
            );
        }
    }

    #[test]
    fn deprecated_adaptive_alias_is_rejected_in_all_profiles() {
        let ast = parse_module("@irq_legacy fn legacy_isr() { }").expect("parse");
        let stable_errs = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable)
            .expect_err("stable must reject deprecated alias");
        let experimental_errs = lower_to_krir_with_surface(&ast, SurfaceProfile::Experimental)
            .expect_err("experimental must reject deprecated alias");

        assert_eq!(
            stable_errs,
            vec![
                "surface feature '@irq_legacy' is deprecated and unavailable under --surface stable for 'legacy_isr'"
            ]
        );
        assert_eq!(
            experimental_errs,
            vec![
                "surface feature '@irq_legacy' is deprecated and unavailable under --surface experimental for 'legacy_isr'"
            ]
        );
    }

    #[test]
    fn adaptive_feature_registry_and_proposal_are_deterministic() {
        assert_eq!(
            serde_json::to_value(adaptive_surface_features()).expect("registry json"),
            json!([
                {
                    "id": "irq_handler_alias",
                    "proposal_id": "irq_handler_alias",
                    "surface_form": "irq_handler",
                    "status": "experimental",
                    "lowering_target": "@ctx(irq)",
                    "safety_notes": "Pure surface alias; lowers to the existing irq context declaration.",
                    "migration_supported": true,
                    "migration_note": "Replace with @ctx(irq) when pinning code back to stable.",
                    "surface_profile_gate": "experimental"
                },
                {
                    "id": "thread_entry_alias",
                    "proposal_id": "thread_entry_alias",
                    "surface_form": "thread_entry",
                    "status": "experimental",
                    "lowering_target": "@ctx(thread)",
                    "safety_notes": "Pure surface alias; lowers to the existing thread context declaration.",
                    "migration_supported": true,
                    "migration_note": "Replace with @ctx(thread) when pinning code back to stable.",
                    "surface_profile_gate": "experimental"
                },
                {
                    "id": "may_block_alias",
                    "proposal_id": "may_block_alias",
                    "surface_form": "may_block",
                    "status": "experimental",
                    "lowering_target": "@eff(block)",
                    "safety_notes": "Pure surface alias; lowers to the existing block effect declaration.",
                    "migration_supported": true,
                    "migration_note": "Replace with @eff(block) when pinning code back to stable.",
                    "surface_profile_gate": "experimental"
                },
                {
                    "id": "irq_legacy_alias",
                    "proposal_id": "irq_legacy_alias",
                    "surface_form": "irq_legacy",
                    "status": "deprecated",
                    "lowering_target": "@ctx(irq)",
                    "safety_notes": "Deprecated surface alias kept only to prove centralized lifecycle gating.",
                    "migration_supported": true,
                    "migration_note": "Replace with @ctx(irq) or @irq_handler depending on the chosen profile policy.",
                    "surface_profile_gate": "stable"
                }
            ])
        );

        assert_eq!(
            serde_json::to_value(adaptive_feature_proposals()).expect("proposal json"),
            json!([
                {
                    "id": "irq_handler_alias",
                    "title": "Experimental @irq_handler surface alias",
                    "motivation": "Provide a governed surface-only shorthand for irq-context entry points.",
                    "syntax_before": "@ctx(irq) fn isr() { }",
                    "syntax_after": "@irq_handler fn isr() { }",
                    "lowering_description": "Lower @irq_handler to the existing canonical @ctx(irq) representation during HIR lowering.",
                    "compatibility_risk": "Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
                    "migration_plan": "Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
                    "status": "experimental"
                },
                {
                    "id": "thread_entry_alias",
                    "title": "Experimental @thread_entry surface alias",
                    "motivation": "Provide a governed surface-only shorthand for thread-only entry points.",
                    "syntax_before": "@ctx(thread) fn worker() { }",
                    "syntax_after": "@thread_entry fn worker() { }",
                    "lowering_description": "Lower @thread_entry to the existing canonical @ctx(thread) representation during HIR lowering.",
                    "compatibility_risk": "Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
                    "migration_plan": "Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
                    "status": "experimental"
                },
                {
                    "id": "may_block_alias",
                    "title": "Experimental @may_block surface alias",
                    "motivation": "Provide a governed surface-only shorthand for declaring block effects.",
                    "syntax_before": "@eff(block) fn worker() { }",
                    "syntax_after": "@may_block fn worker() { }",
                    "lowering_description": "Lower @may_block to the existing canonical @eff(block) representation during HIR lowering.",
                    "compatibility_risk": "Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
                    "migration_plan": "Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
                    "status": "experimental"
                },
                {
                    "id": "irq_legacy_alias",
                    "title": "Deprecated @irq_legacy surface alias",
                    "motivation": "Preserve a historical alias only long enough to exercise deterministic lifecycle gating.",
                    "syntax_before": "@ctx(irq) fn legacy_isr() { }",
                    "syntax_after": "@irq_legacy fn legacy_isr() { }",
                    "lowering_description": "Would lower to the existing canonical @ctx(irq) representation if lifecycle policy allowed it.",
                    "compatibility_risk": "Medium; the alias is deprecated and intentionally unavailable under current surface profiles.",
                    "migration_plan": "Replace with @ctx(irq) or @irq_handler depending on the chosen profile policy.",
                    "status": "deprecated"
                }
            ])
        );
    }

    #[test]
    fn every_adaptive_feature_links_to_one_canonical_proposal() {
        for feature in adaptive_surface_features() {
            let proposal = adaptive_feature_proposal(feature.id)
                .unwrap_or_else(|| panic!("missing proposal for feature '{}'", feature.id));
            assert_eq!(proposal.id, feature.proposal_id);
        }
    }

    #[test]
    fn surface_profiles_resolve_feature_sets_centrally() {
        let features = adaptive_surface_features();
        assert!(
            features
                .iter()
                .all(|feature| !surface_profile_enables_feature(SurfaceProfile::Stable, feature)),
            "stable must not enable experimental-only features"
        );
        assert!(
            features
                .iter()
                .filter(|feature| matches!(feature.status, AdaptiveFeatureStatus::Experimental))
                .all(|feature| surface_profile_enables_feature(
                    SurfaceProfile::Experimental,
                    feature
                )),
            "experimental must enable experimental features"
        );
        assert!(
            features
                .iter()
                .filter(|feature| matches!(feature.status, AdaptiveFeatureStatus::Deprecated))
                .all(|feature| !surface_profile_enables_feature(
                    SurfaceProfile::Experimental,
                    feature
                )),
            "experimental must not enable deprecated features by default"
        );

        let stable_feature = super::AdaptiveSurfaceFeature {
            id: "stable_alias",
            proposal_id: "stable_alias",
            surface_form: "stable_alias",
            status: AdaptiveFeatureStatus::Stable,
            lowering_target: "@ctx(thread)",
            safety_notes: "test-only stable feature",
            migration_supported: true,
            migration_note: "none",
            surface_profile_gate: SurfaceProfile::Stable,
            lowering_rule: super::AdaptiveLoweringRule::ContextAlias(&[krir::Ctx::Thread]),
        };

        assert!(surface_profile_enables_feature(
            SurfaceProfile::Stable,
            &stable_feature
        ));
        assert!(surface_profile_enables_feature(
            SurfaceProfile::Experimental,
            &stable_feature
        ));
    }

    #[test]
    fn proposal_example_file_matches_serialized_proposal() {
        let cases = [
            (
                "irq_handler_alias",
                include_str!("../../../docs/design/examples/irq_handler_alias.proposal.json"),
            ),
            (
                "thread_entry_alias",
                include_str!("../../../docs/design/examples/thread_entry_alias.proposal.json"),
            ),
            (
                "may_block_alias",
                include_str!("../../../docs/design/examples/may_block_alias.proposal.json"),
            ),
            (
                "irq_legacy_alias",
                include_str!("../../../docs/design/examples/irq_legacy_alias.proposal.json"),
            ),
        ];

        for (feature_id, expected) in cases {
            let actual = serde_json::to_string_pretty(
                adaptive_feature_proposal(feature_id).expect("proposal"),
            )
            .expect("proposal");
            assert_eq!(
                actual.trim_end(),
                expected.trim_end(),
                "proposal file drifted"
            );
        }
    }
}
