use std::collections::{BTreeMap, BTreeSet, VecDeque};

use krir::{Ctx, Eff, KrirModule, KrirOp};
use passes::{AnalysisReport, NoYieldSpan};
use serde::Serialize;
use serde_json::{Map, Value};

const CONTRACTS_SCHEMA_VERSION_V1: &str = "kernrift_contracts_v1";
const CONTRACTS_SCHEMA_VERSION_V2: &str = "kernrift_contracts_v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractsSchema {
    V1,
    V2,
}

impl ContractsSchema {
    fn version_str(self) -> &'static str {
        match self {
            Self::V1 => CONTRACTS_SCHEMA_VERSION_V1,
            Self::V2 => CONTRACTS_SCHEMA_VERSION_V2,
        }
    }
}

pub fn emit_krir_json(module: &KrirModule) -> Result<String, serde_json::Error> {
    let mut canonical = module.clone();
    canonical.canonicalize();
    serde_json::to_string_pretty(&canonical)
}

#[derive(Debug, Clone, Serialize)]
struct CapsSymbol {
    name: String,
    caps_req: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CapsManifest {
    module_caps: Vec<String>,
    symbols: Vec<CapsSymbol>,
}

pub fn emit_caps_manifest_json(module: &KrirModule) -> Result<String, serde_json::Error> {
    let mut module_caps = module.module_caps.clone();
    module_caps.sort();
    module_caps.dedup();

    let mut symbols = module
        .functions
        .iter()
        .map(|f| {
            let mut caps = f.caps_req.clone();
            caps.sort();
            caps.dedup();
            CapsSymbol {
                name: f.name.clone(),
                caps_req: caps,
            }
        })
        .collect::<Vec<_>>();
    symbols.sort_by(|a, b| a.name.cmp(&b.name));

    let manifest = CapsManifest {
        module_caps,
        symbols,
    };
    serde_json::to_string_pretty(&manifest)
}

#[derive(Debug, Clone, Serialize)]
struct LockEdgeJson {
    from: String,
    to: String,
}

#[derive(Debug, Clone, Serialize)]
struct LockgraphJson {
    edges: Vec<LockEdgeJson>,
    max_lock_depth: u64,
}

pub fn emit_lockgraph_json(report: &AnalysisReport) -> Result<String, serde_json::Error> {
    let mut edges = report
        .lock_edges
        .iter()
        .map(|e| LockEdgeJson {
            from: e.from.clone(),
            to: e.to.clone(),
        })
        .collect::<Vec<_>>();
    edges.sort_by(|a, b| (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str())));

    let payload = LockgraphJson {
        edges,
        max_lock_depth: report.max_lock_depth,
    };
    serde_json::to_string_pretty(&payload)
}

pub fn emit_report_json(report: &AnalysisReport, metrics: &[String]) -> Result<String, String> {
    let mut root = Map::new();
    let requested = metrics.iter().map(|m| m.as_str()).collect::<BTreeSet<_>>();
    for metric in requested {
        match metric {
            "max_lock_depth" => {
                root.insert(
                    "max_lock_depth".to_string(),
                    Value::Number(report.max_lock_depth.into()),
                );
            }
            "no_yield_spans" => {
                let spans = no_yield_json(&report.no_yield_spans);
                root.insert("no_yield_spans".to_string(), Value::Object(spans));
            }
            other => {
                return Err(format!("unsupported report metric '{}'", other));
            }
        }
    }
    serde_json::to_string_pretty(&Value::Object(root))
        .map_err(|e| format!("failed to serialize report JSON: {}", e))
}

pub fn emit_contracts_json(module: &KrirModule, report: &AnalysisReport) -> Result<String, String> {
    emit_contracts_json_with_schema(module, report, ContractsSchema::V1)
}

pub fn emit_contracts_json_with_schema(
    module: &KrirModule,
    report: &AnalysisReport,
    schema: ContractsSchema,
) -> Result<String, String> {
    let value = contracts_value(module, report, schema)?;
    serde_json::to_string_pretty(&value)
        .map_err(|e| format!("failed to serialize contracts JSON: {}", e))
}

pub fn emit_contracts_json_canonical(
    module: &KrirModule,
    report: &AnalysisReport,
) -> Result<String, String> {
    emit_contracts_json_canonical_with_schema(module, report, ContractsSchema::V1)
}

pub fn emit_contracts_json_canonical_with_schema(
    module: &KrirModule,
    report: &AnalysisReport,
    schema: ContractsSchema,
) -> Result<String, String> {
    let value = contracts_value(module, report, schema)?;
    serde_json::to_string(&value).map_err(|e| format!("failed to serialize contracts JSON: {}", e))
}

