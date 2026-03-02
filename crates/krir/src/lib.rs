use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ctx {
    Boot,
    Irq,
    Nmi,
    Thread,
}

impl Ctx {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Boot => "boot",
            Self::Irq => "irq",
            Self::Nmi => "nmi",
            Self::Thread => "thread",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Eff {
    Alloc,
    Block,
    DmaMap,
    Ioport,
    Mmio,
    PreemptOff,
    Yield,
}

impl Eff {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Alloc => "alloc",
            Self::Block => "block",
            Self::DmaMap => "dma_map",
            Self::Ioport => "ioport",
            Self::Mmio => "mmio",
            Self::PreemptOff => "preempt_off",
            Self::Yield => "yield",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Alloc,
            Self::Block,
            Self::DmaMap,
            Self::Ioport,
            Self::Mmio,
            Self::PreemptOff,
            Self::Yield,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct FunctionAttrs {
    pub noyield: bool,
    pub critical: bool,
    pub leaf: bool,
    pub hotpath: bool,
    pub lock_budget: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum KrirOp {
    Call { callee: String },
    CriticalEnter,
    CriticalExit,
    YieldPoint,
    AllocPoint,
    BlockPoint,
    Acquire { lock_class: String },
    Release { lock_class: String },
    MmioRead,
    MmioWrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Function {
    pub name: String,
    pub is_extern: bool,
    pub ctx_ok: Vec<Ctx>,
    pub eff_used: Vec<Eff>,
    pub caps_req: Vec<String>,
    pub attrs: FunctionAttrs,
    pub ops: Vec<KrirOp>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct CallEdge {
    pub caller: String,
    pub callee: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct KrirModule {
    pub module_caps: Vec<String>,
    pub functions: Vec<Function>,
    pub call_edges: Vec<CallEdge>,
}

impl KrirModule {
    pub fn canonicalize(&mut self) {
        self.module_caps.sort();
        self.module_caps.dedup();

        self.functions.sort_by(|a, b| a.name.cmp(&b.name));
        for f in &mut self.functions {
            f.ctx_ok.sort_by_key(|ctx| ctx.as_str());
            f.ctx_ok.dedup();
            f.eff_used.sort_by_key(|eff| eff.as_str());
            f.eff_used.dedup();
            f.caps_req.sort();
            f.caps_req.dedup();
        }

        self.call_edges.sort();
        self.call_edges.dedup();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutableValueType {
    Unit,
}

impl ExecutableValueType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unit => "unit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutableValue {
    Unit,
}

impl ExecutableValue {
    pub fn value_type(&self) -> ExecutableValueType {
        match self {
            Self::Unit => ExecutableValueType::Unit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutableSignature {
    pub params: Vec<ExecutableValueType>,
    pub result: ExecutableValueType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutableFacts {
    pub ctx_ok: Vec<Ctx>,
    pub eff_used: Vec<Eff>,
    pub caps_req: Vec<String>,
    pub attrs: FunctionAttrs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ExecutableOp {
    Call { callee: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "terminator", rename_all = "snake_case")]
pub enum ExecutableTerminator {
    Return { value: ExecutableValue },
}

impl ExecutableTerminator {
    fn value_type(&self) -> ExecutableValueType {
        match self {
            Self::Return { value } => value.value_type(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutableBlock {
    pub label: String,
    pub ops: Vec<ExecutableOp>,
    pub terminator: ExecutableTerminator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutableFunction {
    pub name: String,
    pub is_extern: bool,
    pub signature: ExecutableSignature,
    pub facts: ExecutableFacts,
    pub entry_block: String,
    pub blocks: Vec<ExecutableBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct ExecutableKrirModule {
    pub module_caps: Vec<String>,
    pub functions: Vec<ExecutableFunction>,
    pub call_edges: Vec<CallEdge>,
}

impl ExecutableKrirModule {
    pub fn canonicalize(&mut self) {
        self.module_caps.sort();
        self.module_caps.dedup();

        self.functions.sort_by(|a, b| a.name.cmp(&b.name));
        for function in &mut self.functions {
            function.facts.ctx_ok.sort_by_key(|ctx| ctx.as_str());
            function.facts.ctx_ok.dedup();
            function.facts.eff_used.sort_by_key(|eff| eff.as_str());
            function.facts.eff_used.dedup();
            function.facts.caps_req.sort();
            function.facts.caps_req.dedup();
        }

        self.call_edges.sort();
        self.call_edges.dedup();
    }

    pub fn validate(&self) -> Result<(), String> {
        for function in &self.functions {
            if function.is_extern {
                return Err(format!(
                    "executable KRIR function '{}' must not be extern",
                    function.name
                ));
            }

            if !function.signature.params.is_empty() {
                return Err(format!(
                    "executable KRIR function '{}' must not declare parameters in v0.1",
                    function.name
                ));
            }

            if function.blocks.is_empty() {
                return Err(format!(
                    "executable KRIR function '{}' must contain at least one block",
                    function.name
                ));
            }

            let mut labels = BTreeSet::new();
            let mut found_entry = false;
            for block in &function.blocks {
                if !labels.insert(block.label.as_str()) {
                    return Err(format!(
                        "executable KRIR function '{}' has duplicate block label '{}'",
                        function.name, block.label
                    ));
                }

                if block.label == function.entry_block {
                    found_entry = true;
                }

                if block.terminator.value_type() != function.signature.result {
                    return Err(format!(
                        "executable KRIR function '{}' terminator type does not match signature",
                        function.name
                    ));
                }
            }

            if !found_entry {
                return Err(format!(
                    "executable KRIR function '{}' entry block '{}' is missing",
                    function.name, function.entry_block
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CallEdge, Ctx, Eff, ExecutableBlock, ExecutableFacts, ExecutableFunction,
        ExecutableKrirModule, ExecutableOp, ExecutableSignature, ExecutableTerminator,
        ExecutableValue, ExecutableValueType, FunctionAttrs,
    };
    use serde_json::json;

    fn executable_function(name: &str) -> ExecutableFunction {
        ExecutableFunction {
            name: name.to_string(),
            is_extern: false,
            signature: ExecutableSignature {
                params: vec![],
                result: ExecutableValueType::Unit,
            },
            facts: ExecutableFacts {
                ctx_ok: vec![Ctx::Thread],
                eff_used: vec![Eff::Block],
                caps_req: vec!["PhysMap".to_string()],
                attrs: FunctionAttrs::default(),
            },
            entry_block: "entry".to_string(),
            blocks: vec![ExecutableBlock {
                label: "entry".to_string(),
                ops: vec![ExecutableOp::Call {
                    callee: "helper".to_string(),
                }],
                terminator: ExecutableTerminator::Return {
                    value: ExecutableValue::Unit,
                },
            }],
        }
    }

    #[test]
    fn executable_krir_serialization_is_deterministic_and_explicit() {
        let mut module = ExecutableKrirModule {
            module_caps: vec![
                "Mmio".to_string(),
                "PhysMap".to_string(),
                "Mmio".to_string(),
            ],
            functions: vec![executable_function("entry")],
            call_edges: vec![
                CallEdge {
                    caller: "entry".to_string(),
                    callee: "helper".to_string(),
                },
                CallEdge {
                    caller: "entry".to_string(),
                    callee: "helper".to_string(),
                },
            ],
        };
        module.canonicalize();
        module.validate().expect("valid executable KRIR");

        let value = serde_json::to_value(&module).expect("serialize");
        assert_eq!(
            value,
            json!({
                "module_caps": ["Mmio", "PhysMap"],
                "functions": [{
                    "name": "entry",
                    "is_extern": false,
                    "signature": {
                        "params": [],
                        "result": "unit"
                    },
                    "facts": {
                        "ctx_ok": ["thread"],
                        "eff_used": ["block"],
                        "caps_req": ["PhysMap"],
                        "attrs": {
                            "noyield": false,
                            "critical": false,
                            "leaf": false,
                            "hotpath": false,
                            "lock_budget": null
                        }
                    },
                    "entry_block": "entry",
                    "blocks": [{
                        "label": "entry",
                        "ops": [{
                            "op": "call",
                            "callee": "helper"
                        }],
                        "terminator": {
                            "terminator": "return",
                            "value": {
                                "kind": "unit"
                            }
                        }
                    }]
                }],
                "call_edges": [{
                    "caller": "entry",
                    "callee": "helper"
                }]
            })
        );
    }

    #[test]
    fn executable_krir_validation_rejects_missing_entry_block() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                entry_block: "missing".to_string(),
                ..executable_function("entry")
            }],
            call_edges: vec![],
        };

        assert_eq!(
            module.validate(),
            Err("executable KRIR function 'entry' entry block 'missing' is missing".to_string())
        );
    }

    #[test]
    fn executable_krir_validation_rejects_non_unit_params_in_v0_1() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                signature: ExecutableSignature {
                    params: vec![ExecutableValueType::Unit],
                    result: ExecutableValueType::Unit,
                },
                ..executable_function("entry")
            }],
            call_edges: vec![],
        };

        assert_eq!(
            module.validate(),
            Err("executable KRIR function 'entry' must not declare parameters in v0.1".to_string())
        );
    }

    #[test]
    fn executable_krir_canonicalize_preserves_block_order() {
        let mut module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                blocks: vec![
                    ExecutableBlock {
                        label: "zeta".to_string(),
                        ops: vec![],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    },
                    ExecutableBlock {
                        label: "alpha".to_string(),
                        ops: vec![],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    },
                ],
                ..executable_function("entry")
            }],
            call_edges: vec![],
        };

        module.canonicalize();

        assert_eq!(
            module.functions[0]
                .blocks
                .iter()
                .map(|block| block.label.as_str())
                .collect::<Vec<_>>(),
            vec!["zeta", "alpha"]
        );
    }

    #[test]
    fn executable_krir_canonicalize_preserves_param_order() {
        let mut module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                signature: ExecutableSignature {
                    params: vec![ExecutableValueType::Unit, ExecutableValueType::Unit],
                    result: ExecutableValueType::Unit,
                },
                ..executable_function("entry")
            }],
            call_edges: vec![],
        };

        module.canonicalize();

        assert_eq!(
            module.functions[0].signature.params,
            vec![ExecutableValueType::Unit, ExecutableValueType::Unit]
        );
    }
}
