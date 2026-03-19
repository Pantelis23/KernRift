use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

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
    Call {
        callee: String,
    },
    BranchIfZero {
        slot: String,
        then_callee: String,
        else_callee: String,
    },
    BranchIfEq {
        slot: String,
        compare_value: String,
        then_callee: String,
        else_callee: String,
    },
    BranchIfMaskNonZero {
        slot: String,
        mask_value: String,
        then_callee: String,
        else_callee: String,
    },
    CriticalEnter,
    CriticalExit,
    YieldPoint,
    AllocPoint,
    BlockPoint,
    Acquire {
        lock_class: String,
    },
    Release {
        lock_class: String,
    },
    MmioRead {
        ty: MmioScalarType,
        addr: MmioAddrExpr,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capture_slot: Option<String>,
    },
    MmioWrite {
        ty: MmioScalarType,
        addr: MmioAddrExpr,
        value: MmioValueExpr,
    },
    RawMmioRead {
        ty: MmioScalarType,
        addr: MmioAddrExpr,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capture_slot: Option<String>,
    },
    RawMmioWrite {
        ty: MmioScalarType,
        addr: MmioAddrExpr,
        value: MmioValueExpr,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MmioAddrExpr {
    Ident { name: String },
    IntLiteral { value: String },
    IdentPlusOffset { base: String, offset: String },
}

impl MmioAddrExpr {
    pub fn as_source(&self) -> String {
        match self {
            Self::Ident { name } => name.clone(),
            Self::IntLiteral { value } => value.clone(),
            Self::IdentPlusOffset { base, offset } => format!("{base} + {offset}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MmioValueExpr {
    Ident { name: String },
    IntLiteral { value: String },
}

impl MmioValueExpr {
    pub fn as_source(&self) -> String {
        match self {
            Self::Ident { name } => name.clone(),
            Self::IntLiteral { value } => value.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MmioScalarType {
    U8,
    U16,
    U32,
    U64,
}

impl MmioScalarType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
        }
    }
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
pub struct MmioBaseDecl {
    pub name: String,
    pub addr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MmioRegAccess {
    Ro,
    Wo,
    Rw,
}

impl MmioRegAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ro => "ro",
            Self::Wo => "wo",
            Self::Rw => "rw",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct MmioRegisterDecl {
    pub base: String,
    pub name: String,
    pub offset: String,
    pub ty: MmioScalarType,
    pub access: MmioRegAccess,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct CallEdge {
    pub caller: String,
    pub callee: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct KrirModule {
    pub module_caps: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mmio_bases: Vec<MmioBaseDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mmio_registers: Vec<MmioRegisterDecl>,
    pub functions: Vec<Function>,
    pub call_edges: Vec<CallEdge>,
}

impl KrirModule {
    pub fn canonicalize(&mut self) {
        self.module_caps.sort();
        self.module_caps.dedup();
        self.mmio_bases.sort_by(|a, b| a.name.cmp(&b.name));
        self.mmio_bases.dedup();
        self.mmio_registers.sort_by(|a, b| {
            (a.base.as_str(), a.offset.as_str(), a.name.as_str()).cmp(&(
                b.base.as_str(),
                b.offset.as_str(),
                b.name.as_str(),
            ))
        });
        self.mmio_registers.dedup();

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
    Call {
        callee: String,
    },
    BranchIfZero {
        ty: MmioScalarType,
        then_callee: String,
        else_callee: String,
    },
    BranchIfEqImm {
        ty: MmioScalarType,
        compare_value: u64,
        then_callee: String,
        else_callee: String,
    },
    BranchIfMaskNonZeroImm {
        ty: MmioScalarType,
        mask_value: u64,
        then_callee: String,
        else_callee: String,
    },
    MmioRead {
        ty: MmioScalarType,
        addr: u64,
        capture_value: bool,
    },
    MmioWriteImm {
        ty: MmioScalarType,
        addr: u64,
        value: u64,
    },
    MmioWriteValue {
        ty: MmioScalarType,
        addr: u64,
    },
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutableExternDecl {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct ExecutableKrirModule {
    pub module_caps: Vec<String>,
    pub functions: Vec<ExecutableFunction>,
    pub extern_declarations: Vec<ExecutableExternDecl>,
    pub call_edges: Vec<CallEdge>,
}

impl ExecutableKrirModule {
    pub fn canonicalize(&mut self) {
        self.module_caps.sort();
        self.module_caps.dedup();

        self.functions.sort_by(|a, b| a.name.cmp(&b.name));
        self.extern_declarations.sort_by(|a, b| a.name.cmp(&b.name));
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
        let mut function_names = BTreeSet::new();
        let mut extern_names = BTreeSet::new();

        for function in &self.functions {
            if !function_names.insert(function.name.as_str()) {
                return Err(format!(
                    "executable KRIR has duplicate function '{}'",
                    function.name
                ));
            }

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

        for extern_decl in &self.extern_declarations {
            if !extern_names.insert(extern_decl.name.as_str()) {
                return Err(format!(
                    "executable KRIR has duplicate extern declaration '{}'",
                    extern_decl.name
                ));
            }
            if function_names.contains(extern_decl.name.as_str()) {
                return Err(format!(
                    "executable KRIR extern declaration '{}' duplicates function",
                    extern_decl.name
                ));
            }
        }

        for function in &self.functions {
            for block in &function.blocks {
                for op in &block.ops {
                    match op {
                        ExecutableOp::Call { callee } => {
                            if !function_names.contains(callee.as_str())
                                && !extern_names.contains(callee.as_str())
                            {
                                return Err(format!(
                                    "executable KRIR function '{}' calls undeclared target '{}'",
                                    function.name, callee
                                ));
                            }
                        }
                        ExecutableOp::BranchIfZero {
                            then_callee,
                            else_callee,
                            ..
                        } => {
                            for callee in [then_callee, else_callee] {
                                if !function_names.contains(callee.as_str())
                                    && !extern_names.contains(callee.as_str())
                                {
                                    return Err(format!(
                                        "executable KRIR function '{}' calls undeclared target '{}'",
                                        function.name, callee
                                    ));
                                }
                            }
                        }
                        ExecutableOp::BranchIfEqImm {
                            then_callee,
                            else_callee,
                            ..
                        } => {
                            for callee in [then_callee, else_callee] {
                                if !function_names.contains(callee.as_str())
                                    && !extern_names.contains(callee.as_str())
                                {
                                    return Err(format!(
                                        "executable KRIR function '{}' calls undeclared target '{}'",
                                        function.name, callee
                                    ));
                                }
                            }
                        }
                        ExecutableOp::BranchIfMaskNonZeroImm {
                            then_callee,
                            else_callee,
                            ..
                        } => {
                            for callee in [then_callee, else_callee] {
                                if !function_names.contains(callee.as_str())
                                    && !extern_names.contains(callee.as_str())
                                {
                                    return Err(format!(
                                        "executable KRIR function '{}' calls undeclared target '{}'",
                                        function.name, callee
                                    ));
                                }
                            }
                        }
                        ExecutableOp::MmioRead { .. }
                        | ExecutableOp::MmioWriteImm { .. }
                        | ExecutableOp::MmioWriteValue { .. } => {}
                    }
                }
            }
        }

        Ok(())
    }
}

pub fn lower_current_krir_to_executable_krir(
    module: &KrirModule,
) -> Result<ExecutableKrirModule, Vec<String>> {
    let mmio_bases = build_mmio_base_map(module)?;
    let mut lowered = ExecutableKrirModule {
        module_caps: module.module_caps.clone(),
        functions: Vec::new(),
        extern_declarations: Vec::new(),
        call_edges: module.call_edges.clone(),
    };
    let mut errors = Vec::new();

    for function in &module.functions {
        if function.is_extern {
            lowered.extern_declarations.push(ExecutableExternDecl {
                name: function.name.clone(),
            });
            continue;
        }

        let mut exec_ops = Vec::new();
        let mut executable_slot_name = None::<String>;
        let mut last_read = None::<(usize, String, MmioScalarType)>;
        for op in &function.ops {
            match op {
                KrirOp::Call { callee } => exec_ops.push(ExecutableOp::Call {
                    callee: callee.clone(),
                }),
                KrirOp::BranchIfZero {
                    slot,
                    then_callee,
                    else_callee,
                } => {
                    let Some(captured_slot) = executable_slot_name.as_deref() else {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: branch test slot '{}' requires a prior mmio_read<...>(..., {}) or raw_mmio_read<...>(..., {}) in the same function",
                            function.name,
                            format_branch_if_zero_invocation(slot, then_callee, else_callee),
                            slot,
                            slot,
                            slot
                        ));
                        continue;
                    };
                    if slot != captured_slot {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: branch test slot '{}' does not match the captured executable slot '{}' in this function",
                            function.name,
                            format_branch_if_zero_invocation(slot, then_callee, else_callee),
                            slot,
                            captured_slot
                        ));
                        continue;
                    }
                    let Some((read_index, _, read_ty)) = last_read.as_ref() else {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: branch test slot '{}' requires a prior mmio_read<...>(..., {}) or raw_mmio_read<...>(..., {}) in the same function",
                            function.name,
                            format_branch_if_zero_invocation(slot, then_callee, else_callee),
                            slot,
                            slot,
                            slot
                        ));
                        continue;
                    };
                    let Some(ExecutableOp::MmioRead { capture_value, .. }) =
                        exec_ops.get_mut(*read_index)
                    else {
                        unreachable!("branch-if-zero must point at a prior mmio read op");
                    };
                    *capture_value = true;
                    exec_ops.push(ExecutableOp::BranchIfZero {
                        ty: *read_ty,
                        then_callee: then_callee.clone(),
                        else_callee: else_callee.clone(),
                    });
                }
                KrirOp::BranchIfEq {
                    slot,
                    compare_value,
                    then_callee,
                    else_callee,
                } => {
                    let Some(captured_slot) = executable_slot_name.as_deref() else {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: branch test slot '{}' requires a prior mmio_read<...>(..., {}) or raw_mmio_read<...>(..., {}) in the same function",
                            function.name,
                            format_branch_if_eq_invocation(
                                slot,
                                compare_value,
                                then_callee,
                                else_callee
                            ),
                            slot,
                            slot,
                            slot
                        ));
                        continue;
                    };
                    if slot != captured_slot {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: branch test slot '{}' does not match the captured executable slot '{}' in this function",
                            function.name,
                            format_branch_if_eq_invocation(
                                slot,
                                compare_value,
                                then_callee,
                                else_callee
                            ),
                            slot,
                            captured_slot
                        ));
                        continue;
                    }
                    let Some((read_index, _, read_ty)) = last_read.as_ref() else {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: branch test slot '{}' requires a prior mmio_read<...>(..., {}) or raw_mmio_read<...>(..., {}) in the same function",
                            function.name,
                            format_branch_if_eq_invocation(
                                slot,
                                compare_value,
                                then_callee,
                                else_callee
                            ),
                            slot,
                            slot,
                            slot
                        ));
                        continue;
                    };
                    let resolved_compare_value =
                        match resolve_executable_branch_compare_value(*read_ty, compare_value) {
                            Ok(value) => value,
                            Err(err) => {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported {}: {}",
                                    function.name,
                                    format_branch_if_eq_invocation(
                                        slot,
                                        compare_value,
                                        then_callee,
                                        else_callee
                                    ),
                                    err
                                ));
                                continue;
                            }
                        };
                    let Some(ExecutableOp::MmioRead { capture_value, .. }) =
                        exec_ops.get_mut(*read_index)
                    else {
                        unreachable!("branch-if-eq must point at a prior mmio read op");
                    };
                    *capture_value = true;
                    exec_ops.push(ExecutableOp::BranchIfEqImm {
                        ty: *read_ty,
                        compare_value: resolved_compare_value,
                        then_callee: then_callee.clone(),
                        else_callee: else_callee.clone(),
                    });
                }
                KrirOp::BranchIfMaskNonZero {
                    slot,
                    mask_value,
                    then_callee,
                    else_callee,
                } => {
                    let Some(captured_slot) = executable_slot_name.as_deref() else {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: branch test slot '{}' requires a prior mmio_read<...>(..., {}) or raw_mmio_read<...>(..., {}) in the same function",
                            function.name,
                            format_branch_if_mask_nonzero_invocation(
                                slot,
                                mask_value,
                                then_callee,
                                else_callee
                            ),
                            slot,
                            slot,
                            slot
                        ));
                        continue;
                    };
                    if slot != captured_slot {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: branch test slot '{}' does not match the captured executable slot '{}' in this function",
                            function.name,
                            format_branch_if_mask_nonzero_invocation(
                                slot,
                                mask_value,
                                then_callee,
                                else_callee
                            ),
                            slot,
                            captured_slot
                        ));
                        continue;
                    }
                    let Some((read_index, _, read_ty)) = last_read.as_ref() else {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: branch test slot '{}' requires a prior mmio_read<...>(..., {}) or raw_mmio_read<...>(..., {}) in the same function",
                            function.name,
                            format_branch_if_mask_nonzero_invocation(
                                slot,
                                mask_value,
                                then_callee,
                                else_callee
                            ),
                            slot,
                            slot,
                            slot
                        ));
                        continue;
                    };
                    let resolved_mask_value =
                        match resolve_executable_branch_mask_value(*read_ty, mask_value) {
                            Ok(value) => value,
                            Err(err) => {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported {}: {}",
                                    function.name,
                                    format_branch_if_mask_nonzero_invocation(
                                        slot,
                                        mask_value,
                                        then_callee,
                                        else_callee
                                    ),
                                    err
                                ));
                                continue;
                            }
                        };
                    let Some(ExecutableOp::MmioRead { capture_value, .. }) =
                        exec_ops.get_mut(*read_index)
                    else {
                        unreachable!("branch-if-mask-nonzero must point at a prior mmio read op");
                    };
                    *capture_value = true;
                    exec_ops.push(ExecutableOp::BranchIfMaskNonZeroImm {
                        ty: *read_ty,
                        mask_value: resolved_mask_value,
                        then_callee: then_callee.clone(),
                        else_callee: else_callee.clone(),
                    });
                }
                KrirOp::CriticalEnter => errors.push(format!(
                    "canonical-exec: function '{}' contains unsupported critical region",
                    function.name
                )),
                KrirOp::CriticalExit => {}
                KrirOp::YieldPoint => errors.push(format!(
                    "canonical-exec: function '{}' contains unsupported yieldpoint()",
                    function.name
                )),
                KrirOp::AllocPoint => errors.push(format!(
                    "canonical-exec: function '{}' contains unsupported allocpoint()",
                    function.name
                )),
                KrirOp::BlockPoint => errors.push(format!(
                    "canonical-exec: function '{}' contains unsupported blockpoint()",
                    function.name
                )),
                KrirOp::Acquire { lock_class } => errors.push(format!(
                    "canonical-exec: function '{}' contains unsupported acquire({})",
                    function.name, lock_class
                )),
                KrirOp::Release { lock_class } => errors.push(format!(
                    "canonical-exec: function '{}' contains unsupported release({})",
                    function.name, lock_class
                )),
                KrirOp::MmioRead {
                    ty,
                    addr,
                    capture_slot,
                }
                | KrirOp::RawMmioRead {
                    ty,
                    addr,
                    capture_slot,
                } => {
                    let slot_name = capture_slot
                        .clone()
                        .unwrap_or_else(|| DEFAULT_EXECUTABLE_MMIO_SLOT.to_string());
                    if let Some(existing) = &executable_slot_name {
                        if existing != &slot_name {
                            errors.push(format!(
                                "canonical-exec: function '{}' contains unsupported {}: executable value slot '{}' conflicts with already-captured slot '{}' in the same function",
                                function.name,
                                format_mmio_read_invocation(*ty, addr, capture_slot.as_deref()),
                                slot_name,
                                existing
                            ));
                            continue;
                        }
                    } else {
                        executable_slot_name = Some(slot_name.clone());
                    }
                    match resolve_executable_mmio_addr(addr, &mmio_bases) {
                        Ok(resolved) => {
                            exec_ops.push(ExecutableOp::MmioRead {
                                ty: *ty,
                                addr: resolved,
                                capture_value: false,
                            });
                            last_read = Some((exec_ops.len() - 1, slot_name, *ty));
                        }
                        Err(reason) => errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: {}",
                            function.name,
                            format_mmio_read_invocation(*ty, addr, capture_slot.as_deref()),
                            reason
                        )),
                    }
                }
                KrirOp::MmioWrite { ty, addr, value }
                | KrirOp::RawMmioWrite { ty, addr, value } => {
                    let resolved_addr = match resolve_executable_mmio_addr(addr, &mmio_bases) {
                        Ok(resolved) => resolved,
                        Err(reason) => {
                            errors.push(format!(
                                "canonical-exec: function '{}' contains unsupported {}: {}",
                                function.name,
                                format_mmio_write_invocation(*ty, addr, value),
                                reason
                            ));
                            continue;
                        }
                    };
                    let resolved_value = match resolve_executable_mmio_write_value(
                        *ty,
                        value,
                        executable_slot_name.as_deref(),
                        last_read.as_ref().map(|(_, slot, ty)| (slot.as_str(), *ty)),
                    ) {
                        Ok(immediate) => immediate,
                        Err(reason) => {
                            errors.push(format!(
                                "canonical-exec: function '{}' contains unsupported {}: {}",
                                function.name,
                                format_mmio_write_invocation(*ty, addr, value),
                                reason
                            ));
                            continue;
                        }
                    };
                    match resolved_value {
                        ExecutableMmioWriteValue::Immediate(immediate) => {
                            exec_ops.push(ExecutableOp::MmioWriteImm {
                                ty: *ty,
                                addr: resolved_addr,
                                value: immediate,
                            });
                        }
                        ExecutableMmioWriteValue::SavedValue => {
                            let &(read_index, _, _) = last_read
                                .as_ref()
                                .expect("saved executable write must have a prior captured read");
                            let Some(ExecutableOp::MmioRead { capture_value, .. }) =
                                exec_ops.get_mut(read_index)
                            else {
                                unreachable!(
                                    "saved executable write must point at a prior mmio read op"
                                );
                            };
                            *capture_value = true;
                            exec_ops.push(ExecutableOp::MmioWriteValue {
                                ty: *ty,
                                addr: resolved_addr,
                            });
                        }
                    }
                }
            }
        }

        lowered.functions.push(ExecutableFunction {
            name: function.name.clone(),
            is_extern: false,
            signature: ExecutableSignature {
                params: vec![],
                result: ExecutableValueType::Unit,
            },
            facts: ExecutableFacts {
                ctx_ok: function.ctx_ok.clone(),
                eff_used: function.eff_used.clone(),
                caps_req: function.caps_req.clone(),
                attrs: function.attrs.clone(),
            },
            entry_block: "entry".to_string(),
            blocks: vec![ExecutableBlock {
                label: "entry".to_string(),
                ops: exec_ops,
                terminator: ExecutableTerminator::Return {
                    value: ExecutableValue::Unit,
                },
            }],
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    lowered.canonicalize();
    lowered.validate().map_err(|err| vec![err])?;
    Ok(lowered)
}

fn build_mmio_base_map(module: &KrirModule) -> Result<BTreeMap<&str, u64>, Vec<String>> {
    let mut bases = BTreeMap::new();
    let mut errors = Vec::new();
    for base in &module.mmio_bases {
        match parse_integer_literal_u64(&base.addr) {
            Ok(addr) => {
                bases.insert(base.name.as_str(), addr);
            }
            Err(reason) => errors.push(format!(
                "canonical-exec: failed to parse mmio base '{}' address '{}': {}",
                base.name, base.addr, reason
            )),
        }
    }
    if errors.is_empty() {
        Ok(bases)
    } else {
        Err(errors)
    }
}

fn resolve_executable_mmio_addr(
    addr: &MmioAddrExpr,
    mmio_bases: &BTreeMap<&str, u64>,
) -> Result<u64, String> {
    match addr {
        MmioAddrExpr::IntLiteral { value } => parse_integer_literal_u64(value),
        MmioAddrExpr::Ident { name } => mmio_bases
            .get(name.as_str())
            .copied()
            .ok_or_else(|| format!("unknown mmio base '{}'", name)),
        MmioAddrExpr::IdentPlusOffset { base, offset } => {
            let base_addr = mmio_bases
                .get(base.as_str())
                .copied()
                .ok_or_else(|| format!("unknown mmio base '{}'", base))?;
            let offset_value = parse_integer_literal_u64(offset)?;
            base_addr
                .checked_add(offset_value)
                .ok_or_else(|| format!("mmio address overflow for '{} + {}'", base, offset))
        }
    }
}

enum ExecutableMmioWriteValue {
    Immediate(u64),
    SavedValue,
}

const DEFAULT_EXECUTABLE_MMIO_SLOT: &str = "value";

fn resolve_executable_mmio_write_value(
    ty: MmioScalarType,
    value: &MmioValueExpr,
    executable_slot_name: Option<&str>,
    available_read_value: Option<(&str, MmioScalarType)>,
) -> Result<ExecutableMmioWriteValue, String> {
    match value {
        MmioValueExpr::IntLiteral { value } => {
            let parsed = parse_integer_literal_u64(value)?;
            let max_value = match ty {
                MmioScalarType::U8 => u8::MAX as u64,
                MmioScalarType::U16 => u16::MAX as u64,
                MmioScalarType::U32 => u32::MAX as u64,
                MmioScalarType::U64 => u64::MAX,
            };
            if parsed > max_value {
                Err(format!(
                    "literal value '{}' does not fit {}",
                    value,
                    ty.as_str()
                ))
            } else {
                Ok(ExecutableMmioWriteValue::Immediate(parsed))
            }
        }
        MmioValueExpr::Ident { name } => {
            let is_implicit_value = name == DEFAULT_EXECUTABLE_MMIO_SLOT;
            let Some(slot_name) = executable_slot_name else {
                return Err(if is_implicit_value {
                    format!(
                        "implicit write value '{}' requires a prior mmio_read<{}>(...) or raw_mmio_read<{}>(...) in the same function",
                        name,
                        ty.as_str(),
                        ty.as_str()
                    )
                } else {
                    format!(
                        "named write value '{}' requires a prior mmio_read<{}>(..., {}) or raw_mmio_read<{}>(..., {}) in the same function",
                        name,
                        ty.as_str(),
                        name,
                        ty.as_str(),
                        name
                    )
                });
            };
            if name != slot_name {
                return Err(format!(
                    "named write value '{}' does not match the captured executable slot '{}' in this function",
                    name, slot_name
                ));
            }
            let Some((read_slot, read_ty)) = available_read_value else {
                return Err(if is_implicit_value {
                    format!(
                        "implicit write value '{}' requires a prior mmio_read<{}>(...) or raw_mmio_read<{}>(...) in the same function",
                        name,
                        ty.as_str(),
                        ty.as_str()
                    )
                } else {
                    format!(
                        "named write value '{}' requires a prior mmio_read<{}>(..., {}) or raw_mmio_read<{}>(..., {}) in the same function",
                        name,
                        ty.as_str(),
                        name,
                        ty.as_str(),
                        name
                    )
                });
            };
            debug_assert_eq!(read_slot, slot_name);
            if read_ty != ty {
                return Err(if is_implicit_value {
                    format!(
                        "implicit write value '{}' has type {} from the prior read and does not match write type {}",
                        name,
                        read_ty.as_str(),
                        ty.as_str()
                    )
                } else {
                    format!(
                        "named write value '{}' has type {} from the prior read and does not match write type {}",
                        name,
                        read_ty.as_str(),
                        ty.as_str()
                    )
                });
            }
            Ok(ExecutableMmioWriteValue::SavedValue)
        }
    }
}

fn resolve_executable_branch_compare_value(
    ty: MmioScalarType,
    compare_value: &str,
) -> Result<u64, String> {
    let parsed = parse_integer_literal_u64(compare_value)?;
    let max_value = match ty {
        MmioScalarType::U8 => u8::MAX as u64,
        MmioScalarType::U16 => u16::MAX as u64,
        MmioScalarType::U32 => u32::MAX as u64,
        MmioScalarType::U64 => u64::MAX,
    };
    if parsed > max_value {
        Err(format!(
            "branch comparison literal '{}' does not fit {}",
            compare_value,
            ty.as_str()
        ))
    } else {
        Ok(parsed)
    }
}

fn resolve_executable_branch_mask_value(
    ty: MmioScalarType,
    mask_value: &str,
) -> Result<u64, String> {
    let parsed = parse_integer_literal_u64(mask_value)?;
    let max_value = match ty {
        MmioScalarType::U8 => u8::MAX as u64,
        MmioScalarType::U16 => u16::MAX as u64,
        MmioScalarType::U32 => u32::MAX as u64,
        MmioScalarType::U64 => u64::MAX,
    };
    if parsed > max_value {
        Err(format!(
            "branch mask literal '{}' does not fit {}",
            mask_value,
            ty.as_str()
        ))
    } else {
        Ok(parsed)
    }
}

fn parse_integer_literal_u64(raw: &str) -> Result<u64, String> {
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16)
            .map_err(|_| format!("'{}' is not a valid unsigned integer literal", raw))
    } else {
        raw.parse::<u64>()
            .map_err(|_| format!("'{}' is not a valid unsigned integer literal", raw))
    }
}

