use std::collections::{BTreeMap, BTreeSet};

use krir::{Ctx, Eff, Function, KrirModule, KrirOp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckError {
    pub pass: &'static str,
    pub message: String,
}

const PASS_LOCK: &str = "lockgraph";
const PASS_ANALYSIS: &str = "analysis";
const PASS_CRITICAL_REGION: &str = "critical-region";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoYieldSpan {
    Bounded(u64),
    Unbounded,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AnalysisReport {
    pub lock_edges: Vec<LockEdge>,
    pub max_lock_depth: u64,
    pub no_yield_spans: BTreeMap<String, NoYieldSpan>,
}

#[derive(Debug, Clone, Default)]
struct FnSummary {
    acquired_set: BTreeSet<String>,
    internal_edges: BTreeSet<LockEdge>,
    max_nested_acquires: u64,
    max_call_cost: u64,
    has_yield: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct EffectProvenance {
    direct: bool,
    via_callee: BTreeSet<String>,
    via_extern: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct FunctionEffectSemantics {
    transitive: BTreeSet<Eff>,
    provenance: BTreeMap<Eff, EffectProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct FunctionCapabilitySemantics {
    transitive: BTreeSet<String>,
    provenance: BTreeMap<String, EffectProvenance>,
}

#[derive(Debug, Clone)]
struct LockFrame {
    class: String,
    cost: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Temp,
    Perm,
}

pub fn run_checks(module: &KrirModule) -> Result<(), Vec<CheckError>> {
    let (_, errs) = analyze_module(module);
    if errs.is_empty() { Ok(()) } else { Err(errs) }
}

pub fn analyze_module(module: &KrirModule) -> (AnalysisReport, Vec<CheckError>) {
    let mut errs = Vec::new();
    errs.extend(ctx_check(module));
    errs.extend(effect_check(module));
    errs.extend(critical_alloc_boundary_check(module));
    errs.extend(cap_check(module));
    errs.extend(critical_region_balance_check(module));

    let fn_map = fn_map(module);
    let (summaries, summary_errs) = build_interproc_summaries(module, &fn_map);
    errs.extend(summary_errs);

    let mut report = if errs.is_empty() {
        build_report(module, &fn_map, &summaries)
    } else {
        AnalysisReport::default()
    };
    canonicalize_report(&mut report);
    canonicalize_errors(&mut errs);

    (report, errs)
}

fn ctx_check(module: &KrirModule) -> Vec<CheckError> {
    let mut errs = Vec::new();
    let map = fn_map(module);

    for edge in &module.call_edges {
        let Some(caller) = map.get(&edge.caller) else {
            continue;
        };
        let Some(callee) = map.get(&edge.callee) else {
            continue;
        };

        let caller_ctx: BTreeSet<_> = caller.ctx_ok.iter().cloned().collect();
        let callee_ctx: BTreeSet<_> = callee.ctx_ok.iter().cloned().collect();
        if !caller_ctx.is_subset(&callee_ctx) {
            errs.push(CheckError {
                pass: "ctx-check",
                message: format!(
                    "call '{} -> {}' violates ctx rule: caller ctx_ok must be subset of callee ctx_ok",
                    edge.caller, edge.callee
                ),
            });
        }
    }

    for f in &module.functions {
        let has_yield = f.ops.iter().any(|op| matches!(op, KrirOp::YieldPoint));
        if has_yield && (f.ctx_ok.contains(&Ctx::Irq) || f.ctx_ok.contains(&Ctx::Nmi)) {
            errs.push(CheckError {
                pass: "ctx-check",
                message: format!(
                    "function '{}' contains yieldpoint but is allowed in irq/nmi context",
                    f.name
                ),
            });
        }
    }

    errs
}

fn effect_check(module: &KrirModule) -> Vec<CheckError> {
    let mut errs = Vec::new();
    let map = fn_map(module);
    let effect_semantics = effect_semantics_by_function(module);

    for edge in &module.call_edges {
        let Some(caller) = map.get(&edge.caller) else {
            continue;
        };
        let Some(callee) = map.get(&edge.callee) else {
            continue;
        };

        for ctx in &caller.ctx_ok {
            let bad = callee
                .eff_used
                .iter()
                .filter(|eff| !matches!((*ctx, **eff), (Ctx::Irq, Eff::Block)))
                .filter(|eff| !is_effect_allowed(*ctx, **eff))
                .map(|eff| eff.as_str().to_string())
                .collect::<Vec<_>>();

            if !bad.is_empty() {
                errs.push(CheckError {
                    pass: "effect-check",
                    message: format!(
                        "call '{} -> {}' in ctx '{}' uses forbidden effects: {}",
                        edge.caller,
                        edge.callee,
                        ctx.as_str(),
                        bad.join(", ")
                    ),
                });
            }
        }
    }

    for f in &module.functions {
        if f.ctx_ok.contains(&Ctx::Irq)
            && let Some(semantics) = effect_semantics.get(&f.name)
            && semantics.transitive.contains(&Eff::Block)
            && let Some(provenance) = semantics.provenance.get(&Eff::Block)
        {
            errs.push(CheckError {
                pass: "ctx-check",
                message: format!(
                    "CTX_IRQ_BLOCK_BOUNDARY: function '{}' is @ctx(irq) and uses block effect ({})",
                    f.name,
                    format_effect_provenance(provenance)
                ),
            });
        }

        if f.attrs.noyield && f.ops.iter().any(|op| matches!(op, KrirOp::YieldPoint)) {
            errs.push(CheckError {
                pass: "effect-check",
                message: format!(
                    "function '{}' is @noyield but contains yieldpoint()",
                    f.name
                ),
            });
        }
    }

    errs
}

fn cap_check(module: &KrirModule) -> Vec<CheckError> {
    let mut errs = Vec::new();
    let module_caps: BTreeSet<_> = module.module_caps.iter().cloned().collect();
    let map = fn_map(module);
    let capability_semantics = capability_semantics_by_function(module);

    for f in &module.functions {
        let req: BTreeSet<_> = f.caps_req.iter().cloned().collect();
        if !req.is_subset(&module_caps) {
            let missing: Vec<_> = req.difference(&module_caps).cloned().collect();
            errs.push(CheckError {
                pass: "cap-check",
                message: format!(
                    "function '{}' requires unavailable caps: {}",
                    f.name,
                    missing.join(", ")
                ),
            });
        }
    }

    for edge in &module.call_edges {
        let Some(callee) = map.get(&edge.callee) else {
            continue;
        };
        let req: BTreeSet<_> = callee.caps_req.iter().cloned().collect();
        if !req.is_subset(&module_caps) {
            let missing: Vec<_> = req.difference(&module_caps).cloned().collect();
            errs.push(CheckError {
                pass: "cap-check",
                message: format!(
                    "call '{} -> {}' violates caps_avail=module_caps, missing: {}",
                    edge.caller,
                    edge.callee,
                    missing.join(", ")
                ),
            });
        }
    }

    for function in module.functions.iter().filter(|f| !f.is_extern) {
        let declared: BTreeSet<_> = function.caps_req.iter().cloned().collect();
        let Some(semantics) = capability_semantics.get(&function.name) else {
            continue;
        };
        for capability in &semantics.transitive {
            if !module_caps.contains(capability) {
                continue;
            }
            if declared.contains(capability) {
                continue;
            }
            let Some(provenance) = semantics.provenance.get(capability) else {
                continue;
            };
            errs.push(CheckError {
                pass: "cap-check",
                message: format!(
                    "CAPABILITY_BOUNDARY: function '{}' reaches capability '{}' without declaring @caps({}) ({})",
                    function.name,
                    capability,
                    capability,
                    format_effect_provenance(provenance)
                ),
            });
        }
    }

    errs
}

fn effect_semantics_by_function(module: &KrirModule) -> BTreeMap<String, FunctionEffectSemantics> {
    let names = module
        .functions
        .iter()
        .map(|f| f.name.clone())
        .collect::<Vec<_>>();
    let index_by_name = names
        .iter()
        .enumerate()
        .map(|(idx, name)| (name.clone(), idx))
        .collect::<BTreeMap<_, _>>();

    let mut direct = vec![BTreeSet::<Eff>::new(); names.len()];
    let mut is_extern = vec![false; names.len()];
    for function in &module.functions {
        if let Some(&idx) = index_by_name.get(&function.name) {
            direct[idx].extend(function.eff_used.iter().copied());
            is_extern[idx] = function.is_extern;
        }
    }

    let mut outgoing_sets = vec![BTreeSet::<usize>::new(); names.len()];
    for edge in &module.call_edges {
        let (Some(&caller), Some(&callee)) = (
            index_by_name.get(&edge.caller),
            index_by_name.get(&edge.callee),
        ) else {
            continue;
        };
        outgoing_sets[caller].insert(callee);
    }
    let outgoing = outgoing_sets
        .into_iter()
        .map(|set| set.into_iter().collect::<Vec<_>>())
        .collect::<Vec<_>>();

    let mut transitive = direct.clone();
    let mut provenance = vec![BTreeMap::<Eff, EffectProvenance>::new(); names.len()];
    for (idx, direct_effects) in direct.iter().enumerate() {
        for effect in direct_effects {
            provenance[idx].entry(*effect).or_default().direct = true;
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for caller_idx in 0..names.len() {
            for &callee_idx in &outgoing[caller_idx] {
                let callee_effects = transitive[callee_idx].iter().copied().collect::<Vec<_>>();
                for effect in callee_effects {
                    if transitive[caller_idx].insert(effect) {
                        changed = true;
                    }

                    let callee_provenance = provenance[callee_idx]
                        .get(&effect)
                        .cloned()
                        .unwrap_or_default();
                    let entry = provenance[caller_idx].entry(effect).or_default();
                    if is_extern[callee_idx] {
                        if entry.via_extern.insert(names[callee_idx].clone()) {
                            changed = true;
                        }
                    } else if entry.via_callee.insert(names[callee_idx].clone()) {
                        changed = true;
                    }

                    let before_callee = entry.via_callee.len();
                    entry.via_callee.extend(callee_provenance.via_callee);
                    if entry.via_callee.len() != before_callee {
                        changed = true;
                    }
                    let before_extern = entry.via_extern.len();
                    entry.via_extern.extend(callee_provenance.via_extern);
                    if entry.via_extern.len() != before_extern {
                        changed = true;
                    }
                }
            }
        }
    }

    let mut by_name = BTreeMap::<String, FunctionEffectSemantics>::new();
    for (idx, name) in names.iter().enumerate() {
        let mut effect_map = BTreeMap::<Eff, EffectProvenance>::new();
        for effect in &transitive[idx] {
            let mut entry = provenance[idx].get(effect).cloned().unwrap_or_default();
            entry.via_callee.remove(name);
            entry.via_extern.remove(name);
            effect_map.insert(*effect, entry);
        }
        by_name.insert(
            name.clone(),
            FunctionEffectSemantics {
                transitive: transitive[idx].clone(),
                provenance: effect_map,
            },
        );
    }

    by_name
}

fn capability_semantics_by_function(
    module: &KrirModule,
) -> BTreeMap<String, FunctionCapabilitySemantics> {
    let names = module
        .functions
        .iter()
        .map(|f| f.name.clone())
        .collect::<Vec<_>>();
    let index_by_name = names
        .iter()
        .enumerate()
        .map(|(idx, name)| (name.clone(), idx))
        .collect::<BTreeMap<_, _>>();

    let mut direct = vec![BTreeSet::<String>::new(); names.len()];
    let mut is_extern = vec![false; names.len()];
    for function in &module.functions {
        if let Some(&idx) = index_by_name.get(&function.name) {
            direct[idx].extend(function.caps_req.iter().cloned());
            is_extern[idx] = function.is_extern;
        }
    }

    let mut outgoing_sets = vec![BTreeSet::<usize>::new(); names.len()];
    for edge in &module.call_edges {
        let (Some(&caller), Some(&callee)) = (
            index_by_name.get(&edge.caller),
            index_by_name.get(&edge.callee),
        ) else {
            continue;
        };
        outgoing_sets[caller].insert(callee);
    }
    let outgoing = outgoing_sets
        .into_iter()
        .map(|set| set.into_iter().collect::<Vec<_>>())
        .collect::<Vec<_>>();

    let mut transitive = direct.clone();
    let mut provenance = vec![BTreeMap::<String, EffectProvenance>::new(); names.len()];
    for (idx, direct_caps) in direct.iter().enumerate() {
        for capability in direct_caps {
            provenance[idx]
                .entry(capability.clone())
                .or_default()
                .direct = true;
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for caller_idx in 0..names.len() {
            for &callee_idx in &outgoing[caller_idx] {
                let callee_caps = transitive[callee_idx].iter().cloned().collect::<Vec<_>>();
                for capability in callee_caps {
                    if transitive[caller_idx].insert(capability.clone()) {
                        changed = true;
                    }

                    let callee_provenance = provenance[callee_idx]
                        .get(&capability)
                        .cloned()
                        .unwrap_or_default();
                    let entry = provenance[caller_idx].entry(capability).or_default();
                    if is_extern[callee_idx] {
                        if entry.via_extern.insert(names[callee_idx].clone()) {
                            changed = true;
                        }
                    } else if entry.via_callee.insert(names[callee_idx].clone()) {
                        changed = true;
                    }

                    let before_callee = entry.via_callee.len();
                    entry.via_callee.extend(callee_provenance.via_callee);
                    if entry.via_callee.len() != before_callee {
                        changed = true;
                    }
                    let before_extern = entry.via_extern.len();
                    entry.via_extern.extend(callee_provenance.via_extern);
                    if entry.via_extern.len() != before_extern {
                        changed = true;
                    }
                }
            }
        }
    }

    let mut by_name = BTreeMap::<String, FunctionCapabilitySemantics>::new();
    for (idx, name) in names.iter().enumerate() {
        let mut cap_map = BTreeMap::<String, EffectProvenance>::new();
        for capability in &transitive[idx] {
            let mut entry = provenance[idx].get(capability).cloned().unwrap_or_default();
            entry.via_callee.remove(name);
            entry.via_extern.remove(name);
            cap_map.insert(capability.clone(), entry);
        }
        by_name.insert(
            name.clone(),
            FunctionCapabilitySemantics {
                transitive: transitive[idx].clone(),
                provenance: cap_map,
            },
        );
    }

    by_name
}

fn format_effect_provenance(provenance: &EffectProvenance) -> String {
    let via_callee = provenance.via_callee.iter().cloned().collect::<Vec<_>>();
    let via_extern = provenance.via_extern.iter().cloned().collect::<Vec<_>>();
    format!(
        "direct={}, via_callee=[{}], via_extern=[{}]",
        provenance.direct,
        via_callee.join(", "),
        via_extern.join(", ")
    )
}

fn critical_alloc_boundary_check(module: &KrirModule) -> Vec<CheckError> {
    let effect_semantics = effect_semantics_by_function(module);
    let map = fn_map(module);
    let mut errs = Vec::new();

    for function in module.functions.iter().filter(|f| !f.is_extern) {
        let mut depth = 0_u64;
        let mut provenance = EffectProvenance::default();

        for op in &function.ops {
            match op {
                KrirOp::CriticalEnter => depth += 1,
                KrirOp::CriticalExit => depth = depth.saturating_sub(1),
                KrirOp::AllocPoint if depth > 0 => {
                    provenance.direct = true;
                }
                KrirOp::Call { callee } if depth > 0 => {
                    let Some(callee_semantics) = effect_semantics.get(callee) else {
                        continue;
                    };
                    if !callee_semantics.transitive.contains(&Eff::Alloc) {
                        continue;
                    }
                    let Some(callee_provenance) = callee_semantics.provenance.get(&Eff::Alloc)
                    else {
                        continue;
                    };
                    let is_extern = map.get(callee).map(|f| f.is_extern).unwrap_or(false);
                    if is_extern {
                        provenance.via_extern.insert(callee.clone());
                    } else {
                        provenance.via_callee.insert(callee.clone());
                    }
                    provenance
                        .via_callee
                        .extend(callee_provenance.via_callee.iter().cloned());
                    provenance
                        .via_extern
                        .extend(callee_provenance.via_extern.iter().cloned());
                }
                _ => {}
            }
        }

        provenance.via_callee.remove(&function.name);
        provenance.via_extern.remove(&function.name);

        if provenance.direct
            || !provenance.via_callee.is_empty()
            || !provenance.via_extern.is_empty()
        {
            errs.push(CheckError {
                pass: PASS_CRITICAL_REGION,
                message: format!(
                    "CRITICAL_ALLOC_BOUNDARY: function '{}' uses alloc effect in critical region ({})",
                    function.name,
                    format_effect_provenance(&provenance)
                ),
            });
        }
    }

    errs
}

fn critical_region_balance_check(module: &KrirModule) -> Vec<CheckError> {
    let mut errs = Vec::new();

    for function in module.functions.iter().filter(|f| !f.is_extern) {
        let mut depth = 0_u64;
        for op in &function.ops {
            match op {
                KrirOp::CriticalEnter => depth += 1,
                KrirOp::CriticalExit => {
                    if depth == 0 {
                        errs.push(CheckError {
                            pass: PASS_CRITICAL_REGION,
                            message: format!(
                                "CRITICAL_REGION_UNBALANCED: function '{}' exits critical region without matching enter",
                                function.name
                            ),
                        });
                    } else {
                        depth -= 1;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            errs.push(CheckError {
                pass: PASS_CRITICAL_REGION,
                message: format!(
                    "CRITICAL_REGION_UNBALANCED: function '{}' has {} unterminated critical region(s)",
                    function.name, depth
                ),
            });
        }
    }

    errs
}

fn build_interproc_summaries(
    module: &KrirModule,
    fn_map: &BTreeMap<String, &Function>,
) -> (BTreeMap<String, FnSummary>, Vec<CheckError>) {
    let mut errs = Vec::new();
    let order = match topo_order_non_recursive(module, fn_map) {
        Ok(order) => order,
        Err(cycle_errs) => {
            errs.extend(cycle_errs);
            return (BTreeMap::new(), errs);
        }
    };

    let mut summaries = BTreeMap::<String, FnSummary>::new();
    for f in module.functions.iter().filter(|f| f.is_extern) {
        summaries.insert(
            f.name.clone(),
            FnSummary {
                has_yield: f.eff_used.contains(&Eff::Yield),
                ..FnSummary::default()
            },
        );
    }

    for fname in order {
        let Some(function) = fn_map.get(&fname) else {
            continue;
        };
        if function.is_extern {
            continue;
        }

        let mut summary = FnSummary::default();
        let mut held = Vec::<LockFrame>::new();

        for op in &function.ops {
            match op {
                KrirOp::Acquire { lock_class } => {
                    for frame in &held {
                        summary.internal_edges.insert(LockEdge {
                            from: frame.class.clone(),
                            to: lock_class.clone(),
                        });
                    }
                    held.push(LockFrame {
                        class: lock_class.clone(),
                        cost: 0,
                    });
                    summary.acquired_set.insert(lock_class.clone());
                    summary.max_nested_acquires =
                        summary.max_nested_acquires.max(held.len() as u64);
                }
                KrirOp::Release { lock_class } => match held.last() {
                    None => errs.push(CheckError {
                        pass: PASS_LOCK,
                        message: format!(
                            "function '{}' releases '{}' without matching acquire",
                            function.name, lock_class
                        ),
                    }),
                    Some(top) if top.class != *lock_class => errs.push(CheckError {
                        pass: PASS_LOCK,
                        message: format!(
                            "function '{}' release mismatch: expected '{}' on top, found '{}'",
                            function.name, top.class, lock_class
                        ),
                    }),
                    Some(_) => {
                        let frame = held.pop().expect("top exists");
                        if let Some(budget) = function.attrs.lock_budget
                            && frame.cost > budget
                        {
                            errs.push(CheckError {
                                pass: PASS_LOCK,
                                message: format!(
                                    "function '{}' exceeds @lock_budget({}) with cost {} in lock region '{}'",
                                    function.name, budget, frame.cost, frame.class
                                ),
                            });
                        }
                    }
                },
                KrirOp::YieldPoint => {
                    if !held.is_empty() {
                        let stack = held
                            .iter()
                            .map(|f| f.class.as_str())
                            .collect::<Vec<_>>()
                            .join(" -> ");
                        errs.push(CheckError {
                            pass: PASS_LOCK,
                            message: format!(
                                "function '{}' has yieldpoint under lock(s): {}",
                                function.name, stack
                            ),
                        });
                    }
                    summary.has_yield = true;
                }
                KrirOp::CriticalEnter | KrirOp::CriticalExit => {}
                KrirOp::AllocPoint | KrirOp::BlockPoint => {}
                KrirOp::Call { callee } => {
                    let Some(callee_fn) = fn_map.get(callee) else {
                        errs.push(CheckError {
                            pass: PASS_ANALYSIS,
                            message: format!(
                                "function '{}' calls unresolved symbol '{}' during lock/yield analysis",
                                function.name, callee
                            ),
                        });
                        continue;
                    };

                    let Some(callee_summary) = summaries.get(callee).cloned() else {
                        errs.push(CheckError {
                            pass: PASS_ANALYSIS,
                            message: format!(
                                "function '{}' calls '{}' but no summary is available",
                                function.name, callee
                            ),
                        });
                        continue;
                    };

                    if !held.is_empty() && callee_summary.has_yield {
                        let stack = held
                            .iter()
                            .map(|f| f.class.as_str())
                            .collect::<Vec<_>>()
                            .join(" -> ");
                        errs.push(CheckError {
                            pass: PASS_LOCK,
                            message: format!(
                                "function '{}' calls yielding callee '{}' under lock(s): {}",
                                function.name, callee, stack
                            ),
                        });
                        // This call edge is already invalid for lock/yield semantics.
                        // Skip secondary edge/cost updates to avoid compounding noise.
                        continue;
                    }

                    let call_cost = non_leaf_call_cost(callee_fn, &callee_summary);

                    summary.max_call_cost += call_cost;
                    if !held.is_empty() {
                        for frame in &mut held {
                            frame.cost += call_cost;
                        }
                    }

                    summary.has_yield |= callee_summary.has_yield;
                    summary.max_nested_acquires = summary.max_nested_acquires.max(
                        (held.len() as u64).saturating_add(callee_summary.max_nested_acquires),
                    );

                    for edge in &callee_summary.internal_edges {
                        summary.internal_edges.insert(edge.clone());
                    }
                    for lock in &callee_summary.acquired_set {
                        summary.acquired_set.insert(lock.clone());
                    }

                    for frame in &held {
                        for lock in &callee_summary.acquired_set {
                            summary.internal_edges.insert(LockEdge {
                                from: frame.class.clone(),
                                to: lock.clone(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        if !held.is_empty() {
            errs.push(CheckError {
                pass: PASS_LOCK,
                message: format!(
                    "function '{}' ends with {} unreleased lock(s)",
                    function.name,
                    held.len()
                ),
            });
        }

        summaries.insert(function.name.clone(), summary);
    }

    (summaries, errs)
}

fn build_report(
    module: &KrirModule,
    fn_map: &BTreeMap<String, &Function>,
    summaries: &BTreeMap<String, FnSummary>,
) -> AnalysisReport {
    let mut edges = BTreeSet::<LockEdge>::new();
    let mut max_depth = 0_u64;

    for summary in summaries.values() {
        max_depth = max_depth.max(summary.max_nested_acquires);
        for edge in &summary.internal_edges {
            edges.insert(edge.clone());
        }
    }

    let mut spans = BTreeMap::<String, NoYieldSpan>::new();
    for function in module.functions.iter().filter(|f| !f.is_extern) {
        if !function.ctx_ok.contains(&Ctx::Thread) {
            continue;
        }

        let summary = summaries.get(&function.name).cloned().unwrap_or_default();
        if !summary.has_yield {
            spans.insert(function.name.clone(), NoYieldSpan::Unbounded);
            continue;
        }

        let mut span = 0_u64;
        let mut span_max = 0_u64;
        for op in &function.ops {
            match op {
                KrirOp::YieldPoint => {
                    span_max = span_max.max(span);
                    span = 0;
                }
                KrirOp::Call { callee } => {
                    let Some(callee_fn) = fn_map.get(callee) else {
                        debug_assert!(false, "unresolved callee '{}' reached build_report", callee);
                        // HIR should prevent this; if it slips through, be conservative.
                        span += 1;
                        span_max = span_max.max(span);
                        continue;
                    };
                    let Some(callee_summary) = summaries.get(callee).cloned() else {
                        debug_assert!(
                            false,
                            "missing summary for callee '{}' reached build_report",
                            callee
                        );
                        span += 1;
                        span_max = span_max.max(span);
                        continue;
                    };
                    let call_cost = non_leaf_call_cost(callee_fn, &callee_summary);
                    span += call_cost;
                    span_max = span_max.max(span);
                }
                _ => {}
            }
        }
        span_max = span_max.max(span);
        spans.insert(function.name.clone(), NoYieldSpan::Bounded(span_max));
    }

    AnalysisReport {
        lock_edges: edges.into_iter().collect(),
        max_lock_depth: max_depth,
        no_yield_spans: spans,
    }
}

fn canonicalize_report(report: &mut AnalysisReport) {
    report
        .lock_edges
        .sort_by(|a, b| (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str())));
    report.lock_edges.dedup();
    report.no_yield_spans = report
        .no_yield_spans
        .iter()
        .map(|(name, span)| (name.clone(), span.clone()))
        .collect();
}

fn canonicalize_errors(errs: &mut [CheckError]) {
    errs.sort_by(|a, b| (a.pass, a.message.as_str()).cmp(&(b.pass, b.message.as_str())));
}

fn topo_order_non_recursive(
    module: &KrirModule,
    fn_map: &BTreeMap<String, &Function>,
) -> Result<Vec<String>, Vec<CheckError>> {
    let mut state = BTreeMap::<String, VisitState>::new();
    let mut stack = Vec::<String>::new();
    let mut order = Vec::<String>::new();
    let mut errs = Vec::<CheckError>::new();

    for function in module.functions.iter().filter(|f| !f.is_extern) {
        dfs_visit(
            &function.name,
            fn_map,
            &mut state,
            &mut stack,
            &mut order,
            &mut errs,
        );
    }

    if errs.is_empty() {
        Ok(order)
    } else {
        Err(errs)
    }
}

fn dfs_visit(
    name: &str,
    fn_map: &BTreeMap<String, &Function>,
    state: &mut BTreeMap<String, VisitState>,
    stack: &mut Vec<String>,
    order: &mut Vec<String>,
    errs: &mut Vec<CheckError>,
) {
    match state.get(name) {
        Some(VisitState::Perm) => return,
        Some(VisitState::Temp) => {
            let pos = stack.iter().position(|n| n == name).unwrap_or(0);
            let mut cycle = stack[pos..].to_vec();
            cycle.push(name.to_string());
            errs.push(CheckError {
                pass: PASS_ANALYSIS,
                message: format!("recursion unsupported in KR0.1: {}", cycle.join(" -> ")),
            });
            return;
        }
        None => {}
    }

    state.insert(name.to_string(), VisitState::Temp);
    stack.push(name.to_string());

    if let Some(function) = fn_map.get(name) {
        for callee in called_non_extern(function, fn_map) {
            dfs_visit(&callee, fn_map, state, stack, order, errs);
        }
    }

    stack.pop();
    state.insert(name.to_string(), VisitState::Perm);
    order.push(name.to_string());
}

fn called_non_extern(function: &Function, fn_map: &BTreeMap<String, &Function>) -> Vec<String> {
    let mut out = Vec::new();
    for op in &function.ops {
        if let KrirOp::Call { callee } = op
            && let Some(target) = fn_map.get(callee)
            && !target.is_extern
        {
            out.push(callee.clone());
        }
    }
    out
}

fn fn_map(module: &KrirModule) -> BTreeMap<String, &Function> {
    module
        .functions
        .iter()
        .map(|f| (f.name.clone(), f))
        .collect::<BTreeMap<_, _>>()
}

fn non_leaf_call_cost(callee: &Function, callee_summary: &FnSummary) -> u64 {
    if !callee.is_extern && callee.attrs.leaf {
        0
    } else {
        1_u64.saturating_add(callee_summary.max_call_cost)
    }
}

const ALLOWED_BOOT_THREAD: [Eff; 7] = [
    Eff::Alloc,
    Eff::Block,
    Eff::DmaMap,
    Eff::Ioport,
    Eff::Mmio,
    Eff::PreemptOff,
    Eff::Yield,
];
const ALLOWED_IRQ: [Eff; 4] = [Eff::DmaMap, Eff::Ioport, Eff::Mmio, Eff::PreemptOff];
const ALLOWED_NMI: [Eff; 2] = [Eff::Ioport, Eff::PreemptOff];

fn is_effect_allowed(ctx: Ctx, eff: Eff) -> bool {
    match ctx {
        Ctx::Boot | Ctx::Thread => ALLOWED_BOOT_THREAD.contains(&eff),
        Ctx::Irq => ALLOWED_IRQ.contains(&eff),
        Ctx::Nmi => ALLOWED_NMI.contains(&eff),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use krir::{FunctionAttrs, KrirModule};

    fn test_function(
        name: &str,
        is_extern: bool,
        eff_used: Vec<Eff>,
        ops: Vec<KrirOp>,
    ) -> Function {
        Function {
            name: name.to_string(),
            is_extern,
            ctx_ok: vec![Ctx::Thread],
            eff_used,
            caps_req: Vec::new(),
            attrs: FunctionAttrs::default(),
            ops,
        }
    }

    fn test_module(functions: Vec<Function>) -> KrirModule {
        KrirModule {
            module_caps: Vec::new(),
            functions,
            call_edges: Vec::new(),
        }
    }

    #[test]
    fn recursion_rejection_is_labeled_analysis() {
        let module = test_module(vec![
            test_function(
                "a",
                false,
                Vec::new(),
                vec![KrirOp::Call {
                    callee: "b".to_string(),
                }],
            ),
            test_function(
                "b",
                false,
                Vec::new(),
                vec![KrirOp::Call {
                    callee: "a".to_string(),
                }],
            ),
        ]);
        let map = fn_map(&module);
        let (_, errs) = build_interproc_summaries(&module, &map);
        assert!(
            errs.iter().any(|e| {
                e.pass == PASS_ANALYSIS
                    && e.message
                        .starts_with("recursion unsupported in KR0.1: a -> b -> a")
            }),
            "expected analysis recursion error, got {:?}",
            errs
        );
    }

    #[test]
    fn unresolved_callee_reports_analysis_error() {
        let module = test_module(vec![test_function(
            "outer",
            false,
            Vec::new(),
            vec![KrirOp::Call {
                callee: "missing".to_string(),
            }],
        )]);
        let map = fn_map(&module);
        let (_, errs) = build_interproc_summaries(&module, &map);
        assert!(
            errs.iter().any(|e| {
                e.pass == PASS_ANALYSIS
                    && e.message
                        == "function 'outer' calls unresolved symbol 'missing' during lock/yield analysis"
            }),
            "expected unresolved-callee analysis error, got {:?}",
            errs
        );
    }

    #[test]
    fn unbalanced_critical_region_reports_deterministic_error_code() {
        let module = test_module(vec![test_function(
            "entry",
            false,
            Vec::new(),
            vec![KrirOp::CriticalExit],
        )]);
        let (_, errs) = analyze_module(&module);
        assert!(
            errs.iter().any(|e| {
                e.pass == PASS_CRITICAL_REGION
                    && e.message
                        == "CRITICAL_REGION_UNBALANCED: function 'entry' exits critical region without matching enter"
            }),
            "expected CRITICAL_REGION_UNBALANCED error, got {:?}",
            errs
        );
    }

    #[test]
    fn extern_yield_effect_is_seeded_into_summary_and_rejected_under_lock() {
        let module = test_module(vec![
            test_function("maybe_yield", true, vec![Eff::Yield], Vec::new()),
            test_function(
                "outer",
                false,
                Vec::new(),
                vec![
                    KrirOp::Acquire {
                        lock_class: "SchedLock".to_string(),
                    },
                    KrirOp::Call {
                        callee: "maybe_yield".to_string(),
                    },
                    KrirOp::Release {
                        lock_class: "SchedLock".to_string(),
                    },
                ],
            ),
        ]);
        let map = fn_map(&module);
        let (summaries, errs) = build_interproc_summaries(&module, &map);

        assert!(
            summaries
                .get("maybe_yield")
                .expect("extern summary")
                .has_yield,
            "extern summary should inherit has_yield from @eff(yield)"
        );
        assert!(
            errs.iter().any(|e| {
                e.pass == PASS_LOCK
                    && e.message
                        == "function 'outer' calls yielding callee 'maybe_yield' under lock(s): SchedLock"
            }),
            "expected lockgraph yielding-callee-under-lock error, got {:?}",
            errs
        );
    }

    #[test]
    fn yield_propagates_across_two_call_levels() {
        let module = test_module(vec![
            test_function("inner", false, vec![Eff::Yield], vec![KrirOp::YieldPoint]),
            test_function(
                "mid",
                false,
                Vec::new(),
                vec![KrirOp::Call {
                    callee: "inner".to_string(),
                }],
            ),
            test_function(
                "outer",
                false,
                Vec::new(),
                vec![KrirOp::Call {
                    callee: "mid".to_string(),
                }],
            ),
        ]);
        let map = fn_map(&module);
        let (summaries, errs) = build_interproc_summaries(&module, &map);
        assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
        assert!(summaries.get("inner").expect("inner summary").has_yield);
        assert!(summaries.get("mid").expect("mid summary").has_yield);
        assert!(summaries.get("outer").expect("outer summary").has_yield);
    }

    #[test]
    fn canonicalize_report_sorts_and_dedups_lock_edges() {
        let mut report = AnalysisReport {
            lock_edges: vec![
                LockEdge {
                    from: "SchedLock".to_string(),
                    to: "RunQueueLock".to_string(),
                },
                LockEdge {
                    from: "ConsoleLock".to_string(),
                    to: "SchedLock".to_string(),
                },
                LockEdge {
                    from: "ConsoleLock".to_string(),
                    to: "SchedLock".to_string(),
                },
            ],
            max_lock_depth: 0,
            no_yield_spans: BTreeMap::new(),
        };

        canonicalize_report(&mut report);

        assert_eq!(
            report.lock_edges,
            vec![
                LockEdge {
                    from: "ConsoleLock".to_string(),
                    to: "SchedLock".to_string(),
                },
                LockEdge {
                    from: "SchedLock".to_string(),
                    to: "RunQueueLock".to_string(),
                },
            ]
        );
    }

    #[test]
    fn canonicalize_report_keeps_no_yield_spans_in_sorted_key_order() {
        let mut report = AnalysisReport {
            lock_edges: Vec::new(),
            max_lock_depth: 0,
            no_yield_spans: {
                let mut spans = BTreeMap::new();
                spans.insert("worker".to_string(), NoYieldSpan::Unbounded);
                spans.insert("alpha".to_string(), NoYieldSpan::Bounded(2));
                spans
            },
        };

        canonicalize_report(&mut report);

        let keys = report.no_yield_spans.keys().cloned().collect::<Vec<_>>();
        assert_eq!(keys, vec!["alpha".to_string(), "worker".to_string()]);
    }

    #[test]
    fn canonicalize_errors_sorts_by_pass_then_message() {
        let mut errs = vec![
            CheckError {
                pass: "lockgraph",
                message: "z message".to_string(),
            },
            CheckError {
                pass: "analysis",
                message: "b message".to_string(),
            },
            CheckError {
                pass: "analysis",
                message: "a message".to_string(),
            },
        ];

        canonicalize_errors(&mut errs);

        assert_eq!(
            errs,
            vec![
                CheckError {
                    pass: "analysis",
                    message: "a message".to_string(),
                },
                CheckError {
                    pass: "analysis",
                    message: "b message".to_string(),
                },
                CheckError {
                    pass: "lockgraph",
                    message: "z message".to_string(),
                },
            ]
        );
    }
}