fn contracts_value(
    module: &KrirModule,
    report: &AnalysisReport,
    schema: ContractsSchema,
) -> Result<Value, String> {
    let mut canonical = module.clone();
    canonical.canonicalize();

    let caps_text = emit_caps_manifest_json(&canonical)
        .map_err(|e| format!("failed to serialize caps manifest JSON: {}", e))?;
    let lockgraph_text = emit_lockgraph_json(report)
        .map_err(|e| format!("failed to serialize lockgraph JSON: {}", e))?;
    let effect_semantics = if schema == ContractsSchema::V2 {
        Some(effect_semantics_by_function(&canonical))
    } else {
        None
    };
    let capability_semantics = if schema == ContractsSchema::V2 {
        Some(capability_semantics_by_function(&canonical))
    } else {
        None
    };
    let ctx_reachable = if schema == ContractsSchema::V2 {
        Some(ctx_reachable_by_function(&canonical))
    } else {
        None
    };
    let report_value = match schema {
        ContractsSchema::V1 => {
            let report_text = emit_report_json(
                report,
                &["max_lock_depth".to_string(), "no_yield_spans".to_string()],
            )?;
            serde_json::from_str(&report_text)
                .map_err(|e| format!("failed to parse generated report JSON: {}", e))?
        }
        ContractsSchema::V2 => report_v2_value(
            &canonical,
            report,
            effect_semantics
                .as_ref()
                .expect("v2 effect semantics must exist"),
        ),
    };

    let caps_value: Value = serde_json::from_str(&caps_text)
        .map_err(|e| format!("failed to parse generated caps manifest JSON: {}", e))?;
    let lockgraph_value: Value = serde_json::from_str(&lockgraph_text)
        .map_err(|e| format!("failed to parse generated lockgraph JSON: {}", e))?;

    let mut root = Map::new();
    root.insert(
        "schema_version".to_string(),
        Value::String(schema.version_str().to_string()),
    );
    root.insert("capabilities".to_string(), caps_value);
    root.insert(
        "facts".to_string(),
        facts_manifest_value(
            &canonical,
            schema,
            effect_semantics.as_ref(),
            capability_semantics.as_ref(),
            ctx_reachable.as_ref(),
        ),
    );
    root.insert("lockgraph".to_string(), lockgraph_value);
    root.insert("report".to_string(), report_value);

    Ok(Value::Object(root))
}

fn facts_manifest_value(
    module: &KrirModule,
    schema: ContractsSchema,
    effect_semantics: Option<&BTreeMap<String, FunctionEffectSemantics>>,
    capability_semantics: Option<&BTreeMap<String, FunctionCapabilitySemantics>>,
    ctx_reachable: Option<&BTreeMap<String, BTreeSet<Ctx>>>,
) -> Value {
    let symbols = module
        .functions
        .iter()
        .map(|f| {
            let mut attrs = Map::new();
            attrs.insert("noyield".to_string(), Value::Bool(f.attrs.noyield));
            if schema == ContractsSchema::V2 {
                attrs.insert("critical".to_string(), Value::Bool(f.attrs.critical));
            }
            attrs.insert("leaf".to_string(), Value::Bool(f.attrs.leaf));
            attrs.insert("hotpath".to_string(), Value::Bool(f.attrs.hotpath));
            attrs.insert(
                "lock_budget".to_string(),
                match f.attrs.lock_budget {
                    Some(v) => Value::Number(v.into()),
                    None => Value::Null,
                },
            );

            let mut symbol = Map::new();
            symbol.insert("name".to_string(), Value::String(f.name.clone()));
            symbol.insert("is_extern".to_string(), Value::Bool(f.is_extern));
            symbol.insert(
                "ctx_ok".to_string(),
                Value::Array(
                    f.ctx_ok
                        .iter()
                        .map(|ctx| Value::String(ctx.as_str().to_string()))
                        .collect(),
                ),
            );
            if let Some(ctx_map) = ctx_reachable {
                let mut ctxs = ctx_map
                    .get(&f.name)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|ctx| ctx.as_str().to_string())
                    .collect::<Vec<_>>();
                ctxs.sort();
                ctxs.dedup();
                symbol.insert(
                    "ctx_reachable".to_string(),
                    Value::Array(ctxs.into_iter().map(Value::String).collect()),
                );
            }
            symbol.insert(
                "eff_used".to_string(),
                Value::Array(
                    f.eff_used
                        .iter()
                        .map(|eff| Value::String(eff.as_str().to_string()))
                        .collect(),
                ),
            );
            if let Some(eff_map) = effect_semantics {
                let semantics = eff_map.get(&f.name).cloned().unwrap_or_default();
                let mut effs = semantics
                    .transitive
                    .iter()
                    .map(|eff| eff.as_str().to_string())
                    .collect::<Vec<_>>();
                effs.sort();
                effs.dedup();
                symbol.insert(
                    "eff_transitive".to_string(),
                    Value::Array(effs.into_iter().map(Value::String).collect()),
                );
                let entries = semantics
                    .provenance
                    .iter()
                    .map(|(effect, provenance)| {
                        let mut entry = Map::new();
                        entry.insert(
                            "effect".to_string(),
                            Value::String(effect.as_str().to_string()),
                        );
                        entry.insert(
                            "provenance".to_string(),
                            effect_provenance_value(provenance),
                        );
                        Value::Object(entry)
                    })
                    .collect::<Vec<_>>();
                symbol.insert("eff_provenance".to_string(), Value::Array(entries));
            }
            symbol.insert(
                "caps_req".to_string(),
                Value::Array(f.caps_req.iter().cloned().map(Value::String).collect()),
            );
            if let Some(caps_map) = capability_semantics {
                let semantics = caps_map.get(&f.name).cloned().unwrap_or_default();
                symbol.insert(
                    "caps_transitive".to_string(),
                    Value::Array(
                        semantics
                            .transitive
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                );
                let entries = semantics
                    .provenance
                    .iter()
                    .map(|(capability, provenance)| {
                        let mut entry = Map::new();
                        entry.insert("capability".to_string(), Value::String(capability.clone()));
                        entry.insert(
                            "provenance".to_string(),
                            effect_provenance_value(provenance),
                        );
                        Value::Object(entry)
                    })
                    .collect::<Vec<_>>();
                symbol.insert("caps_provenance".to_string(), Value::Array(entries));
            }
            symbol.insert("attrs".to_string(), Value::Object(attrs));
            Value::Object(symbol)
        })
        .collect::<Vec<_>>();

    let mut root = Map::new();
    root.insert("symbols".to_string(), Value::Array(symbols));
    Value::Object(root)
}