fn format_mmio_read_invocation(
    ty: MmioScalarType,
    addr: &MmioAddrExpr,
    capture_slot: Option<&str>,
) -> String {
    match capture_slot {
        Some(capture_slot) => format!(
            "mmio_read<{}>({}, {})",
            ty.as_str(),
            addr.as_source(),
            capture_slot
        ),
        None => format!("mmio_read<{}>({})", ty.as_str(), addr.as_source()),
    }
}

fn format_mmio_write_invocation(
    ty: MmioScalarType,
    addr: &MmioAddrExpr,
    value: &MmioValueExpr,
) -> String {
    format!(
        "mmio_write<{}>({}, {})",
        ty.as_str(),
        addr.as_source(),
        value.as_source()
    )
}

fn format_branch_if_zero_invocation(slot: &str, then_callee: &str, else_callee: &str) -> String {
    format!("branch_if_zero({}, {}, {})", slot, then_callee, else_callee)
}

fn format_branch_if_eq_invocation(
    slot: &str,
    compare_value: &str,
    then_callee: &str,
    else_callee: &str,
) -> String {
    format!(
        "branch_if_eq({}, {}, {}, {})",
        slot, compare_value, then_callee, else_callee
    )
}

fn format_branch_if_mask_nonzero_invocation(
    slot: &str,
    mask_value: &str,
    then_callee: &str,
    else_callee: &str,
) -> String {
    format!(
        "branch_if_mask_nonzero({}, {}, {}, {})",
        slot, mask_value, then_callee, else_callee
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendTargetId {
    X86_64Sysv,
}

impl BackendTargetId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X86_64Sysv => "x86_64-sysv",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetArch {
    X86_64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetAbi {
    Sysv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetEndian {
    Little,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum X86_64IntegerRegister {
    Rax,
    Rbx,
    Rcx,
    Rdx,
    Rsi,
    Rdi,
    Rbp,
    Rsp,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CurrentExecutableReturnConvention {
    UnitNoRegister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FutureScalarReturnConvention {
    IntegerRax,
}

impl FutureScalarReturnConvention {
    pub fn registers(self) -> &'static [X86_64IntegerRegister] {
        match self {
            Self::IntegerRax => &[X86_64IntegerRegister::Rax],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SymbolNamingConvention {
    pub function_prefix: &'static str,
    pub preserve_source_names: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SectionNamingConvention {
    pub text: &'static str,
    pub rodata: &'static str,
    pub data: &'static str,
    pub bss: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FreestandingTargetAssumptions {
    pub no_libc: bool,
    pub no_host_runtime: bool,
    pub toolchain_bridge_not_yet_exercised: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BackendTargetContract {
    pub target_id: BackendTargetId,
    pub arch: TargetArch,
    pub abi: TargetAbi,
    pub endian: TargetEndian,
    pub pointer_bits: u16,
    pub stack_alignment_bytes: u16,
    pub integer_registers: Vec<X86_64IntegerRegister>,
    pub stack_pointer: X86_64IntegerRegister,
    pub frame_pointer: X86_64IntegerRegister,
    pub instruction_pointer: &'static str,
    pub caller_saved: Vec<X86_64IntegerRegister>,
    pub callee_saved: Vec<X86_64IntegerRegister>,
    pub current_executable_return: CurrentExecutableReturnConvention,
    pub future_scalar_return: FutureScalarReturnConvention,
    pub future_argument_registers: Vec<X86_64IntegerRegister>,
    pub symbols: SymbolNamingConvention,
    pub sections: SectionNamingConvention,
    pub freestanding: FreestandingTargetAssumptions,
}

impl BackendTargetContract {
    pub fn x86_64_sysv() -> Self {
        Self {
            target_id: BackendTargetId::X86_64Sysv,
            arch: TargetArch::X86_64,
            abi: TargetAbi::Sysv,
            endian: TargetEndian::Little,
            pointer_bits: 64,
            stack_alignment_bytes: 16,
            integer_registers: vec![
                X86_64IntegerRegister::Rax,
                X86_64IntegerRegister::Rbx,
                X86_64IntegerRegister::Rcx,
                X86_64IntegerRegister::Rdx,
                X86_64IntegerRegister::Rsi,
                X86_64IntegerRegister::Rdi,
                X86_64IntegerRegister::Rbp,
                X86_64IntegerRegister::Rsp,
                X86_64IntegerRegister::R8,
                X86_64IntegerRegister::R9,
                X86_64IntegerRegister::R10,
                X86_64IntegerRegister::R11,
                X86_64IntegerRegister::R12,
                X86_64IntegerRegister::R13,
                X86_64IntegerRegister::R14,
                X86_64IntegerRegister::R15,
            ],
            stack_pointer: X86_64IntegerRegister::Rsp,
            frame_pointer: X86_64IntegerRegister::Rbp,
            instruction_pointer: "rip",
            caller_saved: vec![
                X86_64IntegerRegister::Rax,
                X86_64IntegerRegister::Rcx,
                X86_64IntegerRegister::Rdx,
                X86_64IntegerRegister::Rsi,
                X86_64IntegerRegister::Rdi,
                X86_64IntegerRegister::R8,
                X86_64IntegerRegister::R9,
                X86_64IntegerRegister::R10,
                X86_64IntegerRegister::R11,
            ],
            callee_saved: vec![
                X86_64IntegerRegister::Rbx,
                X86_64IntegerRegister::Rbp,
                X86_64IntegerRegister::R12,
                X86_64IntegerRegister::R13,
                X86_64IntegerRegister::R14,
                X86_64IntegerRegister::R15,
            ],
            current_executable_return: CurrentExecutableReturnConvention::UnitNoRegister,
            future_scalar_return: FutureScalarReturnConvention::IntegerRax,
            future_argument_registers: vec![
                X86_64IntegerRegister::Rdi,
                X86_64IntegerRegister::Rsi,
                X86_64IntegerRegister::Rdx,
                X86_64IntegerRegister::Rcx,
                X86_64IntegerRegister::R8,
                X86_64IntegerRegister::R9,
            ],
            symbols: SymbolNamingConvention {
                function_prefix: "",
                preserve_source_names: true,
            },
            sections: SectionNamingConvention {
                text: ".text",
                rodata: ".rodata",
                data: ".data",
                bss: ".bss",
            },
            freestanding: FreestandingTargetAssumptions {
                no_libc: true,
                no_host_runtime: true,
                toolchain_bridge_not_yet_exercised: true,
            },
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.target_id != BackendTargetId::X86_64Sysv {
            return Err("backend target contract target_id must be x86_64-sysv".to_string());
        }
        if self.arch != TargetArch::X86_64 {
            return Err("backend target contract arch must be x86_64".to_string());
        }
        if self.abi != TargetAbi::Sysv {
            return Err("backend target contract abi must be sysv".to_string());
        }
        if self.endian != TargetEndian::Little {
            return Err("backend target contract endian must be little".to_string());
        }
        if self.pointer_bits != 64 {
            return Err("backend target contract pointer_bits must be 64".to_string());
        }
        if self.stack_alignment_bytes != 16 {
            return Err("backend target contract stack_alignment_bytes must be 16".to_string());
        }

        let integer_set = self
            .integer_registers
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if integer_set.len() != self.integer_registers.len() {
            return Err(
                "backend target contract integer_registers must not contain duplicates".to_string(),
            );
        }
        if !integer_set.contains(&self.stack_pointer) {
            return Err(
                "backend target contract stack_pointer must be in integer_registers".to_string(),
            );
        }
        if !integer_set.contains(&self.frame_pointer) {
            return Err(
                "backend target contract frame_pointer must be in integer_registers".to_string(),
            );
        }

        let caller_saved = self.caller_saved.iter().copied().collect::<BTreeSet<_>>();
        let callee_saved = self.callee_saved.iter().copied().collect::<BTreeSet<_>>();
        if caller_saved.len() != self.caller_saved.len() {
            return Err(
                "backend target contract caller_saved must not contain duplicates".to_string(),
            );
        }
        if callee_saved.len() != self.callee_saved.len() {
            return Err(
                "backend target contract callee_saved must not contain duplicates".to_string(),
            );
        }
        if !caller_saved.is_disjoint(&callee_saved) {
            return Err(
                "backend target contract caller_saved and callee_saved must be disjoint"
                    .to_string(),
            );
        }
        if !caller_saved.is_subset(&integer_set) {
            return Err(
                "backend target contract caller_saved must be a subset of integer_registers"
                    .to_string(),
            );
        }
        if !callee_saved.is_subset(&integer_set) {
            return Err(
                "backend target contract callee_saved must be a subset of integer_registers"
                    .to_string(),
            );
        }

        for reg in self.future_scalar_return.registers() {
            if !integer_set.contains(reg) {
                return Err(
                    "backend target contract future_scalar_return must resolve to integer_registers"
                        .to_string(),
                );
            }
        }
        for reg in &self.future_argument_registers {
            if !integer_set.contains(reg) {
                return Err(
                    "backend target contract future_argument_registers must be a subset of integer_registers"
                        .to_string(),
                );
            }
        }

        if self.symbols.function_prefix.contains(char::is_whitespace) {
            return Err(
                "backend target contract function_prefix must not contain whitespace".to_string(),
            );
        }
        if self.sections.text.is_empty()
            || self.sections.rodata.is_empty()
            || self.sections.data.is_empty()
            || self.sections.bss.is_empty()
        {
            return Err("backend target contract sections must not be empty".to_string());
        }
        if !self.freestanding.no_libc || !self.freestanding.no_host_runtime {
            return Err("backend target contract must remain freestanding in v0.1".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64AsmModule {
    pub section: &'static str,
    pub functions: Vec<X86_64AsmFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64AsmFunction {
    pub symbol: String,
    pub uses_saved_value_slot: bool,
    pub instructions: Vec<X86_64AsmInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum X86_64AsmInstruction {
    Call {
        symbol: String,
    },
    BranchIfZero {
        ty: MmioScalarType,
        then_symbol: String,
        else_symbol: String,
    },
    BranchIfEqImm {
        ty: MmioScalarType,
        compare_value: u64,
        then_symbol: String,
        else_symbol: String,
    },
    BranchIfMaskNonZeroImm {
        ty: MmioScalarType,
        mask_value: u64,
        then_symbol: String,
        else_symbol: String,
    },
    MmioRead {
        ty: MmioScalarType,
        addr: u64,
        capture_value: bool,
    },
    MmioWriteImm {
        ty: MmioScalarType,
        addr: u64,
        value: u64,
    },
    MmioWriteValue {
        ty: MmioScalarType,
        addr: u64,
    },
    Ret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilerOwnedObjectKind {
    LinearRelocatable,
}

impl CompilerOwnedObjectKind {
    fn tag(self) -> u8 {
        match self {
            Self::LinearRelocatable => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompilerOwnedObjectHeader {
    pub magic: [u8; 4],
    pub version_major: u8,
    pub version_minor: u8,
    pub object_kind: CompilerOwnedObjectKind,
    pub target_id: BackendTargetId,
    pub endian: TargetEndian,
    pub pointer_bits: u16,
    pub format_revision: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompilerOwnedCodeSection {
    pub name: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilerOwnedObjectSymbolKind {
    Function,
}

impl CompilerOwnedObjectSymbolKind {
    fn tag(self) -> u8 {
        match self {
            Self::Function => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilerOwnedObjectSymbolDefinition {
    DefinedText,
    UndefinedExternal,
}

impl CompilerOwnedObjectSymbolDefinition {
    fn tag(self) -> u8 {
        match self {
            Self::DefinedText => 1,
            Self::UndefinedExternal => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompilerOwnedObjectSymbol {
    pub name: String,
    pub kind: CompilerOwnedObjectSymbolKind,
    pub definition: CompilerOwnedObjectSymbolDefinition,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilerOwnedFixupKind {
    X86_64CallRel32,
}

impl CompilerOwnedFixupKind {
    fn tag(self) -> u8 {
        match self {
            Self::X86_64CallRel32 => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompilerOwnedObjectFixup {
    pub source_symbol: String,
    pub patch_offset: u64,
    pub kind: CompilerOwnedFixupKind,
    pub target_symbol: String,
    pub width_bytes: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompilerOwnedObject {
    pub header: CompilerOwnedObjectHeader,
    pub code: CompilerOwnedCodeSection,
    pub symbols: Vec<CompilerOwnedObjectSymbol>,
    pub fixups: Vec<CompilerOwnedObjectFixup>,
}

impl CompilerOwnedObject {
    pub fn validate(&self) -> Result<(), String> {
        if self.header.magic != *b"KRBO" {
            return Err("compiler-owned object magic must be KRBO".to_string());
        }
        if self.header.version_major != 0 || self.header.version_minor != 1 {
            return Err("compiler-owned object version must be 0.1".to_string());
        }
        if self.header.format_revision == 0 {
            return Err("compiler-owned object format_revision must be non-zero".to_string());
        }
        if self.code.name.is_empty() {
            return Err("compiler-owned object code section name must not be empty".to_string());
        }

        let mut last_symbol_name: Option<&str> = None;
        let mut symbol_names = BTreeSet::new();
        let mut symbol_defs = BTreeMap::new();
        for symbol in &self.symbols {
            if symbol.name.is_empty() {
                return Err("compiler-owned object symbol name must not be empty".to_string());
            }
            if !symbol_names.insert(symbol.name.as_str()) {
                return Err(format!(
                    "compiler-owned object symbol '{}' must be unique",
                    symbol.name
                ));
            }
            if let Some(prev) = last_symbol_name
                && symbol.name.as_str() < prev
            {
                return Err("compiler-owned object symbols must be sorted by name".to_string());
            }
            last_symbol_name = Some(symbol.name.as_str());
            match symbol.definition {
                CompilerOwnedObjectSymbolDefinition::DefinedText => {
                    if symbol.offset + symbol.size > self.code.bytes.len() as u64 {
                        return Err(format!(
                            "compiler-owned object symbol '{}' exceeds code section bounds",
                            symbol.name
                        ));
                    }
                }
                CompilerOwnedObjectSymbolDefinition::UndefinedExternal => {
                    if symbol.offset != 0 || symbol.size != 0 {
                        return Err(format!(
                            "compiler-owned object undefined external symbol '{}' must have zero offset and size",
                            symbol.name
                        ));
                    }
                }
            }
            symbol_defs.insert(symbol.name.as_str(), symbol.definition);
        }

        let mut last_fixup_key: Option<(u64, &str, &str)> = None;
        for fixup in &self.fixups {
            let Some(source_def) = symbol_defs.get(fixup.source_symbol.as_str()) else {
                return Err(format!(
                    "compiler-owned object fixup source symbol '{}' must exist",
                    fixup.source_symbol
                ));
            };
            if *source_def != CompilerOwnedObjectSymbolDefinition::DefinedText {
                return Err(format!(
                    "compiler-owned object fixup source symbol '{}' must be defined in text",
                    fixup.source_symbol
                ));
            }
            if !symbol_names.contains(fixup.target_symbol.as_str()) {
                return Err(format!(
                    "compiler-owned object fixup target symbol '{}' must exist",
                    fixup.target_symbol
                ));
            }
            if fixup.width_bytes == 0 {
                return Err("compiler-owned object fixup width_bytes must be non-zero".to_string());
            }
            if fixup.patch_offset + u64::from(fixup.width_bytes) > self.code.bytes.len() as u64 {
                return Err(format!(
                    "compiler-owned object fixup for target '{}' exceeds code section bounds",
                    fixup.target_symbol
                ));
            }

            let key = (
                fixup.patch_offset,
                fixup.source_symbol.as_str(),
                fixup.target_symbol.as_str(),
            );
            if let Some(prev) = last_fixup_key
                && key < prev
            {
                return Err(
                    "compiler-owned object fixups must be sorted by patch offset and symbol"
                        .to_string(),
                );
            }
            last_fixup_key = Some(key);
        }

        Ok(())
    }
}

fn validate_executable_krir_linear_structure(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
    lowering_name: &str,
) -> Result<(), String> {
    target.validate()?;
    if target.target_id != BackendTargetId::X86_64Sysv
        || target.arch != TargetArch::X86_64
        || target.abi != TargetAbi::Sysv
    {
        return Err(format!(
            "{lowering_name} requires x86_64-sysv target contract"
        ));
    }

    module.validate()?;

    for function in &module.functions {
        if function.blocks.len() != 1 {
            return Err(format!(
                "{lowering_name} requires exactly one block in function '{}'",
                function.name
            ));
        }
        let entry = &function.blocks[0];
        if entry.label != function.entry_block {
            return Err(format!(
                "{lowering_name} requires entry block '{}' to be first in function '{}'",
                function.entry_block, function.name
            ));
        }
    }

    Ok(())
}

pub fn validate_compiler_owned_object_for_x86_64_asm_export(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<(), String> {
    object.validate()?;
    if object.header.target_id != target.target_id {
        return Err("x86_64 asm export target_id mismatch".to_string());
    }
    if object.header.endian != target.endian {
        return Err("x86_64 asm export endianness mismatch".to_string());
    }
    if object.header.pointer_bits != target.pointer_bits {
        return Err("x86_64 asm export pointer width mismatch".to_string());
    }
    if object.code.name != target.sections.text {
        return Err(
            "x86_64 asm export requires code section to match target text section".to_string(),
        );
    }

    let symbol_defs = object
        .symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.definition))
        .collect::<BTreeMap<_, _>>();

    for fixup in &object.fixups {
        if fixup.kind != CompilerOwnedFixupKind::X86_64CallRel32 {
            return Err(format!(
                "x86_64 asm export requires x86_64_call_rel32 fixups, found {:?}",
                fixup.kind
            ));
        }
        if fixup.width_bytes != 4 {
            return Err(format!(
                "x86_64 asm export requires rel32 fixup width 4 for target '{}'",
                fixup.target_symbol
            ));
        }
        let Some(target_def) = symbol_defs.get(fixup.target_symbol.as_str()) else {
            return Err(format!(
                "x86_64 asm export requires target symbol '{}' for fixup",
                fixup.target_symbol
            ));
        };
        match target_def {
            CompilerOwnedObjectSymbolDefinition::DefinedText
            | CompilerOwnedObjectSymbolDefinition::UndefinedExternal => {}
        }
    }

    Ok(())
}

pub fn export_compiler_owned_object_to_x86_64_asm(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<X86_64AsmModule, String> {
    validate_compiler_owned_object_for_x86_64_asm_export(object, target)?;

    let fixups_by_source = object
        .fixups
        .iter()
        .map(|fixup| {
            (
                (fixup.source_symbol.as_str(), fixup.patch_offset),
                fixup.target_symbol.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let functions = object
        .symbols
        .iter()
        .filter(|symbol| symbol.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|symbol| {
            let start = usize::try_from(symbol.offset).expect("symbol offset must fit usize");
            let end =
                usize::try_from(symbol.offset + symbol.size).expect("symbol end must fit usize");
            let bytes = &object.code.bytes[start..end];
            let mut instructions = Vec::new();
            let uses_saved_value_slot = matches!(bytes.first(), Some(0x53))
                && bytes.len() >= 3
                && bytes[bytes.len() - 2] == 0x5B
                && bytes[bytes.len() - 1] == 0xC3;
            let mut cursor = if uses_saved_value_slot { 1usize } else { 0usize };
            let end = if uses_saved_value_slot {
                bytes.len() - 2
            } else {
                bytes.len()
            };
            while cursor < end {
                if !uses_saved_value_slot && cursor == bytes.len() - 1 && bytes[cursor] == 0xC3 {
                    instructions.push(X86_64AsmInstruction::Ret);
                    cursor += 1;
                    continue;
                }

                if cursor + 5 <= bytes.len() && bytes[cursor] == 0xE8 {
                    if bytes[cursor + 1..cursor + 5] != [0, 0, 0, 0] {
                        return Err(format!(
                            "x86_64 asm export requires zeroed rel32 bytes for call in function '{}'",
                            symbol.name
                        ));
                    }
                    let patch_offset = symbol.offset + cursor as u64 + 1;
                    let target_symbol = fixups_by_source
                        .get(&(symbol.name.as_str(), patch_offset))
                        .ok_or_else(|| {
                            format!(
                                "x86_64 asm export requires fixup for call at offset {} in function '{}'",
                                patch_offset, symbol.name
                            )
                        })?;
                    instructions.push(X86_64AsmInstruction::Call {
                        symbol: (*target_symbol).to_string(),
                    });
                    cursor += 5;
                    continue;
                }

                if let Some((instruction, consumed)) =
                    decode_x86_64_mmio_instruction(bytes, cursor, end)?
                {
                    instructions.push(instruction);
                    cursor += consumed;
                    continue;
                }

                return Err(format!(
                    "x86_64 asm export encountered unsupported code byte 0x{:02x} in function '{}' at offset {}",
                    bytes[cursor], symbol.name, cursor
                ));
            }

            if uses_saved_value_slot {
                instructions.push(X86_64AsmInstruction::Ret);
            }

            Ok(X86_64AsmFunction {
                symbol: symbol.name.clone(),
                uses_saved_value_slot,
                instructions,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(X86_64AsmModule {
        section: target.sections.text,
        functions,
    })
}

pub fn lower_executable_krir_to_x86_64_asm(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<X86_64AsmModule, String> {
    validate_compiler_owned_object_linear_subset(module, target)?;

    let mut canonical = module.clone();
    canonical.canonicalize();
    let functions = canonical
        .functions
        .iter()
        .map(|function| X86_64AsmFunction {
            symbol: function.name.clone(),
            uses_saved_value_slot: executable_function_uses_saved_value_slot(function),
            instructions: function.blocks[0]
                .ops
                .iter()
                .cloned()
                .map(|op| match op {
                    ExecutableOp::Call { callee } => X86_64AsmInstruction::Call { symbol: callee },
                    ExecutableOp::BranchIfZero {
                        ty,
                        then_callee,
                        else_callee,
                    } => X86_64AsmInstruction::BranchIfZero {
                        ty,
                        then_symbol: then_callee,
                        else_symbol: else_callee,
                    },
                    ExecutableOp::BranchIfEqImm {
                        ty,
                        compare_value,
                        then_callee,
                        else_callee,
                    } => X86_64AsmInstruction::BranchIfEqImm {
                        ty,
                        compare_value,
                        then_symbol: then_callee,
                        else_symbol: else_callee,
                    },
                    ExecutableOp::BranchIfMaskNonZeroImm {
                        ty,
                        mask_value,
                        then_callee,
                        else_callee,
                    } => X86_64AsmInstruction::BranchIfMaskNonZeroImm {
                        ty,
                        mask_value,
                        then_symbol: then_callee,
                        else_symbol: else_callee,
                    },
                    ExecutableOp::MmioRead {
                        ty,
                        addr,
                        capture_value,
                    } => X86_64AsmInstruction::MmioRead {
                        ty,
                        addr,
                        capture_value,
                    },
                    ExecutableOp::MmioWriteImm { ty, addr, value } => {
                        X86_64AsmInstruction::MmioWriteImm { ty, addr, value }
                    }
                    ExecutableOp::MmioWriteValue { ty, addr } => {
                        X86_64AsmInstruction::MmioWriteValue { ty, addr }
                    }
                })
                .chain(std::iter::once(X86_64AsmInstruction::Ret))
                .collect(),
        })
        .collect();

    Ok(X86_64AsmModule {
        section: target.sections.text,
        functions,
    })
}

fn executable_function_uses_saved_value_slot(function: &ExecutableFunction) -> bool {
    function.blocks.iter().any(|block| {
        block.ops.iter().any(|op| {
            matches!(
                op,
                ExecutableOp::MmioRead {
                    capture_value: true,
                    ..
                } | ExecutableOp::MmioWriteValue { .. }
            )
        })
    })
}

pub fn emit_x86_64_asm_text(module: &X86_64AsmModule) -> String {
    let mut out = String::new();
    out.push_str(module.section);
    out.push('\n');
    for function in &module.functions {
        let mut branch_index = 0usize;
        out.push('\n');
        out.push_str(".globl ");
        out.push_str(&function.symbol);
        out.push('\n');
        out.push_str(&function.symbol);
        out.push_str(":\n");
        if function.uses_saved_value_slot {
            out.push_str("    push %rbx\n");
        }
        for instruction in &function.instructions {
            match instruction {
                X86_64AsmInstruction::Call { symbol } => {
                    out.push_str("    call ");
                    out.push_str(symbol);
                    out.push('\n');
                }
                X86_64AsmInstruction::BranchIfZero {
                    ty,
                    then_symbol,
                    else_symbol,
                } => {
                    let else_label = format!(".L{}_branch_{}_else", function.symbol, branch_index);
                    let end_label = format!(".L{}_branch_{}_end", function.symbol, branch_index);
                    out.push_str("    ");
                    out.push_str(mmio_saved_value_zero_test_mnemonic(*ty));
                    out.push('\n');
                    out.push_str("    jne ");
                    out.push_str(&else_label);
                    out.push('\n');
                    out.push_str("    call ");
                    out.push_str(then_symbol);
                    out.push('\n');
                    out.push_str("    jmp ");
                    out.push_str(&end_label);
                    out.push('\n');
                    out.push_str(&else_label);
                    out.push_str(":\n");
                    out.push_str("    call ");
                    out.push_str(else_symbol);
                    out.push('\n');
                    out.push_str(&end_label);
                    out.push_str(":\n");
                    branch_index += 1;
                }
                X86_64AsmInstruction::BranchIfEqImm {
                    ty,
                    compare_value,
                    then_symbol,
                    else_symbol,
                } => {
                    let else_label = format!(".L{}_branch_{}_else", function.symbol, branch_index);
                    let end_label = format!(".L{}_branch_{}_end", function.symbol, branch_index);
                    out.push_str("    ");
                    out.push_str(&mmio_accumulator_immediate_mnemonic(*ty, *compare_value));
                    out.push('\n');
                    out.push_str("    ");
                    out.push_str(mmio_saved_value_compare_mnemonic(*ty));
                    out.push('\n');
                    out.push_str("    jne ");
                    out.push_str(&else_label);
                    out.push('\n');
                    out.push_str("    call ");
                    out.push_str(then_symbol);
                    out.push('\n');
                    out.push_str("    jmp ");
                    out.push_str(&end_label);
                    out.push('\n');
                    out.push_str(&else_label);
                    out.push_str(":\n");
                    out.push_str("    call ");
                    out.push_str(else_symbol);
                    out.push('\n');
                    out.push_str(&end_label);
                    out.push_str(":\n");
                    branch_index += 1;
                }
                X86_64AsmInstruction::BranchIfMaskNonZeroImm {
                    ty,
                    mask_value,
                    then_symbol,
                    else_symbol,
                } => {
                    let else_label = format!(".L{}_branch_{}_else", function.symbol, branch_index);
                    let end_label = format!(".L{}_branch_{}_end", function.symbol, branch_index);
                    out.push_str("    ");
                    out.push_str(&mmio_accumulator_immediate_mnemonic(*ty, *mask_value));
                    out.push('\n');
                    out.push_str("    ");
                    out.push_str(mmio_saved_value_mask_test_mnemonic(*ty));
                    out.push('\n');
                    out.push_str("    je ");
                    out.push_str(&else_label);
                    out.push('\n');
                    out.push_str("    call ");
                    out.push_str(then_symbol);
                    out.push('\n');
                    out.push_str("    jmp ");
                    out.push_str(&end_label);
                    out.push('\n');
                    out.push_str(&else_label);
                    out.push_str(":\n");
                    out.push_str("    call ");
                    out.push_str(else_symbol);
                    out.push('\n');
                    out.push_str(&end_label);
                    out.push_str(":\n");
                    branch_index += 1;
                }
                X86_64AsmInstruction::MmioRead {
                    ty,
                    addr,
                    capture_value,
                } => {
                    out.push_str("    movabs $");
                    out.push_str(&format_hex_u64(*addr));
                    out.push_str(", %rax\n");
                    out.push_str("    ");
                    out.push_str(mmio_load_mnemonic(*ty));
                    out.push_str(" (%rax), ");
                    out.push_str(mmio_accumulator_register(*ty));
                    out.push('\n');
                    if *capture_value {
                        out.push_str("    ");
                        out.push_str(mmio_move_saved_value_mnemonic(*ty));
                        out.push(' ');
                        out.push_str(mmio_accumulator_register(*ty));
                        out.push_str(", ");
                        out.push_str(mmio_saved_value_register(*ty));
                        out.push('\n');
                    }
                }
                X86_64AsmInstruction::MmioWriteImm { ty, addr, value } => {
                    out.push_str("    movabs $");
                    out.push_str(&format_hex_u64(*addr));
                    out.push_str(", %rax\n");
                    out.push_str("    ");
                    out.push_str(&mmio_immediate_mnemonic(*ty, *value));
                    out.push('\n');
                    out.push_str("    ");
                    out.push_str(mmio_store_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_value_register(*ty));
                    out.push_str(", (%rax)\n");
                }
                X86_64AsmInstruction::MmioWriteValue { ty, addr } => {
                    out.push_str("    movabs $");
                    out.push_str(&format_hex_u64(*addr));
                    out.push_str(", %rax\n");
                    out.push_str("    ");
                    out.push_str(mmio_store_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_saved_value_register(*ty));
                    out.push_str(", (%rax)\n");
                }
                X86_64AsmInstruction::Ret => {
                    if function.uses_saved_value_slot {
                        out.push_str("    pop %rbx\n");
                    }
                    out.push_str("    ret\n");
                }
            }
        }
    }
    out
}

fn decode_x86_64_mmio_instruction(
    bytes: &[u8],
    cursor: usize,
    end: usize,
) -> Result<Option<(X86_64AsmInstruction, usize)>, String> {
    let Some(prefix) = bytes.get(cursor..cursor + 2) else {
        return Ok(None);
    };
    if prefix != [0x48, 0xB8] {
        return Ok(None);
    }
    if cursor + 10 > end {
        return Ok(None);
    }
    let addr = u64::from_le_bytes(
        bytes[cursor + 2..cursor + 10]
            .try_into()
            .expect("movabs immediate must fit u64"),
    );
    let rest = &bytes[cursor + 10..end];

    for (ty, load, copy) in [
        (MmioScalarType::U8, &[0x8A, 0x00][..], &[0x88, 0xC3][..]),
        (
            MmioScalarType::U16,
            &[0x66, 0x8B, 0x00][..],
            &[0x66, 0x89, 0xC3][..],
        ),
        (MmioScalarType::U32, &[0x8B, 0x00][..], &[0x89, 0xC3][..]),
        (
            MmioScalarType::U64,
            &[0x48, 0x8B, 0x00][..],
            &[0x48, 0x89, 0xC3][..],
        ),
    ] {
        if rest.starts_with(load) {
            let consumed = 10 + load.len();
            if rest[load.len()..].starts_with(copy) {
                return Ok(Some((
                    X86_64AsmInstruction::MmioRead {
                        ty,
                        addr,
                        capture_value: true,
                    },
                    consumed + copy.len(),
                )));
            }
            return Ok(Some((
                X86_64AsmInstruction::MmioRead {
                    ty,
                    addr,
                    capture_value: false,
                },
                consumed,
            )));
        }
    }

    for (ty, imm, store) in [
        (MmioScalarType::U8, &[0xB1][..], &[0x88, 0x08][..]),
        (
            MmioScalarType::U16,
            &[0x66, 0xB9][..],
            &[0x66, 0x89, 0x08][..],
        ),
        (MmioScalarType::U32, &[0xB9][..], &[0x89, 0x08][..]),
        (
            MmioScalarType::U64,
            &[0x48, 0xB9][..],
            &[0x48, 0x89, 0x08][..],
        ),
    ] {
        if rest.starts_with(imm) {
            let immediate_bytes = match ty {
                MmioScalarType::U8 => 1usize,
                MmioScalarType::U16 => 2usize,
                MmioScalarType::U32 => 4usize,
                MmioScalarType::U64 => 8usize,
            };
            if rest.len() < imm.len() + immediate_bytes + store.len() {
                return Ok(None);
            }
            let value_offset = loadless_value_offset(cursor, imm.len());
            let value = match ty {
                MmioScalarType::U8 => bytes[value_offset] as u64,
                MmioScalarType::U16 => u16::from_le_bytes(
                    bytes[value_offset..value_offset + 2]
                        .try_into()
                        .expect("u16 immediate bytes"),
                ) as u64,
                MmioScalarType::U32 => u32::from_le_bytes(
                    bytes[value_offset..value_offset + 4]
                        .try_into()
                        .expect("u32 immediate bytes"),
                ) as u64,
                MmioScalarType::U64 => u64::from_le_bytes(
                    bytes[value_offset..value_offset + 8]
                        .try_into()
                        .expect("u64 immediate bytes"),
                ),
            };
            let store_offset = value_offset + immediate_bytes;
            if &bytes[store_offset..store_offset + store.len()] == store {
                return Ok(Some((
                    X86_64AsmInstruction::MmioWriteImm { ty, addr, value },
                    10 + imm.len() + immediate_bytes + store.len(),
                )));
            }
        }
    }

    for (ty, store) in [
        (MmioScalarType::U8, &[0x88, 0x18][..]),
        (MmioScalarType::U16, &[0x66, 0x89, 0x18][..]),
        (MmioScalarType::U32, &[0x89, 0x18][..]),
        (MmioScalarType::U64, &[0x48, 0x89, 0x18][..]),
    ] {
        if rest.starts_with(store) {
            return Ok(Some((
                X86_64AsmInstruction::MmioWriteValue { ty, addr },
                10 + store.len(),
            )));
        }
    }

    Ok(None)
}

fn loadless_value_offset(cursor: usize, opcode_bytes: usize) -> usize {
    cursor + 10 + opcode_bytes
}

fn executable_op_encoded_len(op: &ExecutableOp) -> u64 {
    match op {
        ExecutableOp::Call { .. } => 5,
        ExecutableOp::BranchIfZero { ty, .. } => mmio_saved_value_zero_test_bytes(*ty) + 16,
        ExecutableOp::BranchIfEqImm {
            ty,
            compare_value: _,
            ..
        } => mmio_saved_value_literal_compare_bytes(*ty) + 16,
        ExecutableOp::BranchIfMaskNonZeroImm {
            ty, mask_value: _, ..
        } => mmio_saved_value_literal_mask_test_bytes(*ty) + 16,
        ExecutableOp::MmioRead {
            ty, capture_value, ..
        } => {
            10 + mmio_load_bytes(*ty)
                + if *capture_value {
                    mmio_saved_value_copy_bytes(*ty)
                } else {
                    0
                }
        }
        ExecutableOp::MmioWriteImm { ty, .. } => {
            10 + mmio_value_load_immediate_bytes(*ty) + mmio_store_bytes(*ty)
        }
        ExecutableOp::MmioWriteValue { ty, .. } => 10 + mmio_saved_value_store_bytes(*ty),
    }
}

fn encode_mmio_read_bytes(out: &mut Vec<u8>, ty: MmioScalarType, addr: u64, capture_value: bool) {
    push_movabs_rax_imm64(out, addr);
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x00]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x00]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x00]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x00]),
    }
    if capture_value {
        push_mov_accumulator_to_saved_value_register(out, ty);
    }
}

fn encode_mmio_write_imm_bytes(out: &mut Vec<u8>, ty: MmioScalarType, addr: u64, value: u64) {
    push_movabs_rax_imm64(out, addr);
    push_mov_imm_to_value_register(out, ty, value);
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x08]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x08]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x08]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x08]),
    }
}

fn encode_mmio_write_saved_value_bytes(out: &mut Vec<u8>, ty: MmioScalarType, addr: u64) {
    push_movabs_rax_imm64(out, addr);
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x18]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x18]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x18]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x18]),
    }
}

fn encode_branch_if_zero_bytes(out: &mut Vec<u8>, ty: MmioScalarType) {
    push_test_saved_value_register_zero(out, ty);
    out.extend_from_slice(&[0x0F, 0x85, 0x0A, 0x00, 0x00, 0x00]);
    out.push(0xE8);
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&[0xE9, 0x05, 0x00, 0x00, 0x00]);
    out.push(0xE8);
    out.extend_from_slice(&[0, 0, 0, 0]);
}

fn encode_branch_if_eq_imm_bytes(out: &mut Vec<u8>, ty: MmioScalarType, compare_value: u64) {
    push_mov_imm_to_accumulator_register(out, ty, compare_value);
    push_cmp_accumulator_to_saved_value_register(out, ty);
    out.extend_from_slice(&[0x0F, 0x85, 0x0A, 0x00, 0x00, 0x00]);
    out.push(0xE8);
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&[0xE9, 0x05, 0x00, 0x00, 0x00]);
    out.push(0xE8);
    out.extend_from_slice(&[0, 0, 0, 0]);
}

fn encode_branch_if_mask_nonzero_imm_bytes(out: &mut Vec<u8>, ty: MmioScalarType, mask_value: u64) {
    push_mov_imm_to_accumulator_register(out, ty, mask_value);
    push_test_accumulator_with_saved_value_register(out, ty);
    out.extend_from_slice(&[0x0F, 0x84, 0x0A, 0x00, 0x00, 0x00]);
    out.push(0xE8);
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&[0xE9, 0x05, 0x00, 0x00, 0x00]);
    out.push(0xE8);
    out.extend_from_slice(&[0, 0, 0, 0]);
}

fn push_movabs_rax_imm64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&[0x48, 0xB8]);
    push_u64_le(out, value);
}

fn push_mov_imm_to_accumulator_register(out: &mut Vec<u8>, ty: MmioScalarType, value: u64) {
    match ty {
        MmioScalarType::U8 => {
            out.push(0xB0);
            out.push(value as u8);
        }
        MmioScalarType::U16 => {
            out.extend_from_slice(&[0x66, 0xB8]);
            push_u16_le(out, value as u16);
        }
        MmioScalarType::U32 => {
            out.push(0xB8);
            push_u32_le(out, value as u32);
        }
        MmioScalarType::U64 => {
            out.extend_from_slice(&[0x48, 0xB8]);
            push_u64_le(out, value);
        }
    }
}

fn push_mov_imm_to_value_register(out: &mut Vec<u8>, ty: MmioScalarType, value: u64) {
    match ty {
        MmioScalarType::U8 => {
            out.push(0xB1);
            out.push(value as u8);
        }
        MmioScalarType::U16 => {
            out.extend_from_slice(&[0x66, 0xB9]);
            push_u16_le(out, value as u16);
        }
        MmioScalarType::U32 => {
            out.push(0xB9);
            push_u32_le(out, value as u32);
        }
        MmioScalarType::U64 => {
            out.extend_from_slice(&[0x48, 0xB9]);
            push_u64_le(out, value);
        }
    }
}

fn push_mov_accumulator_to_saved_value_register(out: &mut Vec<u8>, ty: MmioScalarType) {
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0xC3]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0xC3]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0xC3]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0xC3]),
    }
}

fn push_test_saved_value_register_zero(out: &mut Vec<u8>, ty: MmioScalarType) {
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x84, 0xDB]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x85, 0xDB]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x85, 0xDB]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x85, 0xDB]),
    }
}

fn push_cmp_accumulator_to_saved_value_register(out: &mut Vec<u8>, ty: MmioScalarType) {
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x38, 0xC3]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x39, 0xC3]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x39, 0xC3]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x39, 0xC3]),
    }
}

fn push_test_accumulator_with_saved_value_register(out: &mut Vec<u8>, ty: MmioScalarType) {
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x84, 0xC3]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x85, 0xC3]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x85, 0xC3]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x85, 0xC3]),
    }
}

