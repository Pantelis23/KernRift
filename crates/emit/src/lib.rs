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
    let report_value = match schema {
        ContractsSchema::V1 => {
            let report_text = emit_report_json(
                report,
                &["max_lock_depth".to_string(), "no_yield_spans".to_string()],
            )?;
            serde_json::from_str(&report_text)
                .map_err(|e| format!("failed to parse generated report JSON: {}", e))?
        }
        ContractsSchema::V2 => report_v2_value(&canonical, report),
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
        facts_manifest_value(&canonical, schema),
    );
    root.insert("lockgraph".to_string(), lockgraph_value);
    root.insert("report".to_string(), report_value);

    Ok(Value::Object(root))
}

fn facts_manifest_value(module: &KrirModule, schema: ContractsSchema) -> Value {
    let transitive_effects = if schema == ContractsSchema::V2 {
        Some(transitive_effects_by_function(module))
    } else {
        None
    };
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
            symbol.insert(
                "eff_used".to_string(),
                Value::Array(
                    f.eff_used
                        .iter()
                        .map(|eff| Value::String(eff.as_str().to_string()))
                        .collect(),
                ),
            );
            if let Some(eff_map) = transitive_effects.as_ref() {
                let mut effs = eff_map
                    .get(&f.name)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|eff| eff.as_str().to_string())
                    .collect::<Vec<_>>();
                effs.sort();
                effs.dedup();
                symbol.insert(
                    "eff_transitive".to_string(),
                    Value::Array(effs.into_iter().map(Value::String).collect()),
                );
            }
            symbol.insert(
                "caps_req".to_string(),
                Value::Array(f.caps_req.iter().cloned().map(Value::String).collect()),
            );
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

fn report_v2_value(module: &KrirModule, report: &AnalysisReport) -> Value {
    let non_extern_names = module
        .functions
        .iter()
        .filter(|f| !f.is_extern)
        .map(|f| f.name.clone())
        .collect::<BTreeSet<_>>();
    let outgoing = module
        .call_edges
        .iter()
        .filter(|edge| non_extern_names.contains(&edge.caller))
        .filter(|edge| non_extern_names.contains(&edge.callee))
        .fold(BTreeMap::<&str, Vec<&str>>::new(), |mut map, edge| {
            map.entry(edge.caller.as_str())
                .or_default()
                .push(edge.callee.as_str());
            map
        });

    let mut irq_reachable = module
        .functions
        .iter()
        .filter(|f| !f.is_extern && f.ctx_ok.contains(&Ctx::Irq))
        .map(|f| f.name.clone())
        .collect::<BTreeSet<_>>();
    let mut work = irq_reachable.iter().cloned().collect::<VecDeque<_>>();
    while let Some(caller) = work.pop_front() {
        if let Some(callees) = outgoing.get(caller.as_str()) {
            for callee in callees {
                if irq_reachable.insert((*callee).to_string()) {
                    work.push_back((*callee).to_string());
                }
            }
        }
    }
    let irq_functions = irq_reachable.into_iter().collect::<Vec<_>>();
    let critical_functions = module
        .functions
        .iter()
        .filter(|f| !f.is_extern && f.attrs.critical)
        .map(|f| f.name.clone())
        .collect::<Vec<_>>();
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

    let mut contexts = Map::new();
    contexts.insert(
        "irq_functions".to_string(),
        Value::Array(irq_functions.into_iter().map(Value::String).collect()),
    );
    contexts.insert(
        "critical_functions".to_string(),
        Value::Array(critical_functions.into_iter().map(Value::String).collect()),
    );

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

    let mut report_obj = Map::new();
    report_obj.insert(
        "max_lock_depth".to_string(),
        Value::Number(report.max_lock_depth.into()),
    );
    report_obj.insert(
        "no_yield_spans".to_string(),
        Value::Object(no_yield_json(&report.no_yield_spans)),
    );
    report_obj.insert("contexts".to_string(), Value::Object(contexts));
    report_obj.insert("effects".to_string(), Value::Object(effects));
    Value::Object(report_obj)
}