fn no_yield_json(spans: &BTreeMap<String, NoYieldSpan>) -> Map<String, Value> {
    let mut out = Map::new();
    for (name, span) in spans {
        let value = match span {
            NoYieldSpan::Bounded(v) => Value::Number((*v).into()),
            NoYieldSpan::Unbounded => Value::String("unbounded".to_string()),
        };
        out.insert(name.clone(), value);
    }
    out
}

fn report_v2_value(
    module: &KrirModule,
    report: &AnalysisReport,
    effect_semantics: &BTreeMap<String, FunctionEffectSemantics>,
) -> Value {
    let yield_sites_count = module
        .functions
        .iter()
        .filter(|f| !f.is_extern)
        .flat_map(|f| f.ops.iter())
        .filter(|op| matches!(op, KrirOp::YieldPoint))
        .count() as u64;
    let alloc_sites_count = module
        .functions
        .iter()
        .filter(|f| !f.is_extern)
        .flat_map(|f| f.ops.iter())
        .filter(|op| matches!(op, KrirOp::AllocPoint))
        .count() as u64;
    let block_sites_count = module
        .functions
        .iter()
        .filter(|f| !f.is_extern)
        .flat_map(|f| f.ops.iter())
        .filter(|op| matches!(op, KrirOp::BlockPoint))
        .count() as u64;

    let mut effects = Map::new();
    effects.insert(
        "yield_sites_count".to_string(),
        Value::Number(yield_sites_count.into()),
    );
    effects.insert(
        "alloc_sites_count".to_string(),
        Value::Number(alloc_sites_count.into()),
    );
    effects.insert(
        "block_sites_count".to_string(),
        Value::Number(block_sites_count.into()),
    );

    let (critical_depth_max, critical_violations) =
        critical_region_findings(module, effect_semantics);
    let mut critical = Map::new();
    critical.insert(
        "depth_max".to_string(),
        Value::Number(critical_depth_max.into()),
    );
    critical.insert(
        "violations".to_string(),
        Value::Array(
            critical_violations
                .into_iter()
                .map(|v| {
                    let mut obj = Map::new();
                    obj.insert("function".to_string(), Value::String(v.function));
                    obj.insert("effect".to_string(), Value::String(v.effect));
                    obj.insert(
                        "provenance".to_string(),
                        effect_provenance_value(&v.provenance),
                    );
                    Value::Object(obj)
                })
                .collect(),
        ),
    );

    let mut report_obj = Map::new();
    report_obj.insert(
        "max_lock_depth".to_string(),
        Value::Number(report.max_lock_depth.into()),
    );
    report_obj.insert(
        "no_yield_spans".to_string(),
        Value::Object(no_yield_json(&report.no_yield_spans)),
    );
    report_obj.insert("effects".to_string(), Value::Object(effects));
    report_obj.insert("critical".to_string(), Value::Object(critical));
    Value::Object(report_obj)
}