fn mmio_load_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 | MmioScalarType::U32 => 2,
        MmioScalarType::U16 | MmioScalarType::U64 => 3,
    }
}

fn mmio_value_load_immediate_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 => 2,
        MmioScalarType::U16 => 4,
        MmioScalarType::U32 => 5,
        MmioScalarType::U64 => 10,
    }
}

fn mmio_saved_value_copy_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 | MmioScalarType::U32 => 2,
        MmioScalarType::U16 | MmioScalarType::U64 => 3,
    }
}

fn mmio_saved_value_zero_test_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 | MmioScalarType::U32 => 2,
        MmioScalarType::U16 | MmioScalarType::U64 => 3,
    }
}

fn mmio_accumulator_immediate_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 => 2,
        MmioScalarType::U16 => 4,
        MmioScalarType::U32 => 5,
        MmioScalarType::U64 => 10,
    }
}

fn mmio_saved_value_compare_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 | MmioScalarType::U32 => 2,
        MmioScalarType::U16 | MmioScalarType::U64 => 3,
    }
}

fn mmio_saved_value_literal_compare_bytes(ty: MmioScalarType) -> u64 {
    mmio_accumulator_immediate_bytes(ty) + mmio_saved_value_compare_bytes(ty)
}

fn mmio_saved_value_literal_mask_test_bytes(ty: MmioScalarType) -> u64 {
    mmio_accumulator_immediate_bytes(ty) + mmio_saved_value_compare_bytes(ty)
}