fn transitive_effects_by_function(module: &KrirModule) -> BTreeMap<String, BTreeSet<Eff>> {
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
    for function in &module.functions {
        if let Some(&idx) = index_by_name.get(&function.name) {
            direct[idx].extend(function.eff_used.iter().copied());
        }
    }

    let mut outgoing_sets = vec![BTreeSet::<usize>::new(); names.len()];
    let mut incoming_sets = vec![BTreeSet::<usize>::new(); names.len()];
    for edge in &module.call_edges {
        let (Some(&caller), Some(&callee)) = (
            index_by_name.get(&edge.caller),
            index_by_name.get(&edge.callee),
        ) else {
            continue;
        };
        outgoing_sets[caller].insert(callee);
        incoming_sets[callee].insert(caller);
    }
    let outgoing = outgoing_sets
        .into_iter()
        .map(|set| set.into_iter().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let incoming = incoming_sets
        .into_iter()
        .map(|set| set.into_iter().collect::<Vec<_>>())
        .collect::<Vec<_>>();

    fn dfs_order(node: usize, edges: &[Vec<usize>], seen: &mut [bool], order: &mut Vec<usize>) {
        if seen[node] {
            return;
        }
        seen[node] = true;
        for &next in &edges[node] {
            dfs_order(next, edges, seen, order);
        }
        order.push(node);
    }

    fn dfs_component(
        node: usize,
        cid: usize,
        rev_edges: &[Vec<usize>],
        component: &mut [usize],
        members: &mut Vec<usize>,
    ) {
        if component[node] != usize::MAX {
            return;
        }
        component[node] = cid;
        members.push(node);
        for &next in &rev_edges[node] {
            dfs_component(next, cid, rev_edges, component, members);
        }
    }

    let mut seen = vec![false; names.len()];
    let mut finish_order = Vec::<usize>::new();
    for node in 0..names.len() {
        dfs_order(node, &outgoing, &mut seen, &mut finish_order);
    }

    let mut component = vec![usize::MAX; names.len()];
    let mut components = Vec::<Vec<usize>>::new();
    for &node in finish_order.iter().rev() {
        if component[node] != usize::MAX {
            continue;
        }
        let cid = components.len();
        let mut members = Vec::<usize>::new();
        dfs_component(node, cid, &incoming, &mut component, &mut members);
        members.sort();
        components.push(members);
    }

    let mut component_direct = vec![BTreeSet::<Eff>::new(); components.len()];
    for (cid, members) in components.iter().enumerate() {
        for &member in members {
            component_direct[cid].extend(direct[member].iter().copied());
        }
    }

    let mut component_edges = vec![BTreeSet::<usize>::new(); components.len()];
    for (from, nexts) in outgoing.iter().enumerate() {
        let from_cid = component[from];
        for &to in nexts {
            let to_cid = component[to];
            if from_cid != to_cid {
                component_edges[from_cid].insert(to_cid);
            }
        }
    }

    fn component_closure(
        cid: usize,
        component_direct: &[BTreeSet<Eff>],
        component_edges: &[BTreeSet<usize>],
        memo: &mut [Option<BTreeSet<Eff>>],
    ) -> BTreeSet<Eff> {
        if let Some(cached) = memo[cid].as_ref() {
            return cached.clone();
        }
        let mut out = component_direct[cid].clone();
        for &next in &component_edges[cid] {
            out.extend(component_closure(
                next,
                component_direct,
                component_edges,
                memo,
            ));
        }
        memo[cid] = Some(out.clone());
        out
    }

    let mut memo = vec![None::<BTreeSet<Eff>>; components.len()];
    let mut by_name = BTreeMap::<String, BTreeSet<Eff>>::new();
    for (idx, name) in names.iter().enumerate() {
        let effs = component_closure(
            component[idx],
            &component_direct,
            &component_edges,
            &mut memo,
        );
        by_name.insert(name.clone(), effs);
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
    }

    #[test]
    fn transitive_effects_handle_recursive_scc() {
        let module = KrirModule {
            module_caps: vec![],
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
}