fn effect_provenance_value(provenance: &EffectProvenance) -> Value {
    let mut value = Map::new();
    value.insert("direct".to_string(), Value::Bool(provenance.direct));
    value.insert(
        "via_callee".to_string(),
        Value::Array(
            provenance
                .via_callee
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    value.insert(
        "via_extern".to_string(),
        Value::Array(
            provenance
                .via_extern
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    Value::Object(value)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CriticalViolation {
    function: String,
    effect: String,
    provenance: EffectProvenance,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
struct EffectProvenance {
    direct: bool,
    via_callee: BTreeSet<String>,
    via_extern: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FunctionEffectSemantics {
    transitive: BTreeSet<Eff>,
    provenance: BTreeMap<Eff, EffectProvenance>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FunctionCapabilitySemantics {
    transitive: BTreeSet<String>,
    provenance: BTreeMap<String, EffectProvenance>,
}

fn critical_region_findings(
    module: &KrirModule,
    effect_semantics: &BTreeMap<String, FunctionEffectSemantics>,
) -> (u64, Vec<CriticalViolation>) {
    let mut depth_max = 0_u64;
    let mut violations = BTreeSet::<CriticalViolation>::new();

    for function in module.functions.iter().filter(|f| !f.is_extern) {
        let mut depth = 0_u64;
        let mut fn_max = 0_u64;
        for op in &function.ops {
            match op {
                KrirOp::CriticalEnter => {
                    depth += 1;
                    fn_max = fn_max.max(depth);
                }
                KrirOp::CriticalExit => {
                    depth = depth.saturating_sub(1);
                }
                KrirOp::YieldPoint if depth > 0 => {
                    violations.insert(CriticalViolation {
                        function: function.name.clone(),
                        effect: Eff::Yield.as_str().to_string(),
                        provenance: EffectProvenance {
                            direct: true,
                            ..EffectProvenance::default()
                        },
                    });
                }
                KrirOp::AllocPoint if depth > 0 => {
                    violations.insert(CriticalViolation {
                        function: function.name.clone(),
                        effect: Eff::Alloc.as_str().to_string(),
                        provenance: EffectProvenance {
                            direct: true,
                            ..EffectProvenance::default()
                        },
                    });
                }
                KrirOp::BlockPoint if depth > 0 => {
                    violations.insert(CriticalViolation {
                        function: function.name.clone(),
                        effect: Eff::Block.as_str().to_string(),
                        provenance: EffectProvenance {
                            direct: true,
                            ..EffectProvenance::default()
                        },
                    });
                }
                KrirOp::Call { callee } if depth > 0 => {
                    let callee_semantics =
                        effect_semantics.get(callee).cloned().unwrap_or_default();
                    let is_callee_extern = module
                        .functions
                        .iter()
                        .find(|f| f.name == *callee)
                        .map(|f| f.is_extern)
                        .unwrap_or(false);
                    for effect in callee_semantics.transitive {
                        if matches!(effect, Eff::Yield | Eff::Alloc | Eff::Block) {
                            let inherited = callee_semantics
                                .provenance
                                .get(&effect)
                                .cloned()
                                .unwrap_or_default();
                            let mut provenance = EffectProvenance::default();
                            if is_callee_extern {
                                provenance.via_extern.insert(callee.clone());
                            } else {
                                provenance.via_callee.insert(callee.clone());
                            }
                            provenance.via_callee.extend(inherited.via_callee);
                            provenance.via_extern.extend(inherited.via_extern);
                            violations.insert(CriticalViolation {
                                function: function.name.clone(),
                                effect: effect.as_str().to_string(),
                                provenance,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        depth_max = depth_max.max(fn_max);
    }

    (depth_max, violations.into_iter().collect())
}

#[cfg(test)]
fn transitive_effects_by_function(module: &KrirModule) -> BTreeMap<String, BTreeSet<Eff>> {
    effect_semantics_by_function(module)
        .into_iter()
        .map(|(name, semantics)| (name, semantics.transitive))
        .collect()
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
            provenance
                .get_mut(idx)
                .expect("provenance index")
                .entry(*effect)
                .or_default()
                .direct = true;
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
            provenance
                .get_mut(idx)
                .expect("provenance index")
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

fn ctx_reachable_by_function(module: &KrirModule) -> BTreeMap<String, BTreeSet<Ctx>> {
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

    let mut ctx_sets = vec![BTreeSet::<Ctx>::new(); names.len()];
    for function in &module.functions {
        if let Some(&idx) = index_by_name.get(&function.name) {
            ctx_sets[idx].extend(function.ctx_ok.iter().copied());
        }
    }

    let mut work = (0..names.len()).collect::<VecDeque<_>>();
    while let Some(caller_idx) = work.pop_front() {
        let caller_ctx = ctx_sets[caller_idx].clone();
        for &callee_idx in &outgoing[caller_idx] {
            let before = ctx_sets[callee_idx].len();
            ctx_sets[callee_idx].extend(caller_ctx.iter().copied());
            if ctx_sets[callee_idx].len() != before {
                work.push_back(callee_idx);
            }
        }
    }

    let mut by_name = BTreeMap::<String, BTreeSet<Ctx>>::new();
    for (idx, name) in names.iter().enumerate() {
        by_name.insert(name.clone(), ctx_sets[idx].clone());
    }
    by_name
}

#[cfg(test)]
mod tests {
    use super::*;
    use krir::{CallEdge, Ctx, Eff, Function, FunctionAttrs, KrirModule};
    use passes::LockEdge;

    #[test]
    fn lockgraph_edges_are_emitted_in_lexicographic_order() {
        let report = AnalysisReport {
            lock_edges: vec![
                LockEdge {
                    from: "SchedLock".to_string(),
                    to: "AlphaLock".to_string(),
                },
                LockEdge {
                    from: "ConsoleLock".to_string(),
                    to: "SchedLock".to_string(),
                },
            ],
            max_lock_depth: 2,
            no_yield_spans: BTreeMap::new(),
        };

        let json = emit_lockgraph_json(&report).expect("emit lockgraph");
        let value: Value = serde_json::from_str(&json).expect("parse lockgraph");
        let edges = value["edges"].as_array().expect("edges array");
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0]["from"], Value::String("ConsoleLock".to_string()));
        assert_eq!(edges[0]["to"], Value::String("SchedLock".to_string()));
        assert_eq!(edges[1]["from"], Value::String("SchedLock".to_string()));
        assert_eq!(edges[1]["to"], Value::String("AlphaLock".to_string()));
    }

    #[test]
    fn caps_manifest_is_sorted_and_deduplicated() {
        let module = KrirModule {
            module_caps: vec![
                "IoPort".to_string(),
                "PhysMap".to_string(),
                "IoPort".to_string(),
            ],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "zeta".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Mmio],
                    caps_req: vec!["PhysMap".to_string(), "PhysMap".to_string()],
                    attrs: FunctionAttrs::default(),
                    ops: Vec::new(),
                },
                Function {
                    name: "alpha".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Mmio],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: Vec::new(),
                },
            ],
            call_edges: Vec::new(),
        };

        let json = emit_caps_manifest_json(&module).expect("emit caps");
        let value: Value = serde_json::from_str(&json).expect("parse caps");
        assert_eq!(
            value["module_caps"],
            Value::Array(vec![
                Value::String("IoPort".to_string()),
                Value::String("PhysMap".to_string())
            ])
        );
        let symbols = value["symbols"].as_array().expect("symbols array");
        assert_eq!(symbols[0]["name"], Value::String("alpha".to_string()));
        assert_eq!(symbols[1]["name"], Value::String("zeta".to_string()));
        assert_eq!(
            symbols[1]["caps_req"],
            Value::Array(vec![Value::String("PhysMap".to_string())])
        );
    }

    #[test]
    fn report_json_includes_only_requested_metrics() {
        let report = AnalysisReport {
            lock_edges: Vec::new(),
            max_lock_depth: 9,
            no_yield_spans: BTreeMap::from([("worker".to_string(), NoYieldSpan::Unbounded)]),
        };

        let json = emit_report_json(&report, &["max_lock_depth".to_string()]).expect("emit report");
        let value: Value = serde_json::from_str(&json).expect("parse report");
        let keys = value
            .as_object()
            .expect("report object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(keys, vec!["max_lock_depth".to_string()]);
    }

    #[test]
    fn report_json_key_order_is_stable_for_reordered_input_metrics() {
        let report = AnalysisReport {
            lock_edges: Vec::new(),
            max_lock_depth: 9,
            no_yield_spans: BTreeMap::from([("worker".to_string(), NoYieldSpan::Unbounded)]),
        };

        let json = emit_report_json(
            &report,
            &[
                "no_yield_spans".to_string(),
                "max_lock_depth".to_string(),
                "no_yield_spans".to_string(),
            ],
        )
        .expect("emit report");
        let value: Value = serde_json::from_str(&json).expect("parse report");
        let keys = value
            .as_object()
            .expect("report object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            keys,
            vec!["max_lock_depth".to_string(), "no_yield_spans".to_string()]
        );
    }

    #[test]
    fn krir_json_schema_is_stable_and_functions_are_sorted() {
        let module = KrirModule {
            module_caps: vec!["PhysMap".to_string(), "IoPort".to_string()],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "zeta".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Mmio],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: Vec::new(),
                },
                Function {
                    name: "alpha".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Alloc],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: Vec::new(),
                },
            ],
            call_edges: vec![
                CallEdge {
                    caller: "zeta".to_string(),
                    callee: "alpha".to_string(),
                },
                CallEdge {
                    caller: "alpha".to_string(),
                    callee: "zeta".to_string(),
                },
            ],
        };

        let json = emit_krir_json(&module).expect("emit krir");
        let value: Value = serde_json::from_str(&json).expect("parse krir");
        let top_keys = value
            .as_object()
            .expect("krir object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            top_keys,
            BTreeSet::from([
                "call_edges".to_string(),
                "functions".to_string(),
                "module_caps".to_string()
            ])
        );

        let function_names = value["functions"]
            .as_array()
            .expect("functions array")
            .iter()
            .map(|f| f["name"].as_str().expect("function name").to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            function_names,
            vec!["alpha".to_string(), "zeta".to_string()]
        );
    }

    #[test]
    fn contracts_json_schema_is_stable_and_contains_facts() {
        let module = KrirModule {
            module_caps: vec![
                "PhysMap".to_string(),
                "IoPort".to_string(),
                "IoPort".to_string(),
            ],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "zeta".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread, Ctx::Boot],
                    eff_used: vec![Eff::Mmio],
                    caps_req: vec!["PhysMap".to_string(), "PhysMap".to_string()],
                    attrs: FunctionAttrs {
                        noyield: false,
                        critical: true,
                        leaf: false,
                        hotpath: true,
                        lock_budget: Some(2),
                    },
                    ops: Vec::new(),
                },
                Function {
                    name: "alpha".to_string(),
                    is_extern: true,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Alloc],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: Vec::new(),
                },
            ],
            call_edges: Vec::new(),
        };
        let report = AnalysisReport {
            lock_edges: vec![LockEdge {
                from: "ConsoleLock".to_string(),
                to: "SchedLock".to_string(),
            }],
            max_lock_depth: 2,
            no_yield_spans: BTreeMap::from([("zeta".to_string(), NoYieldSpan::Bounded(3))]),
        };

        let json = emit_contracts_json(&module, &report).expect("emit contracts");
        let value: Value = serde_json::from_str(&json).expect("parse contracts");
        assert_eq!(
            value["schema_version"],
            Value::String(CONTRACTS_SCHEMA_VERSION_V1.to_string())
        );
        let top_keys = value
            .as_object()
            .expect("contracts object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            top_keys,
            BTreeSet::from([
                "capabilities".to_string(),
                "facts".to_string(),
                "lockgraph".to_string(),
                "report".to_string(),
                "schema_version".to_string(),
            ])
        );

        let caps_keys = value["capabilities"]
            .as_object()
            .expect("capabilities object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            caps_keys,
            BTreeSet::from(["module_caps".to_string(), "symbols".to_string()])
        );

        let facts_keys = value["facts"]
            .as_object()
            .expect("facts object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(facts_keys, BTreeSet::from(["symbols".to_string()]));

        let fact_symbols = value["facts"]["symbols"]
            .as_array()
            .expect("facts symbols array");
        assert_eq!(
            fact_symbols[0]["name"],
            Value::String("alpha".to_string()),
            "facts symbols should be sorted by function name"
        );
        let fact_symbol_keys = fact_symbols[0]
            .as_object()
            .expect("fact symbol object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            fact_symbol_keys,
            BTreeSet::from([
                "attrs".to_string(),
                "caps_req".to_string(),
                "ctx_ok".to_string(),
                "eff_used".to_string(),
                "is_extern".to_string(),
                "name".to_string(),
            ])
        );

        let attrs_keys = fact_symbols[0]["attrs"]
            .as_object()
            .expect("attrs object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            attrs_keys,
            BTreeSet::from([
                "hotpath".to_string(),
                "leaf".to_string(),
                "lock_budget".to_string(),
                "noyield".to_string(),
            ])
        );

        let lockgraph_keys = value["lockgraph"]
            .as_object()
            .expect("lockgraph object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            lockgraph_keys,
            BTreeSet::from(["edges".to_string(), "max_lock_depth".to_string()])
        );

        let report_keys = value["report"]
            .as_object()
            .expect("report object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            report_keys,
            BTreeSet::from(["max_lock_depth".to_string(), "no_yield_spans".to_string()])
        );

        let canonical_json =
            emit_contracts_json_canonical(&module, &report).expect("emit canonical");
        assert!(
            !canonical_json.contains('\n'),
            "canonical contracts JSON should be minified"
        );
        let canonical_value: Value =
            serde_json::from_str(&canonical_json).expect("parse canonical contracts");
        assert_eq!(
            canonical_value, value,
            "canonical and pretty contracts payloads must be equivalent"
        );
    }

    #[test]
    fn contracts_v2_facts_include_transitive_effects() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "helper".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread, Ctx::Irq],
                    eff_used: vec![Eff::Alloc],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::AllocPoint],
                },
                Function {
                    name: "isr".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Irq],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "helper".to_string(),
                    }],
                },
            ],
            call_edges: vec![CallEdge {
                caller: "isr".to_string(),
                callee: "helper".to_string(),
            }],
        };
        let report = AnalysisReport::default();

        let json = emit_contracts_json_with_schema(&module, &report, ContractsSchema::V2)
            .expect("emit contracts v2");
        let value: Value = serde_json::from_str(&json).expect("parse contracts");
        let symbols = value["facts"]["symbols"].as_array().expect("symbols array");
        let helper = symbols
            .iter()
            .find(|sym| sym["name"] == "helper")
            .expect("helper symbol");
        let isr = symbols
            .iter()
            .find(|sym| sym["name"] == "isr")
            .expect("isr symbol");
        assert_eq!(
            helper["eff_transitive"],
            Value::Array(vec![Value::String("alloc".to_string())])
        );
        assert_eq!(
            isr["eff_transitive"],
            Value::Array(vec![Value::String("alloc".to_string())])
        );
        let isr_provenance = isr["eff_provenance"]
            .as_array()
            .expect("isr eff_provenance array");
        assert_eq!(isr_provenance.len(), 1);
        assert_eq!(
            isr_provenance[0]["effect"],
            Value::String("alloc".to_string())
        );
        assert_eq!(
            isr_provenance[0]["provenance"],
            serde_json::json!({
                "direct": false,
                "via_callee": ["helper"],
                "via_extern": [],
            })
        );
    }

    #[test]
    fn contracts_v2_effect_provenance_tracks_extern_sources() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "kmalloc".to_string(),
                    is_extern: true,
                    ctx_ok: vec![Ctx::Boot, Ctx::Thread],
                    eff_used: vec![Eff::Alloc],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: Vec::new(),
                },
                Function {
                    name: "entry".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "kmalloc".to_string(),
                    }],
                },
            ],
            call_edges: vec![CallEdge {
                caller: "entry".to_string(),
                callee: "kmalloc".to_string(),
            }],
        };
        let report = AnalysisReport::default();

        let json = emit_contracts_json_with_schema(&module, &report, ContractsSchema::V2)
            .expect("emit contracts v2");
        let value: Value = serde_json::from_str(&json).expect("parse contracts");
        let symbols = value["facts"]["symbols"].as_array().expect("symbols array");
        let entry = symbols
            .iter()
            .find(|sym| sym["name"] == "entry")
            .expect("entry symbol");
        let provenance = entry["eff_provenance"]
            .as_array()
            .expect("entry eff_provenance array");
        assert_eq!(provenance.len(), 1);
        assert_eq!(provenance[0]["effect"], Value::String("alloc".to_string()));
        assert_eq!(
            provenance[0]["provenance"],
            serde_json::json!({
                "direct": false,
                "via_callee": [],
                "via_extern": ["kmalloc"],
            })
        );
    }

    #[test]
    fn transitive_effects_handle_recursive_scc() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "a".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "b".to_string(),
                    }],
                },
                Function {
                    name: "b".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Block],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "a".to_string(),
                    }],
                },
            ],
            call_edges: vec![
                CallEdge {
                    caller: "a".to_string(),
                    callee: "b".to_string(),
                },
                CallEdge {
                    caller: "b".to_string(),
                    callee: "a".to_string(),
                },
            ],
        };

        let transitive = transitive_effects_by_function(&module);
        let expected = BTreeSet::from([Eff::Block]);
        assert_eq!(transitive.get("a"), Some(&expected));
        assert_eq!(transitive.get("b"), Some(&expected));
    }

    #[test]
    fn effect_provenance_direct_only() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![Function {
                name: "f".to_string(),
                is_extern: false,
                ctx_ok: vec![Ctx::Thread],
                eff_used: vec![Eff::Alloc],
                caps_req: vec![],
                attrs: FunctionAttrs::default(),
                ops: vec![KrirOp::AllocPoint],
            }],
            call_edges: vec![],
        };
        let semantics = effect_semantics_by_function(&module);
        let provenance = semantics["f"]
            .provenance
            .get(&Eff::Alloc)
            .expect("alloc provenance");
        assert!(provenance.direct);
        assert!(provenance.via_callee.is_empty());
        assert!(provenance.via_extern.is_empty());
    }

    #[test]
    fn effect_provenance_via_callee_only() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "helper".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Alloc],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::AllocPoint],
                },
                Function {
                    name: "entry".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "helper".to_string(),
                    }],
                },
            ],
            call_edges: vec![CallEdge {
                caller: "entry".to_string(),
                callee: "helper".to_string(),
            }],
        };
        let semantics = effect_semantics_by_function(&module);
        let provenance = semantics["entry"]
            .provenance
            .get(&Eff::Alloc)
            .expect("alloc provenance");
        assert!(!provenance.direct);
        assert_eq!(
            provenance.via_callee,
            BTreeSet::from(["helper".to_string()])
        );
        assert!(provenance.via_extern.is_empty());
    }

    #[test]
    fn effect_provenance_via_extern_only() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "kmalloc".to_string(),
                    is_extern: true,
                    ctx_ok: vec![Ctx::Thread, Ctx::Boot],
                    eff_used: vec![Eff::Alloc],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![],
                },
                Function {
                    name: "entry".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "kmalloc".to_string(),
                    }],
                },
            ],
            call_edges: vec![CallEdge {
                caller: "entry".to_string(),
                callee: "kmalloc".to_string(),
            }],
        };
        let semantics = effect_semantics_by_function(&module);
        let provenance = semantics["entry"]
            .provenance
            .get(&Eff::Alloc)
            .expect("alloc provenance");
        assert!(!provenance.direct);
        assert!(provenance.via_callee.is_empty());
        assert_eq!(
            provenance.via_extern,
            BTreeSet::from(["kmalloc".to_string()])
        );
    }

    #[test]
    fn effect_provenance_mixed_direct_via_callee_and_via_extern() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "kmalloc".to_string(),
                    is_extern: true,
                    ctx_ok: vec![Ctx::Thread, Ctx::Boot],
                    eff_used: vec![Eff::Alloc],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![],
                },
                Function {
                    name: "helper".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Alloc],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "kmalloc".to_string(),
                    }],
                },
                Function {
                    name: "entry".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "helper".to_string(),
                    }],
                },
            ],
            call_edges: vec![
                CallEdge {
                    caller: "helper".to_string(),
                    callee: "kmalloc".to_string(),
                },
                CallEdge {
                    caller: "entry".to_string(),
                    callee: "helper".to_string(),
                },
            ],
        };
        let semantics = effect_semantics_by_function(&module);
        let helper = semantics["helper"]
            .provenance
            .get(&Eff::Alloc)
            .expect("helper alloc provenance");
        assert!(helper.direct);
        assert!(helper.via_callee.is_empty());
        assert_eq!(helper.via_extern, BTreeSet::from(["kmalloc".to_string()]));

        let entry = semantics["entry"]
            .provenance
            .get(&Eff::Alloc)
            .expect("entry alloc provenance");
        assert!(!entry.direct);
        assert_eq!(entry.via_callee, BTreeSet::from(["helper".to_string()]));
        assert_eq!(entry.via_extern, BTreeSet::from(["kmalloc".to_string()]));
    }

    #[test]
    fn effect_provenance_recursive_and_cyclic_is_deterministic() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "kmalloc".to_string(),
                    is_extern: true,
                    ctx_ok: vec![Ctx::Thread, Ctx::Boot],
                    eff_used: vec![Eff::Alloc],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![],
                },
                Function {
                    name: "a".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Alloc],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "b".to_string(),
                    }],
                },
                Function {
                    name: "b".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![
                        KrirOp::Call {
                            callee: "a".to_string(),
                        },
                        KrirOp::Call {
                            callee: "kmalloc".to_string(),
                        },
                    ],
                },
                Function {
                    name: "self_fn".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Block],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "self_fn".to_string(),
                    }],
                },
            ],
            call_edges: vec![
                CallEdge {
                    caller: "a".to_string(),
                    callee: "b".to_string(),
                },
                CallEdge {
                    caller: "b".to_string(),
                    callee: "a".to_string(),
                },
                CallEdge {
                    caller: "b".to_string(),
                    callee: "kmalloc".to_string(),
                },
                CallEdge {
                    caller: "self_fn".to_string(),
                    callee: "self_fn".to_string(),
                },
            ],
        };
        let semantics = effect_semantics_by_function(&module);

        let a = semantics["a"].provenance.get(&Eff::Alloc).expect("a alloc");
        assert!(a.direct);
        assert_eq!(a.via_callee, BTreeSet::from(["b".to_string()]));
        assert_eq!(a.via_extern, BTreeSet::from(["kmalloc".to_string()]));

        let b = semantics["b"].provenance.get(&Eff::Alloc).expect("b alloc");
        assert!(!b.direct);
        assert_eq!(b.via_callee, BTreeSet::from(["a".to_string()]));
        assert_eq!(b.via_extern, BTreeSet::from(["kmalloc".to_string()]));

        let self_fn = semantics["self_fn"]
            .provenance
            .get(&Eff::Block)
            .expect("self_fn block");
        assert!(self_fn.direct);
        assert!(self_fn.via_callee.is_empty());
        assert!(self_fn.via_extern.is_empty());
    }

    #[test]
    fn capability_provenance_direct_only() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![Function {
                name: "f".to_string(),
                is_extern: false,
                ctx_ok: vec![Ctx::Thread],
                eff_used: vec![],
                caps_req: vec!["PhysMap".to_string()],
                attrs: FunctionAttrs::default(),
                ops: vec![],
            }],
            call_edges: vec![],
        };
        let semantics = capability_semantics_by_function(&module);
        let provenance = semantics["f"]
            .provenance
            .get("PhysMap")
            .expect("PhysMap provenance");
        assert!(provenance.direct);
        assert!(provenance.via_callee.is_empty());
        assert!(provenance.via_extern.is_empty());
    }

    #[test]
    fn capability_provenance_via_callee_only() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "helper".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec!["PhysMap".to_string()],
                    attrs: FunctionAttrs::default(),
                    ops: vec![],
                },
                Function {
                    name: "entry".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "helper".to_string(),
                    }],
                },
            ],
            call_edges: vec![CallEdge {
                caller: "entry".to_string(),
                callee: "helper".to_string(),
            }],
        };
        let semantics = capability_semantics_by_function(&module);
        let provenance = semantics["entry"]
            .provenance
            .get("PhysMap")
            .expect("PhysMap provenance");
        assert!(!provenance.direct);
        assert_eq!(
            provenance.via_callee,
            BTreeSet::from(["helper".to_string()])
        );
        assert!(provenance.via_extern.is_empty());
    }

    #[test]
    fn capability_provenance_via_extern_only() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "map_io".to_string(),
                    is_extern: true,
                    ctx_ok: vec![Ctx::Thread, Ctx::Boot],
                    eff_used: vec![],
                    caps_req: vec!["PhysMap".to_string()],
                    attrs: FunctionAttrs::default(),
                    ops: vec![],
                },
                Function {
                    name: "entry".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "map_io".to_string(),
                    }],
                },
            ],
            call_edges: vec![CallEdge {
                caller: "entry".to_string(),
                callee: "map_io".to_string(),
            }],
        };
        let semantics = capability_semantics_by_function(&module);
        let provenance = semantics["entry"]
            .provenance
            .get("PhysMap")
            .expect("PhysMap provenance");
        assert!(!provenance.direct);
        assert!(provenance.via_callee.is_empty());
        assert_eq!(
            provenance.via_extern,
            BTreeSet::from(["map_io".to_string()])
        );
    }

    #[test]
    fn capability_provenance_mixed_direct_via_callee_and_via_extern() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "map_io".to_string(),
                    is_extern: true,
                    ctx_ok: vec![Ctx::Thread, Ctx::Boot],
                    eff_used: vec![],
                    caps_req: vec!["PhysMap".to_string()],
                    attrs: FunctionAttrs::default(),
                    ops: vec![],
                },
                Function {
                    name: "helper".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec!["PhysMap".to_string()],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "map_io".to_string(),
                    }],
                },
                Function {
                    name: "entry".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "helper".to_string(),
                    }],
                },
            ],
            call_edges: vec![
                CallEdge {
                    caller: "helper".to_string(),
                    callee: "map_io".to_string(),
                },
                CallEdge {
                    caller: "entry".to_string(),
                    callee: "helper".to_string(),
                },
            ],
        };
        let semantics = capability_semantics_by_function(&module);

        let helper = semantics["helper"]
            .provenance
            .get("PhysMap")
            .expect("helper PhysMap provenance");
        assert!(helper.direct);
        assert!(helper.via_callee.is_empty());
        assert_eq!(helper.via_extern, BTreeSet::from(["map_io".to_string()]));

        let entry = semantics["entry"]
            .provenance
            .get("PhysMap")
            .expect("entry PhysMap provenance");
        assert!(!entry.direct);
        assert_eq!(entry.via_callee, BTreeSet::from(["helper".to_string()]));
        assert_eq!(entry.via_extern, BTreeSet::from(["map_io".to_string()]));
    }

    #[test]
    fn capability_provenance_recursive_and_cyclic_is_deterministic() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "map_io".to_string(),
                    is_extern: true,
                    ctx_ok: vec![Ctx::Thread, Ctx::Boot],
                    eff_used: vec![],
                    caps_req: vec!["PhysMap".to_string()],
                    attrs: FunctionAttrs::default(),
                    ops: vec![],
                },
                Function {
                    name: "a".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec!["PhysMap".to_string()],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "b".to_string(),
                    }],
                },
                Function {
                    name: "b".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![
                        KrirOp::Call {
                            callee: "a".to_string(),
                        },
                        KrirOp::Call {
                            callee: "map_io".to_string(),
                        },
                    ],
                },
                Function {
                    name: "self_fn".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec!["IoPort".to_string()],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::Call {
                        callee: "self_fn".to_string(),
                    }],
                },
            ],
            call_edges: vec![
                CallEdge {
                    caller: "a".to_string(),
                    callee: "b".to_string(),
                },
                CallEdge {
                    caller: "b".to_string(),
                    callee: "a".to_string(),
                },
                CallEdge {
                    caller: "b".to_string(),
                    callee: "map_io".to_string(),
                },
                CallEdge {
                    caller: "self_fn".to_string(),
                    callee: "self_fn".to_string(),
                },
            ],
        };
        let semantics = capability_semantics_by_function(&module);

        let a = semantics["a"].provenance.get("PhysMap").expect("a PhysMap");
        assert!(a.direct);
        assert_eq!(a.via_callee, BTreeSet::from(["b".to_string()]));
        assert_eq!(a.via_extern, BTreeSet::from(["map_io".to_string()]));

        let b = semantics["b"].provenance.get("PhysMap").expect("b PhysMap");
        assert!(!b.direct);
        assert_eq!(b.via_callee, BTreeSet::from(["a".to_string()]));
        assert_eq!(b.via_extern, BTreeSet::from(["map_io".to_string()]));

        let self_fn = semantics["self_fn"]
            .provenance
            .get("IoPort")
            .expect("self_fn IoPort");
        assert!(self_fn.direct);
        assert!(self_fn.via_callee.is_empty());
        assert!(self_fn.via_extern.is_empty());
    }

    #[test]
    fn contracts_v2_report_includes_critical_region_findings() {
        let module = KrirModule {
            module_caps: vec![],
            mmio_bases: Vec::new(),
            mmio_registers: Vec::new(),
            functions: vec![
                Function {
                    name: "helper".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![Eff::Yield],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![KrirOp::YieldPoint],
                },
                Function {
                    name: "entry".to_string(),
                    is_extern: false,
                    ctx_ok: vec![Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                    ops: vec![
                        KrirOp::CriticalEnter,
                        KrirOp::Call {
                            callee: "helper".to_string(),
                        },
                        KrirOp::CriticalExit,
                    ],
                },
            ],
            call_edges: vec![CallEdge {
                caller: "entry".to_string(),
                callee: "helper".to_string(),
            }],
        };
        let report = AnalysisReport::default();
        let json = emit_contracts_json_with_schema(&module, &report, ContractsSchema::V2)
            .expect("emit contracts v2");
        let value: Value = serde_json::from_str(&json).expect("parse json");
        assert_eq!(
            value["report"]["critical"]["depth_max"],
            Value::Number(1_u64.into())
        );
        let violations = value["report"]["critical"]["violations"]
            .as_array()
            .expect("critical violations");
        assert!(violations.iter().any(|v| v["function"] == "entry"
            && v["effect"] == "yield"
            && v["provenance"]["direct"] == false
            && v["provenance"]["via_callee"] == serde_json::json!(["helper"])
            && v["provenance"]["via_extern"] == serde_json::json!([])));
    }
}