fn mmio_store_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 | MmioScalarType::U32 => 2,
        MmioScalarType::U16 | MmioScalarType::U64 => 3,
    }
}

fn mmio_saved_value_store_bytes(ty: MmioScalarType) -> u64 {
    mmio_store_bytes(ty)
}

fn mmio_accumulator_register(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "%al",
        MmioScalarType::U16 => "%ax",
        MmioScalarType::U32 => "%eax",
        MmioScalarType::U64 => "%rax",
    }
}

fn mmio_value_register(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "%cl",
        MmioScalarType::U16 => "%cx",
        MmioScalarType::U32 => "%ecx",
        MmioScalarType::U64 => "%rcx",
    }
}

fn mmio_saved_value_register(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "%bl",
        MmioScalarType::U16 => "%bx",
        MmioScalarType::U32 => "%ebx",
        MmioScalarType::U64 => "%rbx",
    }
}

fn mmio_load_mnemonic(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "movb",
        MmioScalarType::U16 => "movw",
        MmioScalarType::U32 => "movl",
        MmioScalarType::U64 => "movq",
    }
}

fn mmio_store_mnemonic(ty: MmioScalarType) -> &'static str {
    mmio_load_mnemonic(ty)
}

fn mmio_move_saved_value_mnemonic(ty: MmioScalarType) -> &'static str {
    mmio_load_mnemonic(ty)
}

fn mmio_saved_value_zero_test_mnemonic(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "testb %bl, %bl",
        MmioScalarType::U16 => "testw %bx, %bx",
        MmioScalarType::U32 => "testl %ebx, %ebx",
        MmioScalarType::U64 => "testq %rbx, %rbx",
    }
}

fn mmio_saved_value_compare_mnemonic(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "cmpb %al, %bl",
        MmioScalarType::U16 => "cmpw %ax, %bx",
        MmioScalarType::U32 => "cmpl %eax, %ebx",
        MmioScalarType::U64 => "cmpq %rax, %rbx",
    }
}

fn mmio_saved_value_mask_test_mnemonic(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "testb %al, %bl",
        MmioScalarType::U16 => "testw %ax, %bx",
        MmioScalarType::U32 => "testl %eax, %ebx",
        MmioScalarType::U64 => "testq %rax, %rbx",
    }
}

fn mmio_accumulator_immediate_mnemonic(ty: MmioScalarType, value: u64) -> String {
    let mnemonic = match ty {
        MmioScalarType::U8 => "movb",
        MmioScalarType::U16 => "movw",
        MmioScalarType::U32 => "movl",
        MmioScalarType::U64 => "movabs",
    };
    format!(
        "{} ${}, {}",
        mnemonic,
        format_hex_u64(value),
        mmio_accumulator_register(ty)
    )
}

fn mmio_immediate_mnemonic(ty: MmioScalarType, value: u64) -> String {
    let mnemonic = match ty {
        MmioScalarType::U8 => "movb",
        MmioScalarType::U16 => "movw",
        MmioScalarType::U32 => "movl",
        MmioScalarType::U64 => "movabs",
    };
    format!(
        "{mnemonic} ${}, {}",
        format_hex_u64(value),
        mmio_value_register(ty)
    )
}

fn format_hex_u64(value: u64) -> String {
    format!("0x{value:x}")
}

pub fn validate_compiler_owned_object_linear_subset(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<(), String> {
    validate_executable_krir_linear_structure(module, target, "compiler-owned object emission")
}

pub fn lower_executable_krir_to_compiler_owned_object(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<CompilerOwnedObject, String> {
    validate_compiler_owned_object_linear_subset(module, target)?;

    let mut canonical = module.clone();
    canonical.canonicalize();
    let extern_names = canonical
        .extern_declarations
        .iter()
        .map(|decl| decl.name.as_str())
        .collect::<BTreeSet<_>>();

    let mut function_offsets = BTreeMap::new();
    let mut function_sizes = BTreeMap::new();
    let mut cursor = 0u64;
    for function in &canonical.functions {
        let block = &function.blocks[0];
        let uses_saved_value_slot = executable_function_uses_saved_value_slot(function);
        let size = block.ops.iter().map(executable_op_encoded_len).sum::<u64>()
            + if uses_saved_value_slot { 2 } else { 0 }
            + 1;
        function_offsets.insert(function.name.clone(), cursor);
        function_sizes.insert(function.name.clone(), size);
        cursor += size;
    }

    let mut code_bytes = Vec::with_capacity(cursor as usize);
    let mut symbols = Vec::with_capacity(canonical.functions.len());
    let mut fixups = Vec::new();
    let mut unresolved_targets = BTreeSet::new();
    for function in &canonical.functions {
        let block = &function.blocks[0];
        let function_offset = *function_offsets
            .get(&function.name)
            .expect("function offset must exist");
        let function_size = *function_sizes
            .get(&function.name)
            .expect("function size must exist");
        let uses_saved_value_slot = executable_function_uses_saved_value_slot(function);
        let mut local_offset = 0u64;
        if uses_saved_value_slot {
            code_bytes.push(0x53);
            local_offset += 1;
        }
        for op in &block.ops {
            match op {
                ExecutableOp::Call { callee } => {
                    if !function_offsets.contains_key(callee) {
                        if !extern_names.contains(callee.as_str()) {
                            return Err(format!(
                                "compiler-owned object emission requires declared extern target '{}' in function '{}'",
                                callee, function.name
                            ));
                        }
                        unresolved_targets.insert(callee.clone());
                    }
                    code_bytes.push(0xE8);
                    code_bytes.extend_from_slice(&[0, 0, 0, 0]);
                    fixups.push(CompilerOwnedObjectFixup {
                        source_symbol: function.name.clone(),
                        patch_offset: function_offset + local_offset + 1,
                        kind: CompilerOwnedFixupKind::X86_64CallRel32,
                        target_symbol: callee.clone(),
                        width_bytes: 4,
                    });
                    local_offset += 5;
                }
                ExecutableOp::BranchIfZero {
                    ty,
                    then_callee,
                    else_callee,
                } => {
                    for callee in [then_callee, else_callee] {
                        if !function_offsets.contains_key(callee) {
                            if !extern_names.contains(callee.as_str()) {
                                return Err(format!(
                                    "compiler-owned object emission requires declared extern target '{}' in function '{}'",
                                    callee, function.name
                                ));
                            }
                            unresolved_targets.insert(callee.clone());
                        }
                    }
                    let test_bytes = mmio_saved_value_zero_test_bytes(*ty);
                    encode_branch_if_zero_bytes(&mut code_bytes, *ty);
                    fixups.push(CompilerOwnedObjectFixup {
                        source_symbol: function.name.clone(),
                        patch_offset: function_offset + local_offset + test_bytes + 7,
                        kind: CompilerOwnedFixupKind::X86_64CallRel32,
                        target_symbol: then_callee.clone(),
                        width_bytes: 4,
                    });
                    fixups.push(CompilerOwnedObjectFixup {
                        source_symbol: function.name.clone(),
                        patch_offset: function_offset + local_offset + test_bytes + 17,
                        kind: CompilerOwnedFixupKind::X86_64CallRel32,
                        target_symbol: else_callee.clone(),
                        width_bytes: 4,
                    });
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::BranchIfEqImm {
                    ty,
                    compare_value,
                    then_callee,
                    else_callee,
                } => {
                    for callee in [then_callee, else_callee] {
                        if !function_offsets.contains_key(callee) {
                            if !extern_names.contains(callee.as_str()) {
                                return Err(format!(
                                    "compiler-owned object emission requires declared extern target '{}' in function '{}'",
                                    callee, function.name
                                ));
                            }
                            unresolved_targets.insert(callee.clone());
                        }
                    }
                    let compare_bytes = mmio_saved_value_literal_compare_bytes(*ty);
                    encode_branch_if_eq_imm_bytes(&mut code_bytes, *ty, *compare_value);
                    fixups.push(CompilerOwnedObjectFixup {
                        source_symbol: function.name.clone(),
                        patch_offset: function_offset + local_offset + compare_bytes + 7,
                        kind: CompilerOwnedFixupKind::X86_64CallRel32,
                        target_symbol: then_callee.clone(),
                        width_bytes: 4,
                    });
                    fixups.push(CompilerOwnedObjectFixup {
                        source_symbol: function.name.clone(),
                        patch_offset: function_offset + local_offset + compare_bytes + 17,
                        kind: CompilerOwnedFixupKind::X86_64CallRel32,
                        target_symbol: else_callee.clone(),
                        width_bytes: 4,
                    });
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::BranchIfMaskNonZeroImm {
                    ty,
                    mask_value,
                    then_callee,
                    else_callee,
                } => {
                    for callee in [then_callee, else_callee] {
                        if !function_offsets.contains_key(callee) {
                            if !extern_names.contains(callee.as_str()) {
                                return Err(format!(
                                    "compiler-owned object emission requires declared extern target '{}' in function '{}'",
                                    callee, function.name
                                ));
                            }
                            unresolved_targets.insert(callee.clone());
                        }
                    }
                    let compare_bytes = mmio_saved_value_literal_mask_test_bytes(*ty);
                    encode_branch_if_mask_nonzero_imm_bytes(&mut code_bytes, *ty, *mask_value);
                    fixups.push(CompilerOwnedObjectFixup {
                        source_symbol: function.name.clone(),
                        patch_offset: function_offset + local_offset + compare_bytes + 7,
                        kind: CompilerOwnedFixupKind::X86_64CallRel32,
                        target_symbol: then_callee.clone(),
                        width_bytes: 4,
                    });
                    fixups.push(CompilerOwnedObjectFixup {
                        source_symbol: function.name.clone(),
                        patch_offset: function_offset + local_offset + compare_bytes + 17,
                        kind: CompilerOwnedFixupKind::X86_64CallRel32,
                        target_symbol: else_callee.clone(),
                        width_bytes: 4,
                    });
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::MmioRead {
                    ty,
                    addr,
                    capture_value,
                } => {
                    encode_mmio_read_bytes(&mut code_bytes, *ty, *addr, *capture_value);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::MmioWriteImm { ty, addr, value } => {
                    encode_mmio_write_imm_bytes(&mut code_bytes, *ty, *addr, *value);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::MmioWriteValue { ty, addr } => {
                    encode_mmio_write_saved_value_bytes(&mut code_bytes, *ty, *addr);
                    local_offset += executable_op_encoded_len(op);
                }
            }
        }
        if uses_saved_value_slot {
            code_bytes.push(0x5B);
        }
        code_bytes.push(0xC3);
        symbols.push(CompilerOwnedObjectSymbol {
            name: function.name.clone(),
            kind: CompilerOwnedObjectSymbolKind::Function,
            definition: CompilerOwnedObjectSymbolDefinition::DefinedText,
            offset: function_offset,
            size: function_size,
        });
    }

    for unresolved in unresolved_targets {
        symbols.push(CompilerOwnedObjectSymbol {
            name: unresolved,
            kind: CompilerOwnedObjectSymbolKind::Function,
            definition: CompilerOwnedObjectSymbolDefinition::UndefinedExternal,
            offset: 0,
            size: 0,
        });
    }
    symbols.sort_by(|a, b| a.name.cmp(&b.name));

    let object = CompilerOwnedObject {
        header: CompilerOwnedObjectHeader {
            magic: *b"KRBO",
            version_major: 0,
            version_minor: 1,
            object_kind: CompilerOwnedObjectKind::LinearRelocatable,
            target_id: target.target_id,
            endian: target.endian,
            pointer_bits: target.pointer_bits,
            format_revision: 2,
        },
        code: CompilerOwnedCodeSection {
            name: target.sections.text,
            bytes: code_bytes,
        },
        symbols,
        fixups,
    };
    object.validate()?;
    Ok(object)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64ElfRelocatableObject {
    pub format: &'static str,
    pub text_section: &'static str,
    pub text_bytes: Vec<u8>,
    pub function_symbols: Vec<X86_64ElfFunctionSymbol>,
    pub undefined_function_symbols: Vec<String>,
    pub relocations: Vec<X86_64ElfRelocation>,
}

impl X86_64ElfRelocatableObject {
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "elf64-relocatable" {
            return Err("x86_64 ELF object format must be elf64-relocatable".to_string());
        }
        if self.text_section.is_empty() {
            return Err("x86_64 ELF object text_section must not be empty".to_string());
        }

        let mut last_defined_name: Option<&str> = None;
        let mut defined_names = BTreeSet::new();
        let mut defined_ranges = Vec::new();
        for symbol in &self.function_symbols {
            if symbol.name.is_empty() {
                return Err("x86_64 ELF function symbol name must not be empty".to_string());
            }
            if !defined_names.insert(symbol.name.as_str()) {
                return Err(format!(
                    "x86_64 ELF function symbol '{}' must be unique",
                    symbol.name
                ));
            }
            if let Some(prev) = last_defined_name
                && symbol.name.as_str() < prev
            {
                return Err("x86_64 ELF function symbols must be sorted by name".to_string());
            }
            last_defined_name = Some(symbol.name.as_str());
            if symbol.offset + symbol.size > self.text_bytes.len() as u64 {
                return Err(format!(
                    "x86_64 ELF function symbol '{}' exceeds .text bounds",
                    symbol.name
                ));
            }
            defined_ranges.push((
                symbol.offset,
                symbol.offset + symbol.size,
                symbol.name.as_str(),
            ));
        }

        defined_ranges.sort_by_key(|(offset, _, _)| *offset);
        for pair in defined_ranges.windows(2) {
            let (_, prev_end, prev_name) = pair[0];
            let (next_offset, _, next_name) = pair[1];
            if next_offset < prev_end {
                return Err(format!(
                    "x86_64 ELF function symbols '{}' and '{}' must not overlap in .text",
                    prev_name, next_name
                ));
            }
        }

        let mut last_undefined_name: Option<&str> = None;
        let mut undefined_names = BTreeSet::new();
        for symbol in &self.undefined_function_symbols {
            if symbol.is_empty() {
                return Err(
                    "x86_64 ELF undefined function symbol name must not be empty".to_string(),
                );
            }
            if !undefined_names.insert(symbol.as_str()) {
                return Err(format!(
                    "x86_64 ELF undefined function symbol '{}' must be unique",
                    symbol
                ));
            }
            if defined_names.contains(symbol.as_str()) {
                return Err(format!(
                    "x86_64 ELF symbol '{}' must not be both defined and undefined",
                    symbol
                ));
            }
            if let Some(prev) = last_undefined_name
                && symbol.as_str() < prev
            {
                return Err(
                    "x86_64 ELF undefined function symbols must be sorted by name".to_string(),
                );
            }
            last_undefined_name = Some(symbol.as_str());
        }

        let mut last_relocation_key: Option<(u64, &str)> = None;
        let mut referenced_undefined = BTreeSet::new();
        let mut relocation_offsets = BTreeSet::new();
        for relocation in &self.relocations {
            if relocation.offset + 4 > self.text_bytes.len() as u64 {
                return Err(format!(
                    "x86_64 ELF relocation for target '{}' exceeds .text bounds",
                    relocation.target_symbol
                ));
            }
            if !undefined_names.contains(relocation.target_symbol.as_str()) {
                return Err(format!(
                    "x86_64 ELF relocation target '{}' must be an undefined function symbol",
                    relocation.target_symbol
                ));
            }
            if relocation.addend != -4 {
                return Err(format!(
                    "x86_64 ELF relocation target '{}' must use addend -4 in the linear subset",
                    relocation.target_symbol
                ));
            }
            let key = (relocation.offset, relocation.target_symbol.as_str());
            if let Some(prev) = last_relocation_key
                && key < prev
            {
                return Err(
                    "x86_64 ELF relocations must be sorted by offset and target symbol".to_string(),
                );
            }
            if !relocation_offsets.insert(relocation.offset) {
                return Err(format!(
                    "x86_64 ELF relocation patch offset {} must be unique",
                    relocation.offset
                ));
            }
            last_relocation_key = Some(key);
            referenced_undefined.insert(relocation.target_symbol.as_str());
        }

        if self.relocations.is_empty() {
            if !self.undefined_function_symbols.is_empty() {
                return Err(
                    "x86_64 ELF undefined function symbols require .rela.text relocations"
                        .to_string(),
                );
            }
        } else {
            for symbol in &self.undefined_function_symbols {
                if !referenced_undefined.contains(symbol.as_str()) {
                    return Err(format!(
                        "x86_64 ELF undefined function symbol '{}' must be referenced by a relocation",
                        symbol
                    ));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64ElfFunctionSymbol {
    pub name: String,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum X86_64ElfRelocationKind {
    X86_64Plt32,
}

impl X86_64ElfRelocationKind {
    fn elf_type(self) -> u32 {
        match self {
            Self::X86_64Plt32 => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64ElfRelocation {
    pub offset: u64,
    pub kind: X86_64ElfRelocationKind,
    pub target_symbol: String,
    pub addend: i64,
}

pub fn validate_compiler_owned_object_for_x86_64_elf_export(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<(), String> {
    target.validate()?;
    if target.target_id != BackendTargetId::X86_64Sysv
        || target.arch != TargetArch::X86_64
        || target.abi != TargetAbi::Sysv
    {
        return Err("x86_64 ELF export requires x86_64-sysv target contract".to_string());
    }

    object.validate()?;
    if object.header.target_id != target.target_id {
        return Err(
            "x86_64 ELF export requires compiler-owned object target_id to match target contract"
                .to_string(),
        );
    }
    if object.header.endian != target.endian {
        return Err(
            "x86_64 ELF export requires compiler-owned object endian to match target contract"
                .to_string(),
        );
    }
    if object.header.pointer_bits != target.pointer_bits {
        return Err("x86_64 ELF export requires compiler-owned object pointer_bits to match target contract".to_string());
    }
    if object.code.name != target.sections.text {
        return Err("x86_64 ELF export requires compiler-owned object code section to match target text section".to_string());
    }

    Ok(())
}

pub fn validate_x86_64_object_linear_subset(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<(), String> {
    validate_compiler_owned_object_linear_subset(module, target)
        .map_err(|err| err.replace("compiler-owned object emission", "x86_64 object emission"))
}

pub fn export_compiler_owned_object_to_x86_64_elf(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<X86_64ElfRelocatableObject, String> {
    validate_compiler_owned_object_for_x86_64_elf_export(object, target)?;

    let symbol_offsets = object
        .symbols
        .iter()
        .filter(|symbol| symbol.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|symbol| (symbol.name.as_str(), symbol.offset))
        .collect::<BTreeMap<_, _>>();
    let symbol_defs = object
        .symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.definition))
        .collect::<BTreeMap<_, _>>();

    let mut text_bytes = object.code.bytes.clone();
    let mut relocations = Vec::new();
    for fixup in &object.fixups {
        let Some(target_def) = symbol_defs.get(fixup.target_symbol.as_str()) else {
            return Err(format!(
                "x86_64 ELF export requires target symbol '{}' for fixup",
                fixup.target_symbol
            ));
        };

        match (fixup.kind, target_def) {
            (
                CompilerOwnedFixupKind::X86_64CallRel32,
                CompilerOwnedObjectSymbolDefinition::DefinedText,
            ) => {
                if fixup.width_bytes != 4 {
                    return Err(format!(
                        "x86_64 ELF export requires rel32 fixup width 4 for target '{}'",
                        fixup.target_symbol
                    ));
                }
                let target_offset = *symbol_offsets
                    .get(fixup.target_symbol.as_str())
                    .ok_or_else(|| {
                        format!(
                            "x86_64 ELF export requires target symbol '{}' for fixup",
                            fixup.target_symbol
                        )
                    })?;
                let next_ip = fixup.patch_offset + u64::from(fixup.width_bytes);
                let displacement = (target_offset as i64) - (next_ip as i64);
                let rel32 = i32::try_from(displacement).map_err(|_| {
                    format!(
                        "x86_64 ELF export call displacement to '{}' from '{}' does not fit rel32",
                        fixup.target_symbol, fixup.source_symbol
                    )
                })?;
                let patch_offset =
                    usize::try_from(fixup.patch_offset).expect("patch offset must fit usize");
                text_bytes[patch_offset..patch_offset + 4].copy_from_slice(&rel32.to_le_bytes());
            }
            (
                CompilerOwnedFixupKind::X86_64CallRel32,
                CompilerOwnedObjectSymbolDefinition::UndefinedExternal,
            ) => {
                if fixup.width_bytes != 4 {
                    return Err(format!(
                        "x86_64 ELF export requires rel32 fixup width 4 for target '{}'",
                        fixup.target_symbol
                    ));
                }
                relocations.push(X86_64ElfRelocation {
                    offset: fixup.patch_offset,
                    kind: X86_64ElfRelocationKind::X86_64Plt32,
                    target_symbol: fixup.target_symbol.clone(),
                    addend: -4,
                });
            }
        }
    }

    let function_symbols = object
        .symbols
        .iter()
        .filter(|symbol| symbol.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|symbol| X86_64ElfFunctionSymbol {
            name: symbol.name.clone(),
            offset: symbol.offset,
            size: symbol.size,
        })
        .collect::<Vec<_>>();
    let undefined_function_symbols = object
        .symbols
        .iter()
        .filter(|symbol| {
            symbol.definition == CompilerOwnedObjectSymbolDefinition::UndefinedExternal
        })
        .map(|symbol| symbol.name.clone())
        .collect::<Vec<_>>();

    let elf = X86_64ElfRelocatableObject {
        format: "elf64-relocatable",
        text_section: object.code.name,
        text_bytes,
        function_symbols,
        undefined_function_symbols,
        relocations,
    };
    elf.validate()?;
    Ok(elf)
}

pub fn lower_executable_krir_to_x86_64_object(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<X86_64ElfRelocatableObject, String> {
    validate_x86_64_object_linear_subset(module, target)?;
    let object = lower_executable_krir_to_compiler_owned_object(module, target)?;
    export_compiler_owned_object_to_x86_64_elf(&object, target)
}

fn push_u16_into(dst: &mut [u8], value: u16) {
    dst.copy_from_slice(&value.to_le_bytes());
}

fn push_u32_into(dst: &mut [u8], value: u32) {
    dst.copy_from_slice(&value.to_le_bytes());
}

fn push_u64_into(dst: &mut [u8], value: u64) {
    dst.copy_from_slice(&value.to_le_bytes());
}

fn push_u16_le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64_le(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i64_le(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn append_with_alignment(out: &mut Vec<u8>, bytes: &[u8], align: usize) -> usize {
    let align = align.max(1);
    let padding = (align - (out.len() % align)) % align;
    out.extend(std::iter::repeat_n(0, padding));
    let offset = out.len();
    out.extend_from_slice(bytes);
    offset
}

fn push_elf64_sym(
    out: &mut Vec<u8>,
    st_name: u32,
    st_info: u8,
    st_other: u8,
    st_shndx: u16,
    st_value: u64,
    st_size: u64,
) {
    push_u32_le(out, st_name);
    out.push(st_info);
    out.push(st_other);
    push_u16_le(out, st_shndx);
    push_u64_le(out, st_value);
    push_u64_le(out, st_size);
}

pub fn emit_x86_64_object_bytes(object: &X86_64ElfRelocatableObject) -> Vec<u8> {
    object.validate().expect("x86_64 ELF object must validate");

    let mut strtab = vec![0u8];
    let mut name_offsets = BTreeMap::new();
    for symbol in &object.function_symbols {
        let offset = strtab.len() as u32;
        strtab.extend_from_slice(symbol.name.as_bytes());
        strtab.push(0);
        name_offsets.insert(symbol.name.clone(), offset);
    }
    for symbol in &object.undefined_function_symbols {
        let offset = strtab.len() as u32;
        strtab.extend_from_slice(symbol.as_bytes());
        strtab.push(0);
        name_offsets.insert(symbol.clone(), offset);
    }

    let mut symtab = Vec::new();
    push_elf64_sym(&mut symtab, 0, 0, 0, 0, 0, 0);
    push_elf64_sym(&mut symtab, 0, 0x03, 0, 1, 0, 0);
    for symbol in &object.function_symbols {
        push_elf64_sym(
            &mut symtab,
            *name_offsets
                .get(&symbol.name)
                .expect("symbol name offset must exist"),
            0x12,
            0,
            1,
            symbol.offset,
            symbol.size,
        );
    }
    for symbol in &object.undefined_function_symbols {
        push_elf64_sym(
            &mut symtab,
            *name_offsets
                .get(symbol)
                .expect("undefined symbol name offset must exist"),
            0x12,
            0,
            0,
            0,
            0,
        );
    }

    let mut symbol_indices = BTreeMap::new();
    let mut next_symbol_index = 2u32;
    for symbol in &object.function_symbols {
        symbol_indices.insert(symbol.name.clone(), next_symbol_index);
        next_symbol_index += 1;
    }
    for symbol in &object.undefined_function_symbols {
        symbol_indices.insert(symbol.clone(), next_symbol_index);
        next_symbol_index += 1;
    }

    let mut rela_text = Vec::new();
    for relocation in &object.relocations {
        push_u64_le(&mut rela_text, relocation.offset);
        let symbol_index = *symbol_indices
            .get(relocation.target_symbol.as_str())
            .expect("relocation target symbol index must exist");
        let r_info = (u64::from(symbol_index) << 32) | u64::from(relocation.kind.elf_type());
        push_u64_le(&mut rela_text, r_info);
        push_i64_le(&mut rela_text, relocation.addend);
    }

    let mut shstrtab = vec![0u8];
    let text_name = shstrtab.len() as u32;
    shstrtab.extend_from_slice(object.text_section.as_bytes());
    shstrtab.push(0);
    let rela_text_name = if object.relocations.is_empty() {
        None
    } else {
        let offset = shstrtab.len() as u32;
        shstrtab.extend_from_slice(b".rela.text\0");
        Some(offset)
    };
    let symtab_name = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".symtab\0");
    let strtab_name = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".strtab\0");
    let shstrtab_name = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".shstrtab\0");

    let mut bytes = vec![0u8; 64];
    let text_offset = append_with_alignment(&mut bytes, &object.text_bytes, 16) as u64;
    let rela_text_offset = if rela_text.is_empty() {
        None
    } else {
        Some(append_with_alignment(&mut bytes, &rela_text, 8) as u64)
    };
    let symtab_offset = append_with_alignment(&mut bytes, &symtab, 8) as u64;
    let strtab_offset = append_with_alignment(&mut bytes, &strtab, 1) as u64;
    let shstrtab_offset = append_with_alignment(&mut bytes, &shstrtab, 1) as u64;
    let shoff = append_with_alignment(&mut bytes, &[], 8) as u64;

    let mut shdrs = vec![0u8; 64];
    let mut push_shdr = |name: u32,
                         sh_type: u32,
                         flags: u64,
                         addr: u64,
                         offset: u64,
                         size: u64,
                         link: u32,
                         info: u32,
                         addralign: u64,
                         entsize: u64| {
        push_u32_le(&mut shdrs, name);
        push_u32_le(&mut shdrs, sh_type);
        push_u64_le(&mut shdrs, flags);
        push_u64_le(&mut shdrs, addr);
        push_u64_le(&mut shdrs, offset);
        push_u64_le(&mut shdrs, size);
        push_u32_le(&mut shdrs, link);
        push_u32_le(&mut shdrs, info);
        push_u64_le(&mut shdrs, addralign);
        push_u64_le(&mut shdrs, entsize);
    };

    push_shdr(
        text_name,
        1,
        0x6,
        0,
        text_offset,
        object.text_bytes.len() as u64,
        0,
        0,
        16,
        0,
    );
    let symtab_index = if rela_text.is_empty() { 2u32 } else { 3u32 };
    let strtab_index = symtab_index + 1;
    let text_index = 1u32;
    if let (Some(name), Some(offset)) = (rela_text_name, rela_text_offset) {
        push_shdr(
            name,
            4,
            0,
            0,
            offset,
            rela_text.len() as u64,
            symtab_index,
            text_index,
            8,
            24,
        );
    }
    push_shdr(
        symtab_name,
        2,
        0,
        0,
        symtab_offset,
        symtab.len() as u64,
        strtab_index,
        2,
        8,
        24,
    );
    push_shdr(
        strtab_name,
        3,
        0,
        0,
        strtab_offset,
        strtab.len() as u64,
        0,
        0,
        1,
        0,
    );
    push_shdr(
        shstrtab_name,
        3,
        0,
        0,
        shstrtab_offset,
        shstrtab.len() as u64,
        0,
        0,
        1,
        0,
    );
    bytes.extend_from_slice(&shdrs);

    bytes[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[7] = 0;
    push_u16_into(&mut bytes[16..18], 1);
    push_u16_into(&mut bytes[18..20], 62);
    push_u32_into(&mut bytes[20..24], 1);
    push_u64_into(&mut bytes[24..32], 0);
    push_u64_into(&mut bytes[32..40], 0);
    push_u64_into(&mut bytes[40..48], shoff);
    push_u32_into(&mut bytes[48..52], 0);
    push_u16_into(&mut bytes[52..54], 64);
    push_u16_into(&mut bytes[54..56], 0);
    push_u16_into(&mut bytes[56..58], 0);
    push_u16_into(&mut bytes[58..60], 64);
    push_u16_into(&mut bytes[60..62], if rela_text.is_empty() { 5 } else { 6 });
    push_u16_into(&mut bytes[62..64], if rela_text.is_empty() { 4 } else { 5 });

    bytes
}

pub fn emit_compiler_owned_object_bytes(object: &CompilerOwnedObject) -> Vec<u8> {
    object
        .validate()
        .expect("compiler-owned object must validate");

    let mut string_names = object
        .symbols
        .iter()
        .map(|symbol| symbol.name.as_str())
        .chain(
            object
                .fixups
                .iter()
                .flat_map(|fixup| [fixup.source_symbol.as_str(), fixup.target_symbol.as_str()]),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    string_names.sort_unstable();

    let mut strings = vec![0u8];
    let mut name_offsets = BTreeMap::new();
    for name in string_names {
        let offset = strings.len() as u32;
        strings.extend_from_slice(name.as_bytes());
        strings.push(0);
        name_offsets.insert(name.to_string(), offset);
    }

    let mut symbols = Vec::with_capacity(object.symbols.len() * 24);
    for symbol in &object.symbols {
        push_u32_le(
            &mut symbols,
            *name_offsets
                .get(&symbol.name)
                .expect("symbol string offset must exist"),
        );
        symbols.push(symbol.kind.tag());
        symbols.push(symbol.definition.tag());
        symbols.extend_from_slice(&[0, 0]);
        push_u64_le(&mut symbols, symbol.offset);
        push_u64_le(&mut symbols, symbol.size);
    }

    let mut fixups = Vec::with_capacity(object.fixups.len() * 24);
    for fixup in &object.fixups {
        push_u32_le(
            &mut fixups,
            *name_offsets
                .get(&fixup.source_symbol)
                .expect("fixup source string offset must exist"),
        );
        push_u32_le(
            &mut fixups,
            *name_offsets
                .get(&fixup.target_symbol)
                .expect("fixup target string offset must exist"),
        );
        push_u64_le(&mut fixups, fixup.patch_offset);
        fixups.push(fixup.kind.tag());
        fixups.push(fixup.width_bytes);
        fixups.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    }

    const HEADER_SIZE: usize = 48;
    let code_offset = HEADER_SIZE as u32;
    let symbols_offset = code_offset + object.code.bytes.len() as u32;
    let fixups_offset = symbols_offset + symbols.len() as u32;
    let strings_offset = fixups_offset + fixups.len() as u32;

    let mut bytes = vec![0u8; HEADER_SIZE];
    bytes[0..4].copy_from_slice(&object.header.magic);
    bytes[4] = object.header.version_major;
    bytes[5] = object.header.version_minor;
    push_u16_into(&mut bytes[6..8], object.header.format_revision);
    bytes[8] = object.header.object_kind.tag();
    bytes[9] = match object.header.target_id {
        BackendTargetId::X86_64Sysv => 1,
    };
    bytes[10] = match object.header.endian {
        TargetEndian::Little => 1,
    };
    bytes[11] = u8::try_from(object.header.pointer_bits)
        .expect("pointer_bits must fit into u8 for KRBO v0.1");
    push_u32_into(&mut bytes[16..20], code_offset);
    push_u32_into(&mut bytes[20..24], object.code.bytes.len() as u32);
    push_u32_into(&mut bytes[24..28], symbols_offset);
    push_u32_into(&mut bytes[28..32], object.symbols.len() as u32);
    push_u32_into(&mut bytes[32..36], fixups_offset);
    push_u32_into(&mut bytes[36..40], object.fixups.len() as u32);
    push_u32_into(&mut bytes[40..44], strings_offset);
    push_u32_into(&mut bytes[44..48], strings.len() as u32);

    bytes.extend_from_slice(&object.code.bytes);
    bytes.extend_from_slice(&symbols);
    bytes.extend_from_slice(&fixups);
    bytes.extend_from_slice(&strings);
    bytes
}

#[cfg(test)]
mod tests {
    use super::{
        BackendTargetContract, BackendTargetId, CallEdge, CompilerOwnedCodeSection,
        CompilerOwnedFixupKind, CompilerOwnedObject, CompilerOwnedObjectFixup,
        CompilerOwnedObjectHeader, CompilerOwnedObjectKind, CompilerOwnedObjectSymbol,
        CompilerOwnedObjectSymbolDefinition, CompilerOwnedObjectSymbolKind, Ctx, Eff,
        ExecutableBlock, ExecutableExternDecl, ExecutableFacts, ExecutableFunction,
        ExecutableKrirModule, ExecutableOp, ExecutableSignature, ExecutableTerminator,
        ExecutableValue, ExecutableValueType, FunctionAttrs, FutureScalarReturnConvention,
        MmioScalarType, TargetEndian, X86_64IntegerRegister, emit_compiler_owned_object_bytes,
        emit_x86_64_asm_text, emit_x86_64_object_bytes, export_compiler_owned_object_to_x86_64_asm,
        export_compiler_owned_object_to_x86_64_elf, lower_executable_krir_to_compiler_owned_object,
        lower_executable_krir_to_x86_64_asm, lower_executable_krir_to_x86_64_object,
        validate_compiler_owned_object_for_x86_64_asm_export,
        validate_compiler_owned_object_linear_subset, validate_x86_64_object_linear_subset,
    };
    use serde_json::json;
    use std::{
        collections::BTreeSet,
        fs,
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
    };

    fn hex_encode(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }

    fn read_u16(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
    }

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }

    fn read_u64(bytes: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ])
    }

    static ELF_SMOKE_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempPath {
        path: PathBuf,
    }

    impl TempPath {
        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempPath {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    fn find_optional_tool(candidates: &[&str]) -> Option<String> {
        candidates.iter().find_map(|candidate| {
            Command::new(candidate)
                .arg("--version")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|_| (*candidate).to_string())
        })
    }

    fn write_temp_file(prefix: &str, suffix: &str, bytes: &[u8]) -> TempPath {
        let unique = ELF_SMOKE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "kernrift-{}-{}-{}{}",
            prefix,
            std::process::id(),
            unique,
            suffix
        ));
        fs::write(&path, bytes).expect("write temporary ELF object");
        TempPath { path }
    }

    fn write_temp_elf_file(prefix: &str, bytes: &[u8]) -> TempPath {
        write_temp_file(prefix, ".o", bytes)
    }

    fn run_tool_capture(tool: &str, args: &[&str], path: &Path) -> String {
        let output = Command::new(tool)
            .args(args)
            .arg(path)
            .output()
            .expect("run ELF compatibility tool");
        assert!(
            output.status.success(),
            "tool '{}' failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            tool,
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("tool output must be utf8")
    }

    fn readelf_smoke_output(path: &Path) -> Option<String> {
        let tool = find_optional_tool(&["readelf", "llvm-readelf"])?;
        Some(run_tool_capture(&tool, &["-h", "-S", "-s", "-r"], path))
    }

    fn objdump_smoke_check(path: &Path) -> bool {
        let Some(tool) = find_optional_tool(&["objdump", "llvm-objdump"]) else {
            return false;
        };
        let _ = run_tool_capture(&tool, &["-d", "-r"], path);
        true
    }

    fn find_optional_linker() -> Option<String> {
        find_optional_tool(&["ld", "ld.lld"])
    }

    fn find_optional_asm_compiler() -> Option<String> {
        find_optional_tool(&["cc", "clang", "gcc"])
    }

    fn temp_output_path(prefix: &str, suffix: &str) -> PathBuf {
        let unique = ELF_SMOKE_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "kernrift-{}-{}-{}{}",
            prefix,
            std::process::id(),
            unique,
            suffix
        ))
    }

    fn run_linker_capture(
        tool: &str,
        args: &[&str],
        inputs: &[&Path],
        output_path: &Path,
    ) -> std::process::Output {
        Command::new(tool)
            .args(args)
            .arg("-o")
            .arg(output_path)
            .args(inputs)
            .output()
            .expect("run linker compatibility tool")
    }

    fn linked_output_bytes(path: &Path) -> Vec<u8> {
        fs::read(path).expect("read linked ELF output")
    }

    fn compile_asm_source_to_object(compiler: &str, prefix: &str, source: &str) -> TempPath {
        let source_file = write_temp_file(prefix, ".s", source.as_bytes());
        let object_path = temp_output_path(prefix, ".o");
        let output = Command::new(compiler)
            .arg("-c")
            .arg(source_file.path())
            .arg("-o")
            .arg(&object_path)
            .output()
            .expect("run assembly compiler");
        assert!(
            output.status.success(),
            "compiler '{}' failed to assemble {}\nstdout:\n{}\nstderr:\n{}",
            compiler,
            source_file.path().display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        TempPath { path: object_path }
    }

    fn assert_is_elf64_executable(bytes: &[u8]) {
        assert!(
            bytes.len() >= 64,
            "linked artifact is too small to be ELF64"
        );
        assert_eq!(&bytes[0..4], b"\x7fELF");
        assert_eq!(bytes[4], 2, "expected ELF64");
        assert_eq!(bytes[5], 1, "expected little-endian ELF");
        assert_eq!(bytes[6], 1, "expected ELF version 1");
        assert_eq!(read_u16(bytes, 18), 62, "expected x86_64 machine");
        let elf_type = read_u16(bytes, 16);
        assert!(
            elf_type == 2 || elf_type == 3,
            "expected ET_EXEC or ET_DYN, got {}",
            elf_type
        );
        assert_ne!(read_u64(bytes, 24), 0, "expected non-zero ELF entry");
    }

    fn run_executable_smoke(path: &Path) -> Option<std::process::Output> {
        let mut permissions = fs::metadata(path)
            .expect("read linked artifact metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("mark linked artifact executable");

        match Command::new(path).output() {
            Ok(output) => Some(output),
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => None,
            Err(err) => panic!(
                "failed to execute linked artifact '{}': {err}",
                path.display()
            ),
        }
    }

    #[test]
    fn krir_mmio_ops_encode_scalar_type_deterministically() {
        assert_eq!(
            serde_json::to_value(super::KrirOp::MmioRead {
                ty: MmioScalarType::U16,
                addr: super::MmioAddrExpr::IdentPlusOffset {
                    base: "mmio_base".to_string(),
                    offset: "2".to_string(),
                },
                capture_slot: None,
            })
            .expect("serialize mmio_read"),
            json!({
                "op": "mmio_read",
                "ty": "u16",
                "addr": {"kind": "ident_plus_offset", "base": "mmio_base", "offset": "2"}
            })
        );
        assert_eq!(
            serde_json::to_value(super::KrirOp::MmioWrite {
                ty: MmioScalarType::U64,
                addr: super::MmioAddrExpr::Ident {
                    name: "mmio_base".to_string(),
                },
                value: super::MmioValueExpr::Ident {
                    name: "payload".to_string(),
                }
            })
            .expect("serialize mmio_write"),
            json!({
                "op": "mmio_write",
                "ty": "u64",
                "addr": {"kind": "ident", "name": "mmio_base"},
                "value": {"kind": "ident", "name": "payload"}
            })
        );
        assert_eq!(
            serde_json::to_value(super::KrirOp::RawMmioRead {
                ty: MmioScalarType::U8,
                addr: super::MmioAddrExpr::IntLiteral {
                    value: "0x1014".to_string(),
                },
                capture_slot: None,
            })
            .expect("serialize raw_mmio_read"),
            json!({
                "op": "raw_mmio_read",
                "ty": "u8",
                "addr": {"kind": "int_literal", "value": "0x1014"}
            })
        );
        assert_eq!(
            serde_json::to_value(super::KrirOp::RawMmioWrite {
                ty: MmioScalarType::U32,
                addr: super::MmioAddrExpr::Ident {
                    name: "UART0".to_string(),
                },
                value: super::MmioValueExpr::IntLiteral {
                    value: "0xff".to_string(),
                }
            })
            .expect("serialize raw_mmio_write"),
            json!({
                "op": "raw_mmio_write",
                "ty": "u32",
                "addr": {"kind": "ident", "name": "UART0"},
                "value": {"kind": "int_literal", "value": "0xff"}
            })
        );
    }

    #[test]
    fn krir_module_mmio_bases_encode_deterministically() {
        let mut module = super::KrirModule {
            module_caps: vec!["Mmio".to_string()],
            mmio_bases: vec![
                super::MmioBaseDecl {
                    name: "UART0".to_string(),
                    addr: "0x1000".to_string(),
                },
                super::MmioBaseDecl {
                    name: "APIC".to_string(),
                    addr: "0xfee00000".to_string(),
                },
            ],
            mmio_registers: Vec::new(),
            functions: Vec::new(),
            call_edges: Vec::new(),
        };
        module.canonicalize();
        assert_eq!(
            serde_json::to_value(&module).expect("serialize module"),
            json!({
                "module_caps": ["Mmio"],
                "mmio_bases": [
                    {"name": "APIC", "addr": "0xfee00000"},
                    {"name": "UART0", "addr": "0x1000"}
                ],
                "functions": [],
                "call_edges": []
            })
        );
    }

    #[test]
    fn krir_module_mmio_registers_encode_deterministically() {
        let mut module = super::KrirModule {
            module_caps: vec!["Mmio".to_string()],
            mmio_bases: vec![super::MmioBaseDecl {
                name: "UART0".to_string(),
                addr: "0x1000".to_string(),
            }],
            mmio_registers: vec![
                super::MmioRegisterDecl {
                    base: "UART0".to_string(),
                    name: "SR".to_string(),
                    offset: "0x04".to_string(),
                    ty: super::MmioScalarType::U32,
                    access: super::MmioRegAccess::Ro,
                },
                super::MmioRegisterDecl {
                    base: "UART0".to_string(),
                    name: "DR".to_string(),
                    offset: "0x00".to_string(),
                    ty: super::MmioScalarType::U32,
                    access: super::MmioRegAccess::Rw,
                },
            ],
            functions: Vec::new(),
            call_edges: Vec::new(),
        };
        module.canonicalize();
        assert_eq!(
            serde_json::to_value(&module).expect("serialize module"),
            json!({
                "module_caps": ["Mmio"],
                "mmio_bases": [
                    {"name": "UART0", "addr": "0x1000"}
                ],
                "mmio_registers": [
                    {"base": "UART0", "name": "DR", "offset": "0x00", "ty": "u32", "access": "rw"},
                    {"base": "UART0", "name": "SR", "offset": "0x04", "ty": "u32", "access": "ro"}
                ],
                "functions": [],
                "call_edges": []
            })
        );
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ParsedElfSection {
        name: String,
        sh_type: u32,
        flags: u64,
        offset: u64,
        size: u64,
        link: u32,
        info: u32,
        addralign: u64,
        entsize: u64,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ParsedElfSymbol {
        name: String,
        info: u8,
        shndx: u16,
        value: u64,
        size: u64,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ParsedElfRelocation {
        offset: u64,
        sym: u32,
        kind: u32,
        addend: i64,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ParsedCompilerOwnedHeader {
        magic: [u8; 4],
        version_major: u8,
        version_minor: u8,
        format_revision: u16,
        object_kind: u8,
        target_id: u8,
        endian: u8,
        pointer_bits: u8,
        code_offset: u32,
        code_size: u32,
        symbols_offset: u32,
        symbols_count: u32,
        fixups_offset: u32,
        fixups_count: u32,
        strings_offset: u32,
        strings_size: u32,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ParsedCompilerOwnedSymbol {
        name: String,
        kind: u8,
        definition: u8,
        offset: u64,
        size: u64,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ParsedCompilerOwnedFixup {
        source_symbol: String,
        patch_offset: u64,
        kind: u8,
        target_symbol: String,
        width_bytes: u8,
    }

    fn read_cstr(bytes: &[u8], offset: usize) -> String {
        let end = bytes[offset..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|idx| offset + idx)
            .expect("null terminator");
        String::from_utf8(bytes[offset..end].to_vec()).expect("utf8")
    }

    fn parse_elf64_sections(bytes: &[u8]) -> Vec<ParsedElfSection> {
        assert_eq!(&bytes[0..4], b"\x7fELF");
        assert_eq!(bytes[4], 2);
        assert_eq!(bytes[5], 1);
        assert_eq!(bytes[6], 1);
        assert_eq!(read_u16(bytes, 16), 1);
        assert_eq!(read_u16(bytes, 18), 62);

        let shoff = read_u64(bytes, 40) as usize;
        let shentsize = read_u16(bytes, 58) as usize;
        let shnum = read_u16(bytes, 60) as usize;
        let shstrndx = read_u16(bytes, 62) as usize;

        let shstr_hdr = shoff + shentsize * shstrndx;
        let shstr_off = read_u64(bytes, shstr_hdr + 24) as usize;
        let shstr_size = read_u64(bytes, shstr_hdr + 32) as usize;
        let shstr = &bytes[shstr_off..shstr_off + shstr_size];

        (0..shnum)
            .map(|idx| {
                let off = shoff + shentsize * idx;
                let name_off = read_u32(bytes, off) as usize;
                ParsedElfSection {
                    name: read_cstr(shstr, name_off),
                    sh_type: read_u32(bytes, off + 4),
                    flags: read_u64(bytes, off + 8),
                    offset: read_u64(bytes, off + 24),
                    size: read_u64(bytes, off + 32),
                    link: read_u32(bytes, off + 40),
                    info: read_u32(bytes, off + 44),
                    addralign: read_u64(bytes, off + 48),
                    entsize: read_u64(bytes, off + 56),
                }
            })
            .collect()
    }

    fn parse_elf64_symbols(bytes: &[u8], sections: &[ParsedElfSection]) -> Vec<ParsedElfSymbol> {
        let symtab_idx = sections
            .iter()
            .position(|section| section.name == ".symtab")
            .expect("symtab section");
        let symtab = &sections[symtab_idx];
        let strtab = &sections[symtab.link as usize];
        let strtab_bytes = &bytes[strtab.offset as usize..(strtab.offset + strtab.size) as usize];
        let symtab_bytes = &bytes[symtab.offset as usize..(symtab.offset + symtab.size) as usize];
        symtab_bytes
            .chunks(symtab.entsize as usize)
            .map(|entry| {
                let name_off = read_u32(entry, 0) as usize;
                ParsedElfSymbol {
                    name: if name_off == 0 {
                        String::new()
                    } else {
                        read_cstr(strtab_bytes, name_off)
                    },
                    info: entry[4],
                    shndx: read_u16(entry, 6),
                    value: read_u64(entry, 8),
                    size: read_u64(entry, 16),
                }
            })
            .collect()
    }

    fn parse_elf64_relocations(
        bytes: &[u8],
        sections: &[ParsedElfSection],
    ) -> Vec<ParsedElfRelocation> {
        let Some(rela_idx) = sections
            .iter()
            .position(|section| section.name == ".rela.text")
        else {
            return Vec::new();
        };
        let rela = &sections[rela_idx];
        let rela_bytes = &bytes[rela.offset as usize..(rela.offset + rela.size) as usize];
        rela_bytes
            .chunks(rela.entsize as usize)
            .map(|entry| {
                let r_info = read_u64(entry, 8);
                ParsedElfRelocation {
                    offset: read_u64(entry, 0),
                    sym: (r_info >> 32) as u32,
                    kind: (r_info & 0xffff_ffff) as u32,
                    addend: i64::from_le_bytes([
                        entry[16], entry[17], entry[18], entry[19], entry[20], entry[21],
                        entry[22], entry[23],
                    ]),
                }
            })
            .collect()
    }

    fn parse_compiler_owned_header(bytes: &[u8]) -> ParsedCompilerOwnedHeader {
        ParsedCompilerOwnedHeader {
            magic: [bytes[0], bytes[1], bytes[2], bytes[3]],
            version_major: bytes[4],
            version_minor: bytes[5],
            format_revision: read_u16(bytes, 6),
            object_kind: bytes[8],
            target_id: bytes[9],
            endian: bytes[10],
            pointer_bits: bytes[11],
            code_offset: read_u32(bytes, 16),
            code_size: read_u32(bytes, 20),
            symbols_offset: read_u32(bytes, 24),
            symbols_count: read_u32(bytes, 28),
            fixups_offset: read_u32(bytes, 32),
            fixups_count: read_u32(bytes, 36),
            strings_offset: read_u32(bytes, 40),
            strings_size: read_u32(bytes, 44),
        }
    }

    fn parse_compiler_owned_symbols(
        bytes: &[u8],
        header: &ParsedCompilerOwnedHeader,
    ) -> Vec<ParsedCompilerOwnedSymbol> {
        let strings = &bytes[header.strings_offset as usize
            ..(header.strings_offset + header.strings_size) as usize];
        let symbols = &bytes[header.symbols_offset as usize..(header.fixups_offset) as usize];
        symbols
            .chunks(24)
            .take(header.symbols_count as usize)
            .map(|entry| ParsedCompilerOwnedSymbol {
                name: read_cstr(strings, read_u32(entry, 0) as usize),
                kind: entry[4],
                definition: entry[5],
                offset: read_u64(entry, 8),
                size: read_u64(entry, 16),
            })
            .collect()
    }

    fn parse_compiler_owned_fixups(
        bytes: &[u8],
        header: &ParsedCompilerOwnedHeader,
    ) -> Vec<ParsedCompilerOwnedFixup> {
        let strings = &bytes[header.strings_offset as usize
            ..(header.strings_offset + header.strings_size) as usize];
        let fixups = &bytes[header.fixups_offset as usize..(header.strings_offset) as usize];
        fixups
            .chunks(24)
            .take(header.fixups_count as usize)
            .map(|entry| ParsedCompilerOwnedFixup {
                source_symbol: read_cstr(strings, read_u32(entry, 0) as usize),
                target_symbol: read_cstr(strings, read_u32(entry, 4) as usize),
                patch_offset: read_u64(entry, 8),
                kind: entry[16],
                width_bytes: entry[17],
            })
            .collect()
    }

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

    fn unit_return_function(name: &str) -> ExecutableFunction {
        ExecutableFunction {
            blocks: vec![ExecutableBlock {
                label: "entry".to_string(),
                ops: vec![],
                terminator: ExecutableTerminator::Return {
                    value: ExecutableValue::Unit,
                },
            }],
            ..executable_function(name)
        }
    }

    fn executable_extern_decl(name: &str) -> ExecutableExternDecl {
        ExecutableExternDecl {
            name: name.to_string(),
        }
    }

    fn asm_export_fixture_target() -> BackendTargetContract {
        BackendTargetContract::x86_64_sysv()
    }

    fn asm_export_fixture_object(bytes: Vec<u8>) -> CompilerOwnedObject {
        CompilerOwnedObject {
            header: CompilerOwnedObjectHeader {
                magic: *b"KRBO",
                version_major: 0,
                version_minor: 1,
                object_kind: CompilerOwnedObjectKind::LinearRelocatable,
                target_id: BackendTargetId::X86_64Sysv,
                endian: TargetEndian::Little,
                pointer_bits: 64,
                format_revision: 2,
            },
            code: CompilerOwnedCodeSection {
                name: ".text",
                bytes,
            },
            symbols: vec![CompilerOwnedObjectSymbol {
                name: "entry".to_string(),
                kind: CompilerOwnedObjectSymbolKind::Function,
                definition: CompilerOwnedObjectSymbolDefinition::DefinedText,
                offset: 0,
                size: 1,
            }],
            fixups: vec![],
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
            extern_declarations: vec![executable_extern_decl("helper")],
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
                "extern_declarations": [{
                    "name": "helper"
                }],
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
            extern_declarations: vec![],
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
            extern_declarations: vec![],
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
            extern_declarations: vec![],
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
            extern_declarations: vec![],
            call_edges: vec![],
        };

        module.canonicalize();

        assert_eq!(
            module.functions[0].signature.params,
            vec![ExecutableValueType::Unit, ExecutableValueType::Unit]
        );
    }

    #[test]
    fn x86_64_sysv_target_contract_is_deterministic_and_valid() {
        let contract = BackendTargetContract::x86_64_sysv();
        contract.validate().expect("valid target contract");

        let value = serde_json::to_value(&contract).expect("serialize");
        assert_eq!(
            value,
            json!({
                "target_id": "x86_64_sysv",
                "arch": "x86_64",
                "abi": "sysv",
                "endian": "little",
                "pointer_bits": 64,
                "stack_alignment_bytes": 16,
                "integer_registers": [
                    "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "rsp",
                    "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"
                ],
                "stack_pointer": "rsp",
                "frame_pointer": "rbp",
                "instruction_pointer": "rip",
                "caller_saved": ["rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11"],
                "callee_saved": ["rbx", "rbp", "r12", "r13", "r14", "r15"],
                "current_executable_return": "unit_no_register",
                "future_scalar_return": "integer_rax",
                "future_argument_registers": ["rdi", "rsi", "rdx", "rcx", "r8", "r9"],
                "symbols": {
                    "function_prefix": "",
                    "preserve_source_names": true
                },
                "sections": {
                    "text": ".text",
                    "rodata": ".rodata",
                    "data": ".data",
                    "bss": ".bss"
                },
                "freestanding": {
                    "no_libc": true,
                    "no_host_runtime": true,
                    "toolchain_bridge_not_yet_exercised": true
                }
            })
        );
    }

    #[test]
    fn x86_64_sysv_target_contract_register_sets_are_partitioned() {
        let contract = BackendTargetContract::x86_64_sysv();
        let integer_set = contract
            .integer_registers
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let caller_saved = contract
            .caller_saved
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let callee_saved = contract
            .callee_saved
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();

        assert!(caller_saved.is_disjoint(&callee_saved));
        assert!(caller_saved.is_subset(&integer_set));
        assert!(callee_saved.is_subset(&integer_set));
        assert_eq!(contract.stack_alignment_bytes, 16);
        assert_eq!(
            contract.future_argument_registers,
            vec![
                X86_64IntegerRegister::Rdi,
                X86_64IntegerRegister::Rsi,
                X86_64IntegerRegister::Rdx,
                X86_64IntegerRegister::Rcx,
                X86_64IntegerRegister::R8,
                X86_64IntegerRegister::R9,
            ]
        );
        assert_eq!(
            contract.future_scalar_return.registers(),
            &[X86_64IntegerRegister::Rax]
        );
    }

    #[test]
    fn x86_64_sysv_target_contract_validation_rejects_overlapping_saved_sets() {
        let mut contract = BackendTargetContract::x86_64_sysv();
        contract.callee_saved.push(X86_64IntegerRegister::Rax);

        assert_eq!(
            contract.validate(),
            Err(
                "backend target contract caller_saved and callee_saved must be disjoint"
                    .to_string()
            )
        );
    }

    #[test]
    fn x86_64_sysv_target_contract_validation_rejects_unknown_scalar_return_register_mapping() {
        let mut contract = BackendTargetContract::x86_64_sysv();
        contract
            .integer_registers
            .retain(|reg| *reg != X86_64IntegerRegister::Rax);
        contract
            .caller_saved
            .retain(|reg| *reg != X86_64IntegerRegister::Rax);
        contract.future_scalar_return = FutureScalarReturnConvention::IntegerRax;

        assert_eq!(
            contract.validate(),
            Err(
                "backend target contract future_scalar_return must resolve to integer_registers"
                    .to_string()
            )
        );
    }

    #[test]
    fn executable_krir_lowers_empty_function_to_x86_64_asm_text() {
        let mut module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                facts: ExecutableFacts {
                    eff_used: vec![],
                    caps_req: vec![],
                    ..executable_function("entry").facts
                },
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![],
            call_edges: vec![],
        };
        module.canonicalize();

        let asm =
            lower_executable_krir_to_x86_64_asm(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 asm");

        assert_eq!(
            emit_x86_64_asm_text(&asm),
            ".text\n\n.globl entry\nentry:\n    ret\n"
        );
    }

    #[test]
    fn executable_krir_lowers_ordered_direct_calls_to_ordered_call_instructions() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![
                            ExecutableOp::Call {
                                callee: "alpha".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "beta".to_string(),
                            },
                        ],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
                ExecutableFunction {
                    name: "beta".to_string(),
                    ..unit_return_function("beta")
                },
            ],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let asm =
            lower_executable_krir_to_x86_64_asm(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 asm");

        assert_eq!(
            emit_x86_64_asm_text(&asm),
            ".text\n\n.globl alpha\nalpha:\n    ret\n\n.globl beta\nbeta:\n    ret\n\n.globl entry\nentry:\n    call alpha\n    call beta\n    ret\n"
        );
    }

    #[test]
    fn executable_krir_lowers_functions_in_deterministic_symbol_order() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("zeta"), unit_return_function("alpha")],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let asm =
            lower_executable_krir_to_x86_64_asm(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 asm");

        assert_eq!(
            asm.functions
                .iter()
                .map(|function| function.symbol.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
    }

    #[test]
    fn executable_krir_x86_64_lowering_rejects_multiple_blocks() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                blocks: vec![
                    ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    },
                    ExecutableBlock {
                        label: "late".to_string(),
                        ops: vec![],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    },
                ],
                ..executable_function("entry")
            }],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        assert_eq!(
            lower_executable_krir_to_x86_64_asm(&module, &BackendTargetContract::x86_64_sysv()),
            Err(
                "compiler-owned object emission requires exactly one block in function 'entry'"
                    .to_string()
            )
        );
    }

    #[test]
    fn executable_krir_x86_64_lowering_supports_declared_external_direct_call_target() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "missing".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![executable_extern_decl("missing")],
            call_edges: vec![],
        };

        let target = BackendTargetContract::x86_64_sysv();
        let object = lower_executable_krir_to_compiler_owned_object(&module, &target)
            .expect("lower compiler-owned object");
        assert!(object.symbols.iter().any(|symbol| {
            symbol.name == "missing"
                && symbol.definition == CompilerOwnedObjectSymbolDefinition::UndefinedExternal
        }));
        validate_compiler_owned_object_for_x86_64_asm_export(&object, &target)
            .expect("validate asm export");
        let exported =
            export_compiler_owned_object_to_x86_64_asm(&object, &target).expect("export asm");
        let wrapped = lower_executable_krir_to_x86_64_asm(&module, &target).expect("wrap asm");

        assert_eq!(wrapped, exported);
        assert_eq!(
            emit_x86_64_asm_text(&exported),
            ".text\n\n.globl entry\nentry:\n    call missing\n    ret\n"
        );
    }

    #[test]
    fn executable_krir_x86_64_asm_export_preserves_mixed_internal_and_external_calls() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![ExecutableOp::Call {
                            callee: "helper".to_string(),
                        }],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "helper".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![ExecutableOp::Call {
                            callee: "ext".to_string(),
                        }],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("helper")
                },
            ],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let target = BackendTargetContract::x86_64_sysv();
        let object = lower_executable_krir_to_compiler_owned_object(&module, &target)
            .expect("lower compiler-owned object");
        let exported =
            export_compiler_owned_object_to_x86_64_asm(&object, &target).expect("export asm");
        let wrapped = lower_executable_krir_to_x86_64_asm(&module, &target).expect("wrap asm");

        assert_eq!(wrapped, exported);
        assert_eq!(
            emit_x86_64_asm_text(&exported),
            ".text\n\n.globl entry\nentry:\n    call helper\n    ret\n\n.globl helper\nhelper:\n    call ext\n    ret\n"
        );
    }

    #[test]
    fn executable_krir_validation_rejects_undeclared_direct_call_target() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "missing".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        assert_eq!(
            module.validate(),
            Err("executable KRIR function 'entry' calls undeclared target 'missing'".to_string())
        );
    }

    #[test]
    fn executable_krir_x86_64_asm_export_is_derived_from_compiler_owned_object() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![ExecutableOp::Call {
                            callee: "alpha".to_string(),
                        }],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
            ],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let target = BackendTargetContract::x86_64_sysv();
        let object = lower_executable_krir_to_compiler_owned_object(&module, &target)
            .expect("lower compiler-owned object");
        let exported =
            export_compiler_owned_object_to_x86_64_asm(&object, &target).expect("export asm");
        let wrapped = lower_executable_krir_to_x86_64_asm(&module, &target).expect("wrap asm");

        assert_eq!(wrapped, exported);
        assert_eq!(
            emit_x86_64_asm_text(&exported),
            ".text\n\n.globl alpha\nalpha:\n    ret\n\n.globl entry\nentry:\n    call alpha\n    ret\n"
        );
    }

    #[test]
    fn x86_64_asm_export_rejects_unsupported_code_byte_in_defined_symbol() {
        let target = asm_export_fixture_target();
        let object = asm_export_fixture_object(vec![0x90]);

        assert_eq!(
            export_compiler_owned_object_to_x86_64_asm(&object, &target),
            Err(
                "x86_64 asm export encountered unsupported code byte 0x90 in function 'entry' at offset 0"
                    .to_string()
            )
        );
    }

    #[test]
    fn x86_64_asm_export_rejects_call_without_matching_fixup() {
        let target = asm_export_fixture_target();
        let mut object = asm_export_fixture_object(vec![0xE8, 0, 0, 0, 0, 0xC3]);
        object.symbols[0].size = 6;

        assert_eq!(
            export_compiler_owned_object_to_x86_64_asm(&object, &target),
            Err(
                "x86_64 asm export requires fixup for call at offset 1 in function 'entry'"
                    .to_string()
            )
        );
    }

    #[test]
    fn x86_64_asm_export_rejects_wrong_fixup_width() {
        let target = asm_export_fixture_target();
        let mut object = asm_export_fixture_object(vec![0xE8, 0, 0, 0, 0, 0xC3]);
        object.symbols[0].size = 6;
        object.symbols.push(CompilerOwnedObjectSymbol {
            name: "helper".to_string(),
            kind: CompilerOwnedObjectSymbolKind::Function,
            definition: CompilerOwnedObjectSymbolDefinition::DefinedText,
            offset: 5,
            size: 1,
        });
        object.fixups.push(CompilerOwnedObjectFixup {
            source_symbol: "entry".to_string(),
            patch_offset: 1,
            kind: CompilerOwnedFixupKind::X86_64CallRel32,
            target_symbol: "helper".to_string(),
            width_bytes: 2,
        });

        assert_eq!(
            validate_compiler_owned_object_for_x86_64_asm_export(&object, &target),
            Err("x86_64 asm export requires rel32 fixup width 4 for target 'helper'".to_string())
        );
    }

    #[test]
    fn x86_64_asm_export_rejects_pointer_width_mismatch() {
        let mut target = asm_export_fixture_target();
        let object = asm_export_fixture_object(vec![0xC3]);
        target.pointer_bits = 32;

        assert_eq!(
            validate_compiler_owned_object_for_x86_64_asm_export(&object, &target),
            Err("x86_64 asm export pointer width mismatch".to_string())
        );
    }

    #[test]
    fn executable_krir_emits_empty_function_to_deterministic_compiler_owned_object() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("entry")],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let object = lower_executable_krir_to_compiler_owned_object(
            &module,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower compiler-owned object");
        object.validate().expect("valid compiler-owned object");
        let bytes = emit_compiler_owned_object_bytes(&object);
        let header = parse_compiler_owned_header(&bytes);
        let symbols = parse_compiler_owned_symbols(&bytes, &header);
        let fixups = parse_compiler_owned_fixups(&bytes, &header);

        assert_eq!(
            header,
            ParsedCompilerOwnedHeader {
                magic: *b"KRBO",
                version_major: 0,
                version_minor: 1,
                format_revision: 2,
                object_kind: 1,
                target_id: 1,
                endian: 1,
                pointer_bits: 64,
                code_offset: 48,
                code_size: 1,
                symbols_offset: 49,
                symbols_count: 1,
                fixups_offset: 73,
                fixups_count: 0,
                strings_offset: 73,
                strings_size: 7,
            }
        );
        assert_eq!(object.code.bytes, vec![0xC3]);
        assert_eq!(
            symbols,
            vec![ParsedCompilerOwnedSymbol {
                name: "entry".to_string(),
                kind: 1,
                definition: 1,
                offset: 0,
                size: 1,
            }]
        );
        assert!(fixups.is_empty());
        assert_eq!(
            hex_encode(&bytes),
            "4b52424f0001020001010140000000003000000001000000310000000100000049000000000000004900000007000000c301000000010100000000000000000000010000000000000000656e74727900"
        );
    }

    #[test]
    fn executable_krir_emits_functions_in_deterministic_symbol_order_to_compiler_owned_object() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("zeta"), unit_return_function("alpha")],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let object = lower_executable_krir_to_compiler_owned_object(
            &module,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower compiler-owned object");
        let bytes = emit_compiler_owned_object_bytes(&object);
        let header = parse_compiler_owned_header(&bytes);
        let symbols = parse_compiler_owned_symbols(&bytes, &header);

        assert_eq!(
            object
                .symbols
                .iter()
                .map(|symbol| symbol.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
        assert_eq!(
            symbols
                .iter()
                .map(|symbol| symbol.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
    }

    #[test]
    fn executable_krir_emits_direct_call_fixups_in_original_call_order() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![
                            ExecutableOp::Call {
                                callee: "alpha".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "beta".to_string(),
                            },
                        ],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
                ExecutableFunction {
                    name: "beta".to_string(),
                    ..unit_return_function("beta")
                },
            ],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let object = lower_executable_krir_to_compiler_owned_object(
            &module,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower compiler-owned object");
        let bytes = emit_compiler_owned_object_bytes(&object);
        let header = parse_compiler_owned_header(&bytes);
        let fixups = parse_compiler_owned_fixups(&bytes, &header);

        assert_eq!(
            object.code.bytes,
            vec![
                0xC3, 0xC3, 0xE8, 0x00, 0x00, 0x00, 0x00, 0xE8, 0x00, 0x00, 0x00, 0x00, 0xC3
            ]
        );
        assert_eq!(
            object.fixups,
            vec![
                CompilerOwnedObjectFixup {
                    source_symbol: "entry".to_string(),
                    patch_offset: 3,
                    kind: CompilerOwnedFixupKind::X86_64CallRel32,
                    target_symbol: "alpha".to_string(),
                    width_bytes: 4,
                },
                CompilerOwnedObjectFixup {
                    source_symbol: "entry".to_string(),
                    patch_offset: 8,
                    kind: CompilerOwnedFixupKind::X86_64CallRel32,
                    target_symbol: "beta".to_string(),
                    width_bytes: 4,
                },
            ]
        );
        assert_eq!(
            fixups,
            vec![
                ParsedCompilerOwnedFixup {
                    source_symbol: "entry".to_string(),
                    patch_offset: 3,
                    kind: 1,
                    target_symbol: "alpha".to_string(),
                    width_bytes: 4,
                },
                ParsedCompilerOwnedFixup {
                    source_symbol: "entry".to_string(),
                    patch_offset: 8,
                    kind: 1,
                    target_symbol: "beta".to_string(),
                    width_bytes: 4,
                },
            ]
        );
    }

    #[test]
    fn executable_krir_compiler_owned_object_emission_rejects_multiple_blocks() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                blocks: vec![
                    ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    },
                    ExecutableBlock {
                        label: "late".to_string(),
                        ops: vec![],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    },
                ],
                ..executable_function("entry")
            }],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        assert_eq!(
            validate_compiler_owned_object_linear_subset(
                &module,
                &BackendTargetContract::x86_64_sysv()
            ),
            Err(
                "compiler-owned object emission requires exactly one block in function 'entry'"
                    .to_string()
            )
        );
    }

    #[test]
    fn executable_krir_compiler_owned_object_preserves_unresolved_external_target() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "ext".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let object = lower_executable_krir_to_compiler_owned_object(
            &module,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower compiler-owned object");

        assert_eq!(
            object.symbols,
            vec![
                super::CompilerOwnedObjectSymbol {
                    name: "entry".to_string(),
                    kind: super::CompilerOwnedObjectSymbolKind::Function,
                    definition: CompilerOwnedObjectSymbolDefinition::DefinedText,
                    offset: 0,
                    size: 6,
                },
                super::CompilerOwnedObjectSymbol {
                    name: "ext".to_string(),
                    kind: super::CompilerOwnedObjectSymbolKind::Function,
                    definition: CompilerOwnedObjectSymbolDefinition::UndefinedExternal,
                    offset: 0,
                    size: 0,
                },
            ]
        );
        assert_eq!(
            object.fixups,
            vec![CompilerOwnedObjectFixup {
                source_symbol: "entry".to_string(),
                patch_offset: 1,
                kind: CompilerOwnedFixupKind::X86_64CallRel32,
                target_symbol: "ext".to_string(),
                width_bytes: 4,
            }]
        );
    }

    #[test]
    fn executable_krir_emits_empty_function_to_deterministic_elf64_object() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("entry")],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let bytes = emit_x86_64_object_bytes(&object);
        let sections = parse_elf64_sections(&bytes);
        let symbols = parse_elf64_symbols(&bytes, &sections);

        assert_eq!(object.format, "elf64-relocatable");
        assert_eq!(object.text_section, ".text");
        assert_eq!(object.text_bytes, vec![0xC3]);
        assert!(object.undefined_function_symbols.is_empty());
        assert!(object.relocations.is_empty());
        assert_eq!(
            hex_encode(&bytes[0..64]),
            "7f454c4602010100000000000000000001003e000100000000000000000000000000000000000000b80000000000000000000000400000000000400005000400"
        );
        assert_eq!(
            sections,
            vec![
                ParsedElfSection {
                    name: String::new(),
                    sh_type: 0,
                    flags: 0,
                    offset: 0,
                    size: 0,
                    link: 0,
                    info: 0,
                    addralign: 0,
                    entsize: 0,
                },
                ParsedElfSection {
                    name: ".text".to_string(),
                    sh_type: 1,
                    flags: 0x6,
                    offset: 64,
                    size: 1,
                    link: 0,
                    info: 0,
                    addralign: 16,
                    entsize: 0,
                },
                ParsedElfSection {
                    name: ".symtab".to_string(),
                    sh_type: 2,
                    flags: 0,
                    offset: 72,
                    size: 72,
                    link: 3,
                    info: 2,
                    addralign: 8,
                    entsize: 24,
                },
                ParsedElfSection {
                    name: ".strtab".to_string(),
                    sh_type: 3,
                    flags: 0,
                    offset: 144,
                    size: 7,
                    link: 0,
                    info: 0,
                    addralign: 1,
                    entsize: 0,
                },
                ParsedElfSection {
                    name: ".shstrtab".to_string(),
                    sh_type: 3,
                    flags: 0,
                    offset: 151,
                    size: 33,
                    link: 0,
                    info: 0,
                    addralign: 1,
                    entsize: 0,
                },
            ]
        );
        assert_eq!(
            symbols,
            vec![
                ParsedElfSymbol {
                    name: String::new(),
                    info: 0,
                    shndx: 0,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: String::new(),
                    info: 0x03,
                    shndx: 1,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: "entry".to_string(),
                    info: 0x12,
                    shndx: 1,
                    value: 0,
                    size: 1,
                },
            ]
        );
    }

    #[test]
    fn executable_krir_emits_ordered_direct_calls_to_deterministic_object_text() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![
                            ExecutableOp::Call {
                                callee: "alpha".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "beta".to_string(),
                            },
                        ],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
                ExecutableFunction {
                    name: "beta".to_string(),
                    ..unit_return_function("beta")
                },
            ],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let bytes = emit_x86_64_object_bytes(&object);
        let sections = parse_elf64_sections(&bytes);
        let symbols = parse_elf64_symbols(&bytes, &sections);

        assert_eq!(
            object.text_bytes,
            vec![
                0xC3, 0xC3, 0xE8, 0xF9, 0xFF, 0xFF, 0xFF, 0xE8, 0xF5, 0xFF, 0xFF, 0xFF, 0xC3
            ]
        );
        assert_eq!(
            object
                .function_symbols
                .iter()
                .map(|symbol| (symbol.name.as_str(), symbol.offset, symbol.size))
                .collect::<Vec<_>>(),
            vec![("alpha", 0, 1), ("beta", 1, 1), ("entry", 2, 11)]
        );
        assert!(object.undefined_function_symbols.is_empty());
        assert!(object.relocations.is_empty());
        assert!(
            !sections.iter().any(|section| section.name == ".rela.text"),
            "internal-only export must not emit .rela.text"
        );
        assert_eq!(
            symbols,
            vec![
                ParsedElfSymbol {
                    name: String::new(),
                    info: 0,
                    shndx: 0,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: String::new(),
                    info: 0x03,
                    shndx: 1,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: "alpha".to_string(),
                    info: 0x12,
                    shndx: 1,
                    value: 0,
                    size: 1,
                },
                ParsedElfSymbol {
                    name: "beta".to_string(),
                    info: 0x12,
                    shndx: 1,
                    value: 1,
                    size: 1,
                },
                ParsedElfSymbol {
                    name: "entry".to_string(),
                    info: 0x12,
                    shndx: 1,
                    value: 2,
                    size: 11,
                },
            ]
        );
    }

    #[test]
    fn executable_krir_emits_functions_in_deterministic_symbol_order_to_object() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("zeta"), unit_return_function("alpha")],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");

        assert_eq!(
            object
                .function_symbols
                .iter()
                .map(|symbol| symbol.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
    }

    #[test]
    fn x86_64_elf_export_is_derived_from_compiler_owned_object() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![ExecutableOp::Call {
                            callee: "alpha".to_string(),
                        }],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
            ],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let target = BackendTargetContract::x86_64_sysv();
        let compiler_owned = lower_executable_krir_to_compiler_owned_object(&module, &target)
            .expect("lower compiler-owned object");
        let exported = export_compiler_owned_object_to_x86_64_elf(&compiler_owned, &target)
            .expect("export compiler-owned object");
        let wrapped = lower_executable_krir_to_x86_64_object(&module, &target)
            .expect("compatibility wrapper");

        assert_eq!(wrapped, exported);
        assert_eq!(
            exported.text_bytes,
            vec![0xC3, 0xE8, 0xFA, 0xFF, 0xFF, 0xFF, 0xC3]
        );
        assert!(exported.relocations.is_empty());
    }

    #[test]
    fn executable_krir_x86_64_object_emission_rejects_multiple_blocks() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                blocks: vec![
                    ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    },
                    ExecutableBlock {
                        label: "late".to_string(),
                        ops: vec![],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    },
                ],
                ..executable_function("entry")
            }],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        assert_eq!(
            validate_x86_64_object_linear_subset(&module, &BackendTargetContract::x86_64_sysv()),
            Err(
                "x86_64 object emission requires exactly one block in function 'entry'".to_string()
            )
        );
    }

    #[test]
    fn executable_krir_x86_64_object_export_preserves_unresolved_external_relocation() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "ext".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let bytes = emit_x86_64_object_bytes(&object);
        let sections = parse_elf64_sections(&bytes);
        let symbols = parse_elf64_symbols(&bytes, &sections);
        let relocations = parse_elf64_relocations(&bytes, &sections);

        assert_eq!(object.text_bytes, vec![0xE8, 0x00, 0x00, 0x00, 0x00, 0xC3]);
        assert_eq!(object.undefined_function_symbols, vec!["ext".to_string()]);
        assert_eq!(
            object.relocations,
            vec![super::X86_64ElfRelocation {
                offset: 1,
                kind: super::X86_64ElfRelocationKind::X86_64Plt32,
                target_symbol: "ext".to_string(),
                addend: -4,
            }]
        );
        assert!(
            sections.iter().any(|section| section.name == ".rela.text"),
            "expected .rela.text section"
        );
        let rela_idx = sections
            .iter()
            .position(|section| section.name == ".rela.text")
            .expect("rela.text section index");
        let symtab_idx = sections
            .iter()
            .position(|section| section.name == ".symtab")
            .expect("symtab section index");
        let text_idx = sections
            .iter()
            .position(|section| section.name == ".text")
            .expect("text section index");
        assert_eq!(sections[rela_idx].link, symtab_idx as u32);
        assert_eq!(sections[rela_idx].info, text_idx as u32);
        assert_eq!(sections[rela_idx].entsize, 24);
        assert_eq!(
            symbols,
            vec![
                ParsedElfSymbol {
                    name: String::new(),
                    info: 0,
                    shndx: 0,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: String::new(),
                    info: 0x03,
                    shndx: 1,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: "entry".to_string(),
                    info: 0x12,
                    shndx: 1,
                    value: 0,
                    size: 6,
                },
                ParsedElfSymbol {
                    name: "ext".to_string(),
                    info: 0x12,
                    shndx: 0,
                    value: 0,
                    size: 0,
                },
            ]
        );
        assert_eq!(
            relocations,
            vec![ParsedElfRelocation {
                offset: 1,
                sym: 3,
                kind: 4,
                addend: -4,
            }]
        );
    }

    #[test]
    fn executable_krir_x86_64_object_export_mixed_internal_and_external_calls_is_deterministic() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![
                            ExecutableOp::Call {
                                callee: "alpha".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "ext_b".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "ext_a".to_string(),
                            },
                        ],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
            ],
            extern_declarations: vec![
                executable_extern_decl("ext_a"),
                executable_extern_decl("ext_b"),
            ],
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let bytes = emit_x86_64_object_bytes(&object);
        let sections = parse_elf64_sections(&bytes);
        let symbols = parse_elf64_symbols(&bytes, &sections);
        let relocations = parse_elf64_relocations(&bytes, &sections);

        assert_eq!(
            object
                .function_symbols
                .iter()
                .map(|symbol| symbol.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "entry"]
        );
        assert_eq!(
            object.undefined_function_symbols,
            vec!["ext_a".to_string(), "ext_b".to_string()]
        );
        assert_eq!(
            object.relocations,
            vec![
                super::X86_64ElfRelocation {
                    offset: 7,
                    kind: super::X86_64ElfRelocationKind::X86_64Plt32,
                    target_symbol: "ext_b".to_string(),
                    addend: -4,
                },
                super::X86_64ElfRelocation {
                    offset: 12,
                    kind: super::X86_64ElfRelocationKind::X86_64Plt32,
                    target_symbol: "ext_a".to_string(),
                    addend: -4,
                },
            ]
        );
        assert_eq!(
            object.text_bytes,
            vec![
                0xC3, 0xE8, 0xFA, 0xFF, 0xFF, 0xFF, 0xE8, 0x00, 0x00, 0x00, 0x00, 0xE8, 0x00, 0x00,
                0x00, 0x00, 0xC3,
            ]
        );
        assert_eq!(
            symbols,
            vec![
                ParsedElfSymbol {
                    name: String::new(),
                    info: 0,
                    shndx: 0,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: String::new(),
                    info: 0x03,
                    shndx: 1,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: "alpha".to_string(),
                    info: 0x12,
                    shndx: 1,
                    value: 0,
                    size: 1,
                },
                ParsedElfSymbol {
                    name: "entry".to_string(),
                    info: 0x12,
                    shndx: 1,
                    value: 1,
                    size: 16,
                },
                ParsedElfSymbol {
                    name: "ext_a".to_string(),
                    info: 0x12,
                    shndx: 0,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: "ext_b".to_string(),
                    info: 0x12,
                    shndx: 0,
                    value: 0,
                    size: 0,
                },
            ]
        );
        assert_eq!(
            relocations,
            vec![
                ParsedElfRelocation {
                    offset: 7,
                    sym: 5,
                    kind: 4,
                    addend: -4,
                },
                ParsedElfRelocation {
                    offset: 12,
                    sym: 4,
                    kind: 4,
                    addend: -4,
                },
            ]
        );
        let rela_idx = sections
            .iter()
            .position(|section| section.name == ".rela.text")
            .expect("rela.text section index");
        let symtab_idx = sections
            .iter()
            .position(|section| section.name == ".symtab")
            .expect("symtab section index");
        let text_idx = sections
            .iter()
            .position(|section| section.name == ".text")
            .expect("text section index");
        assert_eq!(sections[rela_idx].link, symtab_idx as u32);
        assert_eq!(sections[rela_idx].info, text_idx as u32);
        assert_eq!(sections[rela_idx].entsize, 24);
    }

    #[test]
    fn executable_krir_x86_64_object_export_repeated_external_calls_share_symbol_index() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                name: "entry".to_string(),
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![
                        ExecutableOp::Call {
                            callee: "ext".to_string(),
                        },
                        ExecutableOp::Call {
                            callee: "ext".to_string(),
                        },
                    ],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let bytes = emit_x86_64_object_bytes(&object);
        let sections = parse_elf64_sections(&bytes);
        let symbols = parse_elf64_symbols(&bytes, &sections);
        let relocations = parse_elf64_relocations(&bytes, &sections);

        assert_eq!(object.undefined_function_symbols, vec!["ext".to_string()]);
        assert_eq!(
            symbols,
            vec![
                ParsedElfSymbol {
                    name: String::new(),
                    info: 0,
                    shndx: 0,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: String::new(),
                    info: 0x03,
                    shndx: 1,
                    value: 0,
                    size: 0,
                },
                ParsedElfSymbol {
                    name: "entry".to_string(),
                    info: 0x12,
                    shndx: 1,
                    value: 0,
                    size: 11,
                },
                ParsedElfSymbol {
                    name: "ext".to_string(),
                    info: 0x12,
                    shndx: 0,
                    value: 0,
                    size: 0,
                },
            ]
        );
        assert_eq!(
            relocations,
            vec![
                ParsedElfRelocation {
                    offset: 1,
                    sym: 3,
                    kind: 4,
                    addend: -4,
                },
                ParsedElfRelocation {
                    offset: 6,
                    sym: 3,
                    kind: 4,
                    addend: -4,
                },
            ]
        );
    }

    #[test]
    fn x86_64_elf_internal_only_export_is_accepted_by_external_tools() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![
                            ExecutableOp::Call {
                                callee: "alpha".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "beta".to_string(),
                            },
                        ],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
                ExecutableFunction {
                    name: "beta".to_string(),
                    ..unit_return_function("beta")
                },
            ],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let bytes = emit_x86_64_object_bytes(&object);
        let file = write_temp_elf_file("internal-only", &bytes);
        let Some(readelf_output) = readelf_smoke_output(file.path()) else {
            return;
        };

        let second_output = readelf_smoke_output(file.path()).expect("readelf still available");
        assert_eq!(readelf_output, second_output);
        assert!(readelf_output.contains(".text"));
        assert!(readelf_output.contains(".symtab"));
        assert!(readelf_output.contains("alpha"));
        assert!(readelf_output.contains("beta"));
        assert!(readelf_output.contains("entry"));
        assert!(!readelf_output.contains(".rela.text"));
        let _ = objdump_smoke_check(file.path());
    }

    #[test]
    fn x86_64_elf_external_only_export_is_accepted_by_external_tools() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "ext".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let bytes = emit_x86_64_object_bytes(&object);
        let file = write_temp_elf_file("external-only", &bytes);
        let Some(readelf_output) = readelf_smoke_output(file.path()) else {
            return;
        };

        assert!(readelf_output.contains(".rela.text"));
        assert!(readelf_output.contains("R_X86_64_PLT32"));
        assert!(readelf_output.contains(" ext"));
        let _ = objdump_smoke_check(file.path());
    }

    #[test]
    fn x86_64_elf_mixed_internal_external_export_is_accepted_by_external_tools() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![
                            ExecutableOp::Call {
                                callee: "alpha".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "ext_b".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "ext_a".to_string(),
                            },
                        ],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
            ],
            extern_declarations: vec![
                executable_extern_decl("ext_a"),
                executable_extern_decl("ext_b"),
            ],
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let bytes = emit_x86_64_object_bytes(&object);
        let file = write_temp_elf_file("mixed-internal-external", &bytes);
        let Some(readelf_output) = readelf_smoke_output(file.path()) else {
            return;
        };

        assert!(readelf_output.contains(".text"));
        assert!(readelf_output.contains(".symtab"));
        assert!(readelf_output.contains(".rela.text"));
        assert!(readelf_output.contains("alpha"));
        assert!(readelf_output.contains("entry"));
        assert!(readelf_output.contains("ext_a"));
        assert!(readelf_output.contains("ext_b"));
        assert!(readelf_output.contains("R_X86_64_PLT32"));
        let _ = objdump_smoke_check(file.path());
    }

    #[test]
    fn x86_64_elf_internal_only_object_is_accepted_by_relocatable_link_smoke() {
        let Some(linker) = find_optional_linker() else {
            return;
        };

        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![
                            ExecutableOp::Call {
                                callee: "alpha".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "beta".to_string(),
                            },
                        ],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
                ExecutableFunction {
                    name: "beta".to_string(),
                    ..unit_return_function("beta")
                },
            ],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let input = write_temp_elf_file("ld-r-internal", &emit_x86_64_object_bytes(&object));
        let output_path = temp_output_path("ld-r-internal", ".o");
        let output = run_linker_capture(&linker, &["-r"], &[input.path()], &output_path);
        assert!(
            output.status.success(),
            "linker '{}' rejected internal-only object\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let linked_bytes = linked_output_bytes(&output_path);
        let sections = parse_elf64_sections(&linked_bytes);
        let symbols = parse_elf64_symbols(&linked_bytes, &sections);
        assert!(sections.iter().any(|section| section.name == ".text"));
        assert!(sections.iter().any(|section| section.name == ".symtab"));
        assert!(!sections.iter().any(|section| section.name == ".rela.text"));
        assert!(symbols.iter().any(|symbol| symbol.name == "alpha"));
        assert!(symbols.iter().any(|symbol| symbol.name == "beta"));
        assert!(symbols.iter().any(|symbol| symbol.name == "entry"));
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_unresolved_external_with_resolver_links_successfully() {
        let Some(linker) = find_optional_linker() else {
            return;
        };

        let unresolved = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                name: "entry".to_string(),
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "ext".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };
        let resolver = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("ext")],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let unresolved_obj = lower_executable_krir_to_x86_64_object(
            &unresolved,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower unresolved object");
        let resolver_obj = lower_executable_krir_to_x86_64_object(
            &resolver,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower resolver object");

        let unresolved_path = write_temp_elf_file(
            "ld-resolve-unresolved",
            &emit_x86_64_object_bytes(&unresolved_obj),
        );
        let resolver_path = write_temp_elf_file(
            "ld-resolve-resolver",
            &emit_x86_64_object_bytes(&resolver_obj),
        );
        let output_path = temp_output_path("ld-resolve-linked", ".o");
        let output = run_linker_capture(
            &linker,
            &["-r"],
            &[unresolved_path.path(), resolver_path.path()],
            &output_path,
        );
        assert!(
            output.status.success(),
            "linker '{}' failed to resolve external relocation\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let linked_bytes = linked_output_bytes(&output_path);
        let sections = parse_elf64_sections(&linked_bytes);
        let symbols = parse_elf64_symbols(&linked_bytes, &sections);
        assert!(sections.iter().any(|section| section.name == ".text"));
        assert!(
            symbols
                .iter()
                .any(|symbol| symbol.name == "entry" && symbol.shndx != 0)
        );
        assert!(
            symbols
                .iter()
                .any(|symbol| symbol.name == "ext" && symbol.shndx != 0)
        );
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_mixed_internal_external_object_links_successfully() {
        let Some(linker) = find_optional_linker() else {
            return;
        };

        let mixed = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![
                            ExecutableOp::Call {
                                callee: "alpha".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "ext".to_string(),
                            },
                        ],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
            ],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };
        let resolver = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("ext")],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let mixed_obj =
            lower_executable_krir_to_x86_64_object(&mixed, &BackendTargetContract::x86_64_sysv())
                .expect("lower mixed object");
        let resolver_obj = lower_executable_krir_to_x86_64_object(
            &resolver,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower resolver object");

        let mixed_path = write_temp_elf_file("ld-mixed", &emit_x86_64_object_bytes(&mixed_obj));
        let resolver_path = write_temp_elf_file(
            "ld-mixed-resolver",
            &emit_x86_64_object_bytes(&resolver_obj),
        );
        let output_path = temp_output_path("ld-mixed-linked", ".o");
        let output = run_linker_capture(
            &linker,
            &["-r"],
            &[mixed_path.path(), resolver_path.path()],
            &output_path,
        );
        assert!(
            output.status.success(),
            "linker '{}' failed on mixed internal/external object\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let linked_bytes = linked_output_bytes(&output_path);
        let sections = parse_elf64_sections(&linked_bytes);
        let symbols = parse_elf64_symbols(&linked_bytes, &sections);
        assert!(sections.iter().any(|section| section.name == ".text"));
        assert!(
            symbols
                .iter()
                .any(|symbol| symbol.name == "alpha" && symbol.shndx != 0)
        );
        assert!(
            symbols
                .iter()
                .any(|symbol| symbol.name == "entry" && symbol.shndx != 0)
        );
        assert!(
            symbols
                .iter()
                .any(|symbol| symbol.name == "ext" && symbol.shndx != 0)
        );
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_unresolved_external_without_resolver_fails_link_smoke() {
        let Some(linker) = find_optional_linker() else {
            return;
        };

        let unresolved = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                name: "entry".to_string(),
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "ext".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let unresolved_obj = lower_executable_krir_to_x86_64_object(
            &unresolved,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower unresolved object");
        let unresolved_path = write_temp_elf_file(
            "ld-fail-unresolved",
            &emit_x86_64_object_bytes(&unresolved_obj),
        );
        let output_path = temp_output_path("ld-fail-unresolved-out", ".out");
        let output = run_linker_capture(
            &linker,
            &["-e", "entry"],
            &[unresolved_path.path()],
            &output_path,
        );
        assert!(
            !output.status.success(),
            "linker '{}' unexpectedly accepted unresolved external object",
            linker
        );
        let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
        assert!(
            stderr.contains("undefined") || stderr.contains("unresolved"),
            "expected unresolved-symbol failure from linker '{}', stderr was:\n{}",
            linker,
            String::from_utf8_lossy(&output.stderr)
        );
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_internal_only_object_is_accepted_by_final_link_smoke() {
        let Some(linker) = find_optional_linker() else {
            return;
        };
        let Some(compiler) = find_optional_asm_compiler() else {
            return;
        };

        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![ExecutableOp::Call {
                            callee: "alpha".to_string(),
                        }],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
            ],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let startup = compile_asm_source_to_object(
            &compiler,
            "final-link-start",
            ".globl _start\n.text\n_start:\n    call entry\n    mov $60, %rax\n    xor %rdi, %rdi\n    syscall\n",
        );
        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let input = write_temp_elf_file("final-link-internal", &emit_x86_64_object_bytes(&object));
        let output_path = temp_output_path("final-link-internal", ".out");
        let output = run_linker_capture(
            &linker,
            &["-e", "_start"],
            &[startup.path(), input.path()],
            &output_path,
        );
        assert!(
            output.status.success(),
            "linker '{}' rejected final link for internal-only object\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let linked_bytes = linked_output_bytes(&output_path);
        assert_is_elf64_executable(&linked_bytes);
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_internal_only_object_executes_in_runtime_smoke() {
        let Some(linker) = find_optional_linker() else {
            return;
        };
        let Some(compiler) = find_optional_asm_compiler() else {
            return;
        };

        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![ExecutableOp::Call {
                            callee: "alpha".to_string(),
                        }],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
            ],
            extern_declarations: vec![],
            call_edges: vec![],
        };

        let startup = compile_asm_source_to_object(
            &compiler,
            "runtime-start",
            ".globl _start\n.text\n_start:\n    call entry\n    mov $60, %rax\n    xor %rdi, %rdi\n    syscall\n",
        );
        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");
        let input = write_temp_elf_file("runtime-internal", &emit_x86_64_object_bytes(&object));
        let output_path = temp_output_path("runtime-internal", ".out");
        let output = run_linker_capture(
            &linker,
            &["-e", "_start"],
            &[startup.path(), input.path()],
            &output_path,
        );
        assert!(
            output.status.success(),
            "linker '{}' rejected runtime smoke executable\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let linked_bytes = linked_output_bytes(&output_path);
        assert_is_elf64_executable(&linked_bytes);

        let Some(runtime) = run_executable_smoke(&output_path) else {
            let _ = fs::remove_file(&output_path);
            return;
        };
        assert!(
            runtime.status.success(),
            "runtime smoke executable failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&runtime.stdout),
            String::from_utf8_lossy(&runtime.stderr)
        );
        assert_eq!(runtime.status.code(), Some(0));
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_unresolved_external_with_resolver_is_accepted_by_final_link_smoke() {
        let Some(linker) = find_optional_linker() else {
            return;
        };
        let Some(compiler) = find_optional_asm_compiler() else {
            return;
        };

        let unresolved = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                name: "entry".to_string(),
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "ext".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let startup = compile_asm_source_to_object(
            &compiler,
            "final-link-start-unresolved",
            ".globl _start\n.text\n_start:\n    call entry\n    mov $60, %rax\n    xor %rdi, %rdi\n    syscall\n",
        );
        let resolver = compile_asm_source_to_object(
            &compiler,
            "final-link-resolver",
            ".globl ext\n.text\next:\n    ret\n",
        );
        let unresolved_obj = lower_executable_krir_to_x86_64_object(
            &unresolved,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower unresolved object");
        let unresolved_path = write_temp_elf_file(
            "final-link-unresolved",
            &emit_x86_64_object_bytes(&unresolved_obj),
        );
        let output_path = temp_output_path("final-link-unresolved", ".out");
        let output = run_linker_capture(
            &linker,
            &["-e", "_start"],
            &[startup.path(), unresolved_path.path(), resolver.path()],
            &output_path,
        );
        assert!(
            output.status.success(),
            "linker '{}' failed final link with external resolver\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let linked_bytes = linked_output_bytes(&output_path);
        assert_is_elf64_executable(&linked_bytes);
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_unresolved_external_with_resolver_executes_in_runtime_smoke() {
        let Some(linker) = find_optional_linker() else {
            return;
        };
        let Some(compiler) = find_optional_asm_compiler() else {
            return;
        };

        let unresolved = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                name: "entry".to_string(),
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "ext".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let startup = compile_asm_source_to_object(
            &compiler,
            "runtime-start-unresolved",
            ".globl _start\n.text\n_start:\n    call entry\n    mov $60, %rax\n    xor %rdi, %rdi\n    syscall\n",
        );
        let resolver = compile_asm_source_to_object(
            &compiler,
            "runtime-resolver",
            ".globl ext\n.text\next:\n    ret\n",
        );
        let unresolved_obj = lower_executable_krir_to_x86_64_object(
            &unresolved,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower unresolved object");
        let unresolved_path = write_temp_elf_file(
            "runtime-unresolved",
            &emit_x86_64_object_bytes(&unresolved_obj),
        );
        let output_path = temp_output_path("runtime-unresolved", ".out");
        let output = run_linker_capture(
            &linker,
            &["-e", "_start"],
            &[startup.path(), unresolved_path.path(), resolver.path()],
            &output_path,
        );
        assert!(
            output.status.success(),
            "linker '{}' failed runtime smoke final link with resolver\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let linked_bytes = linked_output_bytes(&output_path);
        assert_is_elf64_executable(&linked_bytes);

        let Some(runtime) = run_executable_smoke(&output_path) else {
            let _ = fs::remove_file(&output_path);
            return;
        };
        assert!(
            runtime.status.success(),
            "runtime smoke executable with resolver failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&runtime.stdout),
            String::from_utf8_lossy(&runtime.stderr)
        );
        assert_eq!(runtime.status.code(), Some(0));
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_mixed_internal_external_object_is_accepted_by_final_link_smoke() {
        let Some(linker) = find_optional_linker() else {
            return;
        };
        let Some(compiler) = find_optional_asm_compiler() else {
            return;
        };

        let mixed = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![
                            ExecutableOp::Call {
                                callee: "alpha".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "ext".to_string(),
                            },
                        ],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
            ],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let startup = compile_asm_source_to_object(
            &compiler,
            "final-link-start-mixed",
            ".globl _start\n.text\n_start:\n    call entry\n    mov $60, %rax\n    xor %rdi, %rdi\n    syscall\n",
        );
        let resolver = compile_asm_source_to_object(
            &compiler,
            "final-link-resolver-mixed",
            ".globl ext\n.text\next:\n    ret\n",
        );
        let mixed_obj =
            lower_executable_krir_to_x86_64_object(&mixed, &BackendTargetContract::x86_64_sysv())
                .expect("lower mixed object");
        let mixed_path =
            write_temp_elf_file("final-link-mixed", &emit_x86_64_object_bytes(&mixed_obj));
        let output_path = temp_output_path("final-link-mixed", ".out");
        let output = run_linker_capture(
            &linker,
            &["-e", "_start"],
            &[startup.path(), mixed_path.path(), resolver.path()],
            &output_path,
        );
        assert!(
            output.status.success(),
            "linker '{}' failed final link for mixed internal/external object\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let linked_bytes = linked_output_bytes(&output_path);
        assert_is_elf64_executable(&linked_bytes);
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_mixed_internal_external_object_executes_in_runtime_smoke() {
        let Some(linker) = find_optional_linker() else {
            return;
        };
        let Some(compiler) = find_optional_asm_compiler() else {
            return;
        };

        let mixed = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![
                ExecutableFunction {
                    name: "entry".to_string(),
                    blocks: vec![ExecutableBlock {
                        label: "entry".to_string(),
                        ops: vec![
                            ExecutableOp::Call {
                                callee: "alpha".to_string(),
                            },
                            ExecutableOp::Call {
                                callee: "ext".to_string(),
                            },
                        ],
                        terminator: ExecutableTerminator::Return {
                            value: ExecutableValue::Unit,
                        },
                    }],
                    ..executable_function("entry")
                },
                ExecutableFunction {
                    name: "alpha".to_string(),
                    ..unit_return_function("alpha")
                },
            ],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let startup = compile_asm_source_to_object(
            &compiler,
            "runtime-start-mixed",
            ".globl _start\n.text\n_start:\n    call entry\n    mov $60, %rax\n    xor %rdi, %rdi\n    syscall\n",
        );
        let resolver = compile_asm_source_to_object(
            &compiler,
            "runtime-resolver-mixed",
            ".globl ext\n.text\next:\n    ret\n",
        );
        let mixed_obj =
            lower_executable_krir_to_x86_64_object(&mixed, &BackendTargetContract::x86_64_sysv())
                .expect("lower mixed object");
        let mixed_path =
            write_temp_elf_file("runtime-mixed", &emit_x86_64_object_bytes(&mixed_obj));
        let output_path = temp_output_path("runtime-mixed", ".out");
        let output = run_linker_capture(
            &linker,
            &["-e", "_start"],
            &[startup.path(), mixed_path.path(), resolver.path()],
            &output_path,
        );
        assert!(
            output.status.success(),
            "linker '{}' failed runtime smoke final link for mixed object\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let linked_bytes = linked_output_bytes(&output_path);
        assert_is_elf64_executable(&linked_bytes);

        let Some(runtime) = run_executable_smoke(&output_path) else {
            let _ = fs::remove_file(&output_path);
            return;
        };
        assert!(
            runtime.status.success(),
            "runtime smoke executable for mixed object failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&runtime.stdout),
            String::from_utf8_lossy(&runtime.stderr)
        );
        assert_eq!(runtime.status.code(), Some(0));
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_unresolved_external_without_resolver_fails_final_link_smoke() {
        let Some(linker) = find_optional_linker() else {
            return;
        };
        let Some(compiler) = find_optional_asm_compiler() else {
            return;
        };

        let unresolved = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                name: "entry".to_string(),
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "ext".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
                ..executable_function("entry")
            }],
            extern_declarations: vec![executable_extern_decl("ext")],
            call_edges: vec![],
        };

        let startup = compile_asm_source_to_object(
            &compiler,
            "final-link-start-fail",
            ".globl _start\n.text\n_start:\n    call entry\n    mov $60, %rax\n    xor %rdi, %rdi\n    syscall\n",
        );
        let unresolved_obj = lower_executable_krir_to_x86_64_object(
            &unresolved,
            &BackendTargetContract::x86_64_sysv(),
        )
        .expect("lower unresolved object");
        let unresolved_path = write_temp_elf_file(
            "final-link-fail-unresolved",
            &emit_x86_64_object_bytes(&unresolved_obj),
        );
        let output_path = temp_output_path("final-link-fail-unresolved", ".out");
        let output = run_linker_capture(
            &linker,
            &["-e", "_start"],
            &[startup.path(), unresolved_path.path()],
            &output_path,
        );
        assert!(
            !output.status.success(),
            "linker '{}' unexpectedly accepted unresolved final link",
            linker
        );
        let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
        assert!(
            stderr.contains("undefined") || stderr.contains("unresolved"),
            "expected unresolved-symbol failure from linker '{}', stderr was:\n{}",
            linker,
            String::from_utf8_lossy(&output.stderr)
        );
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn x86_64_elf_validation_rejects_undefined_symbols_without_relocations() {
        let object = super::X86_64ElfRelocatableObject {
            format: "elf64-relocatable",
            text_section: ".text",
            text_bytes: vec![0xC3],
            function_symbols: vec![super::X86_64ElfFunctionSymbol {
                name: "entry".to_string(),
                offset: 0,
                size: 1,
            }],
            undefined_function_symbols: vec!["ext".to_string()],
            relocations: vec![],
        };

        assert_eq!(
            object.validate(),
            Err("x86_64 ELF undefined function symbols require .rela.text relocations".to_string())
        );
    }

    #[test]
    fn x86_64_elf_validation_rejects_relocations_targeting_defined_symbols() {
        let object = super::X86_64ElfRelocatableObject {
            format: "elf64-relocatable",
            text_section: ".text",
            text_bytes: vec![0xE8, 0x00, 0x00, 0x00, 0x00, 0xC3],
            function_symbols: vec![super::X86_64ElfFunctionSymbol {
                name: "entry".to_string(),
                offset: 0,
                size: 6,
            }],
            undefined_function_symbols: vec![],
            relocations: vec![super::X86_64ElfRelocation {
                offset: 1,
                kind: super::X86_64ElfRelocationKind::X86_64Plt32,
                target_symbol: "entry".to_string(),
                addend: -4,
            }],
        };

        assert_eq!(
            object.validate(),
            Err(
                "x86_64 ELF relocation target 'entry' must be an undefined function symbol"
                    .to_string()
            )
        );
    }

    #[test]
    fn x86_64_elf_validation_rejects_duplicate_relocation_patch_offsets() {
        let object = super::X86_64ElfRelocatableObject {
            format: "elf64-relocatable",
            text_section: ".text",
            text_bytes: vec![0xE8, 0x00, 0x00, 0x00, 0x00, 0xC3],
            function_symbols: vec![super::X86_64ElfFunctionSymbol {
                name: "entry".to_string(),
                offset: 0,
                size: 6,
            }],
            undefined_function_symbols: vec!["ext_a".to_string(), "ext_b".to_string()],
            relocations: vec![
                super::X86_64ElfRelocation {
                    offset: 1,
                    kind: super::X86_64ElfRelocationKind::X86_64Plt32,
                    target_symbol: "ext_a".to_string(),
                    addend: -4,
                },
                super::X86_64ElfRelocation {
                    offset: 1,
                    kind: super::X86_64ElfRelocationKind::X86_64Plt32,
                    target_symbol: "ext_b".to_string(),
                    addend: -4,
                },
            ],
        };

        assert_eq!(
            object.validate(),
            Err("x86_64 ELF relocation patch offset 1 must be unique".to_string())
        );
    }
}
