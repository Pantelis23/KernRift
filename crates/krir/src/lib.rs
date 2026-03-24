use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ctx {
    Boot,
    Irq,
    Nmi,
    Thread,
    /// Host build-tool context — runs on the development machine, not the target kernel.
    Host,
}

impl Ctx {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Boot => "boot",
            Self::Irq => "irq",
            Self::Nmi => "nmi",
            Self::Thread => "thread",
            Self::Host => "host",
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
    /// Read environment variables (host-only).
    Env,
    /// Filesystem access (host-only).
    Fs,
    /// Spawn child processes (host-only).
    Process,
    /// Write to stdout/stderr (host-only).
    Stdout,
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
            Self::Env => "env",
            Self::Fs => "fs",
            Self::Process => "process",
            Self::Stdout => "stdout",
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
            Self::Env,
            Self::Fs,
            Self::Process,
            Self::Stdout,
        ]
    }
}

/// Scheduler hook kind — which scheduler event this function is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedHook {
    SchedIn,
    SchedOut,
}

impl SchedHook {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SchedIn => "sched_in",
            Self::SchedOut => "sched_out",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct FunctionAttrs {
    pub noyield: bool,
    pub critical: bool,
    pub leaf: bool,
    pub hotpath: bool,
    pub lock_budget: Option<u64>,
    /// Scheduler hook kind; `Some` means this function is a sched_in or sched_out callback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<SchedHook>,
    /// C ABI boundary declaration — this function is a stable C-callable export.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_export: bool,
}

/// Per-cpu variable declaration: `percpu NAME: T;`
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PercpuDecl {
    pub name: String,
    pub ty: MmioScalarType,
}

/// A resolved argument value for `CallWithArgs` — either an immediate, a
/// stack-frame slot (by byte offset from %rsp), or the current saved-value
/// register (%rbx).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutableCallArg {
    /// Load an immediate constant into the argument register.
    Imm { value: u64 },
    /// Load a value from `byte_offset(%rsp)` into the argument register.
    Slot { byte_offset: u32 },
    /// Move the current saved-value register (%rbx) into the argument register.
    SavedValue,
}

/// Comparison operation for `CompareIntoSlot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

impl CmpOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Lt => "lt",
            Self::Gt => "gt",
            Self::Le => "le",
            Self::Ge => "ge",
        }
    }
}

/// Floating-point arithmetic operation (SSE2 scalar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FArithOp {
    FAdd,
    FSub,
    FMul,
    FDiv,
}

impl FArithOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FAdd => "fadd",
            Self::FSub => "fsub",
            Self::FMul => "fmul",
            Self::FDiv => "fdiv",
        }
    }
}

/// Arithmetic operation for `CellArithImm` / `SlotArithImm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArithOp {
    Add,
    Sub,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

impl ArithOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Shl => "shl",
            Self::Shr => "shr",
        }
    }
}

/// Named no-argument x86-64 kernel instructions emitted by `asm!(NAME)`.
/// Only valid inside an unsafe block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelIntrinsic {
    Cli,
    Sti,
    Hlt,
    Nop,
    Mfence,
    Sfence,
    Lfence,
    Wbinvd,
    Pause,
    Int3,
    Cpuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum KrirOp {
    Call {
        callee: String,
    },
    CallWithArgs {
        callee: String,
        args: Vec<MmioValueExpr>,
    },
    /// `tail_call(callee[, args...])` — jump to `callee` after tearing down the current frame.
    /// Enables infinite driver loops with zero stack growth.
    TailCall {
        callee: String,
        args: Vec<MmioValueExpr>,
    },
    CallCapture {
        callee: String,
        capture_slot: String,
    },
    /// Call an extern function with arguments and capture its return value into a stack cell.
    CallCaptureWithArgs {
        callee: String,
        args: Vec<MmioValueExpr>,
        capture_slot: String,
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
    UnsafeEnter,
    UnsafeExit,
    YieldPoint,
    AllocPoint,
    BlockPoint,
    Acquire {
        lock_class: String,
    },
    Release {
        lock_class: String,
    },
    ReturnSlot {
        slot: String,
    },
    StackCell {
        ty: MmioScalarType,
        cell: String,
    },
    StackStore {
        ty: MmioScalarType,
        cell: String,
        value: MmioValueExpr,
    },
    StackLoad {
        ty: MmioScalarType,
        cell: String,
        slot: String,
    },
    CellArithImm {
        ty: MmioScalarType,
        cell: String,
        arith_op: ArithOp,
        imm: u64,
    },
    /// `slot_add/sub/and/or/xor/shl/shr<T>(dst, src)` — two-source slot arithmetic.
    /// Reads `src` slot, applies `arith_op`, stores result back into `dst` slot.
    /// Uses `%rax`/`%rcx` as scratch — does not touch the saved-value `%rbx`.
    SlotArith {
        ty: MmioScalarType,
        dst: String,
        src: String,
        arith_op: ArithOp,
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
    /// Load a scalar value from an address stored in a named slot. Only valid inside an unsafe block.
    RawPtrLoad {
        ty: MmioScalarType,
        addr_slot: String,
        out_slot: String,
    },
    /// Store a scalar value to an address stored in a named slot. Only valid inside an unsafe block.
    RawPtrStore {
        ty: MmioScalarType,
        addr_slot: String,
        value: MmioValueExpr,
    },
    /// `slice_len(slice, slot)` — extract the `len` (u64) component of a `[T]` param.
    SliceLen {
        slice: String,
        slot: String,
    },
    /// `slice_ptr(slice, slot)` — extract the `ptr` (u64) component of a `[T]` param.
    SlicePtr {
        slice: String,
        slot: String,
    },
    /// `percpu_read<T>(NAME, slot)` — reads a per-cpu variable into `slot`.
    PercpuRead {
        ty: MmioScalarType,
        name: String,
        slot: String,
    },
    /// `percpu_write<T>(NAME, value)` — writes `value` into a per-cpu variable.
    PercpuWrite {
        ty: MmioScalarType,
        name: String,
        value: MmioValueExpr,
    },
    /// Evaluate `lhs cmp_op rhs` (comparison) and store 0 or 1 into `out` slot.
    /// `out` must already be allocated via `StackCell`.
    CompareIntoSlot {
        cmp_op: CmpOp,
        lhs: String,
        rhs: String,
        out: String,
    },
    /// Opens a loop scope. Pairs with `LoopEnd`.
    LoopBegin,
    /// Jumps back to the matching `LoopBegin` head.
    LoopEnd,
    /// Exits the innermost loop unconditionally.
    LoopBreak,
    /// Jumps to the condition check of the innermost loop.
    LoopContinue,
    /// If `slot == 0`, exit the innermost loop.
    BranchIfZeroLoopBreak {
        slot: String,
    },
    /// If `slot != 0`, exit the innermost loop.
    BranchIfNonZeroLoopBreak {
        slot: String,
    },
    /// Floating-point arithmetic: `dst = dst op src` (SSE2 scalar). Requires Task 11 backend.
    FloatArith {
        ty: MmioScalarType,
        fop: FArithOp,
        dst: String,
        src: String,
    },
    /// Emit a named kernel intrinsic instruction. Only valid inside an unsafe block.
    InlineAsm(KernelIntrinsic),
    /// Load the address of a C-string literal into a u64 stack cell.
    /// `value` is the raw string content (without NUL terminator); the NUL is appended on emit.
    LoadStaticCstrAddr {
        value: String,
        cell: String,
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
    FloatLiteral { value: String },
}

impl MmioValueExpr {
    pub fn as_source(&self) -> String {
        match self {
            Self::Ident { name } => name.clone(),
            Self::IntLiteral { value } => value.clone(),
            Self::FloatLiteral { value } => value.clone(),
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
    F32,
    F64,
}

impl MmioScalarType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }

    /// Width in bytes.
    pub fn byte_width(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U16 => 2,
            Self::U32 | Self::F32 => 4,
            Self::U64 | Self::F64 => 8,
        }
    }

    /// True for the two floating-point variants.
    pub fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }
}

/// A KRIR function parameter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KrirParamTy {
    /// Single integer value (u8/u16/u32/u64).
    Scalar { ty: MmioScalarType },
    /// Fat-pointer slice `[T]`: (ptr: u64, len: u64) under SysV ABI.
    Slice { elem: MmioScalarType },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Function {
    pub name: String,
    pub is_extern: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<(String, KrirParamTy)>,
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
    /// Declared spinlock classes: `spinlock NAME;`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lock_classes: Vec<String>,
    /// Declared per-cpu variables: `percpu NAME: T;`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub percpu_vars: Vec<PercpuDecl>,
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
    U8,
    U16,
    U32,
    U64,
}

impl ExecutableValueType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
        }
    }
}

fn executable_value_type_from_mmio_scalar(ty: MmioScalarType) -> ExecutableValueType {
    match ty {
        MmioScalarType::U8 => ExecutableValueType::U8,
        MmioScalarType::U16 => ExecutableValueType::U16,
        MmioScalarType::U32 => ExecutableValueType::U32,
        MmioScalarType::U64 => ExecutableValueType::U64,
        MmioScalarType::F32 => ExecutableValueType::U32,
        MmioScalarType::F64 => ExecutableValueType::U64,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutableValue {
    Unit,
    SavedValue { ty: MmioScalarType },
}

impl ExecutableValue {
    pub fn value_type(&self) -> ExecutableValueType {
        match self {
            Self::Unit => ExecutableValueType::Unit,
            Self::SavedValue { ty } => executable_value_type_from_mmio_scalar(*ty),
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
    CallWithArgs {
        callee: String,
        args: Vec<ExecutableCallArg>,
    },
    CallCapture {
        callee: String,
        ty: MmioScalarType,
    },
    /// Call an extern function with args; store the return value directly into a stack slot.
    CallCaptureWithArgs {
        callee: String,
        args: Vec<ExecutableCallArg>,
        ty: MmioScalarType,
        slot_idx: u8,
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
    StackStoreImm {
        ty: MmioScalarType,
        value: u64,
        slot_idx: u8,
    },
    StackStoreValue {
        ty: MmioScalarType,
        slot_idx: u8,
    },
    StackLoad {
        ty: MmioScalarType,
        slot_idx: u8,
    },
    SlotArithImm {
        ty: MmioScalarType,
        slot_idx: u8,
        arith_op: ArithOp,
        imm: u64,
    },
    SlotArithSlot {
        ty: MmioScalarType,
        dst_slot_idx: u8,
        src_slot_idx: u8,
        arith_op: ArithOp,
    },
    ParamLoad {
        param_idx: u8,
        ty: MmioScalarType,
    },
    // MMIO ops whose address comes from a u64 function parameter spilled to the stack frame.
    MmioReadParamAddr {
        param_idx: u8,
        ty: MmioScalarType,
        capture_value: bool,
    },
    MmioWriteImmParamAddr {
        param_idx: u8,
        ty: MmioScalarType,
        value: u64,
    },
    MmioWriteValueParamAddr {
        param_idx: u8,
        ty: MmioScalarType,
    },
    /// Loop begin marker — generates a head label in ASM.
    LoopBegin,
    /// Jump back to loop head — generates a backward jmp in ASM.
    LoopEnd,
    /// Unconditional jump to loop end — break.
    LoopBreak,
    /// Jump to loop head — continue.
    LoopContinue,
    /// Conditional break: if slot_idx == 0, jump to loop end.
    BranchIfZeroLoopBreak {
        slot_idx: u8,
    },
    /// Conditional break: if slot_idx != 0, jump to loop end.
    BranchIfNonZeroLoopBreak {
        slot_idx: u8,
    },
    /// Compare lhs_idx op rhs_idx and store 0 or 1 in out_idx.
    CompareIntoSlot {
        cmp_op: CmpOp,
        lhs_idx: u8,
        rhs_idx: u8,
        out_idx: u8,
    },
    /// Load a scalar from the address held in addr_slot_idx and store into out_slot_idx.
    RawPtrLoad {
        ty: MmioScalarType,
        addr_slot_idx: u8,
        out_slot_idx: u8,
    },
    /// Store a scalar value to the address held in addr_slot_idx.
    RawPtrStore {
        ty: MmioScalarType,
        addr_slot_idx: u8,
        value: MmioValueExpr,
    },
    /// Emit a named kernel intrinsic instruction. Only valid inside an unsafe block.
    InlineAsm(KernelIntrinsic),
    /// Load the address of a module-level C-string constant into a u64 stack slot.
    /// `str_idx` indexes into `ExecutableKrirModule::static_strings`.
    LoadStaticCstrAddr {
        str_idx: u8,
        slot_idx: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "terminator", rename_all = "snake_case")]
pub enum ExecutableTerminator {
    Return {
        value: ExecutableValue,
    },
    /// Tear down the current frame, load args into SysV registers, then `jmp callee`.
    /// No return address is pushed — enables infinite loops with zero stack growth.
    TailCall {
        callee: String,
        args: Vec<ExecutableCallArg>,
    },
}

impl ExecutableTerminator {
    fn value_type(&self) -> ExecutableValueType {
        match self {
            Self::Return { value } => value.value_type(),
            Self::TailCall { .. } => ExecutableValueType::Unit,
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
    /// C-string literals referenced by `LoadStaticCstrAddr` ops.
    /// Index N corresponds to `LoadStaticCstrAddr { str_idx: N, .. }`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub static_strings: Vec<String>,
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
        let mut function_results = BTreeMap::new();

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

            function_results.insert(function.name.as_str(), function.signature.result);
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
                        ExecutableOp::Call { callee }
                        | ExecutableOp::CallWithArgs { callee, .. } => {
                            if !function_names.contains(callee.as_str())
                                && !extern_names.contains(callee.as_str())
                            {
                                return Err(format!(
                                    "executable KRIR function '{}' calls undeclared target '{}'",
                                    function.name, callee
                                ));
                            }
                        }
                        ExecutableOp::CallCapture { callee, ty } => {
                            let Some(result) = function_results.get(callee.as_str()) else {
                                if !extern_names.contains(callee.as_str()) {
                                    return Err(format!(
                                        "executable KRIR function '{}' calls undeclared target '{}'",
                                        function.name, callee
                                    ));
                                }
                                return Err(format!(
                                    "executable KRIR function '{}' must not capture extern target '{}'",
                                    function.name, callee
                                ));
                            };
                            if *result != executable_value_type_from_mmio_scalar(*ty) {
                                return Err(format!(
                                    "executable KRIR function '{}' captures '{}' as {} but callee returns {}",
                                    function.name,
                                    callee,
                                    ty.as_str(),
                                    result.as_str()
                                ));
                            }
                        }
                        ExecutableOp::CallCaptureWithArgs { callee, .. } => {
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
                        | ExecutableOp::MmioWriteValue { .. }
                        | ExecutableOp::StackStoreImm { .. }
                        | ExecutableOp::StackStoreValue { .. }
                        | ExecutableOp::StackLoad { .. }
                        | ExecutableOp::SlotArithImm { .. }
                        | ExecutableOp::SlotArithSlot { .. }
                        | ExecutableOp::ParamLoad { .. }
                        | ExecutableOp::MmioReadParamAddr { .. }
                        | ExecutableOp::MmioWriteImmParamAddr { .. }
                        | ExecutableOp::MmioWriteValueParamAddr { .. }
                        | ExecutableOp::LoopBegin
                        | ExecutableOp::LoopEnd
                        | ExecutableOp::LoopBreak
                        | ExecutableOp::LoopContinue
                        | ExecutableOp::BranchIfZeroLoopBreak { .. }
                        | ExecutableOp::BranchIfNonZeroLoopBreak { .. }
                        | ExecutableOp::CompareIntoSlot { .. }
                        | ExecutableOp::RawPtrLoad { .. }
                        | ExecutableOp::RawPtrStore { .. }
                        | ExecutableOp::InlineAsm(_)
                        | ExecutableOp::LoadStaticCstrAddr { .. } => {}
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ExecutableCapturedValueSource {
    DeferredRead { op_index: usize },
    SavedSlot,
}

#[derive(Clone)]
struct ExecutableCapturedValue {
    slot: String,
    ty: MmioScalarType,
    source: ExecutableCapturedValueSource,
}

fn infer_executable_function_result_types(
    module: &KrirModule,
) -> Result<BTreeMap<String, Option<MmioScalarType>>, Vec<String>> {
    let mut results = BTreeMap::new();
    let mut errors = Vec::new();

    for function in &module.functions {
        if function.is_extern {
            continue;
        }

        let mut executable_slot_name = None::<String>;
        let mut last_read = None::<(String, MmioScalarType)>;
        let mut result_ty = None::<MmioScalarType>;
        let mut saw_return_slot = false;

        // Pre-pass: build cell→type from StackCell declarations so that
        // CallCaptureWithArgs captures can be resolved to their types.
        let mut cell_type_map: std::collections::HashMap<&str, MmioScalarType> =
            std::collections::HashMap::new();
        for op in &function.ops {
            if let KrirOp::StackCell { ty, cell } = op {
                cell_type_map.insert(cell.as_str(), *ty);
            }
        }

        for (index, op) in function.ops.iter().enumerate() {
            match op {
                KrirOp::MmioRead {
                    ty, capture_slot, ..
                }
                | KrirOp::RawMmioRead {
                    ty, capture_slot, ..
                } => {
                    let slot_name = capture_slot
                        .clone()
                        .unwrap_or_else(|| DEFAULT_EXECUTABLE_MMIO_SLOT.to_string());
                    if executable_slot_name.is_none() {
                        executable_slot_name = Some(slot_name.clone());
                    }
                    last_read = Some((slot_name, *ty));
                }
                KrirOp::CallCaptureWithArgs { capture_slot, .. } => {
                    if let Some(&ty) = cell_type_map.get(capture_slot.as_str()) {
                        if executable_slot_name.is_none() {
                            executable_slot_name = Some(capture_slot.clone());
                        }
                        last_read = Some((capture_slot.clone(), ty));
                    }
                }
                KrirOp::ReturnSlot { slot } => {
                    let invocation = format_return_slot_invocation(slot);
                    if saw_return_slot || index + 1 != function.ops.len() {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: return_slot(...) must be the final statement in the function",
                            function.name,
                            invocation
                        ));
                        continue;
                    }
                    saw_return_slot = true;

                    if let Some(captured_slot) = executable_slot_name.as_deref()
                        && slot != captured_slot
                    {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: return slot '{}' does not match the captured executable slot '{}' in this function",
                            function.name,
                            invocation,
                            slot,
                            captured_slot
                        ));
                        continue;
                    }

                    let Some((read_slot, read_ty)) = last_read.as_ref() else {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: return slot '{}' requires a prior mmio_read, raw_mmio_read, or call_capture_with_args that captures into '{}' in the same function",
                            function.name,
                            invocation,
                            slot,
                            slot
                        ));
                        continue;
                    };

                    if read_slot != slot {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: return slot '{}' requires a prior mmio_read, raw_mmio_read, or call_capture_with_args that captures into '{}' in the same function",
                            function.name,
                            invocation,
                            slot,
                            slot
                        ));
                        continue;
                    }

                    result_ty = Some(*read_ty);
                }
                _ => {}
            }
        }

        results.insert(function.name.clone(), result_ty);
    }

    if errors.is_empty() {
        Ok(results)
    } else {
        Err(errors)
    }
}

pub fn lower_current_krir_to_executable_krir(
    module: &KrirModule,
) -> Result<ExecutableKrirModule, Vec<String>> {
    let mmio_bases = build_mmio_base_map(module)?;
    let function_result_types = infer_executable_function_result_types(module)?;

    // Pre-pass: collect unique C-string literals in module order for stable indexing.
    let mut string_indices: BTreeMap<String, u8> = BTreeMap::new();
    let mut static_strings: Vec<String> = Vec::new();
    for function in &module.functions {
        for op in &function.ops {
            if let KrirOp::LoadStaticCstrAddr { value, .. } = op {
                if !string_indices.contains_key(value.as_str()) {
                    let idx = u8::try_from(static_strings.len()).unwrap_or_else(|_| {
                        panic!("too many string literals (max 256)")
                    });
                    string_indices.insert(value.clone(), idx);
                    static_strings.push(value.clone());
                }
            }
        }
    }

    let mut lowered = ExecutableKrirModule {
        module_caps: module.module_caps.clone(),
        functions: Vec::new(),
        extern_declarations: Vec::new(),
        call_edges: module.call_edges.clone(),
        static_strings,
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
        let mut cell_slot_map: BTreeMap<String, (u8, MmioScalarType)> = BTreeMap::new();
        let mut next_slot_idx: u8 = 0;
        let mut last_value = None::<ExecutableCapturedValue>;
        let mut tail_call_terminator: Option<(String, Vec<ExecutableCallArg>)> = None;
        // Build lookup maps from param name to ABI slot indices.
        // Scalar params: 1 ABI slot each.  Slice params: 2 ABI slots (ptr then len).
        let mut param_map: std::collections::BTreeMap<&str, (u8, MmioScalarType)> =
            std::collections::BTreeMap::new();
        let mut slice_param_map: std::collections::BTreeMap<&str, (u8, u8, MmioScalarType)> =
            std::collections::BTreeMap::new();
        let mut abi_idx: u8 = 0;
        for (name, ty) in &function.params {
            match ty {
                KrirParamTy::Scalar { ty: scalar_ty } => {
                    param_map.insert(name.as_str(), (abi_idx, *scalar_ty));
                    abi_idx += 1;
                }
                KrirParamTy::Slice { elem } => {
                    slice_param_map.insert(name.as_str(), (abi_idx, abi_idx + 1, *elem));
                    abi_idx += 2;
                }
            }
        }
        for op in &function.ops {
            match op {
                KrirOp::Call { callee } => exec_ops.push(ExecutableOp::Call {
                    callee: callee.clone(),
                }),
                KrirOp::CallWithArgs { callee, args } => {
                    if args.len() > 6 {
                        errors.push(format!(
                            "canonical-exec: function '{}' calls '{}' with {} args; SysV ABI allows at most 6",
                            function.name, callee, args.len()
                        ));
                        continue;
                    }
                    let mut exec_args = Vec::with_capacity(args.len());
                    let mut arg_ok = true;
                    for arg_val in args {
                        let exec_arg = match arg_val {
                            MmioValueExpr::IntLiteral { value } => {
                                match parse_integer_literal_u64(value) {
                                    Ok(v) => ExecutableCallArg::Imm { value: v },
                                    Err(reason) => {
                                        errors.push(format!(
                                            "canonical-exec: function '{}' call_with_args immediate '{}': {}",
                                            function.name, value, reason
                                        ));
                                        arg_ok = false;
                                        break;
                                    }
                                }
                            }
                            MmioValueExpr::FloatLiteral { value } => {
                                errors.push(format!(
                                    "float literal '{}' not supported in MMIO write",
                                    value
                                ));
                                arg_ok = false;
                                break;
                            }
                            MmioValueExpr::Ident { name } => {
                                if let Some(&(slot_idx, _)) = cell_slot_map.get(name.as_str()) {
                                    ExecutableCallArg::Slot {
                                        byte_offset: 8u32 * u32::from(slot_idx),
                                    }
                                } else if let Some(&(param_idx, _)) = param_map.get(name.as_str()) {
                                    ExecutableCallArg::Slot {
                                        byte_offset: 8u32 * u32::from(next_slot_idx)
                                            + 8u32 * u32::from(param_idx),
                                    }
                                } else if executable_slot_name.as_deref() == Some(name.as_str()) {
                                    ExecutableCallArg::SavedValue
                                } else {
                                    errors.push(format!(
                                        "canonical-exec: function '{}' call_with_args arg '{}': not a declared stack cell, param, or captured slot",
                                        function.name, name
                                    ));
                                    arg_ok = false;
                                    break;
                                }
                            }
                        };
                        exec_args.push(exec_arg);
                    }
                    if arg_ok {
                        exec_ops.push(ExecutableOp::CallWithArgs {
                            callee: callee.clone(),
                            args: exec_args,
                        });
                    }
                }
                KrirOp::TailCall { callee, args } => {
                    if args.len() > 6 {
                        errors.push(format!(
                            "canonical-exec: function '{}' tail_calls '{}' with {} args; SysV ABI allows at most 6",
                            function.name, callee, args.len()
                        ));
                        continue;
                    }
                    let mut exec_args = Vec::with_capacity(args.len());
                    let mut arg_ok = true;
                    for arg_val in args {
                        let exec_arg = match arg_val {
                            MmioValueExpr::IntLiteral { value } => {
                                match parse_integer_literal_u64(value) {
                                    Ok(v) => ExecutableCallArg::Imm { value: v },
                                    Err(reason) => {
                                        errors.push(format!(
                                            "canonical-exec: function '{}' tail_call immediate '{}': {}",
                                            function.name, value, reason
                                        ));
                                        arg_ok = false;
                                        break;
                                    }
                                }
                            }
                            MmioValueExpr::FloatLiteral { value } => {
                                errors.push(format!(
                                    "float literal '{}' not supported in MMIO write",
                                    value
                                ));
                                arg_ok = false;
                                break;
                            }
                            MmioValueExpr::Ident { name } => {
                                if let Some(&(slot_idx, _)) = cell_slot_map.get(name.as_str()) {
                                    ExecutableCallArg::Slot {
                                        byte_offset: 8u32 * u32::from(slot_idx),
                                    }
                                } else if let Some(&(param_idx, _)) = param_map.get(name.as_str()) {
                                    ExecutableCallArg::Slot {
                                        byte_offset: 8u32 * u32::from(next_slot_idx)
                                            + 8u32 * u32::from(param_idx),
                                    }
                                } else if executable_slot_name.as_deref() == Some(name.as_str()) {
                                    ExecutableCallArg::SavedValue
                                } else {
                                    errors.push(format!(
                                        "canonical-exec: function '{}' tail_call arg '{}': not a declared stack cell, param, or captured slot",
                                        function.name, name
                                    ));
                                    arg_ok = false;
                                    break;
                                }
                            }
                        };
                        exec_args.push(exec_arg);
                    }
                    if arg_ok {
                        tail_call_terminator = Some((callee.clone(), exec_args));
                    }
                }
                KrirOp::CallCapture {
                    callee,
                    capture_slot,
                } => {
                    let invocation = format_call_capture_invocation(callee, capture_slot);
                    let Some(return_ty) = function_result_types.get(callee).copied().flatten()
                    else {
                        if module
                            .functions
                            .iter()
                            .any(|f| f.name == *callee && f.is_extern)
                        {
                            errors.push(format!(
                                "canonical-exec: function '{}' contains unsupported {}: captured call target '{}' must be an internal executable function with scalar return in the current subset",
                                function.name,
                                invocation,
                                callee
                            ));
                        } else {
                            errors.push(format!(
                                "canonical-exec: function '{}' contains unsupported {}: captured call target '{}' returns unit in the current executable subset",
                                function.name,
                                invocation,
                                callee
                            ));
                        }
                        continue;
                    };
                    if let Some(existing) = &executable_slot_name {
                        if existing != capture_slot {
                            errors.push(format!(
                                "canonical-exec: function '{}' contains unsupported {}: executable value slot '{}' conflicts with already-captured slot '{}' in the same function",
                                function.name,
                                invocation,
                                capture_slot,
                                existing
                            ));
                            continue;
                        }
                    } else {
                        executable_slot_name = Some(capture_slot.clone());
                    }
                    exec_ops.push(ExecutableOp::CallCapture {
                        callee: callee.clone(),
                        ty: return_ty,
                    });
                    last_value = Some(ExecutableCapturedValue {
                        slot: capture_slot.clone(),
                        ty: return_ty,
                        source: ExecutableCapturedValueSource::SavedSlot,
                    });
                }
                KrirOp::CallCaptureWithArgs {
                    callee,
                    args,
                    capture_slot,
                } => {
                    if args.len() > 6 {
                        errors.push(format!(
                            "canonical-exec: function '{}' calls '{}' with {} args; SysV ABI allows at most 6",
                            function.name, callee, args.len()
                        ));
                        continue;
                    }
                    // Resolve the capture slot's index and type from cell_slot_map.
                    let Some(&(slot_idx, ty)) = cell_slot_map.get(capture_slot.as_str()) else {
                        errors.push(format!(
                            "canonical-exec: function '{}' call_capture_with_args: capture slot '{}' not found in cell_slot_map",
                            function.name, capture_slot
                        ));
                        continue;
                    };
                    // Resolve each argument.
                    let mut exec_args = Vec::with_capacity(args.len());
                    let mut arg_ok = true;
                    for arg_val in args {
                        let exec_arg = match arg_val {
                            MmioValueExpr::IntLiteral { value } => {
                                match parse_integer_literal_u64(value) {
                                    Ok(v) => ExecutableCallArg::Imm { value: v },
                                    Err(reason) => {
                                        errors.push(format!(
                                            "canonical-exec: function '{}' call_capture_with_args immediate '{}': {}",
                                            function.name, value, reason
                                        ));
                                        arg_ok = false;
                                        break;
                                    }
                                }
                            }
                            MmioValueExpr::FloatLiteral { value } => {
                                errors.push(format!(
                                    "float literal '{}' not supported in call_capture_with_args",
                                    value
                                ));
                                arg_ok = false;
                                break;
                            }
                            MmioValueExpr::Ident { name } => {
                                if let Some(&(s_idx, _)) = cell_slot_map.get(name.as_str()) {
                                    ExecutableCallArg::Slot {
                                        byte_offset: 8u32 * u32::from(s_idx),
                                    }
                                } else if let Some(&(param_idx, _)) = param_map.get(name.as_str()) {
                                    ExecutableCallArg::Slot {
                                        byte_offset: 8u32 * u32::from(next_slot_idx)
                                            + 8u32 * u32::from(param_idx),
                                    }
                                } else if executable_slot_name.as_deref() == Some(name.as_str()) {
                                    ExecutableCallArg::SavedValue
                                } else {
                                    errors.push(format!(
                                        "canonical-exec: function '{}' call_capture_with_args arg '{}': not a declared stack cell, param, or captured slot",
                                        function.name, name
                                    ));
                                    arg_ok = false;
                                    break;
                                }
                            }
                        };
                        exec_args.push(exec_arg);
                    }
                    if arg_ok {
                        exec_ops.push(ExecutableOp::CallCaptureWithArgs {
                            callee: callee.clone(),
                            args: exec_args,
                            ty,
                            slot_idx,
                        });
                        // Load the result back from the stack slot into %rbx so
                        // ReturnSavedValue and downstream SavedSlot consumers work.
                        exec_ops.push(ExecutableOp::StackLoad { ty, slot_idx });
                        if executable_slot_name.is_none() {
                            executable_slot_name = Some(capture_slot.clone());
                        }
                        last_value = Some(ExecutableCapturedValue {
                            slot: capture_slot.clone(),
                            ty,
                            source: ExecutableCapturedValueSource::SavedSlot,
                        });
                    }
                }
                KrirOp::BranchIfZero {
                    slot,
                    then_callee,
                    else_callee,
                } => {
                    // New-style path: slot is a CompareIntoSlot result (or any stack cell).
                    // Load the cell into %rbx via StackLoad, then branch.
                    if let Some(&(slot_idx, slot_ty)) = cell_slot_map.get(slot.as_str()) {
                        let captured_slot_matches = executable_slot_name
                            .as_deref()
                            .map(|cs| cs == slot)
                            .unwrap_or(false);
                        if !captured_slot_matches {
                            // Not the mmio-captured slot — load from stack cell into %rbx.
                            exec_ops.push(ExecutableOp::StackLoad {
                                ty: slot_ty,
                                slot_idx,
                            });
                        }
                        exec_ops.push(ExecutableOp::BranchIfZero {
                            ty: slot_ty,
                            then_callee: then_callee.clone(),
                            else_callee: else_callee.clone(),
                        });
                        continue;
                    }
                    // Old-style path: slot must be the mmio/call captured slot.
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
                    let Some(current_value) = last_value.as_ref() else {
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
                    let read_ty = current_value.ty;
                    if let ExecutableCapturedValueSource::DeferredRead { op_index } =
                        current_value.source
                    {
                        let Some(ExecutableOp::MmioRead { capture_value, .. }) =
                            exec_ops.get_mut(op_index)
                        else {
                            unreachable!("branch-if-zero must point at a prior mmio read op");
                        };
                        *capture_value = true;
                    }
                    exec_ops.push(ExecutableOp::BranchIfZero {
                        ty: read_ty,
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
                    let Some(current_value) = last_value.as_ref() else {
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
                    let read_ty = current_value.ty;
                    let resolved_compare_value =
                        match resolve_executable_branch_compare_value(read_ty, compare_value) {
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
                    if let ExecutableCapturedValueSource::DeferredRead { op_index } =
                        current_value.source
                    {
                        let Some(ExecutableOp::MmioRead { capture_value, .. }) =
                            exec_ops.get_mut(op_index)
                        else {
                            unreachable!("branch-if-eq must point at a prior mmio read op");
                        };
                        *capture_value = true;
                    }
                    exec_ops.push(ExecutableOp::BranchIfEqImm {
                        ty: read_ty,
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
                    let Some(current_value) = last_value.as_ref() else {
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
                    let read_ty = current_value.ty;
                    let resolved_mask_value =
                        match resolve_executable_branch_mask_value(read_ty, mask_value) {
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
                    if let ExecutableCapturedValueSource::DeferredRead { op_index } =
                        current_value.source
                    {
                        let Some(ExecutableOp::MmioRead { capture_value, .. }) =
                            exec_ops.get_mut(op_index)
                        else {
                            unreachable!(
                                "branch-if-mask-nonzero must point at a prior mmio read op"
                            );
                        };
                        *capture_value = true;
                    }
                    exec_ops.push(ExecutableOp::BranchIfMaskNonZeroImm {
                        ty: read_ty,
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
                KrirOp::UnsafeEnter => {}
                KrirOp::UnsafeExit => {}
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
                KrirOp::ReturnSlot { .. } => {}
                KrirOp::StackCell { ty, cell } => {
                    let invocation = format_stack_cell_invocation(*ty, cell);
                    if let Some(&(_, existing_ty)) = cell_slot_map.get(cell.as_str()) {
                        if existing_ty != *ty {
                            errors.push(format!(
                                "canonical-exec: function '{}' contains unsupported {}: stack cell '{}' is redeclared as {} but was already declared as {} in this function",
                                function.name,
                                invocation,
                                cell,
                                ty.as_str(),
                                existing_ty.as_str()
                            ));
                        }
                        // idempotent re-declaration of same cell/type: no-op
                    } else {
                        cell_slot_map.insert(cell.clone(), (next_slot_idx, *ty));
                        next_slot_idx = next_slot_idx.saturating_add(1);
                    }
                }
                KrirOp::StackStore { ty, cell, value } => {
                    let invocation = format_cell_store_invocation(*ty, cell, value);
                    let slot_idx =
                        match resolve_executable_stack_cell_slot(*ty, cell, &cell_slot_map) {
                            Ok(idx) => idx,
                            Err(reason) => {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported {}: {}",
                                    function.name, invocation, reason
                                ));
                                continue;
                            }
                        };
                    let resolved_value = match value {
                        MmioValueExpr::FloatLiteral { value: fv } => Err(format!(
                            "float literal '{}' not supported in MMIO write",
                            fv
                        )),
                        MmioValueExpr::IntLiteral { .. } => resolve_executable_mmio_write_value(
                            *ty,
                            value,
                            executable_slot_name.as_deref(),
                            None,
                        ),
                        MmioValueExpr::Ident { name } => {
                            if let Some(&(param_idx, param_ty)) = param_map.get(name.as_str()) {
                                exec_ops.push(ExecutableOp::ParamLoad {
                                    param_idx,
                                    ty: param_ty,
                                });
                                Ok(ExecutableMmioWriteValue::SavedValue)
                            } else if matches!(
                                last_value.as_ref().map(|value| value.source),
                                Some(ExecutableCapturedValueSource::SavedSlot)
                            ) {
                                resolve_executable_saved_slot_write_value(
                                    *ty,
                                    name,
                                    executable_slot_name.as_deref(),
                                    last_value
                                        .as_ref()
                                        .map(|value| (value.slot.as_str(), value.ty)),
                                )
                            } else {
                                resolve_executable_mmio_write_value(
                                    *ty,
                                    value,
                                    executable_slot_name.as_deref(),
                                    last_value.as_ref().and_then(|value| match value.source {
                                        ExecutableCapturedValueSource::DeferredRead { .. } => {
                                            Some((value.slot.as_str(), value.ty))
                                        }
                                        ExecutableCapturedValueSource::SavedSlot => None,
                                    }),
                                )
                            }
                        }
                    };
                    let resolved_value = match resolved_value {
                        Ok(value) => value,
                        Err(reason) => {
                            errors.push(format!(
                                "canonical-exec: function '{}' contains unsupported {}: {}",
                                function.name, invocation, reason
                            ));
                            continue;
                        }
                    };
                    match resolved_value {
                        ExecutableMmioWriteValue::Immediate(value) => {
                            exec_ops.push(ExecutableOp::StackStoreImm {
                                ty: *ty,
                                value,
                                slot_idx,
                            });
                        }
                        ExecutableMmioWriteValue::SavedValue => {
                            if let Some(current_value) = last_value.as_ref()
                                && let ExecutableCapturedValueSource::DeferredRead { op_index } =
                                    current_value.source
                            {
                                let Some(ExecutableOp::MmioRead { capture_value, .. }) =
                                    exec_ops.get_mut(op_index)
                                else {
                                    unreachable!(
                                        "saved executable stack store must point at a prior mmio read op"
                                    );
                                };
                                *capture_value = true;
                            }
                            exec_ops.push(ExecutableOp::StackStoreValue { ty: *ty, slot_idx });
                        }
                    }
                }
                KrirOp::StackLoad { ty, cell, slot } => {
                    let invocation = format_cell_load_invocation(*ty, cell, slot);
                    let slot_idx =
                        match resolve_executable_stack_cell_slot(*ty, cell, &cell_slot_map) {
                            Ok(idx) => idx,
                            Err(reason) => {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported {}: {}",
                                    function.name, invocation, reason
                                ));
                                continue;
                            }
                        };
                    if let Some(existing) = &executable_slot_name {
                        if existing != slot {
                            errors.push(format!(
                                "canonical-exec: function '{}' contains unsupported {}: executable value slot '{}' conflicts with already-captured slot '{}' in the same function",
                                function.name,
                                invocation,
                                slot,
                                existing
                            ));
                            continue;
                        }
                    } else {
                        executable_slot_name = Some(slot.clone());
                    }
                    exec_ops.push(ExecutableOp::StackLoad { ty: *ty, slot_idx });
                    last_value = Some(ExecutableCapturedValue {
                        slot: slot.clone(),
                        ty: *ty,
                        source: ExecutableCapturedValueSource::SavedSlot,
                    });
                }
                KrirOp::CellArithImm {
                    ty,
                    cell,
                    arith_op,
                    imm,
                } => {
                    let invocation =
                        format!("cell_{}<{}>({})", arith_op.as_str(), ty.as_str(), cell);
                    let slot_idx =
                        match resolve_executable_stack_cell_slot(*ty, cell, &cell_slot_map) {
                            Ok(idx) => idx,
                            Err(reason) => {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported {}: {}",
                                    function.name, invocation, reason
                                ));
                                continue;
                            }
                        };
                    let valid = match arith_op {
                        ArithOp::Shl | ArithOp::Shr => {
                            if *imm > 63 {
                                Err(format!("shift count {} is out of range 0-63", imm))
                            } else {
                                Ok(())
                            }
                        }
                        _ => {
                            if *imm > u32::MAX as u64 {
                                Err(format!(
                                    "immediate {} does not fit in u32 for {} operation",
                                    imm,
                                    arith_op.as_str()
                                ))
                            } else {
                                Ok(())
                            }
                        }
                    };
                    if let Err(reason) = valid {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported {}: {}",
                            function.name, invocation, reason
                        ));
                        continue;
                    }
                    exec_ops.push(ExecutableOp::SlotArithImm {
                        ty: *ty,
                        slot_idx,
                        arith_op: *arith_op,
                        imm: *imm,
                    });
                }
                KrirOp::SlotArith {
                    ty,
                    dst,
                    src,
                    arith_op,
                } => {
                    let invocation =
                        format!("slot_{}<{}>({})", arith_op.as_str(), ty.as_str(), dst);
                    let dst_slot_idx =
                        match resolve_executable_stack_cell_slot(*ty, dst, &cell_slot_map) {
                            Ok(idx) => idx,
                            Err(reason) => {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported {}: {}",
                                    function.name, invocation, reason
                                ));
                                continue;
                            }
                        };
                    let src_slot_idx =
                        match resolve_executable_stack_cell_slot(*ty, src, &cell_slot_map) {
                            Ok(idx) => idx,
                            Err(reason) => {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported {}: {}",
                                    function.name, invocation, reason
                                ));
                                continue;
                            }
                        };
                    exec_ops.push(ExecutableOp::SlotArithSlot {
                        ty: *ty,
                        dst_slot_idx,
                        src_slot_idx,
                        arith_op: *arith_op,
                    });
                }
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
                    // If the address is a u64 parameter, emit a param-addr read op directly.
                    let param_addr = if let MmioAddrExpr::Ident { name } = addr {
                        if let Some(&(param_idx, param_ty)) = param_map.get(name.as_str()) {
                            if param_ty != MmioScalarType::U64 {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported {}: parameter '{}' must be u64 to use as MMIO address",
                                    function.name,
                                    format_mmio_read_invocation(*ty, addr, capture_slot.as_deref()),
                                    name
                                ));
                                continue;
                            }
                            Some(param_idx)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(param_idx) = param_addr {
                        exec_ops.push(ExecutableOp::MmioReadParamAddr {
                            param_idx,
                            ty: *ty,
                            capture_value: false,
                        });
                        last_value = Some(ExecutableCapturedValue {
                            slot: slot_name,
                            ty: *ty,
                            source: ExecutableCapturedValueSource::DeferredRead {
                                op_index: exec_ops.len() - 1,
                            },
                        });
                    } else {
                        match resolve_executable_mmio_addr(addr, &mmio_bases) {
                            Ok(resolved) => {
                                exec_ops.push(ExecutableOp::MmioRead {
                                    ty: *ty,
                                    addr: resolved,
                                    capture_value: false,
                                });
                                last_value = Some(ExecutableCapturedValue {
                                    slot: slot_name,
                                    ty: *ty,
                                    source: ExecutableCapturedValueSource::DeferredRead {
                                        op_index: exec_ops.len() - 1,
                                    },
                                });
                            }
                            Err(reason) => errors.push(format!(
                                "canonical-exec: function '{}' contains unsupported {}: {}",
                                function.name,
                                format_mmio_read_invocation(*ty, addr, capture_slot.as_deref()),
                                reason
                            )),
                        }
                    }
                }
                KrirOp::MmioWrite { ty, addr, value }
                | KrirOp::RawMmioWrite { ty, addr, value } => {
                    // Determine whether the address is a u64 parameter or a constant.
                    enum MmioWriteAddrSource {
                        Constant(u64),
                        Param(u8),
                    }
                    let addr_source = if let MmioAddrExpr::Ident { name } = addr {
                        if let Some(&(param_idx, param_ty)) = param_map.get(name.as_str()) {
                            if param_ty != MmioScalarType::U64 {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported {}: parameter '{}' must be u64 to use as MMIO address",
                                    function.name,
                                    format_mmio_write_invocation(*ty, addr, value),
                                    name
                                ));
                                continue;
                            }
                            Ok(MmioWriteAddrSource::Param(param_idx))
                        } else {
                            resolve_executable_mmio_addr(addr, &mmio_bases)
                                .map(MmioWriteAddrSource::Constant)
                        }
                    } else {
                        resolve_executable_mmio_addr(addr, &mmio_bases)
                            .map(MmioWriteAddrSource::Constant)
                    };
                    let addr_source = match addr_source {
                        Ok(src) => src,
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
                    let resolved_value = match value {
                        MmioValueExpr::FloatLiteral { value: fv } => Err(format!(
                            "float literal '{}' not supported in MMIO write",
                            fv
                        )),
                        MmioValueExpr::IntLiteral { .. } => resolve_executable_mmio_write_value(
                            *ty,
                            value,
                            executable_slot_name.as_deref(),
                            None,
                        ),
                        MmioValueExpr::Ident { name } => {
                            // Check if the ident refers to a function parameter
                            if let Some(&(param_idx, param_ty)) = param_map.get(name.as_str()) {
                                exec_ops.push(ExecutableOp::ParamLoad {
                                    param_idx,
                                    ty: param_ty,
                                });
                                Ok(ExecutableMmioWriteValue::SavedValue)
                            } else if matches!(
                                last_value.as_ref().map(|value| value.source),
                                Some(ExecutableCapturedValueSource::SavedSlot)
                            ) {
                                resolve_executable_saved_slot_write_value(
                                    *ty,
                                    name,
                                    executable_slot_name.as_deref(),
                                    last_value
                                        .as_ref()
                                        .map(|value| (value.slot.as_str(), value.ty)),
                                )
                            } else {
                                resolve_executable_mmio_write_value(
                                    *ty,
                                    value,
                                    executable_slot_name.as_deref(),
                                    last_value.as_ref().and_then(|value| match value.source {
                                        ExecutableCapturedValueSource::DeferredRead { .. } => {
                                            Some((value.slot.as_str(), value.ty))
                                        }
                                        ExecutableCapturedValueSource::SavedSlot => None,
                                    }),
                                )
                            }
                        }
                    };
                    let resolved_value = match resolved_value {
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
                        ExecutableMmioWriteValue::Immediate(immediate) => match addr_source {
                            MmioWriteAddrSource::Constant(addr) => {
                                exec_ops.push(ExecutableOp::MmioWriteImm {
                                    ty: *ty,
                                    addr,
                                    value: immediate,
                                });
                            }
                            MmioWriteAddrSource::Param(param_idx) => {
                                exec_ops.push(ExecutableOp::MmioWriteImmParamAddr {
                                    param_idx,
                                    ty: *ty,
                                    value: immediate,
                                });
                            }
                        },
                        ExecutableMmioWriteValue::SavedValue => {
                            if let Some(current_value) = last_value.as_ref()
                                && let ExecutableCapturedValueSource::DeferredRead { op_index } =
                                    current_value.source
                            {
                                match exec_ops.get_mut(op_index) {
                                    Some(ExecutableOp::MmioRead { capture_value, .. })
                                    | Some(ExecutableOp::MmioReadParamAddr {
                                        capture_value, ..
                                    }) => {
                                        *capture_value = true;
                                    }
                                    _ => unreachable!(
                                        "saved executable write must point at a prior mmio read op"
                                    ),
                                }
                            }
                            match addr_source {
                                MmioWriteAddrSource::Constant(addr) => {
                                    exec_ops.push(ExecutableOp::MmioWriteValue { ty: *ty, addr });
                                }
                                MmioWriteAddrSource::Param(param_idx) => {
                                    exec_ops.push(ExecutableOp::MmioWriteValueParamAddr {
                                        param_idx,
                                        ty: *ty,
                                    });
                                }
                            }
                        }
                    }
                }
                KrirOp::SliceLen { slice, slot } => {
                    if let Some(&(_, len_abi_idx, _)) = slice_param_map.get(slice.as_str()) {
                        if let Some(existing) = &executable_slot_name {
                            if existing != slot {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported slice_len({}, {}): executable value slot '{}' conflicts with already-captured slot '{}' in the same function",
                                    function.name, slice, slot, slot, existing
                                ));
                                continue;
                            }
                        } else {
                            executable_slot_name = Some(slot.clone());
                        }
                        exec_ops.push(ExecutableOp::ParamLoad {
                            param_idx: len_abi_idx,
                            ty: MmioScalarType::U64,
                        });
                        last_value = Some(ExecutableCapturedValue {
                            slot: slot.clone(),
                            ty: MmioScalarType::U64,
                            source: ExecutableCapturedValueSource::SavedSlot,
                        });
                    } else {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported slice_len({}, {}): '{}' is not a slice parameter",
                            function.name, slice, slot, slice
                        ));
                    }
                }
                KrirOp::SlicePtr { slice, slot } => {
                    if let Some(&(ptr_abi_idx, _, _)) = slice_param_map.get(slice.as_str()) {
                        if let Some(existing) = &executable_slot_name {
                            if existing != slot {
                                errors.push(format!(
                                    "canonical-exec: function '{}' contains unsupported slice_ptr({}, {}): executable value slot '{}' conflicts with already-captured slot '{}' in the same function",
                                    function.name, slice, slot, slot, existing
                                ));
                                continue;
                            }
                        } else {
                            executable_slot_name = Some(slot.clone());
                        }
                        exec_ops.push(ExecutableOp::ParamLoad {
                            param_idx: ptr_abi_idx,
                            ty: MmioScalarType::U64,
                        });
                        last_value = Some(ExecutableCapturedValue {
                            slot: slot.clone(),
                            ty: MmioScalarType::U64,
                            source: ExecutableCapturedValueSource::SavedSlot,
                        });
                    } else {
                        errors.push(format!(
                            "canonical-exec: function '{}' contains unsupported slice_ptr({}, {}): '{}' is not a slice parameter",
                            function.name, slice, slot, slice
                        ));
                    }
                }
                KrirOp::PercpuRead { ty, name, slot } => errors.push(format!(
                    "canonical-exec: function '{}' contains unsupported percpu_read<{}>({}, {})",
                    function.name,
                    ty.as_str(),
                    name,
                    slot
                )),
                KrirOp::PercpuWrite { ty, name, value } => errors.push(format!(
                    "canonical-exec: function '{}' contains unsupported percpu_write<{}>({}, {})",
                    function.name,
                    ty.as_str(),
                    name,
                    value.as_source()
                )),
                KrirOp::CompareIntoSlot {
                    cmp_op,
                    lhs,
                    rhs,
                    out,
                } => {
                    let lhs_idx = if let Some(&(idx, _)) = cell_slot_map.get(lhs.as_str()) {
                        Some(idx)
                    } else if let Some(&(pidx, _)) = param_map.get(lhs.as_str()) {
                        Some(next_slot_idx + pidx)
                    } else {
                        errors.push(format!(
                            "canonical-exec: function '{}' compare_{}: lhs '{}' not a declared stack cell or param",
                            function.name, cmp_op.as_str(), lhs
                        ));
                        None
                    };
                    let rhs_idx = if let Some(&(idx, _)) = cell_slot_map.get(rhs.as_str()) {
                        Some(idx)
                    } else if let Some(&(pidx, _)) = param_map.get(rhs.as_str()) {
                        Some(next_slot_idx + pidx)
                    } else {
                        errors.push(format!(
                            "canonical-exec: function '{}' compare_{}: rhs '{}' not a declared stack cell or param",
                            function.name, cmp_op.as_str(), rhs
                        ));
                        None
                    };
                    let out_idx = if let Some(&(idx, _)) = cell_slot_map.get(out.as_str()) {
                        Some(idx)
                    } else if let Some(&(pidx, _)) = param_map.get(out.as_str()) {
                        Some(next_slot_idx + pidx)
                    } else {
                        errors.push(format!(
                            "canonical-exec: function '{}' compare_{}: out '{}' not a declared stack cell or param",
                            function.name, cmp_op.as_str(), out
                        ));
                        None
                    };
                    if let (Some(li), Some(ri), Some(oi)) = (lhs_idx, rhs_idx, out_idx) {
                        exec_ops.push(ExecutableOp::CompareIntoSlot {
                            cmp_op: *cmp_op,
                            lhs_idx: li,
                            rhs_idx: ri,
                            out_idx: oi,
                        });
                    }
                }
                KrirOp::LoopBegin => exec_ops.push(ExecutableOp::LoopBegin),
                KrirOp::LoopEnd => exec_ops.push(ExecutableOp::LoopEnd),
                KrirOp::LoopBreak => exec_ops.push(ExecutableOp::LoopBreak),
                KrirOp::LoopContinue => exec_ops.push(ExecutableOp::LoopContinue),
                KrirOp::BranchIfZeroLoopBreak { slot } => {
                    let slot_idx = if let Some(&(idx, _)) = cell_slot_map.get(slot.as_str()) {
                        Some(idx)
                    } else if let Some(&(pidx, _)) = param_map.get(slot.as_str()) {
                        Some(next_slot_idx + pidx)
                    } else {
                        errors.push(format!(
                            "canonical-exec: function '{}' branch_if_zero_loop_break: '{}' not a declared stack cell or param",
                            function.name, slot
                        ));
                        None
                    };
                    if let Some(idx) = slot_idx {
                        exec_ops.push(ExecutableOp::BranchIfZeroLoopBreak { slot_idx: idx });
                    }
                }
                KrirOp::BranchIfNonZeroLoopBreak { slot } => {
                    let slot_idx = if let Some(&(idx, _)) = cell_slot_map.get(slot.as_str()) {
                        Some(idx)
                    } else if let Some(&(pidx, _)) = param_map.get(slot.as_str()) {
                        Some(next_slot_idx + pidx)
                    } else {
                        errors.push(format!(
                            "canonical-exec: function '{}' branch_if_nonzero_loop_break: '{}' not a declared stack cell or param",
                            function.name, slot
                        ));
                        None
                    };
                    if let Some(idx) = slot_idx {
                        exec_ops.push(ExecutableOp::BranchIfNonZeroLoopBreak { slot_idx: idx });
                    }
                }
                KrirOp::RawPtrLoad {
                    ty,
                    addr_slot,
                    out_slot,
                } => {
                    let addr_idx = match cell_slot_map.get(addr_slot.as_str()) {
                        Some(&(idx, _)) => idx,
                        None => {
                            errors.push(format!(
                                "canonical-exec: function '{}' raw_ptr_load: addr_slot '{}' not a declared stack cell",
                                function.name, addr_slot
                            ));
                            continue;
                        }
                    };
                    let out_idx = match cell_slot_map.get(out_slot.as_str()) {
                        Some(&(idx, _)) => idx,
                        None => {
                            errors.push(format!(
                                "canonical-exec: function '{}' raw_ptr_load: out_slot '{}' not a declared stack cell",
                                function.name, out_slot
                            ));
                            continue;
                        }
                    };
                    exec_ops.push(ExecutableOp::RawPtrLoad {
                        ty: *ty,
                        addr_slot_idx: addr_idx,
                        out_slot_idx: out_idx,
                    });
                }
                KrirOp::RawPtrStore {
                    ty,
                    addr_slot,
                    value,
                } => {
                    let addr_idx = match cell_slot_map.get(addr_slot.as_str()) {
                        Some(&(idx, _)) => idx,
                        None => {
                            errors.push(format!(
                                "canonical-exec: function '{}' raw_ptr_store: addr_slot '{}' not a declared stack cell",
                                function.name, addr_slot
                            ));
                            continue;
                        }
                    };
                    let resolved_value = match value {
                        MmioValueExpr::FloatLiteral { value: fv } => Err(format!(
                            "float literal '{}' not supported in raw_ptr_store",
                            fv
                        )),
                        MmioValueExpr::IntLiteral { .. } => resolve_executable_mmio_write_value(
                            *ty,
                            value,
                            executable_slot_name.as_deref(),
                            None,
                        ),
                        MmioValueExpr::Ident { name } => {
                            if let Some(&(param_idx, param_ty)) = param_map.get(name.as_str()) {
                                exec_ops.push(ExecutableOp::ParamLoad {
                                    param_idx,
                                    ty: param_ty,
                                });
                                Ok(ExecutableMmioWriteValue::SavedValue)
                            } else if matches!(
                                last_value.as_ref().map(|value| value.source),
                                Some(ExecutableCapturedValueSource::SavedSlot)
                            ) {
                                resolve_executable_saved_slot_write_value(
                                    *ty,
                                    name,
                                    executable_slot_name.as_deref(),
                                    last_value
                                        .as_ref()
                                        .map(|value| (value.slot.as_str(), value.ty)),
                                )
                            } else {
                                resolve_executable_mmio_write_value(
                                    *ty,
                                    value,
                                    executable_slot_name.as_deref(),
                                    last_value.as_ref().and_then(|value| match value.source {
                                        ExecutableCapturedValueSource::DeferredRead { .. } => {
                                            Some((value.slot.as_str(), value.ty))
                                        }
                                        ExecutableCapturedValueSource::SavedSlot => None,
                                    }),
                                )
                            }
                        }
                    };
                    match resolved_value {
                        Ok(_) => {
                            exec_ops.push(ExecutableOp::RawPtrStore {
                                ty: *ty,
                                addr_slot_idx: addr_idx,
                                value: value.clone(),
                            });
                        }
                        Err(reason) => {
                            errors.push(format!(
                                "canonical-exec: function '{}' raw_ptr_store: {}",
                                function.name, reason
                            ));
                            continue;
                        }
                    }
                }
                KrirOp::FloatArith { .. } => {
                    errors.push(format!(
                        "float arithmetic requires SSE2 backend (Task 14): function '{}'",
                        function.name
                    ));
                }
                KrirOp::InlineAsm(intr) => {
                    exec_ops.push(ExecutableOp::InlineAsm(intr.clone()));
                }
                KrirOp::LoadStaticCstrAddr { value, cell } => {
                    let str_idx = match string_indices.get(value.as_str()) {
                        Some(&idx) => idx,
                        None => {
                            errors.push(format!(
                                "canonical-exec: function '{}' LoadStaticCstrAddr: string '{}' not in index (internal error)",
                                function.name, value
                            ));
                            continue;
                        }
                    };
                    let slot_idx = match cell_slot_map.get(cell.as_str()) {
                        Some(&(idx, _)) => idx,
                        None => {
                            errors.push(format!(
                                "canonical-exec: function '{}' LoadStaticCstrAddr: cell '{}' not declared",
                                function.name, cell
                            ));
                            continue;
                        }
                    };
                    exec_ops.push(ExecutableOp::LoadStaticCstrAddr { str_idx, slot_idx });
                }
            }
        }

        let result_ty = function_result_types.get(&function.name).copied().flatten();

        // Expand slice params to their ABI slots in the ExecutableSignature.
        // A Scalar param occupies 1 slot; a Slice param occupies 2 (ptr: u64, len: u64).
        let abi_params: Vec<ExecutableValueType> = function
            .params
            .iter()
            .flat_map(|(_, ty)| match ty {
                KrirParamTy::Scalar { ty } => {
                    vec![executable_value_type_from_mmio_scalar(*ty)]
                }
                KrirParamTy::Slice { .. } => {
                    vec![ExecutableValueType::U64, ExecutableValueType::U64]
                }
            })
            .collect();

        lowered.functions.push(ExecutableFunction {
            name: function.name.clone(),
            is_extern: false,
            signature: ExecutableSignature {
                params: abi_params,
                result: result_ty
                    .map(executable_value_type_from_mmio_scalar)
                    .unwrap_or(ExecutableValueType::Unit),
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
                terminator: if let Some((tc_callee, tc_args)) = tail_call_terminator {
                    ExecutableTerminator::TailCall {
                        callee: tc_callee,
                        args: tc_args,
                    }
                } else {
                    ExecutableTerminator::Return {
                        value: result_ty
                            .map(|ty| ExecutableValue::SavedValue { ty })
                            .unwrap_or(ExecutableValue::Unit),
                    }
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

fn resolve_executable_stack_cell_slot(
    ty: MmioScalarType,
    cell: &str,
    cell_slot_map: &BTreeMap<String, (u8, MmioScalarType)>,
) -> Result<u8, String> {
    let Some(&(slot_idx, cell_ty)) = cell_slot_map.get(cell) else {
        return Err(if cell == "let" {
            "KernRift has no `let` keyword; declare variables with their type, e.g. `u64 x = ...`"
                .to_string()
        } else {
            format!(
                "assignment to undeclared variable '{}'; declare it first with `{} {} = ...`",
                cell,
                ty.as_str(),
                cell
            )
        });
    };
    if cell_ty != ty {
        return Err(format!(
            "stack cell '{}' has type {} from its declaration and does not match access type {}",
            cell,
            cell_ty.as_str(),
            ty.as_str()
        ));
    }
    Ok(slot_idx)
}

fn resolve_executable_mmio_write_value(
    ty: MmioScalarType,
    value: &MmioValueExpr,
    executable_slot_name: Option<&str>,
    available_read_value: Option<(&str, MmioScalarType)>,
) -> Result<ExecutableMmioWriteValue, String> {
    match value {
        MmioValueExpr::FloatLiteral { value } => Err(format!(
            "float literal '{}' not supported in MMIO write",
            value
        )),
        MmioValueExpr::IntLiteral { value } => {
            let parsed = parse_integer_literal_u64(value)?;
            let max_value = match ty {
                MmioScalarType::U8 => u8::MAX as u64,
                MmioScalarType::U16 => u16::MAX as u64,
                MmioScalarType::U32 => u32::MAX as u64,
                MmioScalarType::U64 => u64::MAX,
                MmioScalarType::F32 => u32::MAX as u64,
                MmioScalarType::F64 => u64::MAX,
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

fn resolve_executable_saved_slot_write_value(
    ty: MmioScalarType,
    name: &str,
    executable_slot_name: Option<&str>,
    available_saved_value: Option<(&str, MmioScalarType)>,
) -> Result<ExecutableMmioWriteValue, String> {
    let is_implicit_value = name == DEFAULT_EXECUTABLE_MMIO_SLOT;
    let Some(slot_name) = executable_slot_name else {
        return Err(if is_implicit_value {
            format!(
                "implicit write value '{}' requires a prior call_capture(...) in the same function",
                name
            )
        } else {
            format!(
                "named write value '{}' requires a prior call_capture(..., {}) in the same function",
                name, name
            )
        });
    };
    if name != slot_name {
        return Err(format!(
            "named write value '{}' does not match the captured executable slot '{}' in this function",
            name, slot_name
        ));
    }
    let Some((saved_slot, saved_ty)) = available_saved_value else {
        return Err(if is_implicit_value {
            format!(
                "implicit write value '{}' requires a prior call_capture(...) in the same function",
                name
            )
        } else {
            format!(
                "named write value '{}' requires a prior call_capture(..., {}) in the same function",
                name, name
            )
        });
    };
    debug_assert_eq!(saved_slot, slot_name);
    if saved_ty != ty {
        return Err(if is_implicit_value {
            format!(
                "implicit write value '{}' has type {} from the prior captured call result and does not match write type {}",
                name,
                saved_ty.as_str(),
                ty.as_str()
            )
        } else {
            format!(
                "named write value '{}' has type {} from the prior captured call result and does not match write type {}",
                name,
                saved_ty.as_str(),
                ty.as_str()
            )
        });
    }
    Ok(ExecutableMmioWriteValue::SavedValue)
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
        MmioScalarType::F32 => u32::MAX as u64,
        MmioScalarType::F64 => u64::MAX,
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
        MmioScalarType::F32 => u32::MAX as u64,
        MmioScalarType::F64 => u64::MAX,
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

fn format_call_capture_invocation(callee: &str, slot: &str) -> String {
    format!("call_capture({}, {})", callee, slot)
}

fn format_return_slot_invocation(slot: &str) -> String {
    format!("return_slot({})", slot)
}

fn format_stack_cell_invocation(ty: MmioScalarType, cell: &str) -> String {
    format!("stack_cell<{}>({})", ty.as_str(), cell)
}

fn format_cell_store_invocation(ty: MmioScalarType, cell: &str, value: &MmioValueExpr) -> String {
    format!(
        "cell_store<{}>({}, {})",
        ty.as_str(),
        cell,
        value.as_source()
    )
}

fn format_cell_load_invocation(ty: MmioScalarType, cell: &str, slot: &str) -> String {
    format!("cell_load<{}>({}, {})", ty.as_str(), cell, slot)
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
    X86_64Win64,
    X86_64MachO,
    Aarch64Sysv,
    Aarch64MachO,
    Aarch64Win,
}

impl BackendTargetId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X86_64Sysv => "x86_64-sysv",
            Self::X86_64Win64 => "x86_64-win64",
            Self::X86_64MachO => "x86_64-macho",
            Self::Aarch64Sysv => "aarch64-sysv",
            Self::Aarch64MachO => "aarch64-macho",
            Self::Aarch64Win => "aarch64-win",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "x86_64-sysv" | "x86_64-linux" => Ok(Self::X86_64Sysv),
            "x86_64-win64" | "x86_64-windows" => Ok(Self::X86_64Win64),
            "x86_64-macho" | "x86_64-darwin" | "x86_64-macos" => Ok(Self::X86_64MachO),
            "aarch64-sysv" | "aarch64-linux" => Ok(Self::Aarch64Sysv),
            "aarch64-macho" | "aarch64-darwin" | "aarch64-macos" => Ok(Self::Aarch64MachO),
            "aarch64-win" | "aarch64-windows" => Ok(Self::Aarch64Win),
            _ => Err(format!(
                "unknown target '{}'; supported: x86_64-sysv, x86_64-win64, x86_64-macho, aarch64-sysv, aarch64-macho, aarch64-win",
                s
            )),
        }
    }

    /// Return the default `BackendTargetContract` for this target ID.
    pub fn default_contract(self) -> BackendTargetContract {
        match self {
            Self::X86_64Sysv => BackendTargetContract::x86_64_sysv(),
            Self::X86_64Win64 => BackendTargetContract::x86_64_win64(),
            Self::X86_64MachO => BackendTargetContract::x86_64_macho(),
            Self::Aarch64Sysv => BackendTargetContract::aarch64_sysv(),
            Self::Aarch64MachO => BackendTargetContract::aarch64_macho(),
            Self::Aarch64Win => BackendTargetContract::aarch64_win(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetArch {
    X86_64,
    AArch64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetAbi {
    Sysv,
    Win64,
    Aapcs64,
    Aapcs64Win,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AArch64IntegerRegister {
    X0,
    X1,
    X2,
    X3,
    X4,
    X5,
    X6,
    X7,
    X8,
    X9,
    X10,
    X11,
    X12,
    X13,
    X14,
    X15,
    // X16 (IP0) and X17 (IP1) omitted — linker scratch reserved
    // X18 omitted — platform-reserved on all three AArch64 targets
    X19,
    X20,
    X21,
    X22,
    X23,
    X24,
    X25,
    X26,
    X27,
    X28,
    X29, // frame pointer
    X30, // link register
    Sp,
    Xzr,
}

/// Unified register type spanning all supported architectures.
/// Derives `Ord` because `validate()` uses `BTreeSet<IntegerRegister>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(untagged)]
pub enum IntegerRegister {
    X86_64(X86_64IntegerRegister),
    AArch64(AArch64IntegerRegister),
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
    IntegerX0,
}

impl FutureScalarReturnConvention {
    pub fn registers(self) -> Vec<IntegerRegister> {
        match self {
            Self::IntegerRax => vec![IntegerRegister::X86_64(X86_64IntegerRegister::Rax)],
            Self::IntegerX0 => vec![IntegerRegister::AArch64(AArch64IntegerRegister::X0)],
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
    pub integer_registers: Vec<IntegerRegister>,
    pub stack_pointer: IntegerRegister,
    pub frame_pointer: IntegerRegister,
    pub instruction_pointer: &'static str,
    pub caller_saved: Vec<IntegerRegister>,
    pub callee_saved: Vec<IntegerRegister>,
    pub current_executable_return: CurrentExecutableReturnConvention,
    pub future_scalar_return: FutureScalarReturnConvention,
    pub future_argument_registers: Vec<IntegerRegister>,
    pub symbols: SymbolNamingConvention,
    pub sections: SectionNamingConvention,
    pub freestanding: FreestandingTargetAssumptions,
}

impl BackendTargetContract {
    pub fn x86_64_sysv() -> Self {
        use X86_64IntegerRegister::*;
        let x = |r| IntegerRegister::X86_64(r);
        Self {
            target_id: BackendTargetId::X86_64Sysv,
            arch: TargetArch::X86_64,
            abi: TargetAbi::Sysv,
            endian: TargetEndian::Little,
            pointer_bits: 64,
            stack_alignment_bytes: 16,
            integer_registers: vec![
                x(Rax),
                x(Rbx),
                x(Rcx),
                x(Rdx),
                x(Rsi),
                x(Rdi),
                x(Rbp),
                x(Rsp),
                x(R8),
                x(R9),
                x(R10),
                x(R11),
                x(R12),
                x(R13),
                x(R14),
                x(R15),
            ],
            stack_pointer: x(Rsp),
            frame_pointer: x(Rbp),
            instruction_pointer: "rip",
            caller_saved: vec![
                x(Rax),
                x(Rcx),
                x(Rdx),
                x(Rsi),
                x(Rdi),
                x(R8),
                x(R9),
                x(R10),
                x(R11),
            ],
            callee_saved: vec![x(Rbx), x(Rbp), x(R12), x(R13), x(R14), x(R15)],
            current_executable_return: CurrentExecutableReturnConvention::UnitNoRegister,
            future_scalar_return: FutureScalarReturnConvention::IntegerRax,
            future_argument_registers: vec![x(Rdi), x(Rsi), x(Rdx), x(Rcx), x(R8), x(R9)],
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

    /// Windows x86-64 ABI (Microsoft calling convention).
    ///
    /// Parameter registers: rcx, rdx, r8, r9 (first 4 integer args).
    /// Callee-saved: rbx, rbp, rdi, rsi, r12–r15.
    /// Shadow space: 32 bytes allocated by the caller (not yet emitted by codegen).
    pub fn x86_64_win64() -> Self {
        use X86_64IntegerRegister::*;
        let x = |r| IntegerRegister::X86_64(r);
        Self {
            target_id: BackendTargetId::X86_64Win64,
            arch: TargetArch::X86_64,
            abi: TargetAbi::Win64,
            endian: TargetEndian::Little,
            pointer_bits: 64,
            stack_alignment_bytes: 16,
            integer_registers: vec![
                x(Rax),
                x(Rbx),
                x(Rcx),
                x(Rdx),
                x(Rsi),
                x(Rdi),
                x(Rbp),
                x(Rsp),
                x(R8),
                x(R9),
                x(R10),
                x(R11),
                x(R12),
                x(R13),
                x(R14),
                x(R15),
            ],
            stack_pointer: x(Rsp),
            frame_pointer: x(Rbp),
            instruction_pointer: "rip",
            caller_saved: vec![x(Rax), x(Rcx), x(Rdx), x(R8), x(R9), x(R10), x(R11)],
            callee_saved: vec![
                x(Rbx),
                x(Rbp),
                x(Rdi),
                x(Rsi),
                x(R12),
                x(R13),
                x(R14),
                x(R15),
            ],
            current_executable_return: CurrentExecutableReturnConvention::UnitNoRegister,
            future_scalar_return: FutureScalarReturnConvention::IntegerRax,
            future_argument_registers: vec![x(Rcx), x(Rdx), x(R8), x(R9)],
            symbols: SymbolNamingConvention {
                function_prefix: "",
                preserve_source_names: true,
            },
            sections: SectionNamingConvention {
                text: ".text",
                rodata: ".rdata",
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

    /// macOS x86-64 (Mach-O, SysV ABI with underscore-prefixed symbols).
    ///
    /// Same calling convention as Linux SysV.  Differs in:
    ///   - Symbol prefix: `_` on all exported functions.
    ///   - Section names: `__TEXT,__text` etc.
    pub fn x86_64_macho() -> Self {
        use X86_64IntegerRegister::*;
        let x = |r| IntegerRegister::X86_64(r);
        Self {
            target_id: BackendTargetId::X86_64MachO,
            arch: TargetArch::X86_64,
            abi: TargetAbi::Sysv,
            endian: TargetEndian::Little,
            pointer_bits: 64,
            stack_alignment_bytes: 16,
            integer_registers: vec![
                x(Rax),
                x(Rbx),
                x(Rcx),
                x(Rdx),
                x(Rsi),
                x(Rdi),
                x(Rbp),
                x(Rsp),
                x(R8),
                x(R9),
                x(R10),
                x(R11),
                x(R12),
                x(R13),
                x(R14),
                x(R15),
            ],
            stack_pointer: x(Rsp),
            frame_pointer: x(Rbp),
            instruction_pointer: "rip",
            caller_saved: vec![
                x(Rax),
                x(Rcx),
                x(Rdx),
                x(Rsi),
                x(Rdi),
                x(R8),
                x(R9),
                x(R10),
                x(R11),
            ],
            callee_saved: vec![x(Rbx), x(Rbp), x(R12), x(R13), x(R14), x(R15)],
            current_executable_return: CurrentExecutableReturnConvention::UnitNoRegister,
            future_scalar_return: FutureScalarReturnConvention::IntegerRax,
            future_argument_registers: vec![x(Rdi), x(Rsi), x(Rdx), x(Rcx), x(R8), x(R9)],
            symbols: SymbolNamingConvention {
                function_prefix: "_",
                preserve_source_names: true,
            },
            sections: SectionNamingConvention {
                text: "__TEXT,__text",
                rodata: "__TEXT,__const",
                data: "__DATA,__data",
                bss: "__DATA,__bss",
            },
            freestanding: FreestandingTargetAssumptions {
                no_libc: true,
                no_host_runtime: true,
                toolchain_bridge_not_yet_exercised: true,
            },
        }
    }

    /// AArch64 Linux SysV ABI (AAPCS64).
    pub fn aarch64_sysv() -> Self {
        use AArch64IntegerRegister::*;
        let a = |r| IntegerRegister::AArch64(r);
        Self {
            target_id: BackendTargetId::Aarch64Sysv,
            arch: TargetArch::AArch64,
            abi: TargetAbi::Aapcs64,
            endian: TargetEndian::Little,
            pointer_bits: 64,
            stack_alignment_bytes: 16,
            integer_registers: vec![
                a(X0),
                a(X1),
                a(X2),
                a(X3),
                a(X4),
                a(X5),
                a(X6),
                a(X7),
                a(X8),
                a(X9),
                a(X10),
                a(X11),
                a(X12),
                a(X13),
                a(X14),
                a(X15),
                a(X19),
                a(X20),
                a(X21),
                a(X22),
                a(X23),
                a(X24),
                a(X25),
                a(X26),
                a(X27),
                a(X28),
                a(X29),
                a(X30),
                a(Sp),
            ],
            stack_pointer: a(Sp),
            frame_pointer: a(X29),
            instruction_pointer: "pc",
            caller_saved: vec![
                a(X0),
                a(X1),
                a(X2),
                a(X3),
                a(X4),
                a(X5),
                a(X6),
                a(X7),
                a(X8),
                a(X9),
                a(X10),
                a(X11),
                a(X12),
                a(X13),
                a(X14),
                a(X15),
            ],
            callee_saved: vec![
                a(X19),
                a(X20),
                a(X21),
                a(X22),
                a(X23),
                a(X24),
                a(X25),
                a(X26),
                a(X27),
                a(X28),
                a(X29),
                a(X30),
            ],
            current_executable_return: CurrentExecutableReturnConvention::UnitNoRegister,
            future_scalar_return: FutureScalarReturnConvention::IntegerX0,
            future_argument_registers: vec![a(X0), a(X1), a(X2), a(X3), a(X4), a(X5), a(X6), a(X7)],
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

    /// AArch64 macOS (Mach-O, AAPCS64 with underscore-prefixed symbols).
    pub fn aarch64_macho() -> Self {
        let mut c = Self::aarch64_sysv();
        c.target_id = BackendTargetId::Aarch64MachO;
        c.sections = SectionNamingConvention {
            text: "__TEXT,__text",
            rodata: "__TEXT,__const",
            data: "__DATA,__data",
            bss: "__DATA,__bss",
        };
        c.symbols = SymbolNamingConvention {
            function_prefix: "_",
            preserve_source_names: true,
        };
        c
    }

    /// AArch64 Windows (AAPCS64 Windows variant).
    pub fn aarch64_win() -> Self {
        use AArch64IntegerRegister::*;
        let a = |r| IntegerRegister::AArch64(r);
        let mut c = Self::aarch64_sysv();
        c.target_id = BackendTargetId::Aarch64Win;
        c.abi = TargetAbi::Aapcs64Win;
        c.sections = SectionNamingConvention {
            text: ".text",
            rodata: ".rdata",
            data: ".data",
            bss: ".bss",
        };
        c.caller_saved.retain(|r| *r != a(X15));
        c.callee_saved.push(a(X15));
        c
    }

    pub fn validate(&self) -> Result<(), String> {
        let known_combo = matches!(
            (self.target_id, self.arch, self.abi),
            (
                BackendTargetId::X86_64Sysv,
                TargetArch::X86_64,
                TargetAbi::Sysv
            ) | (
                BackendTargetId::X86_64Win64,
                TargetArch::X86_64,
                TargetAbi::Win64
            ) | (
                BackendTargetId::X86_64MachO,
                TargetArch::X86_64,
                TargetAbi::Sysv
            ) | (
                BackendTargetId::Aarch64Sysv,
                TargetArch::AArch64,
                TargetAbi::Aapcs64
            ) | (
                BackendTargetId::Aarch64MachO,
                TargetArch::AArch64,
                TargetAbi::Aapcs64
            ) | (
                BackendTargetId::Aarch64Win,
                TargetArch::AArch64,
                TargetAbi::Aapcs64Win
            )
        );
        if !known_combo {
            return Err(format!(
                "unrecognized backend target combination: {}",
                self.target_id.as_str()
            ));
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
            if !integer_set.contains(&reg) {
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
    /// C-string literals referenced by `LoadStaticCstrAddr` instructions.
    /// Each entry is `(label, value)` where value has no NUL terminator (`.asciz` adds it).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub string_data: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64AsmFunction {
    pub symbol: String,
    pub uses_saved_value_slot: bool,
    pub n_stack_cells: u8,
    pub n_params: usize,
    pub instructions: Vec<X86_64AsmInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum X86_64AsmInstruction {
    Call {
        symbol: String,
    },
    CallWithArgs {
        symbol: String,
        args: Vec<ExecutableCallArg>,
    },
    TailCall {
        symbol: String,
        args: Vec<ExecutableCallArg>,
    },
    CallCapture {
        ty: MmioScalarType,
        symbol: String,
    },
    /// Call extern with args and store the return value to a stack slot.
    CallCaptureWithArgs {
        symbol: String,
        args: Vec<ExecutableCallArg>,
        ty: MmioScalarType,
        slot_idx: u8,
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
    StackStoreImm {
        ty: MmioScalarType,
        value: u64,
        slot_idx: u8,
    },
    StackStoreValue {
        ty: MmioScalarType,
        slot_idx: u8,
    },
    StackLoad {
        ty: MmioScalarType,
        slot_idx: u8,
    },
    SlotArithImm {
        ty: MmioScalarType,
        slot_idx: u8,
        arith_op: ArithOp,
        imm: u64,
    },
    SlotArithSlot {
        ty: MmioScalarType,
        dst_slot_idx: u8,
        src_slot_idx: u8,
        arith_op: ArithOp,
    },
    ParamLoad {
        param_idx: u8,
        ty: MmioScalarType,
    },
    MmioReadParamAddr {
        param_idx: u8,
        ty: MmioScalarType,
        capture_value: bool,
    },
    MmioWriteImmParamAddr {
        param_idx: u8,
        ty: MmioScalarType,
        value: u64,
    },
    MmioWriteValueParamAddr {
        param_idx: u8,
        ty: MmioScalarType,
    },
    ReturnSavedValue {
        ty: MmioScalarType,
    },
    Ret,
    /// Emits `name:` in ASM text; zero bytes in object.
    Label(String),
    /// Emits `jmp name` — REL32 relative jump.
    JmpLabel(String),
    /// Emits `jz name` — jump if zero flag set.
    JmpIfZeroLabel(String),
    /// Emits `jnz name` — jump if zero flag not set.
    JmpIfNonZeroLabel(String),
    /// `movzx eax, byte [rsp + 8*slot_idx]` — load U8 bool slot into eax.
    LoadSlotU8ToEax {
        slot_idx: u8,
    },
    /// `test eax, eax` — sets ZF if eax == 0.
    TestEaxEax,
    /// Compare lhs_idx op rhs_idx and store 0 or 1 in out_idx.
    CompareSlots {
        cmp_op: CmpOp,
        lhs_idx: u8,
        rhs_idx: u8,
        out_idx: u8,
    },
    /// Load scalar from address held in addr_slot_idx into out_slot_idx.
    RawPtrLoad {
        ty: MmioScalarType,
        addr_slot_idx: u8,
        out_slot_idx: u8,
    },
    /// Store scalar value to address held in addr_slot_idx.
    RawPtrStoreImm {
        ty: MmioScalarType,
        addr_slot_idx: u8,
        value: u64,
    },
    /// Store saved value register to address held in addr_slot_idx.
    RawPtrStoreSavedValue {
        ty: MmioScalarType,
        addr_slot_idx: u8,
    },
    /// Emit a named kernel intrinsic instruction bytes directly.
    InlineAsm(KernelIntrinsic),
    /// `lea __str_N(%rip), %rbx` + store to slot.
    /// The string label is emitted in `.section .rodata` after the `.text` section.
    LoadStaticCstrAddr {
        str_label: String,
        slot_idx: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AArch64AsmModule {
    pub section: &'static str,
    pub functions: Vec<AArch64AsmFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AArch64AsmFunction {
    pub symbol: String,
    pub uses_saved_value_slot: bool,
    pub n_stack_cells: u8,
    pub n_params: usize,
    pub instructions: Vec<AArch64AsmInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum AArch64AsmInstruction {
    Call {
        symbol: String,
    },
    CallWithArgs {
        symbol: String,
        args: Vec<ExecutableCallArg>,
    },
    TailCall {
        symbol: String,
        args: Vec<ExecutableCallArg>,
    },
    CallCapture {
        ty: MmioScalarType,
        symbol: String,
    },
    /// Call extern with args and store the return value to a stack slot.
    CallCaptureWithArgs {
        symbol: String,
        args: Vec<ExecutableCallArg>,
        ty: MmioScalarType,
        slot_idx: u8,
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
    StackStoreImm {
        ty: MmioScalarType,
        value: u64,
        slot_idx: u8,
    },
    StackStoreValue {
        ty: MmioScalarType,
        slot_idx: u8,
    },
    StackLoad {
        ty: MmioScalarType,
        slot_idx: u8,
    },
    SlotArithImm {
        ty: MmioScalarType,
        slot_idx: u8,
        arith_op: ArithOp,
        imm: u64,
    },
    SlotArithSlot {
        ty: MmioScalarType,
        dst_slot_idx: u8,
        src_slot_idx: u8,
        arith_op: ArithOp,
    },
    ParamLoad {
        param_idx: u8,
        ty: MmioScalarType,
    },
    MmioReadParamAddr {
        param_idx: u8,
        ty: MmioScalarType,
        capture_value: bool,
    },
    MmioWriteImmParamAddr {
        param_idx: u8,
        ty: MmioScalarType,
        value: u64,
    },
    MmioWriteValueParamAddr {
        param_idx: u8,
        ty: MmioScalarType,
    },
    ReturnSavedValue {
        ty: MmioScalarType,
    },
    Ret,
    /// Emits `name:` in ASM text; zero bytes in object.
    Label(String),
    /// Emits `b name` — unconditional branch.
    JmpLabel(String),
    /// Emits conditional branch if zero.
    JmpIfZeroLabel(String),
    /// Emits conditional branch if non-zero.
    JmpIfNonZeroLabel(String),
    /// Load U8 bool slot into a scratch register for comparison.
    LoadSlotU8ToEax {
        slot_idx: u8,
    },
    /// Test scratch register against itself — sets flags.
    TestEaxEax,
    /// Compare lhs_idx op rhs_idx and store 0 or 1 in out_idx.
    CompareSlots {
        cmp_op: CmpOp,
        lhs_idx: u8,
        rhs_idx: u8,
        out_idx: u8,
    },
    /// Load scalar from address held in addr_slot_idx into out_slot_idx.
    RawPtrLoad {
        ty: MmioScalarType,
        addr_slot_idx: u8,
        out_slot_idx: u8,
    },
    /// Store scalar value to address held in addr_slot_idx.
    RawPtrStoreImm {
        ty: MmioScalarType,
        addr_slot_idx: u8,
        value: u64,
    },
    /// Store saved value register to address held in addr_slot_idx.
    RawPtrStoreSavedValue {
        ty: MmioScalarType,
        addr_slot_idx: u8,
    },
    /// Emit a named kernel intrinsic instruction bytes directly.
    InlineAsm(KernelIntrinsic),
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
    match target.arch {
        TargetArch::X86_64 | TargetArch::AArch64 => {}
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
                symbol: format!("{}{}", target.symbols.function_prefix, symbol.name),
                uses_saved_value_slot,
                n_stack_cells: 0,
                n_params: 0,
                instructions,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(X86_64AsmModule {
        section: target.sections.text,
        functions,
        string_data: vec![],
    })
}

pub fn lower_executable_krir_to_x86_64_asm(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<X86_64AsmModule, String> {
    validate_compiler_owned_object_linear_subset(module, target)?;

    let mut canonical = module.clone();
    canonical.canonicalize();

    struct LoopFrame {
        head_label: String,
        end_label: String,
    }

    let functions = canonical
        .functions
        .iter()
        .map(|function| {
            let mut instrs: Vec<X86_64AsmInstruction> = Vec::new();
            let mut loop_stack: Vec<LoopFrame> = Vec::new();
            let mut loop_counter = 0usize;

            for op in &function.blocks[0].ops {
                match op {
                    ExecutableOp::Call { callee } => {
                        instrs.push(X86_64AsmInstruction::Call {
                            symbol: callee.clone(),
                        });
                    }
                    ExecutableOp::CallCapture { callee, ty } => {
                        instrs.push(X86_64AsmInstruction::CallCapture {
                            ty: *ty,
                            symbol: callee.clone(),
                        });
                    }
                    ExecutableOp::CallCaptureWithArgs {
                        callee,
                        args,
                        ty,
                        slot_idx,
                    } => {
                        instrs.push(X86_64AsmInstruction::CallCaptureWithArgs {
                            symbol: callee.clone(),
                            args: args.clone(),
                            ty: *ty,
                            slot_idx: *slot_idx,
                        });
                    }
                    ExecutableOp::BranchIfZero {
                        ty,
                        then_callee,
                        else_callee,
                    } => {
                        instrs.push(X86_64AsmInstruction::BranchIfZero {
                            ty: *ty,
                            then_symbol: then_callee.clone(),
                            else_symbol: else_callee.clone(),
                        });
                    }
                    ExecutableOp::BranchIfEqImm {
                        ty,
                        compare_value,
                        then_callee,
                        else_callee,
                    } => {
                        instrs.push(X86_64AsmInstruction::BranchIfEqImm {
                            ty: *ty,
                            compare_value: *compare_value,
                            then_symbol: then_callee.clone(),
                            else_symbol: else_callee.clone(),
                        });
                    }
                    ExecutableOp::BranchIfMaskNonZeroImm {
                        ty,
                        mask_value,
                        then_callee,
                        else_callee,
                    } => {
                        instrs.push(X86_64AsmInstruction::BranchIfMaskNonZeroImm {
                            ty: *ty,
                            mask_value: *mask_value,
                            then_symbol: then_callee.clone(),
                            else_symbol: else_callee.clone(),
                        });
                    }
                    ExecutableOp::MmioRead {
                        ty,
                        addr,
                        capture_value,
                    } => {
                        instrs.push(X86_64AsmInstruction::MmioRead {
                            ty: *ty,
                            addr: *addr,
                            capture_value: *capture_value,
                        });
                    }
                    ExecutableOp::MmioWriteImm { ty, addr, value } => {
                        instrs.push(X86_64AsmInstruction::MmioWriteImm {
                            ty: *ty,
                            addr: *addr,
                            value: *value,
                        });
                    }
                    ExecutableOp::MmioWriteValue { ty, addr } => {
                        instrs.push(X86_64AsmInstruction::MmioWriteValue {
                            ty: *ty,
                            addr: *addr,
                        });
                    }
                    ExecutableOp::StackStoreImm {
                        ty,
                        value,
                        slot_idx,
                    } => {
                        instrs.push(X86_64AsmInstruction::StackStoreImm {
                            ty: *ty,
                            value: *value,
                            slot_idx: *slot_idx,
                        });
                    }
                    ExecutableOp::StackStoreValue { ty, slot_idx } => {
                        instrs.push(X86_64AsmInstruction::StackStoreValue {
                            ty: *ty,
                            slot_idx: *slot_idx,
                        });
                    }
                    ExecutableOp::StackLoad { ty, slot_idx } => {
                        instrs.push(X86_64AsmInstruction::StackLoad {
                            ty: *ty,
                            slot_idx: *slot_idx,
                        });
                    }
                    ExecutableOp::SlotArithImm {
                        ty,
                        slot_idx,
                        arith_op,
                        imm,
                    } => {
                        instrs.push(X86_64AsmInstruction::SlotArithImm {
                            ty: *ty,
                            slot_idx: *slot_idx,
                            arith_op: *arith_op,
                            imm: *imm,
                        });
                    }
                    ExecutableOp::SlotArithSlot {
                        ty,
                        dst_slot_idx,
                        src_slot_idx,
                        arith_op,
                    } => {
                        instrs.push(X86_64AsmInstruction::SlotArithSlot {
                            ty: *ty,
                            dst_slot_idx: *dst_slot_idx,
                            src_slot_idx: *src_slot_idx,
                            arith_op: *arith_op,
                        });
                    }
                    ExecutableOp::ParamLoad { param_idx, ty } => {
                        instrs.push(X86_64AsmInstruction::ParamLoad {
                            param_idx: *param_idx,
                            ty: *ty,
                        });
                    }
                    ExecutableOp::MmioReadParamAddr {
                        param_idx,
                        ty,
                        capture_value,
                    } => {
                        instrs.push(X86_64AsmInstruction::MmioReadParamAddr {
                            param_idx: *param_idx,
                            ty: *ty,
                            capture_value: *capture_value,
                        });
                    }
                    ExecutableOp::MmioWriteImmParamAddr {
                        param_idx,
                        ty,
                        value,
                    } => {
                        instrs.push(X86_64AsmInstruction::MmioWriteImmParamAddr {
                            param_idx: *param_idx,
                            ty: *ty,
                            value: *value,
                        });
                    }
                    ExecutableOp::MmioWriteValueParamAddr { param_idx, ty } => {
                        instrs.push(X86_64AsmInstruction::MmioWriteValueParamAddr {
                            param_idx: *param_idx,
                            ty: *ty,
                        });
                    }
                    ExecutableOp::CallWithArgs { callee, args } => {
                        instrs.push(X86_64AsmInstruction::CallWithArgs {
                            symbol: callee.clone(),
                            args: args.clone(),
                        });
                    }
                    ExecutableOp::LoopBegin => {
                        let head = format!("{}__loop_{}_head", function.name, loop_counter);
                        let end = format!("{}__loop_{}_end", function.name, loop_counter);
                        loop_counter += 1;
                        instrs.push(X86_64AsmInstruction::Label(head.clone()));
                        loop_stack.push(LoopFrame {
                            head_label: head,
                            end_label: end,
                        });
                    }
                    ExecutableOp::LoopEnd => {
                        let frame = loop_stack.last().expect("LoopEnd without LoopBegin");
                        instrs.push(X86_64AsmInstruction::JmpLabel(frame.head_label.clone()));
                        let frame = loop_stack.pop().unwrap();
                        instrs.push(X86_64AsmInstruction::Label(frame.end_label));
                    }
                    ExecutableOp::LoopBreak => {
                        let end = loop_stack
                            .last()
                            .expect("LoopBreak outside loop")
                            .end_label
                            .clone();
                        instrs.push(X86_64AsmInstruction::JmpLabel(end));
                    }
                    ExecutableOp::LoopContinue => {
                        let head = loop_stack
                            .last()
                            .expect("LoopContinue outside loop")
                            .head_label
                            .clone();
                        instrs.push(X86_64AsmInstruction::JmpLabel(head));
                    }
                    ExecutableOp::BranchIfZeroLoopBreak { slot_idx } => {
                        let end = loop_stack
                            .last()
                            .expect("BranchIfZeroLoopBreak outside loop")
                            .end_label
                            .clone();
                        instrs.push(X86_64AsmInstruction::LoadSlotU8ToEax {
                            slot_idx: *slot_idx,
                        });
                        instrs.push(X86_64AsmInstruction::TestEaxEax);
                        instrs.push(X86_64AsmInstruction::JmpIfZeroLabel(end));
                    }
                    ExecutableOp::BranchIfNonZeroLoopBreak { slot_idx } => {
                        let end = loop_stack
                            .last()
                            .expect("BranchIfNonZeroLoopBreak outside loop")
                            .end_label
                            .clone();
                        instrs.push(X86_64AsmInstruction::LoadSlotU8ToEax {
                            slot_idx: *slot_idx,
                        });
                        instrs.push(X86_64AsmInstruction::TestEaxEax);
                        instrs.push(X86_64AsmInstruction::JmpIfNonZeroLabel(end));
                    }
                    ExecutableOp::CompareIntoSlot {
                        cmp_op,
                        lhs_idx,
                        rhs_idx,
                        out_idx,
                    } => {
                        instrs.push(X86_64AsmInstruction::CompareSlots {
                            cmp_op: *cmp_op,
                            lhs_idx: *lhs_idx,
                            rhs_idx: *rhs_idx,
                            out_idx: *out_idx,
                        });
                    }
                    ExecutableOp::RawPtrLoad {
                        ty,
                        addr_slot_idx,
                        out_slot_idx,
                    } => {
                        instrs.push(X86_64AsmInstruction::RawPtrLoad {
                            ty: *ty,
                            addr_slot_idx: *addr_slot_idx,
                            out_slot_idx: *out_slot_idx,
                        });
                    }
                    ExecutableOp::RawPtrStore {
                        ty,
                        addr_slot_idx,
                        value,
                    } => match value {
                        MmioValueExpr::IntLiteral { value: raw } => {
                            let v = parse_integer_literal_u64(raw).expect(
                                "raw_ptr_store: integer literal already validated during lowering",
                            );
                            instrs.push(X86_64AsmInstruction::RawPtrStoreImm {
                                ty: *ty,
                                addr_slot_idx: *addr_slot_idx,
                                value: v,
                            });
                        }
                        MmioValueExpr::Ident { .. } => {
                            instrs.push(X86_64AsmInstruction::RawPtrStoreSavedValue {
                                ty: *ty,
                                addr_slot_idx: *addr_slot_idx,
                            });
                        }
                        MmioValueExpr::FloatLiteral { .. } => {
                            unreachable!(
                                "float RawPtrStore not supported (Task 14): function '{}'",
                                function.name
                            );
                        }
                    },
                    ExecutableOp::InlineAsm(intr) => {
                        instrs.push(X86_64AsmInstruction::InlineAsm(intr.clone()));
                    }
                    ExecutableOp::LoadStaticCstrAddr { str_idx, slot_idx } => {
                        let label = format!("__str_{}", str_idx);
                        instrs.push(X86_64AsmInstruction::LoadStaticCstrAddr {
                            str_label: label,
                            slot_idx: *slot_idx,
                        });
                    }
                }
            }

            // Terminator
            match function.blocks[0].terminator.clone() {
                ExecutableTerminator::Return {
                    value: ExecutableValue::SavedValue { ty },
                } => {
                    instrs.push(X86_64AsmInstruction::ReturnSavedValue { ty });
                    instrs.push(X86_64AsmInstruction::Ret);
                }
                ExecutableTerminator::Return {
                    value: ExecutableValue::Unit,
                } => {
                    instrs.push(X86_64AsmInstruction::Ret);
                }
                ExecutableTerminator::TailCall { callee, args } => {
                    instrs.push(X86_64AsmInstruction::TailCall {
                        symbol: callee,
                        args,
                    });
                }
            }

            Ok(X86_64AsmFunction {
                symbol: format!("{}{}", target.symbols.function_prefix, function.name),
                uses_saved_value_slot: executable_function_uses_saved_value_slot(function),
                n_stack_cells: executable_function_n_stack_cells(function),
                n_params: function.signature.params.len(),
                instructions: instrs,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let string_data: Vec<(String, String)> = canonical
        .static_strings
        .iter()
        .enumerate()
        .map(|(i, s)| (format!("__str_{}", i), s.clone()))
        .collect();

    Ok(X86_64AsmModule {
        section: target.sections.text,
        functions,
        string_data,
    })
}

pub fn lower_executable_krir_to_aarch64_asm(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<AArch64AsmModule, String> {
    validate_executable_krir_linear_structure(
        module,
        target,
        "lower_executable_krir_to_aarch64_asm",
    )?;

    let mut canonical = module.clone();
    canonical.canonicalize();

    struct LoopFrame {
        head_label: String,
        end_label: String,
    }

    let functions = canonical
        .functions
        .iter()
        .map(|function| {
            let mut instrs: Vec<AArch64AsmInstruction> = Vec::new();
            let mut loop_stack: Vec<LoopFrame> = Vec::new();
            let mut loop_counter = 0usize;

            for op in &function.blocks[0].ops {
                match op {
                    ExecutableOp::Call { callee } => {
                        instrs.push(AArch64AsmInstruction::Call {
                            symbol: callee.clone(),
                        });
                    }
                    ExecutableOp::CallCapture { callee, ty } => {
                        instrs.push(AArch64AsmInstruction::CallCapture {
                            ty: *ty,
                            symbol: callee.clone(),
                        });
                    }
                    ExecutableOp::CallCaptureWithArgs {
                        callee,
                        args,
                        ty,
                        slot_idx,
                    } => {
                        instrs.push(AArch64AsmInstruction::CallCaptureWithArgs {
                            symbol: callee.clone(),
                            args: args.clone(),
                            ty: *ty,
                            slot_idx: *slot_idx,
                        });
                    }
                    ExecutableOp::BranchIfZero {
                        ty,
                        then_callee,
                        else_callee,
                    } => {
                        instrs.push(AArch64AsmInstruction::BranchIfZero {
                            ty: *ty,
                            then_symbol: then_callee.clone(),
                            else_symbol: else_callee.clone(),
                        });
                    }
                    ExecutableOp::BranchIfEqImm {
                        ty,
                        compare_value,
                        then_callee,
                        else_callee,
                    } => {
                        instrs.push(AArch64AsmInstruction::BranchIfEqImm {
                            ty: *ty,
                            compare_value: *compare_value,
                            then_symbol: then_callee.clone(),
                            else_symbol: else_callee.clone(),
                        });
                    }
                    ExecutableOp::BranchIfMaskNonZeroImm {
                        ty,
                        mask_value,
                        then_callee,
                        else_callee,
                    } => {
                        instrs.push(AArch64AsmInstruction::BranchIfMaskNonZeroImm {
                            ty: *ty,
                            mask_value: *mask_value,
                            then_symbol: then_callee.clone(),
                            else_symbol: else_callee.clone(),
                        });
                    }
                    ExecutableOp::MmioRead {
                        ty,
                        addr,
                        capture_value,
                    } => {
                        instrs.push(AArch64AsmInstruction::MmioRead {
                            ty: *ty,
                            addr: *addr,
                            capture_value: *capture_value,
                        });
                    }
                    ExecutableOp::MmioWriteImm { ty, addr, value } => {
                        instrs.push(AArch64AsmInstruction::MmioWriteImm {
                            ty: *ty,
                            addr: *addr,
                            value: *value,
                        });
                    }
                    ExecutableOp::MmioWriteValue { ty, addr } => {
                        instrs.push(AArch64AsmInstruction::MmioWriteValue {
                            ty: *ty,
                            addr: *addr,
                        });
                    }
                    ExecutableOp::StackStoreImm {
                        ty,
                        value,
                        slot_idx,
                    } => {
                        instrs.push(AArch64AsmInstruction::StackStoreImm {
                            ty: *ty,
                            value: *value,
                            slot_idx: *slot_idx,
                        });
                    }
                    ExecutableOp::StackStoreValue { ty, slot_idx } => {
                        instrs.push(AArch64AsmInstruction::StackStoreValue {
                            ty: *ty,
                            slot_idx: *slot_idx,
                        });
                    }
                    ExecutableOp::StackLoad { ty, slot_idx } => {
                        instrs.push(AArch64AsmInstruction::StackLoad {
                            ty: *ty,
                            slot_idx: *slot_idx,
                        });
                    }
                    ExecutableOp::SlotArithImm {
                        ty,
                        slot_idx,
                        arith_op,
                        imm,
                    } => {
                        instrs.push(AArch64AsmInstruction::SlotArithImm {
                            ty: *ty,
                            slot_idx: *slot_idx,
                            arith_op: *arith_op,
                            imm: *imm,
                        });
                    }
                    ExecutableOp::SlotArithSlot {
                        ty,
                        dst_slot_idx,
                        src_slot_idx,
                        arith_op,
                    } => {
                        instrs.push(AArch64AsmInstruction::SlotArithSlot {
                            ty: *ty,
                            dst_slot_idx: *dst_slot_idx,
                            src_slot_idx: *src_slot_idx,
                            arith_op: *arith_op,
                        });
                    }
                    ExecutableOp::ParamLoad { param_idx, ty } => {
                        instrs.push(AArch64AsmInstruction::ParamLoad {
                            param_idx: *param_idx,
                            ty: *ty,
                        });
                    }
                    ExecutableOp::MmioReadParamAddr {
                        param_idx,
                        ty,
                        capture_value,
                    } => {
                        instrs.push(AArch64AsmInstruction::MmioReadParamAddr {
                            param_idx: *param_idx,
                            ty: *ty,
                            capture_value: *capture_value,
                        });
                    }
                    ExecutableOp::MmioWriteImmParamAddr {
                        param_idx,
                        ty,
                        value,
                    } => {
                        instrs.push(AArch64AsmInstruction::MmioWriteImmParamAddr {
                            param_idx: *param_idx,
                            ty: *ty,
                            value: *value,
                        });
                    }
                    ExecutableOp::MmioWriteValueParamAddr { param_idx, ty } => {
                        instrs.push(AArch64AsmInstruction::MmioWriteValueParamAddr {
                            param_idx: *param_idx,
                            ty: *ty,
                        });
                    }
                    ExecutableOp::CallWithArgs { callee, args } => {
                        instrs.push(AArch64AsmInstruction::CallWithArgs {
                            symbol: callee.clone(),
                            args: args.clone(),
                        });
                    }
                    ExecutableOp::LoopBegin => {
                        let head = format!("{}__loop_{}_head", function.name, loop_counter);
                        let end = format!("{}__loop_{}_end", function.name, loop_counter);
                        loop_counter += 1;
                        instrs.push(AArch64AsmInstruction::Label(head.clone()));
                        loop_stack.push(LoopFrame {
                            head_label: head,
                            end_label: end,
                        });
                    }
                    ExecutableOp::LoopEnd => {
                        let frame = loop_stack.last().expect("LoopEnd without LoopBegin");
                        instrs.push(AArch64AsmInstruction::JmpLabel(frame.head_label.clone()));
                        let frame = loop_stack.pop().unwrap();
                        instrs.push(AArch64AsmInstruction::Label(frame.end_label));
                    }
                    ExecutableOp::LoopBreak => {
                        let end = loop_stack
                            .last()
                            .expect("LoopBreak outside loop")
                            .end_label
                            .clone();
                        instrs.push(AArch64AsmInstruction::JmpLabel(end));
                    }
                    ExecutableOp::LoopContinue => {
                        let head = loop_stack
                            .last()
                            .expect("LoopContinue outside loop")
                            .head_label
                            .clone();
                        instrs.push(AArch64AsmInstruction::JmpLabel(head));
                    }
                    ExecutableOp::BranchIfZeroLoopBreak { slot_idx } => {
                        let end = loop_stack
                            .last()
                            .expect("BranchIfZeroLoopBreak outside loop")
                            .end_label
                            .clone();
                        instrs.push(AArch64AsmInstruction::LoadSlotU8ToEax {
                            slot_idx: *slot_idx,
                        });
                        instrs.push(AArch64AsmInstruction::TestEaxEax);
                        instrs.push(AArch64AsmInstruction::JmpIfZeroLabel(end));
                    }
                    ExecutableOp::BranchIfNonZeroLoopBreak { slot_idx } => {
                        let end = loop_stack
                            .last()
                            .expect("BranchIfNonZeroLoopBreak outside loop")
                            .end_label
                            .clone();
                        instrs.push(AArch64AsmInstruction::LoadSlotU8ToEax {
                            slot_idx: *slot_idx,
                        });
                        instrs.push(AArch64AsmInstruction::TestEaxEax);
                        instrs.push(AArch64AsmInstruction::JmpIfNonZeroLabel(end));
                    }
                    ExecutableOp::CompareIntoSlot {
                        cmp_op,
                        lhs_idx,
                        rhs_idx,
                        out_idx,
                    } => {
                        instrs.push(AArch64AsmInstruction::CompareSlots {
                            cmp_op: *cmp_op,
                            lhs_idx: *lhs_idx,
                            rhs_idx: *rhs_idx,
                            out_idx: *out_idx,
                        });
                    }
                    ExecutableOp::RawPtrLoad {
                        ty,
                        addr_slot_idx,
                        out_slot_idx,
                    } => {
                        instrs.push(AArch64AsmInstruction::RawPtrLoad {
                            ty: *ty,
                            addr_slot_idx: *addr_slot_idx,
                            out_slot_idx: *out_slot_idx,
                        });
                    }
                    ExecutableOp::RawPtrStore {
                        ty,
                        addr_slot_idx,
                        value,
                    } => match value {
                        MmioValueExpr::IntLiteral { value: raw } => {
                            let v = parse_integer_literal_u64(raw).expect(
                                "raw_ptr_store: integer literal already validated during lowering",
                            );
                            instrs.push(AArch64AsmInstruction::RawPtrStoreImm {
                                ty: *ty,
                                addr_slot_idx: *addr_slot_idx,
                                value: v,
                            });
                        }
                        MmioValueExpr::Ident { .. } => {
                            instrs.push(AArch64AsmInstruction::RawPtrStoreSavedValue {
                                ty: *ty,
                                addr_slot_idx: *addr_slot_idx,
                            });
                        }
                        MmioValueExpr::FloatLiteral { .. } => {
                            unreachable!(
                                "float RawPtrStore not supported (Task 14): function '{}'",
                                function.name
                            );
                        }
                    },
                    ExecutableOp::InlineAsm(intr) => {
                        instrs.push(AArch64AsmInstruction::InlineAsm(intr.clone()));
                    }
                    ExecutableOp::LoadStaticCstrAddr { .. } => {
                        return Err(format!(
                            "aarch64 ASM emit: LoadStaticCstrAddr requires object emit, not ASM (function '{}')",
                            function.name
                        ));
                    }
                }
            }

            // Terminator
            match function.blocks[0].terminator.clone() {
                ExecutableTerminator::Return {
                    value: ExecutableValue::SavedValue { ty },
                } => {
                    instrs.push(AArch64AsmInstruction::ReturnSavedValue { ty });
                    instrs.push(AArch64AsmInstruction::Ret);
                }
                ExecutableTerminator::Return {
                    value: ExecutableValue::Unit,
                } => {
                    instrs.push(AArch64AsmInstruction::Ret);
                }
                ExecutableTerminator::TailCall { callee, args } => {
                    instrs.push(AArch64AsmInstruction::TailCall {
                        symbol: callee,
                        args,
                    });
                }
            }

            Ok(AArch64AsmFunction {
                symbol: format!("{}{}", target.symbols.function_prefix, function.name),
                uses_saved_value_slot: executable_function_uses_saved_value_slot(function),
                n_stack_cells: executable_function_n_stack_cells(function),
                n_params: function.signature.params.len(),
                instructions: instrs,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(AArch64AsmModule {
        section: target.sections.text,
        functions,
    })
}

fn executable_function_uses_saved_value_slot(function: &ExecutableFunction) -> bool {
    function.blocks.iter().any(|block| {
        block.ops.iter().any(|op| {
            if let ExecutableOp::RawPtrStore {
                value: MmioValueExpr::Ident { .. },
                ..
            } = op
            {
                return true;
            }
            matches!(
                op,
                ExecutableOp::CallCapture { .. }
                    | ExecutableOp::BranchIfZero { .. }
                    | ExecutableOp::BranchIfEqImm { .. }
                    | ExecutableOp::BranchIfMaskNonZeroImm { .. }
                    | ExecutableOp::MmioWriteValue { .. }
                    | ExecutableOp::StackStoreValue { .. }
                    | ExecutableOp::StackLoad { .. }
                    | ExecutableOp::SlotArithImm { .. }
                    | ExecutableOp::ParamLoad { .. }
                    | ExecutableOp::MmioWriteValueParamAddr { .. }
                    | ExecutableOp::LoadStaticCstrAddr { .. }
                    | ExecutableOp::MmioRead {
                        capture_value: true,
                        ..
                    }
                    | ExecutableOp::MmioReadParamAddr {
                        capture_value: true,
                        ..
                    }
            )
        })
    })
}

fn executable_function_n_stack_cells(function: &ExecutableFunction) -> u8 {
    let mut max_slot: Option<u8> = None;
    for block in &function.blocks {
        for op in &block.ops {
            let slot = match op {
                ExecutableOp::StackStoreImm { slot_idx, .. } => Some(*slot_idx),
                ExecutableOp::StackStoreValue { slot_idx, .. } => Some(*slot_idx),
                ExecutableOp::StackLoad { slot_idx, .. } => Some(*slot_idx),
                ExecutableOp::SlotArithImm { slot_idx, .. } => Some(*slot_idx),
                ExecutableOp::SlotArithSlot {
                    dst_slot_idx,
                    src_slot_idx,
                    ..
                } => Some((*dst_slot_idx).max(*src_slot_idx)),
                ExecutableOp::BranchIfZeroLoopBreak { slot_idx } => Some(*slot_idx),
                ExecutableOp::BranchIfNonZeroLoopBreak { slot_idx } => Some(*slot_idx),
                ExecutableOp::CompareIntoSlot {
                    lhs_idx,
                    rhs_idx,
                    out_idx,
                    ..
                } => Some((*lhs_idx).max(*rhs_idx).max(*out_idx)),
                ExecutableOp::RawPtrLoad {
                    addr_slot_idx,
                    out_slot_idx,
                    ..
                } => Some((*addr_slot_idx).max(*out_slot_idx)),
                ExecutableOp::RawPtrStore { addr_slot_idx, .. } => Some(*addr_slot_idx),
                ExecutableOp::CallCaptureWithArgs { slot_idx, .. } => Some(*slot_idx),
                ExecutableOp::LoadStaticCstrAddr { slot_idx, .. } => Some(*slot_idx),
                _ => None,
            };
            if let Some(s) = slot {
                max_slot = Some(max_slot.map_or(s, |m: u8| m.max(s)));
            }
        }
    }
    max_slot.map_or(0, |m| m + 1)
}

fn executable_function_uses_frame(function: &ExecutableFunction) -> bool {
    executable_function_n_stack_cells(function) > 0 || !function.signature.params.is_empty()
}

/// Round frame_size up for SysV AMD64 ABI 16-byte stack alignment.
///
/// Before any `call` instruction, RSP must be 16-byte aligned.  At function
/// entry RSP % 16 == 8 (return address on stack).  After `push %rbx` (when
/// `uses_saved_value_slot` is true), RSP % 16 == 0, so the subsequent `sub`
/// must be a multiple of 16.  Without the push, the `sub` must be ≡ 8 mod 16.
fn asm_aligned_frame_size(frame_size_raw: u32, uses_saved_value_slot: bool) -> u32 {
    if uses_saved_value_slot {
        (frame_size_raw + 15) & !15
    } else {
        let r = frame_size_raw % 16;
        if r == 8 {
            frame_size_raw
        } else if r < 8 {
            frame_size_raw + (8 - r)
        } else {
            frame_size_raw + (24 - r)
        }
    }
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
        let uses_frame = function.n_stack_cells > 0 || function.n_params > 0;
        let sc_bytes = 8u32 * u32::from(function.n_stack_cells);
        let frame_size = asm_aligned_frame_size(
            sc_bytes + 8u32 * function.n_params as u32,
            function.uses_saved_value_slot,
        );
        if uses_frame {
            out.push_str(&format!("    sub ${}, %rsp\n", frame_size));
            for i in 0..function.n_params {
                let offset = sc_bytes + 8u32 * i as u32;
                out.push_str(&format!(
                    "    movq {}, {}(%rsp)\n",
                    asm_param_register(i as u8),
                    offset
                ));
            }
        }
        for instruction in &function.instructions {
            match instruction {
                X86_64AsmInstruction::Call { symbol } => {
                    out.push_str("    call ");
                    out.push_str(symbol);
                    out.push('\n');
                }
                X86_64AsmInstruction::CallCapture { ty, symbol } => {
                    out.push_str("    call ");
                    out.push_str(symbol);
                    out.push('\n');
                    out.push_str("    ");
                    out.push_str(mmio_move_saved_value_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_accumulator_register(*ty));
                    out.push_str(", ");
                    out.push_str(mmio_saved_value_register(*ty));
                    out.push('\n');
                }
                X86_64AsmInstruction::CallCaptureWithArgs {
                    symbol,
                    args,
                    ty,
                    slot_idx,
                } => {
                    // Load args into SysV parameter registers.
                    for (i, arg) in args.iter().enumerate() {
                        let reg = asm_param_register(i as u8);
                        match arg {
                            ExecutableCallArg::Imm { value } => {
                                out.push_str(&format!("    movq ${}, {}\n", value, reg));
                            }
                            ExecutableCallArg::Slot { byte_offset } => {
                                if *byte_offset == 0 {
                                    out.push_str(&format!("    movq (%rsp), {}\n", reg));
                                } else {
                                    out.push_str(&format!(
                                        "    movq {}(%rsp), {}\n",
                                        byte_offset, reg
                                    ));
                                }
                            }
                            ExecutableCallArg::SavedValue => {
                                out.push_str(&format!("    movq %rbx, {}\n", reg));
                            }
                        }
                    }
                    out.push_str("    call ");
                    out.push_str(symbol);
                    out.push('\n');
                    // Store the return value from the accumulator to the stack slot.
                    let offset = 8u32 * u32::from(*slot_idx);
                    let mnem = mmio_store_mnemonic(*ty);
                    let acc = mmio_accumulator_register(*ty);
                    if offset == 0 {
                        out.push_str(&format!("    {} {}, (%rsp)\n", mnem, acc));
                    } else {
                        out.push_str(&format!("    {} {}, {}(%rsp)\n", mnem, acc, offset));
                    }
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
                X86_64AsmInstruction::StackStoreImm {
                    ty,
                    value,
                    slot_idx,
                } => {
                    let offset = 8u32 * u32::from(*slot_idx);
                    out.push_str("    ");
                    out.push_str(&mmio_immediate_mnemonic(*ty, *value));
                    out.push('\n');
                    out.push_str("    ");
                    out.push_str(mmio_store_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_value_register(*ty));
                    if offset == 0 {
                        out.push_str(", (%rsp)\n");
                    } else {
                        out.push_str(&format!(", {}(%rsp)\n", offset));
                    }
                }
                X86_64AsmInstruction::StackStoreValue { ty, slot_idx } => {
                    let offset = 8u32 * u32::from(*slot_idx);
                    out.push_str("    ");
                    out.push_str(mmio_store_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_saved_value_register(*ty));
                    if offset == 0 {
                        out.push_str(", (%rsp)\n");
                    } else {
                        out.push_str(&format!(", {}(%rsp)\n", offset));
                    }
                }
                X86_64AsmInstruction::StackLoad { ty, slot_idx } => {
                    let offset = 8u32 * u32::from(*slot_idx);
                    out.push_str("    ");
                    out.push_str(mmio_load_mnemonic(*ty));
                    if offset == 0 {
                        out.push_str(" (%rsp), ");
                    } else {
                        out.push_str(&format!(" {}(%rsp), ", offset));
                    }
                    out.push_str(mmio_saved_value_register(*ty));
                    out.push('\n');
                }
                X86_64AsmInstruction::SlotArithImm {
                    ty,
                    slot_idx,
                    arith_op,
                    imm,
                } => {
                    let offset = 8u32 * u32::from(*slot_idx);
                    // load from stack slot into saved-value register (%rbx family)
                    out.push_str("    ");
                    out.push_str(mmio_load_mnemonic(*ty));
                    if offset == 0 {
                        out.push_str(" (%rsp), ");
                    } else {
                        out.push_str(&format!(" {}(%rsp), ", offset));
                    }
                    out.push_str(mmio_saved_value_register(*ty));
                    out.push('\n');
                    // 64-bit arithmetic on %rbx
                    let mnemonic = match arith_op {
                        ArithOp::Add => "addq",
                        ArithOp::Sub => "subq",
                        ArithOp::And => "andq",
                        ArithOp::Or => "orq",
                        ArithOp::Xor => "xorq",
                        ArithOp::Shl => "shlq",
                        ArithOp::Shr => "shrq",
                    };
                    out.push_str(&format!("    {} ${}, %rbx\n", mnemonic, imm));
                    // store from saved-value register back to stack slot
                    out.push_str("    ");
                    out.push_str(mmio_store_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_saved_value_register(*ty));
                    if offset == 0 {
                        out.push_str(", (%rsp)\n");
                    } else {
                        out.push_str(&format!(", {}(%rsp)\n", offset));
                    }
                }
                X86_64AsmInstruction::SlotArithSlot {
                    ty,
                    dst_slot_idx,
                    src_slot_idx,
                    arith_op,
                } => {
                    let src_offset = 8u32 * u32::from(*src_slot_idx);
                    let dst_offset = 8u32 * u32::from(*dst_slot_idx);
                    // load src into scratch register (%rax for non-shifts, %rcx for shifts)
                    let src_reg = match arith_op {
                        ArithOp::Shl | ArithOp::Shr => mmio_value_register(*ty),
                        _ => mmio_accumulator_register(*ty),
                    };
                    out.push_str("    ");
                    out.push_str(mmio_load_mnemonic(*ty));
                    if src_offset == 0 {
                        out.push_str(" (%rsp), ");
                    } else {
                        out.push_str(&format!(" {}(%rsp), ", src_offset));
                    }
                    out.push_str(src_reg);
                    out.push('\n');
                    // typed op into dst memory slot
                    out.push_str("    ");
                    out.push_str(slot_arith_slot_op_mnemonic(*ty, *arith_op));
                    out.push(' ');
                    out.push_str(src_reg);
                    out.push_str(", ");
                    if dst_offset == 0 {
                        out.push_str("(%rsp)\n");
                    } else {
                        out.push_str(&format!("{}(%rsp)\n", dst_offset));
                    }
                }
                X86_64AsmInstruction::CallWithArgs { symbol, args } => {
                    for (i, arg) in args.iter().enumerate() {
                        let reg = asm_param_register(i as u8);
                        match arg {
                            ExecutableCallArg::Imm { value } => {
                                out.push_str(&format!("    movq ${}, {}\n", value, reg));
                            }
                            ExecutableCallArg::Slot { byte_offset } => {
                                if *byte_offset == 0 {
                                    out.push_str(&format!("    movq (%rsp), {}\n", reg));
                                } else {
                                    out.push_str(&format!(
                                        "    movq {}(%rsp), {}\n",
                                        byte_offset, reg
                                    ));
                                }
                            }
                            ExecutableCallArg::SavedValue => {
                                out.push_str(&format!("    movq %rbx, {}\n", reg));
                            }
                        }
                    }
                    out.push_str("    call ");
                    out.push_str(symbol);
                    out.push('\n');
                }
                X86_64AsmInstruction::ParamLoad { param_idx, ty } => {
                    let sc_bytes = 8u32 * u32::from(function.n_stack_cells);
                    let offset = sc_bytes + 8u32 * u32::from(*param_idx);
                    out.push_str("    ");
                    out.push_str(mmio_load_mnemonic(*ty));
                    out.push_str(&format!(" {}(%rsp), ", offset));
                    out.push_str(mmio_saved_value_register(*ty));
                    out.push('\n');
                }
                X86_64AsmInstruction::MmioReadParamAddr {
                    param_idx,
                    ty,
                    capture_value,
                } => {
                    let sc_bytes = 8u32 * u32::from(function.n_stack_cells);
                    let offset = sc_bytes + 8u32 * u32::from(*param_idx);
                    out.push_str(&format!("    movq {}(%rsp), %rax\n", offset));
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
                X86_64AsmInstruction::MmioWriteImmParamAddr {
                    param_idx,
                    ty,
                    value,
                } => {
                    let sc_bytes = 8u32 * u32::from(function.n_stack_cells);
                    let offset = sc_bytes + 8u32 * u32::from(*param_idx);
                    out.push_str(&format!("    movq {}(%rsp), %rax\n", offset));
                    out.push_str("    ");
                    out.push_str(&mmio_immediate_mnemonic(*ty, *value));
                    out.push('\n');
                    out.push_str("    ");
                    out.push_str(mmio_store_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_value_register(*ty));
                    out.push_str(", (%rax)\n");
                }
                X86_64AsmInstruction::MmioWriteValueParamAddr { param_idx, ty } => {
                    let sc_bytes = 8u32 * u32::from(function.n_stack_cells);
                    let offset = sc_bytes + 8u32 * u32::from(*param_idx);
                    out.push_str(&format!("    movq {}(%rsp), %rax\n", offset));
                    out.push_str("    ");
                    out.push_str(mmio_store_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_saved_value_register(*ty));
                    out.push_str(", (%rax)\n");
                }
                X86_64AsmInstruction::TailCall { symbol, args } => {
                    // Load args BEFORE frame teardown so stack slots are still valid.
                    for (i, arg) in args.iter().enumerate() {
                        let reg = asm_param_register(i as u8);
                        match arg {
                            ExecutableCallArg::Imm { value } => {
                                out.push_str(&format!("    movq ${}, {}\n", value, reg));
                            }
                            ExecutableCallArg::Slot { byte_offset } => {
                                if *byte_offset == 0 {
                                    out.push_str(&format!("    movq (%rsp), {}\n", reg));
                                } else {
                                    out.push_str(&format!(
                                        "    movq {}(%rsp), {}\n",
                                        byte_offset, reg
                                    ));
                                }
                            }
                            ExecutableCallArg::SavedValue => {
                                out.push_str(&format!("    movq %rbx, {}\n", reg));
                            }
                        }
                    }
                    if uses_frame {
                        out.push_str(&format!("    add ${}, %rsp\n", frame_size));
                    }
                    if function.uses_saved_value_slot {
                        out.push_str("    pop %rbx\n");
                    }
                    out.push_str("    jmp ");
                    out.push_str(symbol);
                    out.push('\n');
                }
                X86_64AsmInstruction::ReturnSavedValue { ty } => {
                    out.push_str("    ");
                    out.push_str(mmio_move_return_value_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_saved_value_register(*ty));
                    out.push_str(", ");
                    out.push_str(mmio_accumulator_register(*ty));
                    out.push('\n');
                }
                X86_64AsmInstruction::Ret => {
                    if uses_frame {
                        out.push_str(&format!("    add ${}, %rsp\n", frame_size));
                    }
                    if function.uses_saved_value_slot {
                        out.push_str("    pop %rbx\n");
                    }
                    out.push_str("    ret\n");
                }
                X86_64AsmInstruction::Label(name) => {
                    out.push_str(name);
                    out.push_str(":\n");
                }
                X86_64AsmInstruction::JmpLabel(name) => {
                    out.push_str("    jmp ");
                    out.push_str(name);
                    out.push('\n');
                }
                X86_64AsmInstruction::JmpIfZeroLabel(name) => {
                    out.push_str("    jz ");
                    out.push_str(name);
                    out.push('\n');
                }
                X86_64AsmInstruction::JmpIfNonZeroLabel(name) => {
                    out.push_str("    jnz ");
                    out.push_str(name);
                    out.push('\n');
                }
                X86_64AsmInstruction::LoadSlotU8ToEax { slot_idx } => {
                    let offset = 8u32 * u32::from(*slot_idx);
                    if offset == 0 {
                        out.push_str("    movzx (%rsp), %eax\n");
                    } else {
                        out.push_str(&format!("    movzx {}(%rsp), %eax\n", offset));
                    }
                }
                X86_64AsmInstruction::TestEaxEax => {
                    out.push_str("    test %eax, %eax\n");
                }
                X86_64AsmInstruction::CompareSlots {
                    cmp_op,
                    lhs_idx,
                    rhs_idx,
                    out_idx,
                } => {
                    // Load lhs into eax (zero-extended from qword slot)
                    let lhs_offset = 8u32 * u32::from(*lhs_idx);
                    let rhs_offset = 8u32 * u32::from(*rhs_idx);
                    let out_offset = 8u32 * u32::from(*out_idx);
                    if lhs_offset == 0 {
                        out.push_str("    movq (%rsp), %rax\n");
                    } else {
                        out.push_str(&format!("    movq {}(%rsp), %rax\n", lhs_offset));
                    }
                    // cmp rax, [rhs slot]
                    if rhs_offset == 0 {
                        out.push_str("    cmpq (%rsp), %rax\n");
                    } else {
                        out.push_str(&format!("    cmpq {}(%rsp), %rax\n", rhs_offset));
                    }
                    // setCC al
                    let setcc = match cmp_op {
                        CmpOp::Eq => "sete",
                        CmpOp::Ne => "setne",
                        CmpOp::Lt => "setl",
                        CmpOp::Gt => "setg",
                        CmpOp::Le => "setle",
                        CmpOp::Ge => "setge",
                    };
                    out.push_str(&format!("    {} %al\n", setcc));
                    // movzx eax, al
                    out.push_str("    movzx %al, %eax\n");
                    // store 32-bit result into out slot (zero upper bytes)
                    if out_offset == 0 {
                        out.push_str("    movl %eax, (%rsp)\n");
                    } else {
                        out.push_str(&format!("    movl %eax, {}(%rsp)\n", out_offset));
                    }
                }
                X86_64AsmInstruction::RawPtrLoad {
                    ty,
                    addr_slot_idx,
                    out_slot_idx,
                } => {
                    // 1. Load u64 address from addr_slot into %rax.
                    let addr_offset = 8u32 * u32::from(*addr_slot_idx);
                    if addr_offset == 0 {
                        out.push_str("    movq (%rsp), %rax\n");
                    } else {
                        out.push_str(&format!("    movq {}(%rsp), %rax\n", addr_offset));
                    }
                    // 2. Indirect load [%rax] into accumulator register.
                    out.push_str("    ");
                    out.push_str(mmio_load_mnemonic(*ty));
                    out.push_str(" (%rax), ");
                    out.push_str(mmio_accumulator_register(*ty));
                    out.push('\n');
                    // 3. Store accumulator into out_slot.
                    let out_offset = 8u32 * u32::from(*out_slot_idx);
                    out.push_str("    ");
                    out.push_str(mmio_store_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_accumulator_register(*ty));
                    if out_offset == 0 {
                        out.push_str(", (%rsp)\n");
                    } else {
                        out.push_str(&format!(", {}(%rsp)\n", out_offset));
                    }
                }
                X86_64AsmInstruction::RawPtrStoreImm {
                    ty,
                    addr_slot_idx,
                    value,
                } => {
                    // 1. Load u64 address from addr_slot into %rax.
                    let addr_offset = 8u32 * u32::from(*addr_slot_idx);
                    if addr_offset == 0 {
                        out.push_str("    movq (%rsp), %rax\n");
                    } else {
                        out.push_str(&format!("    movq {}(%rsp), %rax\n", addr_offset));
                    }
                    // 2. Load immediate into value register (%cl/%cx/%ecx/%rcx).
                    out.push_str("    ");
                    out.push_str(&mmio_immediate_mnemonic(*ty, *value));
                    out.push('\n');
                    // 3. Store value register to [%rax].
                    out.push_str("    ");
                    out.push_str(mmio_store_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_value_register(*ty));
                    out.push_str(", (%rax)\n");
                }
                X86_64AsmInstruction::RawPtrStoreSavedValue { ty, addr_slot_idx } => {
                    // 1. Load u64 address from addr_slot into %rax.
                    let addr_offset = 8u32 * u32::from(*addr_slot_idx);
                    if addr_offset == 0 {
                        out.push_str("    movq (%rsp), %rax\n");
                    } else {
                        out.push_str(&format!("    movq {}(%rsp), %rax\n", addr_offset));
                    }
                    // 2. Store saved value register (%bl/%bx/%ebx/%rbx) to [%rax].
                    out.push_str("    ");
                    out.push_str(mmio_store_mnemonic(*ty));
                    out.push(' ');
                    out.push_str(mmio_saved_value_register(*ty));
                    out.push_str(", (%rax)\n");
                }
                X86_64AsmInstruction::LoadStaticCstrAddr { str_label, slot_idx } => {
                    // lea __str_N(%rip), %rbx
                    out.push_str(&format!("    lea {}(%rip), %rbx\n", str_label));
                    // movq %rbx, OFFSET(%rsp)
                    let offset = 8u32 * u32::from(*slot_idx);
                    if offset == 0 {
                        out.push_str("    movq %rbx, (%rsp)\n");
                    } else {
                        out.push_str(&format!("    movq %rbx, {}(%rsp)\n", offset));
                    }
                }
                X86_64AsmInstruction::InlineAsm(intr) => {
                    let mnemonic = match intr {
                        KernelIntrinsic::Cli => "cli",
                        KernelIntrinsic::Sti => "sti",
                        KernelIntrinsic::Hlt => "hlt",
                        KernelIntrinsic::Nop => "nop",
                        KernelIntrinsic::Mfence => "mfence",
                        KernelIntrinsic::Sfence => "sfence",
                        KernelIntrinsic::Lfence => "lfence",
                        KernelIntrinsic::Wbinvd => "wbinvd",
                        KernelIntrinsic::Pause => "pause",
                        KernelIntrinsic::Int3 => "int3",
                        KernelIntrinsic::Cpuid => "cpuid",
                    };
                    out.push_str("    ");
                    out.push_str(mnemonic);
                    out.push('\n');
                }
            }
        }
    }
    // Emit static string literals in .rodata section.
    if !module.string_data.is_empty() {
        out.push_str(".section .rodata\n");
        for (label, value) in &module.string_data {
            // Escape backslashes and double-quotes; keep other chars as-is.
            let escaped = value
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
                .replace('\0', "\\0");
            out.push_str(&format!("{}: .asciz \"{}\"\n", label, escaped));
        }
    }
    out
}

/// Returns (mnemonic, load_reg) for an AArch64 MMIO load.
fn aarch64_load_parts(ty: MmioScalarType) -> (&'static str, &'static str) {
    match ty {
        MmioScalarType::U8 => ("ldrb", "w0"),
        MmioScalarType::U16 => ("ldrh", "w0"),
        MmioScalarType::U32 => ("ldr", "w0"),
        MmioScalarType::U64 => ("ldr", "x0"),
        MmioScalarType::F32 => ("ldr", "w0"),
        MmioScalarType::F64 => ("ldr", "x0"),
    }
}

/// Returns (mnemonic, store_reg) for an AArch64 MMIO store.
fn aarch64_store_parts(ty: MmioScalarType) -> (&'static str, &'static str) {
    match ty {
        MmioScalarType::U8 => ("strb", "w2"),
        MmioScalarType::U16 => ("strh", "w2"),
        MmioScalarType::U32 => ("str", "w2"),
        MmioScalarType::U64 => ("str", "x2"),
        MmioScalarType::F32 => ("str", "w2"),
        MmioScalarType::F64 => ("str", "x2"),
    }
}

pub fn emit_aarch64_asm_text(module: &AArch64AsmModule) -> String {
    let mut out = String::new();
    out.push_str(&format!("    .section {}\n", module.section));
    for func in &module.functions {
        out.push_str(&format!("    .global {}\n", func.symbol));
        out.push_str(&format!("{}:\n", func.symbol));

        // Prologue: stack_cells*8 rounded up to 16-byte alignment, plus 16 for x29/x30.
        let stack_bytes = (func.n_stack_cells as usize * 8 + 15) & !15;
        let frame_bytes = stack_bytes + 16;
        out.push_str(&format!("    stp x29, x30, [sp, #-{}]!\n", frame_bytes));
        out.push_str("    mov x29, sp\n");

        for instr in &func.instructions {
            match instr {
                AArch64AsmInstruction::Call { symbol } => {
                    out.push_str(&format!("    bl {}\n", symbol));
                }
                AArch64AsmInstruction::CallWithArgs { symbol, .. } => {
                    out.push_str(&format!("    bl {}\n", symbol));
                }
                AArch64AsmInstruction::TailCall { symbol, .. } => {
                    out.push_str(&format!("    ldp x29, x30, [sp], #{}\n", frame_bytes));
                    out.push_str(&format!("    b {}\n", symbol));
                }
                AArch64AsmInstruction::BranchIfZero {
                    then_symbol,
                    else_symbol,
                    ..
                } => {
                    out.push_str(&format!("    cbz x0, {}\n", then_symbol));
                    out.push_str(&format!("    b {}\n", else_symbol));
                }
                AArch64AsmInstruction::BranchIfEqImm {
                    compare_value,
                    then_symbol,
                    else_symbol,
                    ..
                } => {
                    out.push_str(&format!("    cmp x0, #{}\n", compare_value));
                    out.push_str(&format!("    b.eq {}\n", then_symbol));
                    out.push_str(&format!("    b {}\n", else_symbol));
                }
                AArch64AsmInstruction::BranchIfMaskNonZeroImm {
                    mask_value,
                    then_symbol,
                    else_symbol,
                    ..
                } => {
                    out.push_str(&format!("    tst x0, #{}\n", mask_value));
                    out.push_str(&format!("    b.ne {}\n", then_symbol));
                    out.push_str(&format!("    b {}\n", else_symbol));
                }
                AArch64AsmInstruction::Label(name) => {
                    out.push_str(&format!("{}:\n", name));
                }
                AArch64AsmInstruction::JmpLabel(label) => {
                    out.push_str(&format!("    b {}\n", label));
                }
                AArch64AsmInstruction::JmpIfZeroLabel(label) => {
                    out.push_str(&format!("    cbz x0, {}\n", label));
                }
                AArch64AsmInstruction::JmpIfNonZeroLabel(label) => {
                    out.push_str(&format!("    cbnz x0, {}\n", label));
                }
                AArch64AsmInstruction::Ret => {
                    out.push_str(&format!("    ldp x29, x30, [sp], #{}\n", frame_bytes));
                    out.push_str("    ret\n");
                }
                AArch64AsmInstruction::CallCapture { symbol, .. } => {
                    out.push_str(&format!("    bl {}\n", symbol));
                }
                AArch64AsmInstruction::CallCaptureWithArgs { symbol, slot_idx, .. } => {
                    out.push_str(&format!("    bl {}\n", symbol));
                    // x0 holds the return value; store it to [x29 + #(16 + slot*8)].
                    let slot_offset = 16 + (*slot_idx as u32) * 8;
                    out.push_str(&format!("    str x0, [x29, #{}]\n", slot_offset));
                }
                AArch64AsmInstruction::MmioRead { addr, ty, .. } => {
                    let (mnem, reg) = aarch64_load_parts(*ty);
                    out.push_str(&format!("    ldr x1, =0x{:x}\n", addr));
                    out.push_str(&format!("    {} {}, [x1]\n", mnem, reg));
                }
                AArch64AsmInstruction::MmioWriteImm { addr, value, ty } => {
                    let (mnem, reg) = aarch64_store_parts(*ty);
                    out.push_str(&format!("    ldr x1, =0x{:x}\n", addr));
                    out.push_str(&format!("    mov {}, #{}\n", reg, value));
                    out.push_str(&format!("    {} {}, [x1]\n", mnem, reg));
                }
                AArch64AsmInstruction::MmioWriteValue { addr, ty } => {
                    let (mnem, reg) = aarch64_store_parts(*ty);
                    out.push_str(&format!("    ldr x1, =0x{:x}\n", addr));
                    out.push_str(&format!("    {} {}, [x1]\n", mnem, reg));
                }
                _ => {
                    // Remaining variants: emit a comment placeholder.
                    out.push_str("    // TODO: unimplemented instruction\n");
                }
            }
        }
        out.push('\n');
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
                MmioScalarType::F32 => 4usize,
                MmioScalarType::F64 => 8usize,
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
                MmioScalarType::F32 => u32::from_le_bytes(
                    bytes[value_offset..value_offset + 4]
                        .try_into()
                        .expect("f32 immediate bytes"),
                ) as u64,
                MmioScalarType::F64 => u64::from_le_bytes(
                    bytes[value_offset..value_offset + 8]
                        .try_into()
                        .expect("f64 immediate bytes"),
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
        ExecutableOp::CallCapture { ty, .. } => 5 + mmio_saved_value_copy_bytes(*ty),
        // Branch encoding: test_bytes + cond_jump(6) + call_then(5) + jmp_over(5) + call_else(5) = test_bytes + 21
        ExecutableOp::BranchIfZero { ty, .. } => mmio_saved_value_zero_test_bytes(*ty) + 21,
        ExecutableOp::BranchIfEqImm {
            ty,
            compare_value: _,
            ..
        } => mmio_saved_value_literal_compare_bytes(*ty) + 21,
        ExecutableOp::BranchIfMaskNonZeroImm {
            ty, mask_value: _, ..
        } => mmio_saved_value_literal_mask_test_bytes(*ty) + 21,
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
        ExecutableOp::StackStoreImm { ty, slot_idx, .. } => {
            mmio_value_load_immediate_bytes(*ty) + stack_cell_access_bytes(*ty, *slot_idx)
        }
        ExecutableOp::StackStoreValue { ty, slot_idx } => stack_cell_access_bytes(*ty, *slot_idx),
        ExecutableOp::StackLoad { ty, slot_idx } => stack_cell_access_bytes(*ty, *slot_idx),
        ExecutableOp::SlotArithImm {
            ty,
            slot_idx,
            arith_op,
            imm,
        } => {
            // load + arith_op + store
            let access = stack_cell_access_bytes(*ty, *slot_idx);
            let op_bytes = slot_arith_imm_op_bytes(*arith_op, *imm);
            access + op_bytes + access
        }
        ExecutableOp::SlotArithSlot {
            ty,
            dst_slot_idx,
            src_slot_idx,
            ..
        } => {
            // load src into scratch reg + typed op into dst memory
            stack_cell_access_bytes(*ty, *src_slot_idx)
                + stack_cell_access_bytes(*ty, *dst_slot_idx)
        }
        // ParamLoad: movb/movw/movl/movq disp8(%rsp), %bl/%bx/%ebx/%rbx
        ExecutableOp::ParamLoad { ty, .. } => match ty {
            MmioScalarType::U8 | MmioScalarType::U32 => 4,
            MmioScalarType::U16 | MmioScalarType::U64 => 5,
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        },
        // Param-addr MMIO: load addr from stack (5 bytes) then same as constant-addr variant minus movabs (10 bytes).
        ExecutableOp::MmioReadParamAddr {
            ty, capture_value, ..
        } => {
            5 + mmio_load_bytes(*ty)
                + if *capture_value {
                    mmio_saved_value_copy_bytes(*ty)
                } else {
                    0
                }
        }
        ExecutableOp::MmioWriteImmParamAddr { ty, .. } => {
            5 + mmio_value_load_immediate_bytes(*ty) + mmio_store_bytes(*ty)
        }
        ExecutableOp::MmioWriteValueParamAddr { ty, .. } => 5 + mmio_saved_value_store_bytes(*ty),
        ExecutableOp::CallWithArgs { args, .. } => {
            args.iter().map(call_arg_encoded_bytes).sum::<u64>() + 5
        }
        ExecutableOp::CallCaptureWithArgs { args, ty, slot_idx, .. } => {
            args.iter().map(call_arg_encoded_bytes).sum::<u64>()
                + 5  // call rel32
                + stack_cell_access_bytes(*ty, *slot_idx)  // store acc -> slot
        }
        // Loop ops:
        ExecutableOp::LoopBegin => 0,    // label only, no bytes
        ExecutableOp::LoopEnd => 5,      // jmp rel32
        ExecutableOp::LoopBreak => 5,    // jmp rel32
        ExecutableOp::LoopContinue => 5, // jmp rel32
        // movzx eax, byte ptr [rsp+disp8] (4 bytes) + test eax, eax (2 bytes) + jz rel32 (6 bytes) = 12
        ExecutableOp::BranchIfZeroLoopBreak { .. } => 12,
        ExecutableOp::BranchIfNonZeroLoopBreak { .. } => 12,
        // movq [rsp+disp8], rax (5) + cmpq [rsp+disp8], rax (5) + setCC al (3) + movzx eax,al (3) + movl eax,[rsp+disp8] (5) = 21
        ExecutableOp::CompareIntoSlot { .. } => 21,
        // RawPtrLoad: load addr (u64) from addr_slot into %rax, then indirect load [%rax] into %al/etc,
        // then store result into out_slot.
        ExecutableOp::RawPtrLoad {
            ty,
            addr_slot_idx,
            out_slot_idx,
        } => {
            stack_cell_access_bytes(MmioScalarType::U64, *addr_slot_idx)
                + mmio_load_bytes(*ty)
                + stack_cell_access_bytes(*ty, *out_slot_idx)
        }
        // RawPtrStore: load addr (u64) from addr_slot into %rax, then store value to [%rax].
        ExecutableOp::RawPtrStore {
            ty,
            addr_slot_idx,
            value,
        } => {
            let value_bytes = match value {
                MmioValueExpr::IntLiteral { .. } => {
                    mmio_value_load_immediate_bytes(*ty) + mmio_store_bytes(*ty)
                }
                MmioValueExpr::Ident { .. } => mmio_saved_value_store_bytes(*ty),
                MmioValueExpr::FloatLiteral { .. } => {
                    unreachable!("float RawPtrStore encoding requires SSE2 backend (Task 14)")
                }
            };
            stack_cell_access_bytes(MmioScalarType::U64, *addr_slot_idx) + value_bytes
        }
        ExecutableOp::InlineAsm(intr) => match intr {
            KernelIntrinsic::Cli
            | KernelIntrinsic::Sti
            | KernelIntrinsic::Hlt
            | KernelIntrinsic::Nop
            | KernelIntrinsic::Int3 => 1,
            KernelIntrinsic::Wbinvd | KernelIntrinsic::Pause | KernelIntrinsic::Cpuid => 2,
            KernelIntrinsic::Mfence | KernelIntrinsic::Sfence | KernelIntrinsic::Lfence => 3,
        },
        // lea [rip+disp32], %rbx  (7 bytes: REX.W + opcode + ModRM + rel32)
        // followed by mov %rbx, slot(%rsp)
        ExecutableOp::LoadStaticCstrAddr { slot_idx, .. } => {
            7 + stack_cell_access_bytes(MmioScalarType::U64, *slot_idx)
        }
    }
}

fn call_arg_encoded_bytes(arg: &ExecutableCallArg) -> u64 {
    match arg {
        ExecutableCallArg::Imm { value } => {
            if *value <= 0x7FFF_FFFF {
                7
            } else {
                10
            }
        }
        ExecutableCallArg::Slot { .. } => 5,
        ExecutableCallArg::SavedValue => 3,
    }
}

fn executable_terminator_encoded_len(function: &ExecutableFunction) -> u64 {
    let frame_bytes = if executable_function_uses_frame(function) {
        rsp_adj_encoded_len(executable_function_frame_size(function))
    } else {
        0
    };
    let pop_rbx = if executable_function_uses_saved_value_slot(function) {
        1u64
    } else {
        0u64
    };
    match &function.blocks[0].terminator {
        ExecutableTerminator::Return {
            value: ExecutableValue::SavedValue { ty },
        } => mmio_saved_value_copy_bytes(*ty) + frame_bytes + pop_rbx + 1,
        ExecutableTerminator::Return {
            value: ExecutableValue::Unit,
        } => frame_bytes + pop_rbx + 1,
        ExecutableTerminator::TailCall { args, .. } => {
            let args_bytes: u64 = args.iter().map(call_arg_encoded_bytes).sum();
            frame_bytes + pop_rbx + args_bytes + 5 // 5 for jmp rel32
        }
    }
}

fn encode_mmio_read_bytes(out: &mut Vec<u8>, ty: MmioScalarType, addr: u64, capture_value: bool) {
    push_movabs_rax_imm64(out, addr);
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x00]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x00]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x00]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x00]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
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
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn encode_mmio_write_saved_value_bytes(out: &mut Vec<u8>, ty: MmioScalarType, addr: u64) {
    push_movabs_rax_imm64(out, addr);
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x18]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x18]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x18]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x18]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn encode_stack_cell_store_imm_slot_bytes(
    out: &mut Vec<u8>,
    ty: MmioScalarType,
    value: u64,
    slot_idx: u8,
) {
    let offset = 8u8 * slot_idx;
    push_mov_imm_to_value_register(out, ty, value);
    if offset == 0 {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x0C, 0x24]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x0C, 0x24]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x0C, 0x24]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x0C, 0x24]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        }
    } else {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x4C, 0x24, offset]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x4C, 0x24, offset]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x4C, 0x24, offset]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x4C, 0x24, offset]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        }
    }
}

fn encode_stack_cell_store_saved_value_slot_bytes(
    out: &mut Vec<u8>,
    ty: MmioScalarType,
    slot_idx: u8,
) {
    let offset = 8u8 * slot_idx;
    if offset == 0 {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x1C, 0x24]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x1C, 0x24]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x1C, 0x24]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x1C, 0x24]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        }
    } else {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x5C, 0x24, offset]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x5C, 0x24, offset]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x5C, 0x24, offset]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x5C, 0x24, offset]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        }
    }
}

fn encode_stack_cell_load_slot_bytes(out: &mut Vec<u8>, ty: MmioScalarType, slot_idx: u8) {
    let offset = 8u8 * slot_idx;
    if offset == 0 {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x1C, 0x24]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x1C, 0x24]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x1C, 0x24]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x1C, 0x24]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        }
    } else {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x5C, 0x24, offset]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x5C, 0x24, offset]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x5C, 0x24, offset]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x5C, 0x24, offset]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        }
    }
}

/// Load a stack slot into `%al/%ax/%eax/%rax` (scratch, reg field = 0).
fn encode_stack_cell_load_slot_bytes_into_rax(out: &mut Vec<u8>, ty: MmioScalarType, slot_idx: u8) {
    let offset = 8u8 * slot_idx;
    if offset == 0 {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x04, 0x24]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x04, 0x24]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x04, 0x24]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x04, 0x24]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        }
    } else {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x44, 0x24, offset]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x44, 0x24, offset]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x44, 0x24, offset]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x44, 0x24, offset]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        }
    }
}

/// Load a stack slot into `%cl/%cx/%ecx/%rcx` (shift-count register, reg field = 1).
fn encode_stack_cell_load_slot_bytes_into_rcx(out: &mut Vec<u8>, ty: MmioScalarType, slot_idx: u8) {
    let offset = 8u8 * slot_idx;
    if offset == 0 {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x0C, 0x24]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x0C, 0x24]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x0C, 0x24]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x0C, 0x24]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        }
    } else {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x4C, 0x24, offset]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x4C, 0x24, offset]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x4C, 0x24, offset]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x4C, 0x24, offset]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
            }
        }
    }
}

/// Emit a typed `op reg, dst_off(%rsp)` for `SlotArithSlot`.
/// Non-shifts use `%al/%ax/%eax/%rax` (reg field = 0) as source.
/// Shifts use `%cl` as count (D2/D3 /4 or /5 opcode form).
fn encode_slot_arith_slot_op_bytes(
    out: &mut Vec<u8>,
    ty: MmioScalarType,
    op: ArithOp,
    dst_slot_idx: u8,
) {
    let offset = 8u8 * dst_slot_idx;
    // SIB byte for [rsp]: scale=00, index=100(none), base=100(rsp) = 0x24.
    // ModRM for [rsp], no disp: mod=00, rm=100(SIB) → 0b00_rrr_100.
    // ModRM for [rsp+disp8]:    mod=01, rm=100(SIB) → 0b01_rrr_100.
    match op {
        ArithOp::Add | ArithOp::Sub | ArithOp::And | ArithOp::Or | ArithOp::Xor => {
            // `op r/m, r` family: U8 opcode X (/r), 16/32/64-bit opcode X+1 (/r).
            // reg field = 0 (%rax family). ModRM base: 0b??_000_100.
            let opcode8: u8 = match op {
                ArithOp::Add => 0x00,
                ArithOp::Sub => 0x28,
                ArithOp::And => 0x20,
                ArithOp::Or => 0x08,
                ArithOp::Xor => 0x30,
                _ => unreachable!(),
            };
            let (modrm_no, modrm_d8) = if offset == 0 {
                (0x04u8, 0x04u8)
            } else {
                (0x04u8, 0x44u8)
            };
            if offset == 0 {
                match ty {
                    MmioScalarType::U8 => out.extend_from_slice(&[opcode8, modrm_no, 0x24]),
                    MmioScalarType::U16 => {
                        out.extend_from_slice(&[0x66, opcode8 + 1, modrm_no, 0x24])
                    }
                    MmioScalarType::U32 => out.extend_from_slice(&[opcode8 + 1, modrm_no, 0x24]),
                    MmioScalarType::U64 => {
                        out.extend_from_slice(&[0x48, opcode8 + 1, modrm_no, 0x24])
                    }
                    MmioScalarType::F32 | MmioScalarType::F64 => {
                        unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
                    }
                }
            } else {
                match ty {
                    MmioScalarType::U8 => out.extend_from_slice(&[opcode8, modrm_d8, 0x24, offset]),
                    MmioScalarType::U16 => {
                        out.extend_from_slice(&[0x66, opcode8 + 1, modrm_d8, 0x24, offset])
                    }
                    MmioScalarType::U32 => {
                        out.extend_from_slice(&[opcode8 + 1, modrm_d8, 0x24, offset])
                    }
                    MmioScalarType::U64 => {
                        out.extend_from_slice(&[0x48, opcode8 + 1, modrm_d8, 0x24, offset])
                    }
                    MmioScalarType::F32 | MmioScalarType::F64 => {
                        unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
                    }
                }
            }
        }
        ArithOp::Shl | ArithOp::Shr => {
            // `shl/shr r/m, CL`: U8 opcode D2, others D3; /4 for SHL, /5 for SHR.
            // ModRM: mod=00/01, reg=/4=4 or /5=5, rm=100(SIB).
            let (modrm_no, modrm_d8) = match op {
                ArithOp::Shl => (0x24u8, 0x64u8), // mod=00/01, reg=4, rm=4
                ArithOp::Shr => (0x2Cu8, 0x6Cu8), // mod=00/01, reg=5, rm=4
                _ => unreachable!(),
            };
            if offset == 0 {
                match ty {
                    MmioScalarType::U8 => out.extend_from_slice(&[0xD2, modrm_no, 0x24]),
                    MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0xD3, modrm_no, 0x24]),
                    MmioScalarType::U32 => out.extend_from_slice(&[0xD3, modrm_no, 0x24]),
                    MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0xD3, modrm_no, 0x24]),
                    MmioScalarType::F32 | MmioScalarType::F64 => {
                        unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
                    }
                }
            } else {
                match ty {
                    MmioScalarType::U8 => out.extend_from_slice(&[0xD2, modrm_d8, 0x24, offset]),
                    MmioScalarType::U16 => {
                        out.extend_from_slice(&[0x66, 0xD3, modrm_d8, 0x24, offset])
                    }
                    MmioScalarType::U32 => out.extend_from_slice(&[0xD3, modrm_d8, 0x24, offset]),
                    MmioScalarType::U64 => {
                        out.extend_from_slice(&[0x48, 0xD3, modrm_d8, 0x24, offset])
                    }
                    MmioScalarType::F32 | MmioScalarType::F64 => {
                        unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
                    }
                }
            }
        }
    }
}

/// Encode a 64-bit arithmetic-immediate instruction on %rbx.
/// Arithmetic (add/sub/and/or/xor): imm ≤ 127 → imm8 form, else imm32 form.
/// Shifts (shl/shr): count == 1 → one-bit shift form, else imm8 count form.
fn encode_slot_arith_imm_bytes(out: &mut Vec<u8>, op: ArithOp, imm: u64) {
    match op {
        // 64-bit arithmetic on %rbx:
        // imm8:  REX.W 0x83 ModRM imm8   (4 bytes, sign-extends imm8 to 64 bits)
        // imm32: REX.W 0x81 ModRM imm32  (7 bytes, sign-extends imm32 to 64 bits)
        // ModRM encoding for %rbx: mod=11 rm=011; /n from opcode group:
        //   ADD  /0 → 0xC3, SUB /5 → 0xEB, AND /4 → 0xE3, OR /1 → 0xCB, XOR /6 → 0xF3
        ArithOp::Add => encode_rbx_arith_imm(out, 0xC3, imm),
        ArithOp::Sub => encode_rbx_arith_imm(out, 0xEB, imm),
        ArithOp::And => encode_rbx_arith_imm(out, 0xE3, imm),
        ArithOp::Or => encode_rbx_arith_imm(out, 0xCB, imm),
        ArithOp::Xor => encode_rbx_arith_imm(out, 0xF3, imm),
        // 64-bit shift on %rbx:
        // SHL /4 → ModRM 0xE3, SHR /5 → ModRM 0xEB
        // count=1: REX.W 0xD1 ModRM      (3 bytes)
        // count>1: REX.W 0xC1 ModRM imm8 (4 bytes)
        ArithOp::Shl => encode_rbx_shift_imm(out, 0xE3, imm),
        ArithOp::Shr => encode_rbx_shift_imm(out, 0xEB, imm),
    }
}

fn encode_rbx_arith_imm(out: &mut Vec<u8>, modrm: u8, imm: u64) {
    if imm <= 127 {
        out.extend_from_slice(&[0x48, 0x83, modrm, imm as u8]);
    } else {
        let b = (imm as u32).to_le_bytes();
        out.extend_from_slice(&[0x48, 0x81, modrm, b[0], b[1], b[2], b[3]]);
    }
}

fn encode_rbx_shift_imm(out: &mut Vec<u8>, modrm: u8, count: u64) {
    if count == 1 {
        out.extend_from_slice(&[0x48, 0xD1, modrm]);
    } else {
        out.extend_from_slice(&[0x48, 0xC1, modrm, count as u8]);
    }
}

// Emit `MOV QWORD PTR [rsp+offset], %reg` where reg is the i-th SysV integer param register.
// Encoding: REX(W+R?), 0x89, ModRM(mod=01, reg=N, rm=4), SIB(0x24), disp8
/// Byte count of `sub rsp, N` / `add rsp, N` for a given frame size.
fn rsp_adj_encoded_len(amount: u32) -> u64 {
    if amount <= 127 { 4 } else { 7 }
}

/// Emit `sub rsp, amount`; uses imm8 (4 bytes) when amount ≤ 127, imm32 (7 bytes) otherwise.
fn emit_sub_rsp(out: &mut Vec<u8>, amount: u32) {
    if amount <= 127 {
        out.extend_from_slice(&[0x48, 0x83, 0xEC, amount as u8]);
    } else {
        let b = amount.to_le_bytes();
        out.extend_from_slice(&[0x48, 0x81, 0xEC, b[0], b[1], b[2], b[3]]);
    }
}

/// Emit `add rsp, amount`; uses imm8 (4 bytes) when amount ≤ 127, imm32 (7 bytes) otherwise.
fn emit_add_rsp(out: &mut Vec<u8>, amount: u32) {
    if amount <= 127 {
        out.extend_from_slice(&[0x48, 0x83, 0xC4, amount as u8]);
    } else {
        let b = amount.to_le_bytes();
        out.extend_from_slice(&[0x48, 0x81, 0xC4, b[0], b[1], b[2], b[3]]);
    }
}

/// Total frame size in bytes: 8 bytes per stack cell + 8 bytes per param.
fn executable_function_frame_size(function: &ExecutableFunction) -> u32 {
    8u32 * u32::from(executable_function_n_stack_cells(function))
        + 8u32 * function.signature.params.len() as u32
}

fn emit_param_spill(out: &mut Vec<u8>, param_idx: usize, offset: u8) {
    let (rex, modrm): (u8, u8) = match param_idx {
        0 => (0x48, 0x7C), // rdi  (reg=7)
        1 => (0x48, 0x74), // rsi  (reg=6)
        2 => (0x48, 0x54), // rdx  (reg=2)
        3 => (0x48, 0x4C), // rcx  (reg=1)
        4 => (0x4C, 0x44), // r8   (REX.R, reg=0)
        5 => (0x4C, 0x4C), // r9   (REX.R, reg=1)
        _ => panic!("SysV ABI supports at most 6 integer register params"),
    };
    out.extend_from_slice(&[rex, 0x89, modrm, 0x24, offset]);
}

/// Encode one call-with-args argument into the corresponding SysV integer arg register.
/// Registers by index: 0→%rdi, 1→%rsi, 2→%rdx, 3→%rcx, 4→%r8, 5→%r9.
/// Sizes: Imm(≤0x7FFFFFFF)→7B, Imm(>0x7FFFFFFF)→10B, Slot→5B (disp8), SavedValue→3B.
fn encode_call_arg_bytes(out: &mut Vec<u8>, arg_idx: u8, arg: &ExecutableCallArg) {
    // (reg_field in ModRM/opcode, is_extended: register number ≥ 8)
    let (reg_field, is_ext): (u8, bool) = match arg_idx {
        0 => (7, false), // %rdi
        1 => (6, false), // %rsi
        2 => (2, false), // %rdx
        3 => (1, false), // %rcx
        4 => (0, true),  // %r8
        5 => (1, true),  // %r9
        _ => panic!("SysV ABI supports at most 6 integer register args"),
    };
    match arg {
        ExecutableCallArg::Imm { value } => {
            let v = *value;
            if v <= 0x7FFF_FFFF {
                // movq $imm32sext, %regN  (7 bytes)
                // MOV r/m64, imm32 (0xC7); dest is R/M → REX.B for extended regs
                let rex = if is_ext { 0x49u8 } else { 0x48u8 };
                out.push(rex);
                out.push(0xC7);
                out.push(0xC0 | reg_field); // mod=11, opcode-ext=0, r/m=reg_field
                out.extend_from_slice(&(v as u32).to_le_bytes());
            } else {
                // movq $imm64, %regN  (10 bytes)
                // MOV r64, imm64 (0xB8+reg); reg in opcode → REX.B for extended regs
                let rex = if is_ext { 0x49u8 } else { 0x48u8 };
                out.push(rex);
                out.push(0xB8 + reg_field);
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        ExecutableCallArg::Slot { byte_offset } => {
            // movq byte_offset(%rsp), %regN  (5 bytes, disp8)
            // MOV r64, r/m64 (0x8B); dest is REG → REX.R for extended regs
            let rex = if is_ext { 0x4Cu8 } else { 0x48u8 }; // REX.W + REX.R
            let modrm = 0x40 | (reg_field << 3) | 4; // mod=01, reg=dest, rm=4 (SIB follows)
            out.extend_from_slice(&[rex, 0x8B, modrm, 0x24, *byte_offset as u8]);
        }
        ExecutableCallArg::SavedValue => {
            // movq %rbx, %regN  (3 bytes)
            // MOV r/m64, r64 (0x89); src=%rbx(3) is REG, dest is R/M → REX.B for extended regs
            let rex = if is_ext { 0x49u8 } else { 0x48u8 }; // REX.W + REX.B
            out.extend_from_slice(&[rex, 0x89, 0xC0 | (3 << 3) | reg_field]);
        }
    }
}

// Emit `MOV ty-sized [rsp+offset] -> %rbx` (load param into saved-value register).
fn encode_param_load_bytes(out: &mut Vec<u8>, ty: MmioScalarType, offset: u8) {
    match ty {
        // movb  disp8(%rsp), %bl  — 0x8A /r  ModRM=0x5C SIB=0x24
        MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x5C, 0x24, offset]),
        // movw  disp8(%rsp), %bx  — 0x66 0x8B /r
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x5C, 0x24, offset]),
        // movl  disp8(%rsp), %ebx — 0x8B /r  (zero-extends to rbx)
        MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x5C, 0x24, offset]),
        // movq  disp8(%rsp), %rbx — REX.W 0x8B /r
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x5C, 0x24, offset]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

// Emit `mov disp8(%rsp), %rax` — loads a u64 param (spilled to stack) into %rax for MMIO addressing.
// Encoding: REX.W (0x48) + MOV r64,r/m64 (0x8B) + ModRM[mod=01,reg=rax(0),rm=4] (0x44) + SIB[0x24] + disp8
fn push_load_param_addr_to_rax(out: &mut Vec<u8>, offset: u8) {
    out.extend_from_slice(&[0x48, 0x8B, 0x44, 0x24, offset]);
}

fn encode_mmio_read_param_addr_bytes(
    out: &mut Vec<u8>,
    ty: MmioScalarType,
    offset: u8,
    capture_value: bool,
) {
    push_load_param_addr_to_rax(out, offset);
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x00]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x00]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x00]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x00]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
    if capture_value {
        push_mov_accumulator_to_saved_value_register(out, ty);
    }
}

fn encode_mmio_write_imm_param_addr_bytes(
    out: &mut Vec<u8>,
    ty: MmioScalarType,
    offset: u8,
    value: u64,
) {
    push_load_param_addr_to_rax(out, offset);
    push_mov_imm_to_value_register(out, ty, value);
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x08]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x08]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x08]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x08]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn encode_mmio_write_saved_value_param_addr_bytes(
    out: &mut Vec<u8>,
    ty: MmioScalarType,
    offset: u8,
) {
    push_load_param_addr_to_rax(out, offset);
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x18]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x18]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x18]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x18]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

/// Store `%al/%ax/%eax/%rax` (accumulator, reg=0) to a stack cell slot.
fn encode_stack_cell_store_accumulator_slot_bytes(
    out: &mut Vec<u8>,
    ty: MmioScalarType,
    slot_idx: u8,
) {
    let offset = 8u8 * slot_idx;
    if offset == 0 {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x04, 0x24]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x04, 0x24]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x04, 0x24]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x04, 0x24]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float RawPtrLoad encoding requires SSE2 backend (Task 14)")
            }
        }
    } else {
        match ty {
            MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x44, 0x24, offset]),
            MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x44, 0x24, offset]),
            MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x44, 0x24, offset]),
            MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x44, 0x24, offset]),
            MmioScalarType::F32 | MmioScalarType::F64 => {
                unreachable!("float RawPtrLoad encoding requires SSE2 backend (Task 14)")
            }
        }
    }
}

/// Encode `RawPtrLoad`: load addr (u64) from addr_slot into %rax,
/// then do an indirect load [%rax] into %al/%ax/%eax/%rax,
/// then store the result into out_slot.
fn encode_raw_ptr_load_bytes(
    out: &mut Vec<u8>,
    ty: MmioScalarType,
    addr_slot_idx: u8,
    out_slot_idx: u8,
) {
    // 1. Load the u64 address from the addr_slot into %rax.
    encode_stack_cell_load_slot_bytes_into_rax(out, MmioScalarType::U64, addr_slot_idx);
    // 2. Indirect load from [%rax] into %al/%ax/%eax/%rax.
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x8A, 0x00]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x8B, 0x00]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x8B, 0x00]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x8B, 0x00]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float RawPtrLoad encoding requires SSE2 backend (Task 14)")
        }
    }
    // 3. Store result (%al/%ax/%eax/%rax) into out_slot on the stack.
    encode_stack_cell_store_accumulator_slot_bytes(out, ty, out_slot_idx);
}

/// Encode `RawPtrStore` with an immediate value: load addr (u64) from addr_slot into %rax,
/// load immediate into %cl/%cx/%ecx/%rcx, then store to [%rax].
fn encode_raw_ptr_store_imm_bytes(
    out: &mut Vec<u8>,
    ty: MmioScalarType,
    addr_slot_idx: u8,
    value: u64,
) {
    // 1. Load the u64 address from the addr_slot into %rax.
    encode_stack_cell_load_slot_bytes_into_rax(out, MmioScalarType::U64, addr_slot_idx);
    // 2. Load the immediate value into the value register (%cl/%cx/%ecx/%rcx).
    push_mov_imm_to_value_register(out, ty, value);
    // 3. Store %cl/%cx/%ecx/%rcx to [%rax].
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x08]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x08]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x08]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x08]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float RawPtrStore encoding requires SSE2 backend (Task 14)")
        }
    }
}

/// Encode `RawPtrStore` with a saved value (from %rbx): load addr (u64) from addr_slot
/// into %rax, then store %bl/%bx/%ebx/%rbx to [%rax].
fn encode_raw_ptr_store_saved_value_bytes(
    out: &mut Vec<u8>,
    ty: MmioScalarType,
    addr_slot_idx: u8,
) {
    // 1. Load the u64 address from the addr_slot into %rax.
    encode_stack_cell_load_slot_bytes_into_rax(out, MmioScalarType::U64, addr_slot_idx);
    // 2. Store %bl/%bx/%ebx/%rbx to [%rax].
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0x18]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0x18]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0x18]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0x18]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float RawPtrStore encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn encode_call_capture_bytes(out: &mut Vec<u8>, ty: MmioScalarType) {
    out.push(0xE8);
    out.extend_from_slice(&[0, 0, 0, 0]);
    push_mov_accumulator_to_saved_value_register(out, ty);
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
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
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
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn push_mov_accumulator_to_saved_value_register(out: &mut Vec<u8>, ty: MmioScalarType) {
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0xC3]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0xC3]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0xC3]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0xC3]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn push_mov_saved_value_to_accumulator_register(out: &mut Vec<u8>, ty: MmioScalarType) {
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x88, 0xD8]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x89, 0xD8]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x89, 0xD8]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x89, 0xD8]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn push_test_saved_value_register_zero(out: &mut Vec<u8>, ty: MmioScalarType) {
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x84, 0xDB]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x85, 0xDB]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x85, 0xDB]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x85, 0xDB]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn push_cmp_accumulator_to_saved_value_register(out: &mut Vec<u8>, ty: MmioScalarType) {
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x38, 0xC3]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x39, 0xC3]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x39, 0xC3]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x39, 0xC3]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn push_test_accumulator_with_saved_value_register(out: &mut Vec<u8>, ty: MmioScalarType) {
    match ty {
        MmioScalarType::U8 => out.extend_from_slice(&[0x84, 0xC3]),
        MmioScalarType::U16 => out.extend_from_slice(&[0x66, 0x85, 0xC3]),
        MmioScalarType::U32 => out.extend_from_slice(&[0x85, 0xC3]),
        MmioScalarType::U64 => out.extend_from_slice(&[0x48, 0x85, 0xC3]),
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_load_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 | MmioScalarType::U32 => 2,
        MmioScalarType::U16 | MmioScalarType::U64 => 3,
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_value_load_immediate_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 => 2,
        MmioScalarType::U16 => 4,
        MmioScalarType::U32 => 5,
        MmioScalarType::U64 => 10,
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_saved_value_copy_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 | MmioScalarType::U32 => 2,
        MmioScalarType::U16 | MmioScalarType::U64 => 3,
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_saved_value_zero_test_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 | MmioScalarType::U32 => 2,
        MmioScalarType::U16 | MmioScalarType::U64 => 3,
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_accumulator_immediate_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 => 2,
        MmioScalarType::U16 => 4,
        MmioScalarType::U32 => 5,
        MmioScalarType::U64 => 10,
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_saved_value_compare_bytes(ty: MmioScalarType) -> u64 {
    match ty {
        MmioScalarType::U8 | MmioScalarType::U32 => 2,
        MmioScalarType::U16 | MmioScalarType::U64 => 3,
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
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
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_saved_value_store_bytes(ty: MmioScalarType) -> u64 {
    mmio_store_bytes(ty)
}

/// Byte count of a stack-cell load or store at the given slot index.
/// slot_idx=0 uses the no-displacement SIB form (3/4 bytes);
/// slot_idx>0 uses the disp8 SIB form (4/5 bytes).
fn stack_cell_access_bytes(ty: MmioScalarType, slot_idx: u8) -> u64 {
    let base: u64 = match ty {
        MmioScalarType::U8 | MmioScalarType::U32 => 3,
        MmioScalarType::U16 | MmioScalarType::U64 => 4,
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    };
    base + if slot_idx > 0 { 1 } else { 0 }
}

/// Byte count of the arithmetic instruction on %rbx for `SlotArithImm`.
/// add/sub/and/or/xor: 4 bytes if imm ≤ 127 (imm8), 7 bytes otherwise (imm32).
/// shl/shr: 3 bytes if count == 1, 4 bytes otherwise.
fn slot_arith_imm_op_bytes(op: ArithOp, imm: u64) -> u64 {
    match op {
        ArithOp::Shl | ArithOp::Shr => {
            if imm == 1 {
                3
            } else {
                4
            }
        }
        _ => {
            if imm <= 127 {
                4
            } else {
                7
            }
        }
    }
}

/// AT&T mnemonic for a typed `op r/m, reg` instruction used in `SlotArithSlot`.
fn slot_arith_slot_op_mnemonic(ty: MmioScalarType, op: ArithOp) -> &'static str {
    match (ty, op) {
        (MmioScalarType::U8, ArithOp::Add) => "addb",
        (MmioScalarType::U8, ArithOp::Sub) => "subb",
        (MmioScalarType::U8, ArithOp::And) => "andb",
        (MmioScalarType::U8, ArithOp::Or) => "orb",
        (MmioScalarType::U8, ArithOp::Xor) => "xorb",
        (MmioScalarType::U8, ArithOp::Shl) => "shlb",
        (MmioScalarType::U8, ArithOp::Shr) => "shrb",
        (MmioScalarType::U16, ArithOp::Add) => "addw",
        (MmioScalarType::U16, ArithOp::Sub) => "subw",
        (MmioScalarType::U16, ArithOp::And) => "andw",
        (MmioScalarType::U16, ArithOp::Or) => "orw",
        (MmioScalarType::U16, ArithOp::Xor) => "xorw",
        (MmioScalarType::U16, ArithOp::Shl) => "shlw",
        (MmioScalarType::U16, ArithOp::Shr) => "shrw",
        (MmioScalarType::U32, ArithOp::Add) => "addl",
        (MmioScalarType::U32, ArithOp::Sub) => "subl",
        (MmioScalarType::U32, ArithOp::And) => "andl",
        (MmioScalarType::U32, ArithOp::Or) => "orl",
        (MmioScalarType::U32, ArithOp::Xor) => "xorl",
        (MmioScalarType::U32, ArithOp::Shl) => "shll",
        (MmioScalarType::U32, ArithOp::Shr) => "shrl",
        (MmioScalarType::U64, ArithOp::Add) => "addq",
        (MmioScalarType::U64, ArithOp::Sub) => "subq",
        (MmioScalarType::U64, ArithOp::And) => "andq",
        (MmioScalarType::U64, ArithOp::Or) => "orq",
        (MmioScalarType::U64, ArithOp::Xor) => "xorq",
        (MmioScalarType::U64, ArithOp::Shl) => "shlq",
        (MmioScalarType::U64, ArithOp::Shr) => "shrq",
        (MmioScalarType::F32 | MmioScalarType::F64, _) => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_accumulator_register(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "%al",
        MmioScalarType::U16 => "%ax",
        MmioScalarType::U32 => "%eax",
        MmioScalarType::U64 => "%rax",
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_value_register(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "%cl",
        MmioScalarType::U16 => "%cx",
        MmioScalarType::U32 => "%ecx",
        MmioScalarType::U64 => "%rcx",
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn asm_param_register(param_idx: u8) -> &'static str {
    match param_idx {
        0 => "%rdi",
        1 => "%rsi",
        2 => "%rdx",
        3 => "%rcx",
        4 => "%r8",
        5 => "%r9",
        _ => panic!("SysV ABI supports at most 6 integer register params"),
    }
}

fn mmio_saved_value_register(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "%bl",
        MmioScalarType::U16 => "%bx",
        MmioScalarType::U32 => "%ebx",
        MmioScalarType::U64 => "%rbx",
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_load_mnemonic(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "movb",
        MmioScalarType::U16 => "movw",
        MmioScalarType::U32 => "movl",
        MmioScalarType::U64 => "movq",
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_store_mnemonic(ty: MmioScalarType) -> &'static str {
    mmio_load_mnemonic(ty)
}

fn mmio_move_saved_value_mnemonic(ty: MmioScalarType) -> &'static str {
    mmio_load_mnemonic(ty)
}

fn mmio_move_return_value_mnemonic(ty: MmioScalarType) -> &'static str {
    mmio_load_mnemonic(ty)
}

fn mmio_saved_value_zero_test_mnemonic(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "testb %bl, %bl",
        MmioScalarType::U16 => "testw %bx, %bx",
        MmioScalarType::U32 => "testl %ebx, %ebx",
        MmioScalarType::U64 => "testq %rbx, %rbx",
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_saved_value_compare_mnemonic(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "cmpb %al, %bl",
        MmioScalarType::U16 => "cmpw %ax, %bx",
        MmioScalarType::U32 => "cmpl %eax, %ebx",
        MmioScalarType::U64 => "cmpq %rax, %rbx",
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_saved_value_mask_test_mnemonic(ty: MmioScalarType) -> &'static str {
    match ty {
        MmioScalarType::U8 => "testb %al, %bl",
        MmioScalarType::U16 => "testw %ax, %bx",
        MmioScalarType::U32 => "testl %eax, %ebx",
        MmioScalarType::U64 => "testq %rax, %rbx",
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
    }
}

fn mmio_accumulator_immediate_mnemonic(ty: MmioScalarType, value: u64) -> String {
    let mnemonic = match ty {
        MmioScalarType::U8 => "movb",
        MmioScalarType::U16 => "movw",
        MmioScalarType::U32 => "movl",
        MmioScalarType::U64 => "movabs",
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
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
        MmioScalarType::F32 | MmioScalarType::F64 => {
            unreachable!("float MMIO encoding requires SSE2 backend (Task 14)")
        }
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
        let pad = if function.facts.attrs.hotpath && !cursor.is_multiple_of(16) {
            16 - (cursor % 16)
        } else {
            0
        };
        cursor += pad;
        let block = &function.blocks[0];
        let n_params = function.signature.params.len() as u64;
        let uses_frame = executable_function_uses_frame(function);
        let size = (if executable_function_uses_saved_value_slot(function) {
            1
        } else {
            0
        }) + if uses_frame {
            // SUB RSP, frame_size + one 5-byte MOV per param spill
            rsp_adj_encoded_len(executable_function_frame_size(function)) + 5 * n_params
        } else {
            0
        } + block.ops.iter().map(executable_op_encoded_len).sum::<u64>()
            + executable_terminator_encoded_len(function);
        function_offsets.insert(function.name.clone(), cursor);
        function_sizes.insert(function.name.clone(), size);
        cursor += size;
    }

    // Second pass setup: compute where each string will land (after all function code).
    let strings_base = cursor;
    let mut string_offsets: Vec<u64> = Vec::with_capacity(canonical.static_strings.len());
    {
        let mut str_cursor = strings_base;
        for s in &canonical.static_strings {
            string_offsets.push(str_cursor);
            str_cursor += s.len() as u64 + 1; // +1 for NUL terminator
        }
    }

    let mut code_bytes = Vec::with_capacity(cursor as usize);
    let mut symbols = Vec::with_capacity(canonical.functions.len());
    let mut fixups = Vec::new();
    let mut unresolved_targets = BTreeSet::new();
    let mut emit_cursor: u64 = 0;
    for function in &canonical.functions {
        let function_offset = *function_offsets
            .get(&function.name)
            .expect("function offset must exist");
        let function_size = *function_sizes
            .get(&function.name)
            .expect("function size must exist");
        let pad = function_offset - emit_cursor;
        // NOP alignment padding for @hotpath
        code_bytes.extend(std::iter::repeat_n(0x90_u8, pad as usize));
        emit_cursor = function_offset;
        let block = &function.blocks[0];
        let uses_saved_value_slot = executable_function_uses_saved_value_slot(function);
        let n_stack_cells = executable_function_n_stack_cells(function);
        let n_params = function.signature.params.len();
        let uses_frame = n_stack_cells > 0 || n_params > 0;
        let mut local_offset = 0u64;
        if uses_saved_value_slot {
            code_bytes.push(0x53);
            local_offset += 1;
        }
        if uses_frame {
            // Frame layout: [cells: 8*n_stack_cells bytes][param_0..param_n-1: 8 bytes each]
            // Cell i is at (8*i)(%rsp); param j is at (8*n_stack_cells + 8*j)(%rsp)
            let sc_bytes = 8u32 * u32::from(n_stack_cells);
            let frame_size = sc_bytes + 8u32 * n_params as u32;
            emit_sub_rsp(&mut code_bytes, frame_size);
            local_offset += rsp_adj_encoded_len(frame_size);
            // Spill SysV integer param registers to their stack slots
            for i in 0..n_params {
                let offset = (sc_bytes + 8u32 * i as u32) as u8;
                emit_param_spill(&mut code_bytes, i, offset);
                local_offset += 5;
            }
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
                ExecutableOp::CallCapture { callee, ty } => {
                    if !function_offsets.contains_key(callee) {
                        if !extern_names.contains(callee.as_str()) {
                            return Err(format!(
                                "compiler-owned object emission requires declared extern target '{}' in function '{}'",
                                callee, function.name
                            ));
                        }
                        unresolved_targets.insert(callee.clone());
                    }
                    encode_call_capture_bytes(&mut code_bytes, *ty);
                    fixups.push(CompilerOwnedObjectFixup {
                        source_symbol: function.name.clone(),
                        patch_offset: function_offset + local_offset + 1,
                        kind: CompilerOwnedFixupKind::X86_64CallRel32,
                        target_symbol: callee.clone(),
                        width_bytes: 4,
                    });
                    local_offset += executable_op_encoded_len(op);
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
                ExecutableOp::StackStoreImm {
                    ty,
                    value,
                    slot_idx,
                } => {
                    encode_stack_cell_store_imm_slot_bytes(&mut code_bytes, *ty, *value, *slot_idx);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::StackStoreValue { ty, slot_idx } => {
                    encode_stack_cell_store_saved_value_slot_bytes(&mut code_bytes, *ty, *slot_idx);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::StackLoad { ty, slot_idx } => {
                    encode_stack_cell_load_slot_bytes(&mut code_bytes, *ty, *slot_idx);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::SlotArithImm {
                    ty,
                    slot_idx,
                    arith_op,
                    imm,
                } => {
                    encode_stack_cell_load_slot_bytes(&mut code_bytes, *ty, *slot_idx);
                    encode_slot_arith_imm_bytes(&mut code_bytes, *arith_op, *imm);
                    encode_stack_cell_store_saved_value_slot_bytes(&mut code_bytes, *ty, *slot_idx);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::SlotArithSlot {
                    ty,
                    dst_slot_idx,
                    src_slot_idx,
                    arith_op,
                } => {
                    match arith_op {
                        ArithOp::Shl | ArithOp::Shr => {
                            encode_stack_cell_load_slot_bytes_into_rcx(
                                &mut code_bytes,
                                *ty,
                                *src_slot_idx,
                            );
                        }
                        _ => {
                            encode_stack_cell_load_slot_bytes_into_rax(
                                &mut code_bytes,
                                *ty,
                                *src_slot_idx,
                            );
                        }
                    }
                    encode_slot_arith_slot_op_bytes(&mut code_bytes, *ty, *arith_op, *dst_slot_idx);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::ParamLoad { param_idx, ty } => {
                    let offset =
                        (8u32 * u32::from(n_stack_cells) + 8u32 * u32::from(*param_idx)) as u8;
                    encode_param_load_bytes(&mut code_bytes, *ty, offset);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::MmioReadParamAddr {
                    param_idx,
                    ty,
                    capture_value,
                } => {
                    let offset =
                        (8u32 * u32::from(n_stack_cells) + 8u32 * u32::from(*param_idx)) as u8;
                    encode_mmio_read_param_addr_bytes(&mut code_bytes, *ty, offset, *capture_value);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::MmioWriteImmParamAddr {
                    param_idx,
                    ty,
                    value,
                } => {
                    let offset =
                        (8u32 * u32::from(n_stack_cells) + 8u32 * u32::from(*param_idx)) as u8;
                    encode_mmio_write_imm_param_addr_bytes(&mut code_bytes, *ty, offset, *value);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::MmioWriteValueParamAddr { param_idx, ty } => {
                    let offset =
                        (8u32 * u32::from(n_stack_cells) + 8u32 * u32::from(*param_idx)) as u8;
                    encode_mmio_write_saved_value_param_addr_bytes(&mut code_bytes, *ty, offset);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::CallWithArgs { callee, args } => {
                    if !function_offsets.contains_key(callee) {
                        if !extern_names.contains(callee.as_str()) {
                            return Err(format!(
                                "compiler-owned object emission requires declared extern target '{}' in function '{}'",
                                callee, function.name
                            ));
                        }
                        unresolved_targets.insert(callee.clone());
                    }
                    // Emit per-arg movq instructions then a call rel32.
                    for (i, arg) in args.iter().enumerate() {
                        encode_call_arg_bytes(&mut code_bytes, i as u8, arg);
                    }
                    code_bytes.push(0xE8);
                    code_bytes.extend_from_slice(&[0, 0, 0, 0]);
                    // Displacement field is after the per-arg bytes and the E8 opcode.
                    let args_len: u64 = args.iter().map(call_arg_encoded_bytes).sum();
                    fixups.push(CompilerOwnedObjectFixup {
                        source_symbol: function.name.clone(),
                        patch_offset: function_offset + local_offset + args_len + 1,
                        kind: CompilerOwnedFixupKind::X86_64CallRel32,
                        target_symbol: callee.clone(),
                        width_bytes: 4,
                    });
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::CallCaptureWithArgs { callee, args, ty, slot_idx } => {
                    if !function_offsets.contains_key(callee) {
                        if !extern_names.contains(callee.as_str()) {
                            return Err(format!(
                                "compiler-owned object emission requires declared extern target '{}' in function '{}'",
                                callee, function.name
                            ));
                        }
                        unresolved_targets.insert(callee.clone());
                    }
                    // Emit per-arg instructions.
                    for (i, arg) in args.iter().enumerate() {
                        encode_call_arg_bytes(&mut code_bytes, i as u8, arg);
                    }
                    let args_len: u64 = args.iter().map(call_arg_encoded_bytes).sum();
                    // Emit call rel32.
                    code_bytes.push(0xE8);
                    code_bytes.extend_from_slice(&[0, 0, 0, 0]);
                    fixups.push(CompilerOwnedObjectFixup {
                        source_symbol: function.name.clone(),
                        patch_offset: function_offset + local_offset + args_len + 1,
                        kind: CompilerOwnedFixupKind::X86_64CallRel32,
                        target_symbol: callee.clone(),
                        width_bytes: 4,
                    });
                    // Store accumulator to capture slot.
                    encode_stack_cell_store_accumulator_slot_bytes(&mut code_bytes, *ty, *slot_idx);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::RawPtrLoad {
                    ty,
                    addr_slot_idx,
                    out_slot_idx,
                } => {
                    encode_raw_ptr_load_bytes(&mut code_bytes, *ty, *addr_slot_idx, *out_slot_idx);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::RawPtrStore {
                    ty,
                    addr_slot_idx,
                    value,
                } => match value {
                    MmioValueExpr::IntLiteral { value: raw } => {
                        match parse_integer_literal_u64(raw) {
                            Ok(v) => {
                                encode_raw_ptr_store_imm_bytes(
                                    &mut code_bytes,
                                    *ty,
                                    *addr_slot_idx,
                                    v,
                                );
                            }
                            Err(e) => {
                                return Err(format!(
                                    "compiler-owned object emission: raw_ptr_store integer literal '{}' in function '{}': {}",
                                    raw, function.name, e
                                ));
                            }
                        }
                        local_offset += executable_op_encoded_len(op);
                    }
                    MmioValueExpr::Ident { .. } => {
                        encode_raw_ptr_store_saved_value_bytes(
                            &mut code_bytes,
                            *ty,
                            *addr_slot_idx,
                        );
                        local_offset += executable_op_encoded_len(op);
                    }
                    MmioValueExpr::FloatLiteral { .. } => {
                        return Err(format!(
                            "compiler-owned object emission: float RawPtrStore not supported in function '{}'",
                            function.name
                        ));
                    }
                },
                ExecutableOp::LoadStaticCstrAddr { str_idx, slot_idx } => {
                    let idx = *str_idx as usize;
                    if idx >= string_offsets.len() {
                        return Err(format!(
                            "compiler-owned object emission: function '{}' LoadStaticCstrAddr str_idx {} out of range",
                            function.name, str_idx
                        ));
                    }
                    let str_abs = string_offsets[idx];
                    // lea is at (function_offset + local_offset); next IP is +7 (REX.W + opcode + ModRM + rel32)
                    let lea_next_ip = function_offset + local_offset + 7;
                    let disp = str_abs as i64 - lea_next_ip as i64;
                    let disp32 = i32::try_from(disp).map_err(|_| {
                        format!(
                            "compiler-owned object emission: function '{}' string literal at index {} out of rel32 range",
                            function.name, str_idx
                        )
                    })?;
                    // lea [rip+disp32], %rbx  →  48 8D 1D [disp32 LE]
                    code_bytes.extend_from_slice(&[0x48, 0x8D, 0x1D]);
                    code_bytes.extend_from_slice(&disp32.to_le_bytes());
                    // Store %rbx (saved-value register) into the stack slot.
                    encode_stack_cell_store_saved_value_slot_bytes(&mut code_bytes, MmioScalarType::U64, *slot_idx);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::InlineAsm(intr) => {
                    let bytes: &[u8] = match intr {
                        KernelIntrinsic::Cli => &[0xFA],
                        KernelIntrinsic::Sti => &[0xFB],
                        KernelIntrinsic::Hlt => &[0xF4],
                        KernelIntrinsic::Nop => &[0x90],
                        KernelIntrinsic::Mfence => &[0x0F, 0xAE, 0xF0],
                        KernelIntrinsic::Sfence => &[0x0F, 0xAE, 0xF8],
                        KernelIntrinsic::Lfence => &[0x0F, 0xAE, 0xE8],
                        KernelIntrinsic::Wbinvd => &[0x0F, 0x09],
                        KernelIntrinsic::Pause => &[0xF3, 0x90],
                        KernelIntrinsic::Int3 => &[0xCC],
                        KernelIntrinsic::Cpuid => &[0x0F, 0xA2],
                    };
                    code_bytes.extend_from_slice(bytes);
                    local_offset += executable_op_encoded_len(op);
                }
                ExecutableOp::LoopBegin
                | ExecutableOp::LoopEnd
                | ExecutableOp::LoopBreak
                | ExecutableOp::LoopContinue
                | ExecutableOp::BranchIfZeroLoopBreak { .. }
                | ExecutableOp::BranchIfNonZeroLoopBreak { .. }
                | ExecutableOp::CompareIntoSlot { .. } => {
                    return Err(format!(
                        "compiler-owned object emission: loop/compare ops require Task 11 intra-function relocation (function '{}')",
                        function.name
                    ));
                }
            }
        }
        match &function.blocks[0].terminator {
            ExecutableTerminator::Return {
                value: ExecutableValue::SavedValue { ty },
            } => {
                push_mov_saved_value_to_accumulator_register(&mut code_bytes, *ty);
                if uses_frame {
                    emit_add_rsp(
                        &mut code_bytes,
                        8u32 * u32::from(n_stack_cells) + 8u32 * n_params as u32,
                    );
                }
                if uses_saved_value_slot {
                    code_bytes.push(0x5B);
                }
                code_bytes.push(0xC3);
            }
            ExecutableTerminator::Return {
                value: ExecutableValue::Unit,
            } => {
                if uses_frame {
                    emit_add_rsp(
                        &mut code_bytes,
                        8u32 * u32::from(n_stack_cells) + 8u32 * n_params as u32,
                    );
                }
                if uses_saved_value_slot {
                    code_bytes.push(0x5B);
                }
                code_bytes.push(0xC3);
            }
            ExecutableTerminator::TailCall { callee, args } => {
                if !function_offsets.contains_key(callee) {
                    if !extern_names.contains(callee.as_str()) {
                        return Err(format!(
                            "compiler-owned object emission requires declared extern target '{}' in function '{}'",
                            callee, function.name
                        ));
                    }
                    unresolved_targets.insert(callee.clone());
                }
                // Load args into SysV registers BEFORE frame teardown (stack slots still valid).
                for (i, arg) in args.iter().enumerate() {
                    encode_call_arg_bytes(&mut code_bytes, i as u8, arg);
                }
                let frame_size = 8u32 * u32::from(n_stack_cells) + 8u32 * n_params as u32;
                let frame_bytes: u64 = if uses_frame {
                    rsp_adj_encoded_len(frame_size)
                } else {
                    0
                };
                let pop_bytes: u64 = if uses_saved_value_slot { 1 } else { 0 };
                if uses_frame {
                    emit_add_rsp(&mut code_bytes, frame_size);
                }
                if uses_saved_value_slot {
                    code_bytes.push(0x5B);
                }
                let args_bytes: u64 = args.iter().map(call_arg_encoded_bytes).sum();
                // jmp rel32 = 0xE9 + 4-byte displacement
                code_bytes.push(0xE9);
                code_bytes.extend_from_slice(&[0, 0, 0, 0]);
                fixups.push(CompilerOwnedObjectFixup {
                    source_symbol: function.name.clone(),
                    patch_offset: function_offset
                        + local_offset
                        + args_bytes
                        + frame_bytes
                        + pop_bytes
                        + 1,
                    kind: CompilerOwnedFixupKind::X86_64CallRel32,
                    target_symbol: callee.clone(),
                    width_bytes: 4,
                });
            }
        }
        // size covers instruction bytes only; NOP alignment bytes preceding this
        // symbol are intentional padding and are not attributed to any symbol.
        symbols.push(CompilerOwnedObjectSymbol {
            name: function.name.clone(),
            kind: CompilerOwnedObjectSymbolKind::Function,
            definition: CompilerOwnedObjectSymbolDefinition::DefinedText,
            offset: function_offset,
            size: function_size,
        });
        emit_cursor += function_size;
    }

    // Append C-string data after all function code.
    for s in &canonical.static_strings {
        code_bytes.extend_from_slice(s.as_bytes());
        code_bytes.push(0); // NUL terminator
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64MachORelocatableObject {
    pub text_bytes: Vec<u8>,
    pub function_symbols: Vec<X86_64MachOFunctionSymbol>,
    pub undefined_function_symbols: Vec<String>,
    pub relocations: Vec<X86_64MachORelocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64MachOFunctionSymbol {
    pub name: String,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64MachORelocation {
    pub offset: u32,
    pub target_symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64CoffRelocatableObject {
    pub text_bytes: Vec<u8>,
    pub function_symbols: Vec<X86_64CoffFunctionSymbol>,
    pub undefined_function_symbols: Vec<String>,
    pub relocations: Vec<X86_64CoffRelocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64CoffFunctionSymbol {
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64CoffRelocation {
    pub section_offset: u32,
    pub target_symbol: String,
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

pub fn validate_compiler_owned_object_for_x86_64_macho_export(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<(), String> {
    target.validate()?;
    if target.target_id != BackendTargetId::X86_64MachO {
        return Err("x86_64 Mach-O export requires x86_64-macho target contract".to_string());
    }
    object.validate()?;
    if object.header.target_id != target.target_id {
        return Err(
            "x86_64 Mach-O export: object target_id must match target contract".to_string(),
        );
    }
    if object.code.name != target.sections.text {
        return Err(
            "x86_64 Mach-O export: object code section must match target text section".to_string(),
        );
    }
    Ok(())
}

pub fn export_compiler_owned_object_to_x86_64_macho(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<X86_64MachORelocatableObject, String> {
    validate_compiler_owned_object_for_x86_64_macho_export(object, target)?;

    let symbol_offsets = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|s| (s.name.as_str(), s.offset))
        .collect::<BTreeMap<_, _>>();
    let symbol_defs = object
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.definition))
        .collect::<BTreeMap<_, _>>();

    let mut text_bytes = object.code.bytes.clone();
    let mut relocations = Vec::new();

    for fixup in &object.fixups {
        let Some(target_def) = symbol_defs.get(fixup.target_symbol.as_str()) else {
            return Err(format!(
                "x86_64 Mach-O export: missing target symbol '{}' for fixup",
                fixup.target_symbol
            ));
        };
        match (fixup.kind, target_def) {
            (
                CompilerOwnedFixupKind::X86_64CallRel32,
                CompilerOwnedObjectSymbolDefinition::DefinedText,
            ) => {
                let target_offset = *symbol_offsets.get(fixup.target_symbol.as_str()).unwrap();
                let next_ip = fixup.patch_offset + u64::from(fixup.width_bytes);
                let displacement = (target_offset as i64) - (next_ip as i64);
                let rel32 = i32::try_from(displacement).map_err(|_| {
                    format!(
                        "x86_64 Mach-O export: call displacement to '{}' does not fit rel32",
                        fixup.target_symbol
                    )
                })?;
                let patch = usize::try_from(fixup.patch_offset).expect("patch offset fits usize");
                text_bytes[patch..patch + 4].copy_from_slice(&rel32.to_le_bytes());
            }
            (
                CompilerOwnedFixupKind::X86_64CallRel32,
                CompilerOwnedObjectSymbolDefinition::UndefinedExternal,
            ) => {
                relocations.push(X86_64MachORelocation {
                    offset: u32::try_from(fixup.patch_offset).expect("patch offset fits u32"),
                    target_symbol: fixup.target_symbol.clone(),
                });
            }
        }
    }

    let function_symbols = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|s| X86_64MachOFunctionSymbol {
            name: s.name.clone(),
            offset: s.offset,
            size: s.size,
        })
        .collect::<Vec<_>>();
    let undefined_function_symbols = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::UndefinedExternal)
        .map(|s| s.name.clone())
        .collect::<Vec<_>>();

    Ok(X86_64MachORelocatableObject {
        text_bytes,
        function_symbols,
        undefined_function_symbols,
        relocations,
    })
}

pub fn lower_executable_krir_to_x86_64_macho_object(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<X86_64MachORelocatableObject, String> {
    validate_x86_64_object_linear_subset(module, target)?;
    let object = lower_executable_krir_to_compiler_owned_object(module, target)?;
    export_compiler_owned_object_to_x86_64_macho(&object, target)
}

pub fn validate_compiler_owned_object_for_x86_64_coff_export(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<(), String> {
    target.validate()?;
    if target.target_id != BackendTargetId::X86_64Win64 {
        return Err("x86_64 COFF export requires x86_64-win64 target contract".to_string());
    }
    object.validate()?;
    if object.header.target_id != target.target_id {
        return Err("x86_64 COFF export: object target_id must match target contract".to_string());
    }
    if object.code.name != target.sections.text {
        return Err(
            "x86_64 COFF export: object code section must match target text section".to_string(),
        );
    }
    Ok(())
}

pub fn export_compiler_owned_object_to_x86_64_coff(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<X86_64CoffRelocatableObject, String> {
    validate_compiler_owned_object_for_x86_64_coff_export(object, target)?;

    let symbol_offsets = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|s| (s.name.as_str(), s.offset))
        .collect::<BTreeMap<_, _>>();
    let symbol_defs = object
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.definition))
        .collect::<BTreeMap<_, _>>();

    let mut text_bytes = object.code.bytes.clone();
    let mut relocations = Vec::new();

    for fixup in &object.fixups {
        let Some(target_def) = symbol_defs.get(fixup.target_symbol.as_str()) else {
            return Err(format!(
                "x86_64 COFF export: missing target symbol '{}' for fixup",
                fixup.target_symbol
            ));
        };
        match (fixup.kind, target_def) {
            (
                CompilerOwnedFixupKind::X86_64CallRel32,
                CompilerOwnedObjectSymbolDefinition::DefinedText,
            ) => {
                let target_offset = *symbol_offsets.get(fixup.target_symbol.as_str()).unwrap();
                let next_ip = fixup.patch_offset + u64::from(fixup.width_bytes);
                let displacement = (target_offset as i64) - (next_ip as i64);
                let rel32 = i32::try_from(displacement).map_err(|_| {
                    format!(
                        "x86_64 COFF export: call displacement to '{}' does not fit rel32",
                        fixup.target_symbol
                    )
                })?;
                let patch = usize::try_from(fixup.patch_offset).expect("patch offset fits usize");
                text_bytes[patch..patch + 4].copy_from_slice(&rel32.to_le_bytes());
            }
            (
                CompilerOwnedFixupKind::X86_64CallRel32,
                CompilerOwnedObjectSymbolDefinition::UndefinedExternal,
            ) => {
                relocations.push(X86_64CoffRelocation {
                    section_offset: u32::try_from(fixup.patch_offset)
                        .expect("patch offset fits u32"),
                    target_symbol: fixup.target_symbol.clone(),
                });
            }
        }
    }

    let function_symbols = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|s| X86_64CoffFunctionSymbol {
            name: s.name.clone(),
            offset: u32::try_from(s.offset).expect("symbol offset fits u32"),
            size: u32::try_from(s.size).expect("symbol size fits u32"),
        })
        .collect::<Vec<_>>();
    let undefined_function_symbols = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::UndefinedExternal)
        .map(|s| s.name.clone())
        .collect::<Vec<_>>();

    Ok(X86_64CoffRelocatableObject {
        text_bytes,
        function_symbols,
        undefined_function_symbols,
        relocations,
    })
}

pub fn lower_executable_krir_to_x86_64_coff_object(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<X86_64CoffRelocatableObject, String> {
    validate_x86_64_object_linear_subset(module, target)?;
    let object = lower_executable_krir_to_compiler_owned_object(module, target)?;
    export_compiler_owned_object_to_x86_64_coff(&object, target)
}

pub fn emit_x86_64_macho_object_bytes(object: &X86_64MachORelocatableObject) -> Vec<u8> {
    // Build string table: leading null + "_name\0" for every symbol
    let mut strtab = vec![0u8];
    let mut name_offsets: BTreeMap<String, u32> = BTreeMap::new();
    for sym in &object.function_symbols {
        let offset = strtab.len() as u32;
        let prefixed = format!("_{}", sym.name);
        strtab.extend_from_slice(prefixed.as_bytes());
        strtab.push(0);
        name_offsets.insert(sym.name.clone(), offset);
    }
    for sym in &object.undefined_function_symbols {
        let offset = strtab.len() as u32;
        let prefixed = format!("_{}", sym);
        strtab.extend_from_slice(prefixed.as_bytes());
        strtab.push(0);
        name_offsets.insert(sym.clone(), offset);
    }

    // nlist_64 entries (16 bytes each): n_strx(4), n_type(1), n_sect(1), n_desc(2), n_value(8)
    let mut symtab: Vec<u8> = Vec::new();
    for sym in &object.function_symbols {
        let strx = *name_offsets.get(&sym.name).expect("symbol name in strtab");
        push_u32_le(&mut symtab, strx);
        symtab.push(0x0F); // N_SECT | N_EXT
        symtab.push(1); // section ordinal 1 = __text
        push_u16_le(&mut symtab, 0);
        push_u64_le(&mut symtab, sym.offset);
    }
    for sym in &object.undefined_function_symbols {
        let strx = *name_offsets
            .get(sym.as_str())
            .expect("undef symbol in strtab");
        push_u32_le(&mut symtab, strx);
        symtab.push(0x01); // N_EXT | N_UNDF
        symtab.push(0); // NO_SECT
        push_u16_le(&mut symtab, 0);
        push_u64_le(&mut symtab, 0);
    }
    let nsyms = (object.function_symbols.len() + object.undefined_function_symbols.len()) as u32;

    // Symbol index map for relocations (defined first, then undefined)
    let mut sym_index: BTreeMap<String, u32> = BTreeMap::new();
    for (i, sym) in object.function_symbols.iter().enumerate() {
        sym_index.insert(sym.name.clone(), i as u32);
    }
    for (i, sym) in object.undefined_function_symbols.iter().enumerate() {
        sym_index.insert(sym.clone(), (object.function_symbols.len() + i) as u32);
    }

    // Relocation entries (8 bytes each): r_address(u32) + r_info(u32)
    let mut reloc_bytes: Vec<u8> = Vec::new();
    for reloc in &object.relocations {
        push_u32_le(&mut reloc_bytes, reloc.offset);
        let idx = *sym_index
            .get(&reloc.target_symbol)
            .expect("reloc target in sym_index");
        // X86_64_RELOC_BRANCH: type=2, extern=1, pcrel=1, length=2 → lower byte = 0xD2
        let r_info = (idx << 8) | 0xD2;
        push_u32_le(&mut reloc_bytes, r_info);
    }
    let nreloc = object.relocations.len() as u32;

    // File layout:
    // [0]   mach_header_64 (32)
    // [32]  LC_SEGMENT_64 + section_64 (72+80 = 152, cmdsize=152)
    // [184] LC_SYMTAB (24)
    // [208] text (padded to 4)
    // [208+P] relocations (8 each, or nothing)
    // [208+P+R] nlist_64 symbol table
    // [...] string table
    let text_offset: u32 = 208;
    let text_len = object.text_bytes.len() as u32;
    let text_padded = (text_len + 3) & !3u32;
    let reloc_offset: u32 = if nreloc == 0 {
        0
    } else {
        text_offset + text_padded
    };
    let sym_offset: u32 = text_offset + text_padded + reloc_bytes.len() as u32;
    let str_offset: u32 = sym_offset + symtab.len() as u32;
    let sizeofcmds: u32 = 152 + 24; // LC_SEGMENT_64+section + LC_SYMTAB

    let mut out: Vec<u8> = Vec::new();

    // mach_header_64 (32 bytes)
    push_u32_le(&mut out, 0xFEED_FACF); // MH_MAGIC_64
    push_u32_le(&mut out, 0x0100_0007); // CPU_TYPE_X86_64
    push_u32_le(&mut out, 0x0000_0003); // CPU_SUBTYPE_X86_64_ALL
    push_u32_le(&mut out, 0x0000_0001); // MH_OBJECT
    push_u32_le(&mut out, 2); // ncmds
    push_u32_le(&mut out, sizeofcmds);
    push_u32_le(&mut out, 0x0000_2000); // MH_SUBSECTIONS_VIA_SYMBOLS
    push_u32_le(&mut out, 0); // reserved

    // LC_SEGMENT_64 (cmd=0x19, cmdsize=152)
    push_u32_le(&mut out, 0x0000_0019); // LC_SEGMENT_64
    push_u32_le(&mut out, 152); // cmdsize = 72 + 80
    out.extend_from_slice(b"__TEXT\0\0\0\0\0\0\0\0\0\0"); // segname (16)
    push_u64_le(&mut out, 0); // vmaddr
    push_u64_le(&mut out, text_len as u64); // vmsize
    push_u64_le(&mut out, text_offset as u64); // fileoff
    push_u64_le(&mut out, text_len as u64); // filesize
    push_u32_le(&mut out, 7); // maxprot  (rwx)
    push_u32_le(&mut out, 5); // initprot (rx)
    push_u32_le(&mut out, 1); // nsects
    push_u32_le(&mut out, 0); // flags

    // section_64 for __text (80 bytes)
    out.extend_from_slice(b"__text\0\0\0\0\0\0\0\0\0\0"); // sectname (16)
    out.extend_from_slice(b"__TEXT\0\0\0\0\0\0\0\0\0\0"); // segname  (16)
    push_u64_le(&mut out, 0); // addr
    push_u64_le(&mut out, text_len as u64); // size
    push_u32_le(&mut out, text_offset); // offset in file
    push_u32_le(&mut out, 4); // align (log2 → 2^4 = 16)
    push_u32_le(&mut out, reloc_offset); // reloff (0 if no relocs)
    push_u32_le(&mut out, nreloc); // nreloc
    push_u32_le(&mut out, 0x8000_0400); // flags (pure_instructions | some_instructions)
    push_u32_le(&mut out, 0); // reserved1
    push_u32_le(&mut out, 0); // reserved2
    push_u32_le(&mut out, 0); // reserved3

    // LC_SYMTAB (24 bytes)
    push_u32_le(&mut out, 0x0000_0002); // LC_SYMTAB
    push_u32_le(&mut out, 24); // cmdsize
    push_u32_le(&mut out, sym_offset);
    push_u32_le(&mut out, nsyms);
    push_u32_le(&mut out, str_offset);
    push_u32_le(&mut out, strtab.len() as u32);

    // Text bytes (padded to 4-byte align)
    out.extend_from_slice(&object.text_bytes);
    while out.len() < (text_offset + text_padded) as usize {
        out.push(0);
    }

    // Relocation entries
    out.extend_from_slice(&reloc_bytes);

    // Symbol table (nlist_64)
    out.extend_from_slice(&symtab);

    // String table
    out.extend_from_slice(&strtab);

    out
}

fn coff_encode_name(
    name: &str,
    strtab: &mut Vec<u8>,
    offsets: &mut BTreeMap<String, u32>,
) -> [u8; 8] {
    if name.len() <= 8 {
        let mut buf = [0u8; 8];
        buf[..name.len()].copy_from_slice(name.as_bytes());
        buf
    } else {
        if !offsets.contains_key(name) {
            // offset is past the 4-byte size prefix
            let str_offset = 4 + strtab.len() as u32;
            offsets.insert(name.to_string(), str_offset);
            strtab.extend_from_slice(name.as_bytes());
            strtab.push(0);
        }
        let off = *offsets.get(name).unwrap();
        let mut buf = [0u8; 8];
        buf[4..8].copy_from_slice(&off.to_le_bytes());
        buf
    }
}

fn push_coff_sym(
    out: &mut Vec<u8>,
    name_buf: [u8; 8],
    value: u32,
    section: i16,
    ty: u16,
    class: u8,
) {
    out.extend_from_slice(&name_buf);
    push_u32_le(out, value);
    out.extend_from_slice(&section.to_le_bytes());
    push_u16_le(out, ty);
    out.push(class);
    out.push(0); // NumberOfAuxSymbols
}

pub fn emit_x86_64_coff_bytes(object: &X86_64CoffRelocatableObject) -> Vec<u8> {
    // String table (for symbol names > 8 chars)
    let mut strtab_strings: Vec<u8> = Vec::new();
    let mut name_strtab_offsets: BTreeMap<String, u32> = BTreeMap::new();

    // Symbol table:
    // Index 0: .text section symbol
    // Index 1..N: defined function symbols
    // Index N+1..: undefined function symbols
    let num_syms = 1 + object.function_symbols.len() + object.undefined_function_symbols.len();

    let mut sym_index: BTreeMap<String, u32> = BTreeMap::new();
    for (i, sym) in object.function_symbols.iter().enumerate() {
        sym_index.insert(sym.name.clone(), (1 + i) as u32);
    }
    for (i, sym) in object.undefined_function_symbols.iter().enumerate() {
        sym_index.insert(sym.clone(), (1 + object.function_symbols.len() + i) as u32);
    }

    // IMAGE_RELOCATION entries (10 bytes each)
    let mut relocs: Vec<u8> = Vec::new();
    for reloc in &object.relocations {
        push_u32_le(&mut relocs, reloc.section_offset);
        let idx = *sym_index
            .get(&reloc.target_symbol)
            .expect("reloc target in sym_index");
        push_u32_le(&mut relocs, idx);
        push_u16_le(&mut relocs, 0x0004); // IMAGE_REL_AMD64_REL32
    }
    let nrelocs = object.relocations.len() as u16;

    // File layout offsets
    let text_raw_offset: u32 = 20 + 40; // COFF header + section header = 60
    let text_len = object.text_bytes.len() as u32;
    let text_padded = (text_len + 3) & !3u32;
    let reloc_ptr: u32 = text_raw_offset + text_padded;
    let sym_table_ptr: u32 = reloc_ptr + relocs.len() as u32;

    let mut out: Vec<u8> = Vec::new();

    // IMAGE_FILE_HEADER (20 bytes)
    push_u16_le(&mut out, 0x8664); // Machine = AMD64
    push_u16_le(&mut out, 1); // NumberOfSections
    push_u32_le(&mut out, 0); // TimeDateStamp
    push_u32_le(&mut out, sym_table_ptr); // PointerToSymbolTable
    push_u32_le(&mut out, num_syms as u32); // NumberOfSymbols
    push_u16_le(&mut out, 0); // SizeOfOptionalHeader
    push_u16_le(&mut out, 0); // Characteristics

    // IMAGE_SECTION_HEADER for .text (40 bytes)
    out.extend_from_slice(b".text\0\0\0"); // Name (8 bytes)
    push_u32_le(&mut out, 0); // VirtualSize
    push_u32_le(&mut out, 0); // VirtualAddress
    push_u32_le(&mut out, text_padded); // SizeOfRawData
    push_u32_le(&mut out, text_raw_offset); // PointerToRawData
    push_u32_le(&mut out, if nrelocs == 0 { 0 } else { reloc_ptr }); // PointerToRelocations
    push_u32_le(&mut out, 0); // PointerToLinenumbers
    push_u16_le(&mut out, nrelocs); // NumberOfRelocations
    push_u16_le(&mut out, 0); // NumberOfLinenumbers
    push_u32_le(&mut out, 0x60500020); // Characteristics

    // Text bytes (padded to 4)
    out.extend_from_slice(&object.text_bytes);
    while out.len() < (text_raw_offset + text_padded) as usize {
        out.push(0);
    }

    // Relocation entries
    out.extend_from_slice(&relocs);

    // Symbol table
    // Section symbol (.text)
    let section_name_buf = {
        let mut b = [0u8; 8];
        b[..5].copy_from_slice(b".text");
        b
    };
    push_coff_sym(&mut out, section_name_buf, 0, 1, 0, 0x03); // StorageClass STATIC

    // Defined function symbols
    for sym in &object.function_symbols {
        let name_buf = coff_encode_name(&sym.name, &mut strtab_strings, &mut name_strtab_offsets);
        push_coff_sym(&mut out, name_buf, sym.offset, 1, 0x0020, 0x02); // EXTERNAL, function
    }

    // Undefined function symbols
    for sym in &object.undefined_function_symbols {
        let name_buf = coff_encode_name(sym, &mut strtab_strings, &mut name_strtab_offsets);
        push_coff_sym(&mut out, name_buf, 0, 0, 0x0020, 0x02); // IMAGE_SYM_UNDEFINED, EXTERNAL
    }

    // String table: 4-byte total size (including itself) + strings
    let strtab_total_size = (4 + strtab_strings.len()) as u32;
    push_u32_le(&mut out, strtab_total_size);
    out.extend_from_slice(&strtab_strings);

    out
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
        BackendTargetId::X86_64Win64 => 2,
        BackendTargetId::X86_64MachO => 3,
        BackendTargetId::Aarch64Sysv => 4,
        BackendTargetId::Aarch64MachO => 5,
        BackendTargetId::Aarch64Win => 6,
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

// ---------------------------------------------------------------------------
// KRBO container format
// ---------------------------------------------------------------------------

pub const KRBO_MAGIC: [u8; 4] = *b"KRBO";
pub const KRBO_VERSION: u8 = 1;
pub const KRBO_ARCH_X86_64: u8 = 0x01;
pub const KRBO_ARCH_AARCH64: u8 = 0x02;

pub const KRBO_FAT_MAGIC: [u8; 8] = *b"KRBOFAT\0";
pub const KRBO_FAT_VERSION: u32 = 1;
pub const KRBO_FAT_ARCH_X86_64: u32 = 0x01;
pub const KRBO_FAT_ARCH_AARCH64: u32 = 0x02;
pub const KRBO_FAT_COMPRESSION_NONE: u32 = 0;
pub const KRBO_FAT_COMPRESSION_LZ4: u32 = 1;

/// Parsed representation of a `.krbo` file header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KrboHeader {
    pub version: u8,
    pub arch: u8,
    pub entry_offset: u32,
    pub code_length: u32,
}

/// Emit a `.krbo` binary blob from raw code bytes, an entry offset, and an arch byte.
///
/// Layout: 16-byte header followed by `code.len()` bytes of machine code.
fn emit_krbo_bytes_raw_arch(code: &[u8], entry_offset: u32, arch: u8) -> Vec<u8> {
    let code_length = code.len() as u32;
    let mut out = Vec::with_capacity(16 + code.len());
    out.extend_from_slice(&KRBO_MAGIC);
    out.push(KRBO_VERSION);
    out.push(arch);
    out.extend_from_slice(&[0u8, 0u8]); // reserved
    out.extend_from_slice(&entry_offset.to_le_bytes());
    out.extend_from_slice(&code_length.to_le_bytes());
    out.extend_from_slice(code);
    out
}

/// Emit an x86-64 `.krbo` binary blob from raw code bytes and an entry offset.
pub fn emit_krbo_bytes_raw(code: &[u8], entry_offset: u32) -> Vec<u8> {
    emit_krbo_bytes_raw_arch(code, entry_offset, KRBO_ARCH_X86_64)
}

/// Emit a `.krbo` binary blob from an `X86_64ElfRelocatableObject`.
pub fn emit_krbo_bytes(object: &X86_64ElfRelocatableObject, entry_offset: u32) -> Vec<u8> {
    emit_krbo_bytes_raw(&object.text_bytes, entry_offset)
}

/// Parse and validate a `.krbo` header from raw bytes.
///
/// Returns `Ok(KrboHeader)` when the header is valid, or `Err(message)` for
/// any structural or semantic violation.
pub fn parse_krbo_header(bytes: &[u8]) -> Result<KrboHeader, String> {
    // Fat-first: fat magic starts with "KRBO", so must check 8 bytes before 4.
    if bytes.len() >= 8 && bytes[0..8] == KRBO_FAT_MAGIC {
        return Err(
            "this is a KRBOFAT fat binary; use parse_krbofat_slice to extract a single-arch slice"
                .to_string(),
        );
    }
    if bytes.len() < 16 {
        return Err("not a .krbo file: too short".to_string());
    }
    if bytes[0..4] != KRBO_MAGIC {
        return Err("not a .krbo file".to_string());
    }
    let version = bytes[4];
    if version != KRBO_VERSION {
        return Err(format!(
            "unsupported .krbo version {} (expected {})",
            version, KRBO_VERSION
        ));
    }
    let arch = bytes[5];
    #[cfg(target_arch = "aarch64")]
    let host_arch = KRBO_ARCH_AARCH64;
    #[cfg(not(target_arch = "aarch64"))]
    let host_arch = KRBO_ARCH_X86_64;
    if arch != host_arch {
        let arch_name = match arch {
            KRBO_ARCH_X86_64 => "x86-64",
            KRBO_ARCH_AARCH64 => "aarch64",
            _ => "unknown",
        };
        let host_name = if host_arch == KRBO_ARCH_X86_64 {
            "x86-64"
        } else {
            "aarch64"
        };
        return Err(format!(
            "this .krbo targets {} but this host is {}",
            arch_name, host_name
        ));
    }
    let entry_offset = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    let code_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    if code_length == 0 {
        return Err("malformed .krbo: empty code section".to_string());
    }
    if entry_offset >= code_length {
        return Err("malformed .krbo: entry_offset out of range".to_string());
    }
    Ok(KrboHeader {
        version,
        arch,
        entry_offset,
        code_length,
    })
}

/// Emit a KRBOFAT fat binary from (arch_id, raw_krbo_bytes) slices.
/// Each slice is LZ4-compressed.
pub fn emit_krbofat_bytes(slices: &[(u32, Vec<u8>)]) -> Result<Vec<u8>, String> {
    use lz4_flex::frame::FrameEncoder;
    use std::io::Write as IoWrite;

    let arch_count = slices.len() as u32;
    // Header: 8 (magic) + 4 (version) + 4 (arch_count) = 16 bytes
    // Entries: arch_count * 32 bytes each
    let header_region = 16 + arch_count as usize * 32;
    // Pad to next 16-byte boundary
    let padding = (16 - (header_region % 16)) % 16;
    let data_start = header_region + padding;

    // Compress all slices
    let mut compressed: Vec<Vec<u8>> = Vec::with_capacity(slices.len());
    for (_, raw) in slices {
        let mut enc = FrameEncoder::new(Vec::new());
        enc.write_all(raw)
            .map_err(|e| format!("lz4 compress: {e}"))?;
        compressed.push(enc.finish().map_err(|e| format!("lz4 finish: {e}"))?);
    }

    // Calculate offsets
    let mut offsets: Vec<u64> = Vec::with_capacity(slices.len());
    let mut cursor = data_start as u64;
    for c in &compressed {
        offsets.push(cursor);
        cursor += c.len() as u64;
    }

    let mut out = Vec::with_capacity(cursor as usize);
    out.extend_from_slice(&KRBO_FAT_MAGIC);
    out.extend_from_slice(&KRBO_FAT_VERSION.to_le_bytes());
    out.extend_from_slice(&arch_count.to_le_bytes());

    for (i, (arch_id, raw)) in slices.iter().enumerate() {
        out.extend_from_slice(&arch_id.to_le_bytes());
        out.extend_from_slice(&KRBO_FAT_COMPRESSION_LZ4.to_le_bytes());
        out.extend_from_slice(&offsets[i].to_le_bytes());
        out.extend_from_slice(&(compressed[i].len() as u64).to_le_bytes());
        out.extend_from_slice(&(raw.len() as u64).to_le_bytes());
    }

    out.extend(std::iter::repeat_n(0u8, padding));
    for c in &compressed {
        out.extend_from_slice(c);
    }

    Ok(out)
}

/// Extract and decompress one arch's krbo slice from a KRBOFAT fat binary.
/// `filename` is used only in error messages.
pub fn parse_krbofat_slice(
    fat: &[u8],
    arch_id: u32,
    filename: Option<&str>,
) -> Result<Vec<u8>, String> {
    use lz4_flex::frame::FrameDecoder;
    use std::io::Read as IoRead;

    let fname = filename.unwrap_or("<file>");

    if fat.len() < 16 {
        return Err(format!("{}: not a KRBOFAT: too short", fname));
    }
    if fat[0..8] != KRBO_FAT_MAGIC {
        return Err(format!("{}: not a KRBOFAT: wrong magic", fname));
    }
    let fat_version = u32::from_le_bytes(fat[8..12].try_into().unwrap());
    if fat_version != KRBO_FAT_VERSION {
        return Err(format!(
            "{}: unsupported KRBOFAT version {} (expected {})",
            fname, fat_version, KRBO_FAT_VERSION
        ));
    }
    let arch_count = u32::from_le_bytes(fat[12..16].try_into().unwrap()) as usize;

    for i in 0..arch_count {
        let e = 16 + i * 32;
        if fat.len() < e + 32 {
            return Err(format!("{}: KRBOFAT: truncated entries", fname));
        }
        let entry_arch = u32::from_le_bytes(fat[e..e + 4].try_into().unwrap());
        let compression = u32::from_le_bytes(fat[e + 4..e + 8].try_into().unwrap());
        let offset = u64::from_le_bytes(fat[e + 8..e + 16].try_into().unwrap()) as usize;
        let comp_size = u64::from_le_bytes(fat[e + 16..e + 24].try_into().unwrap()) as usize;
        let uncomp_size = u64::from_le_bytes(fat[e + 24..e + 32].try_into().unwrap()) as usize;

        if entry_arch != arch_id {
            continue;
        }

        let slice = fat
            .get(offset..offset + comp_size)
            .ok_or_else(|| format!("{}: KRBOFAT: slice data out of bounds", fname))?;

        return match compression {
            KRBO_FAT_COMPRESSION_NONE => Ok(slice.to_vec()),
            KRBO_FAT_COMPRESSION_LZ4 => {
                let mut dec = FrameDecoder::new(slice);
                let mut buf = Vec::with_capacity(uncomp_size);
                dec.read_to_end(&mut buf)
                    .map_err(|e| format!("{}: lz4 decompress: {e}", fname))?;
                Ok(buf)
            }
            other => Err(format!("{}: KRBOFAT: unknown compression {}", fname, other)),
        };
    }

    let arch_name = match arch_id {
        KRBO_FAT_ARCH_X86_64 => "x86_64",
        KRBO_FAT_ARCH_AARCH64 => "arm64",
        _ => "unknown",
    };
    Err(format!(
        "{}: does not contain a slice for {}",
        fname, arch_name
    ))
}

// ---------------------------------------------------------------------------
// AArch64 object emission — ELF, Mach-O, COFF
// ---------------------------------------------------------------------------

/// AArch64 relocation produced during binary encoding of a function.
struct AArch64PendingReloc {
    /// Byte offset of the 26-bit immediate field within the text section.
    patch_offset: u32,
    /// Target symbol name.
    target_symbol: String,
}

/// Intermediate result of encoding one AArch64 function to binary.
struct AArch64EncodedFunction {
    symbol: String,
    /// Byte offset of this function within the text section.
    offset: u32,
    /// Number of bytes emitted for this function.
    size: u32,
    /// External-call relocations whose targets are undefined at this point.
    relocs: Vec<AArch64PendingReloc>,
}

// ── AArch64 binary-encoding helpers ─────────────────────────────────────────

/// Load a 64-bit immediate value into register `rd` using MOVZ + up to three
/// MOVK instructions.  Emits the minimum number of instructions needed.
fn emit_aa64_imm64(out: &mut Vec<u8>, rd: u32, value: u64) {
    let parts: [u16; 4] = [
        (value & 0xFFFF) as u16,
        ((value >> 16) & 0xFFFF) as u16,
        ((value >> 32) & 0xFFFF) as u16,
        ((value >> 48) & 0xFFFF) as u16,
    ];
    // Use the first non-zero halfword as the MOVZ target (clears other bits).
    // Fall back to hw=0 if the value is zero.
    let first_nz = parts.iter().position(|&v| v != 0).unwrap_or(0);
    let hw0 = first_nz as u32;
    // MOVZ Xd, #parts[first_nz], LSL #(hw0 * 16)
    out.extend_from_slice(
        &(0xD280_0000u32 | (hw0 << 21) | ((parts[first_nz] as u32) << 5) | rd).to_le_bytes(),
    );
    for (hw, &part) in parts.iter().enumerate().skip(first_nz + 1) {
        if part != 0 {
            // MOVK Xd, #part, LSL #(hw * 16)
            out.extend_from_slice(
                &(0xF280_0000u32 | ((hw as u32) << 21) | ((part as u32) << 5) | rd).to_le_bytes(),
            );
        }
    }
}

/// Emit a typed store: `STR[BH]? Rt, [Rn]` with zero offset.
///
/// Access width is determined by `ty`:
/// - U8/U16/U32/U64 → STRB/STRH/STR W/STR X
/// - F32/F64        → same as U32/U64 (MMIO cares only about bit width)
fn emit_aa64_str_ty(out: &mut Vec<u8>, rt: u32, rn: u32, ty: MmioScalarType) {
    let base: u32 = match ty {
        MmioScalarType::U8 => 0x3900_0000,
        MmioScalarType::U16 => 0x7900_0000,
        MmioScalarType::U32 | MmioScalarType::F32 => 0xB900_0000,
        MmioScalarType::U64 | MmioScalarType::F64 => 0xF900_0000,
    };
    out.extend_from_slice(&(base | (rn << 5) | rt).to_le_bytes());
}

/// Emit a typed load: `LDR[BH]? Rt, [Rn]` with zero offset (zero-extends).
fn emit_aa64_ldr_ty(out: &mut Vec<u8>, rt: u32, rn: u32, ty: MmioScalarType) {
    let base: u32 = match ty {
        MmioScalarType::U8 => 0x3940_0000,
        MmioScalarType::U16 => 0x7940_0000,
        MmioScalarType::U32 | MmioScalarType::F32 => 0xB940_0000,
        MmioScalarType::U64 | MmioScalarType::F64 => 0xF940_0000,
    };
    out.extend_from_slice(&(base | (rn << 5) | rt).to_le_bytes());
}

// ── AArch64 function encoder ─────────────────────────────────────────────────

/// Encode the prologue + body + epilogue of one AArch64 asm function into `out`.
///
/// Returns the symbol, byte offset, and size suitable for building the symbol
/// table, plus ALL symbol references (both internal calls that the outer loop
/// will patch and external calls left for the linker).
///
/// Register conventions used throughout:
///   x0  — "saved value": holds the result of the last `CallCapture`, `MmioRead`,
///          or `StackLoad`.  Also used as the return value register (AAPCS64).
///   x9  — scratch A: used for MMIO addresses, compare values, masks, and the
///          loop-condition register (replacing x86-64's `eax` in `TestEaxEax`).
///   x10 — scratch B: used for write values in `MmioWriteImm`.
///   x29 — frame pointer; stack slots live at `[x29 + 16 + slot_idx * 8]`.
fn encode_aarch64_function(
    func: &AArch64AsmFunction,
    out: &mut Vec<u8>,
    _defined_symbols: &BTreeSet<&str>, // kept for API compatibility; all relocs unified below
) -> Result<AArch64EncodedFunction, String> {
    let start_offset = out.len() as u32;

    // Frame layout: round stack_cells*8 up to 16, then add 16 for x29/x30.
    let stack_bytes = (func.n_stack_cells as usize * 8 + 15) & !15usize;
    let frame_bytes = stack_bytes + 16;

    // ── Prologue: stp x29, x30, [sp, #-frame_bytes]! ────────────────────────
    // STP (pre-index, 64-bit): 0xA9800000 | (imm7 << 15) | (x30 << 10) | (sp << 5) | x29
    {
        let imm = -(frame_bytes as i32) / 8; // must be in [-64, 63]
        if !(-64..=63).contains(&imm) {
            return Err(format!(
                "aarch64: frame size {} exceeds STP range in function '{}'",
                frame_bytes, func.symbol
            ));
        }
        let imm7 = (imm as u32) & 0x7F;
        out.extend_from_slice(
            &(0xA980_0000 | (imm7 << 15) | (30 << 10) | (31 << 5) | 29).to_le_bytes(),
        );
    }
    // mov x29, sp  →  add x29, sp, #0  →  0x910003FD
    out.extend_from_slice(&0x910003FDu32.to_le_bytes());

    // ── Branch-patch infrastructure ──────────────────────────────────────────
    // Forward branches and label-directed jumps need a two-pass approach:
    // emit a placeholder instruction, record where it is, then fix the
    // displacement once the target label's offset is known.
    #[allow(dead_code)]
    enum BPKind {
        B,      // B  imm26  (0x14000000)
        CbzX0,  // CBZ  X0,  imm19
        CbzX9,  // CBZ  X9,  imm19  (after LoadSlotU8ToEax)
        CbnzX9, // CBNZ X9,  imm19
        BNe,    // B.NE      imm19
        BEq,    // B.EQ      imm19
    }
    // (patch_offset_in_out, target_label, kind)
    let mut branch_patches: Vec<(u32, String, BPKind)> = Vec::new();
    let mut label_offsets: BTreeMap<String, u32> = BTreeMap::new();
    let mut synth_ctr: usize = 0; // counter for synthetic label names
    let mut relocs: Vec<AArch64PendingReloc> = Vec::new();

    // Emit a BL to `sym`, always recording it as a reloc so the outer loop can
    // patch it (for internal targets) or leave it for the linker (external).
    macro_rules! emit_bl {
        ($sym:expr) => {{
            let po = out.len() as u32;
            relocs.push(AArch64PendingReloc {
                patch_offset: po,
                target_symbol: ($sym).to_string(),
            });
            out.extend_from_slice(&0x9400_0000u32.to_le_bytes());
        }};
    }

    // LDR X{rd}, [X29, #(16 + slot*8)]  (64-bit slot load, imm12 scaled by 8)
    macro_rules! slot_ldr {
        ($rd:expr, $slot:expr) => {{
            let imm12 = 2u32 + $slot as u32; // (16 + slot*8) / 8
            out.extend_from_slice(
                &(0xF940_0000u32 | (imm12 << 10) | (29 << 5) | ($rd as u32)).to_le_bytes(),
            );
        }};
    }

    // STR X{rs}, [X29, #(16 + slot*8)]
    macro_rules! slot_str {
        ($rs:expr, $slot:expr) => {{
            let imm12 = 2u32 + $slot as u32;
            out.extend_from_slice(
                &(0xF900_0000u32 | (imm12 << 10) | (29 << 5) | ($rs as u32)).to_le_bytes(),
            );
        }};
    }

    // LDP epilogue helper (post-index restore of x29/x30, then either RET or B).
    macro_rules! emit_ldp_epilogue {
        () => {{
            let imm = frame_bytes as i32 / 8;
            if !(-64..=63).contains(&imm) {
                return Err(format!(
                    "aarch64: frame size {} exceeds LDP range in function '{}'",
                    frame_bytes, func.symbol
                ));
            }
            let imm7 = (imm as u32) & 0x7F;
            out.extend_from_slice(
                &(0xA8C0_0000 | (imm7 << 15) | (30 << 10) | (31 << 5) | 29).to_le_bytes(),
            );
        }};
    }

    // ── Instruction encoding loop ─────────────────────────────────────────────
    for instr in &func.instructions {
        match instr {
            // ── Calls (BL; result in x0) ─────────────────────────────────────
            AArch64AsmInstruction::Call { symbol }
            | AArch64AsmInstruction::CallWithArgs { symbol, .. }
            | AArch64AsmInstruction::CallCapture { symbol, .. } => {
                emit_bl!(symbol);
            }
            // ── CallCaptureWithArgs (BL + store x0 to slot) ──────────────────
            AArch64AsmInstruction::CallCaptureWithArgs {
                symbol,
                slot_idx,
                ..
            } => {
                emit_bl!(symbol);
                slot_str!(0, *slot_idx);
            }

            // ── Tail-call (LDP epilogue + B) ──────────────────────────────────
            AArch64AsmInstruction::TailCall { symbol, args: _ } => {
                emit_ldp_epilogue!();
                let po = out.len() as u32;
                relocs.push(AArch64PendingReloc {
                    patch_offset: po,
                    target_symbol: symbol.clone(),
                });
                out.extend_from_slice(&0x1400_0000u32.to_le_bytes()); // B placeholder
            }

            // ── Return (LDP epilogue + ret) ───────────────────────────────────
            AArch64AsmInstruction::Ret => {
                emit_ldp_epilogue!();
                out.extend_from_slice(&0xD65F03C0u32.to_le_bytes()); // ret
            }

            // ── Label (zero bytes; records position for branch-patch loop) ────
            AArch64AsmInstruction::Label(name) => {
                label_offsets.insert(name.clone(), out.len() as u32);
            }

            // ── Unconditional jump to label ───────────────────────────────────
            AArch64AsmInstruction::JmpLabel(label) => {
                let po = out.len() as u32;
                branch_patches.push((po, label.clone(), BPKind::B));
                out.extend_from_slice(&0x1400_0000u32.to_le_bytes());
            }

            // ── Conditional jumps (x9 was loaded by LoadSlotU8ToEax) ─────────
            AArch64AsmInstruction::JmpIfZeroLabel(label) => {
                let po = out.len() as u32;
                branch_patches.push((po, label.clone(), BPKind::CbzX9));
                out.extend_from_slice(&0xB400_0009u32.to_le_bytes()); // CBZ X9 placeholder
            }
            AArch64AsmInstruction::JmpIfNonZeroLabel(label) => {
                let po = out.len() as u32;
                branch_patches.push((po, label.clone(), BPKind::CbnzX9));
                out.extend_from_slice(&0xB500_0009u32.to_le_bytes()); // CBNZ X9 placeholder
            }

            // ── Loop-condition slot load (x86-64: mov eax,[rbp-n]) ────────────
            // On AArch64 we load the u8 slot into x9 (our "eax" equivalent).
            // LDRB W9, [X29, #(16 + slot_idx * 8)]  — byte access, imm12 = byte offset
            AArch64AsmInstruction::LoadSlotU8ToEax { slot_idx } => {
                let byte_off = 16u32 + (*slot_idx as u32) * 8;
                if byte_off > 4095 {
                    return Err(format!(
                        "aarch64: slot {} LDRB offset {} exceeds imm12 range in '{}'",
                        slot_idx, byte_off, func.symbol
                    ));
                }
                // LDRB W9, [X29, #byte_off]
                out.extend_from_slice(
                    &(0x3940_0000u32 | (byte_off << 10) | (29 << 5) | 9).to_le_bytes(),
                );
            }

            // ── Flags set for loop condition (x86-64: test eax, eax) ─────────
            // On AArch64 we use CBZ/CBNZ directly on x9, so no separate
            // flag-setting instruction is required.
            AArch64AsmInstruction::TestEaxEax => {}

            // ── Return saved value (x0 already holds it — no-op) ─────────────
            AArch64AsmInstruction::ReturnSavedValue { ty } => {
                // AAPCS64: the return value lives in x0, which is also the
                // "saved value" register.  Nothing to do.
                let _ = ty;
            }

            // ── MMIO write: x9 = addr, x10 = value, store ────────────────────
            AArch64AsmInstruction::MmioWriteImm { ty, addr, value } => {
                emit_aa64_imm64(out, 9, *addr);
                emit_aa64_imm64(out, 10, *value);
                emit_aa64_str_ty(out, 10, 9, *ty);
            }

            // ── MMIO read: x9 = addr, x0 = load ──────────────────────────────
            AArch64AsmInstruction::MmioRead {
                ty,
                addr,
                capture_value,
            } => {
                emit_aa64_imm64(out, 9, *addr);
                emit_aa64_ldr_ty(out, 0, 9, *ty); // result in x0 (saved-value register)
                let _ = capture_value;
            }

            // ── MMIO write saved value: x9 = addr, store x0 ──────────────────
            AArch64AsmInstruction::MmioWriteValue { ty, addr } => {
                emit_aa64_imm64(out, 9, *addr);
                emit_aa64_str_ty(out, 0, 9, *ty);
            }

            // ── Stack: store immediate value into slot ────────────────────────
            AArch64AsmInstruction::StackStoreImm {
                ty,
                value,
                slot_idx,
            } => {
                let _ = ty; // always stored as 64-bit in the 8-byte slot
                emit_aa64_imm64(out, 9, *value);
                slot_str!(9, *slot_idx);
            }

            // ── Stack: store saved value (x0) into slot ───────────────────────
            AArch64AsmInstruction::StackStoreValue { ty, slot_idx } => {
                let _ = ty;
                slot_str!(0, *slot_idx);
            }

            // ── Stack: load slot into saved value (x0) ────────────────────────
            AArch64AsmInstruction::StackLoad { ty, slot_idx } => {
                let _ = ty;
                slot_ldr!(0, *slot_idx);
            }

            // ── BranchIfZero: if x0 == 0 → then_symbol, else → else_symbol ───
            AArch64AsmInstruction::BranchIfZero {
                ty,
                then_symbol,
                else_symbol,
            } => {
                let _ = ty;
                let then_lbl = format!("__aa64_then_{}", synth_ctr);
                let merge_lbl = format!("__aa64_merge_{}", synth_ctr);
                synth_ctr += 1;
                // CBZ X0, .then_lbl
                let po = out.len() as u32;
                branch_patches.push((po, then_lbl.clone(), BPKind::CbzX0));
                out.extend_from_slice(&0xB400_0000u32.to_le_bytes());
                // BL else_symbol
                emit_bl!(else_symbol);
                // B .merge_lbl
                let po = out.len() as u32;
                branch_patches.push((po, merge_lbl.clone(), BPKind::B));
                out.extend_from_slice(&0x1400_0000u32.to_le_bytes());
                // .then_lbl:
                label_offsets.insert(then_lbl, out.len() as u32);
                // BL then_symbol
                emit_bl!(then_symbol);
                // .merge_lbl:
                label_offsets.insert(merge_lbl, out.len() as u32);
            }

            // ── BranchIfEqImm: if x0 == compare → then, else → else ──────────
            AArch64AsmInstruction::BranchIfEqImm {
                ty,
                compare_value,
                then_symbol,
                else_symbol,
            } => {
                let _ = ty;
                let else_lbl = format!("__aa64_else_{}", synth_ctr);
                let merge_lbl = format!("__aa64_merge_{}", synth_ctr);
                synth_ctr += 1;
                // x9 = compare_value
                emit_aa64_imm64(out, 9, *compare_value);
                // CMP X0, X9  (SUBS XZR, X0, X9)
                out.extend_from_slice(&0xEB09_001Fu32.to_le_bytes());
                // B.NE .else_lbl  (x0 != compare → else branch)
                let po = out.len() as u32;
                branch_patches.push((po, else_lbl.clone(), BPKind::BNe));
                out.extend_from_slice(&0x5400_0001u32.to_le_bytes()); // B.NE placeholder
                // BL then_symbol
                emit_bl!(then_symbol);
                // B .merge_lbl
                let po = out.len() as u32;
                branch_patches.push((po, merge_lbl.clone(), BPKind::B));
                out.extend_from_slice(&0x1400_0000u32.to_le_bytes());
                // .else_lbl:
                label_offsets.insert(else_lbl, out.len() as u32);
                // BL else_symbol
                emit_bl!(else_symbol);
                // .merge_lbl:
                label_offsets.insert(merge_lbl, out.len() as u32);
            }

            // ── BranchIfMaskNonZeroImm: if (x0 & mask) != 0 → then, else → else
            AArch64AsmInstruction::BranchIfMaskNonZeroImm {
                ty,
                mask_value,
                then_symbol,
                else_symbol,
            } => {
                let _ = ty;
                let else_lbl = format!("__aa64_else_{}", synth_ctr);
                let merge_lbl = format!("__aa64_merge_{}", synth_ctr);
                synth_ctr += 1;
                // x9 = mask_value
                emit_aa64_imm64(out, 9, *mask_value);
                // TST X0, X9  (ANDS XZR, X0, X9)
                out.extend_from_slice(&0xEA09_001Fu32.to_le_bytes());
                // B.EQ .else_lbl  ((x0 & mask) == 0 → else)
                let po = out.len() as u32;
                branch_patches.push((po, else_lbl.clone(), BPKind::BEq));
                out.extend_from_slice(&0x5400_0000u32.to_le_bytes()); // B.EQ placeholder
                // BL then_symbol
                emit_bl!(then_symbol);
                // B .merge_lbl
                let po = out.len() as u32;
                branch_patches.push((po, merge_lbl.clone(), BPKind::B));
                out.extend_from_slice(&0x1400_0000u32.to_le_bytes());
                // .else_lbl:
                label_offsets.insert(else_lbl, out.len() as u32);
                // BL else_symbol
                emit_bl!(else_symbol);
                // .merge_lbl:
                label_offsets.insert(merge_lbl, out.len() as u32);
            }

            _ => {
                return Err(format!(
                    "aarch64 object emission: unsupported instruction {:?} in function '{}'",
                    instr, func.symbol
                ));
            }
        }
    }

    // ── Patch label branches ──────────────────────────────────────────────────
    // AArch64 branch offsets are PC-relative from the branch instruction address
    // (not from PC+4 as on x86-64).  Formula: imm = (target - branch_pc) / 4.
    for (patch_off, label, kind) in &branch_patches {
        let target_off = *label_offsets.get(label.as_str()).ok_or_else(|| {
            format!(
                "aarch64: undefined label '{}' in function '{}'",
                label, func.symbol
            )
        })?;
        let patch_pc = *patch_off as i64;
        let target = target_off as i64;
        let disp = target - patch_pc;
        if disp % 4 != 0 {
            return Err(format!(
                "aarch64: branch to '{}' has non-4-byte-aligned displacement in '{}'",
                label, func.symbol
            ));
        }
        let idx = *patch_off as usize;
        let word: u32 = match kind {
            BPKind::B => {
                let imm26 = (disp / 4) as i32;
                if !(-(1 << 25)..(1 << 25)).contains(&imm26) {
                    return Err(format!("aarch64: B to '{}' exceeds imm26 range", label));
                }
                0x1400_0000 | ((imm26 as u32) & 0x03FF_FFFF)
            }
            BPKind::CbzX0 => {
                let imm19 = (disp / 4) as i32;
                if !(-(1 << 18)..(1 << 18)).contains(&imm19) {
                    return Err(format!(
                        "aarch64: CBZ X0 to '{}' exceeds imm19 range",
                        label
                    ));
                }
                0xB400_0000 | (((imm19 as u32) & 0x0007_FFFF) << 5)
            }
            BPKind::CbzX9 => {
                let imm19 = (disp / 4) as i32;
                if !(-(1 << 18)..(1 << 18)).contains(&imm19) {
                    return Err(format!(
                        "aarch64: CBZ X9 to '{}' exceeds imm19 range",
                        label
                    ));
                }
                0xB400_0000 | (((imm19 as u32) & 0x0007_FFFF) << 5) | 9
            }
            BPKind::CbnzX9 => {
                let imm19 = (disp / 4) as i32;
                if !(-(1 << 18)..(1 << 18)).contains(&imm19) {
                    return Err(format!(
                        "aarch64: CBNZ X9 to '{}' exceeds imm19 range",
                        label
                    ));
                }
                0xB500_0000 | (((imm19 as u32) & 0x0007_FFFF) << 5) | 9
            }
            BPKind::BNe => {
                let imm19 = (disp / 4) as i32;
                if !(-(1 << 18)..(1 << 18)).contains(&imm19) {
                    return Err(format!("aarch64: B.NE to '{}' exceeds imm19 range", label));
                }
                0x5400_0000 | (((imm19 as u32) & 0x0007_FFFF) << 5) | 1 // NE = cond 0b0001
            }
            BPKind::BEq => {
                let imm19 = (disp / 4) as i32;
                if !(-(1 << 18)..(1 << 18)).contains(&imm19) {
                    return Err(format!("aarch64: B.EQ to '{}' exceeds imm19 range", label));
                }
                0x5400_0000 | (((imm19 as u32) & 0x0007_FFFF) << 5) // EQ = cond 0b0000
            }
        };
        out[idx..idx + 4].copy_from_slice(&word.to_le_bytes());
    }

    let size = out.len() as u32 - start_offset;
    Ok(AArch64EncodedFunction {
        symbol: func.symbol.clone(),
        offset: start_offset,
        size,
        relocs,
    })
}

// (text_bytes, [(name, offset, size)], [(patch_offset, target_symbol)])
type AArch64ObjectInner = (Vec<u8>, Vec<(String, u32, u32)>, Vec<(u32, String)>);

/// Shared inner lowering step for all three AArch64 object formats.
///
/// Returns `(text_bytes, symbol_table, relocs)` where:
/// - `text_bytes` is the raw machine code for all functions,
/// - `symbol_table` is a list of `(name, offset, size)` for defined functions,
/// - `relocs` is a list of `(patch_offset_in_text, target_symbol)` for external
///   BL/B targets that need linker fixup.
fn lower_executable_krir_to_aarch64_object_inner(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<AArch64ObjectInner, String> {
    let asm = lower_executable_krir_to_aarch64_asm(module, target)?;

    // Collect all defined symbol names so encode_aarch64_function can
    // distinguish internal from external call targets.
    let defined: BTreeSet<&str> = asm.functions.iter().map(|f| f.symbol.as_str()).collect();

    let mut text_bytes: Vec<u8> = Vec::new();
    let mut encoded_fns: Vec<AArch64EncodedFunction> = Vec::new();

    for func in &asm.functions {
        let enc = encode_aarch64_function(func, &mut text_bytes, &defined)?;
        encoded_fns.push(enc);
    }

    // ── Patch internal BL/B targets using the unified reloc list ─────────────
    // encode_aarch64_function adds ALL BL/B sites to enc.relocs (both internal
    // and external).  Internal targets are patched here; external ones are
    // collected into all_relocs for linker fixup.
    //
    // AArch64 branch displacement formula: disp = target - branch_instruction_pc
    // (NOT target - (pc+4) as on x86-64).
    let fn_offset: BTreeMap<&str, u32> = encoded_fns
        .iter()
        .map(|e| (e.symbol.as_str(), e.offset))
        .collect();

    let mut all_relocs: Vec<(u32, String)> = Vec::new();
    for enc in &encoded_fns {
        for r in &enc.relocs {
            if let Some(&target_off) = fn_offset.get(r.target_symbol.as_str()) {
                // Internal target: patch the imm26 field in-place.
                let patch_pc = r.patch_offset as i64;
                let disp = target_off as i64 - patch_pc;
                if disp % 4 != 0 {
                    return Err(format!(
                        "aarch64 object: symbol '{}' displacement {} not 4-byte aligned",
                        r.target_symbol, disp
                    ));
                }
                let imm26 = (disp / 4) as i32;
                if !(-(1 << 25)..(1 << 25)).contains(&imm26) {
                    return Err(format!(
                        "aarch64 object: displacement to '{}' exceeds imm26 range",
                        r.target_symbol
                    ));
                }
                let idx = r.patch_offset as usize;
                let existing = u32::from_le_bytes(text_bytes[idx..idx + 4].try_into().unwrap());
                // Preserve opcode bits [31:26] (BL=0x25, B=0x05, etc.) and
                // overwrite only the imm26 field [25:0].
                let word = (existing & 0xFC00_0000) | ((imm26 as u32) & 0x03FF_FFFF);
                text_bytes[idx..idx + 4].copy_from_slice(&word.to_le_bytes());
            } else {
                // External target: leave for the linker.
                all_relocs.push((r.patch_offset, r.target_symbol.clone()));
            }
        }
    }

    let symbol_table: Vec<(String, u32, u32)> = encoded_fns
        .iter()
        .map(|e| (e.symbol.clone(), e.offset, e.size))
        .collect();

    Ok((text_bytes, symbol_table, all_relocs))
}

/// Emit an AArch64 `.krbo` executable blob.
///
/// The slice is version=1, arch=`KRBO_ARCH_AARCH64` (0x02): a 16-byte KRBO
/// header followed by raw AArch64 machine code.  The `entry` function's byte
/// offset within the code is stored in the header.
pub fn emit_aarch64_executable_bytes(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    if !module.extern_declarations.is_empty() {
        let unresolved = module
            .extern_declarations
            .iter()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "final executable emit currently requires no extern declarations; unresolved externs: {}",
            unresolved
        ));
    }
    let (text_bytes, sym_table, _relocs) =
        lower_executable_krir_to_aarch64_object_inner(module, target)?;
    let entry_sym = sym_table
        .iter()
        .find(|(name, _, _)| name == "entry")
        .ok_or_else(|| "no 'entry' function found in module".to_string())?;
    let entry_offset = entry_sym.1;
    Ok(emit_krbo_bytes_raw_arch(
        &text_bytes,
        entry_offset,
        KRBO_ARCH_AARCH64,
    ))
}

// ---------------------------------------------------------------------------
// Task 4: AArch64 ELF object emission
// ---------------------------------------------------------------------------

/// Emit an ELF64 relocatable object for the given AArch64 module.
///
/// The emitted file:
/// - starts with the 4-byte ELF magic `\x7fELF`
/// - `e_machine` at offset 18–19 = `0x00B7` (EM_AARCH64, little-endian)
/// - one `.text` section containing all function code
/// - standard ELF64 symbol table and string table
pub fn emit_aarch64_elf_object_bytes(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    let (text_bytes, symbol_table, relocs) =
        lower_executable_krir_to_aarch64_object_inner(module, target)?;

    // Collect undefined symbol names in sorted order for the symbol table.
    let undefined_names: Vec<String> = {
        let mut names: BTreeSet<String> = BTreeSet::new();
        for (_, sym) in &relocs {
            names.insert(sym.clone());
        }
        names.into_iter().collect()
    };

    // Build .strtab: leading null + each name + null.
    let mut strtab = vec![0u8];
    let mut name_offsets: BTreeMap<String, u32> = BTreeMap::new();
    for (name, _, _) in &symbol_table {
        let off = strtab.len() as u32;
        strtab.extend_from_slice(name.as_bytes());
        strtab.push(0);
        name_offsets.insert(name.clone(), off);
    }
    for name in &undefined_names {
        let off = strtab.len() as u32;
        strtab.extend_from_slice(name.as_bytes());
        strtab.push(0);
        name_offsets.insert(name.clone(), off);
    }

    // Build .symtab: null + section sym + defined fns + undefined fns.
    let mut symtab: Vec<u8> = Vec::new();
    push_elf64_sym(&mut symtab, 0, 0, 0, 0, 0, 0); // STN_UNDEF
    push_elf64_sym(&mut symtab, 0, 0x03, 0, 1, 0, 0); // section symbol for .text (shndx=1)
    let mut sym_indices: BTreeMap<String, u32> = BTreeMap::new();
    let mut next_idx = 2u32;
    for (name, offset, size) in &symbol_table {
        push_elf64_sym(
            &mut symtab,
            *name_offsets.get(name).expect("defined name in strtab"),
            0x12, // STB_GLOBAL | STT_FUNC
            0,
            1, // shndx = .text section index
            *offset as u64,
            *size as u64,
        );
        sym_indices.insert(name.clone(), next_idx);
        next_idx += 1;
    }
    for name in &undefined_names {
        push_elf64_sym(
            &mut symtab,
            *name_offsets.get(name).expect("undef name in strtab"),
            0x12,
            0,
            0, // shndx = SHN_UNDEF
            0,
            0,
        );
        sym_indices.insert(name.clone(), next_idx);
        next_idx += 1;
    }

    // Build .rela.text: one Elf64_Rela per reloc (24 bytes each).
    // R_AARCH64_CALL26 = 283 (0x11B)
    let mut rela_text: Vec<u8> = Vec::new();
    for (patch_offset, target_sym) in &relocs {
        push_u64_le(&mut rela_text, *patch_offset as u64); // r_offset
        let sym_idx = *sym_indices.get(target_sym).expect("reloc sym in index");
        let r_info = ((sym_idx as u64) << 32) | 283u64; // R_AARCH64_CALL26
        push_u64_le(&mut rela_text, r_info); // r_info
        push_i64_le(&mut rela_text, 0); // r_addend = 0
    }

    // Build .shstrtab.
    let mut shstrtab = vec![0u8];
    let text_name_sh = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".text\0");
    let rela_text_name_sh = if rela_text.is_empty() {
        None
    } else {
        let off = shstrtab.len() as u32;
        shstrtab.extend_from_slice(b".rela.text\0");
        Some(off)
    };
    let symtab_name_sh = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".symtab\0");
    let strtab_name_sh = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".strtab\0");
    let shstrtab_name_sh = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".shstrtab\0");

    // Layout: 64-byte ELF header, then sections, then section headers.
    let mut bytes = vec![0u8; 64];
    let text_offset = append_with_alignment(&mut bytes, &text_bytes, 16) as u64;
    let rela_text_offset = if rela_text.is_empty() {
        None
    } else {
        Some(append_with_alignment(&mut bytes, &rela_text, 8) as u64)
    };
    let symtab_offset = append_with_alignment(&mut bytes, &symtab, 8) as u64;
    let strtab_offset = append_with_alignment(&mut bytes, &strtab, 1) as u64;
    let shstrtab_offset = append_with_alignment(&mut bytes, &shstrtab, 1) as u64;
    let shoff = append_with_alignment(&mut bytes, &[], 8) as u64;

    // Section headers.
    let mut shdrs = vec![0u8; 64]; // null section header
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

    // sh[1]: .text
    push_shdr(
        text_name_sh,
        1,
        0x6,
        0,
        text_offset,
        text_bytes.len() as u64,
        0,
        0,
        16,
        0,
    );
    let symtab_idx = if rela_text.is_empty() { 2u32 } else { 3u32 };
    let strtab_idx = symtab_idx + 1;
    // sh[2] (optional): .rela.text
    if let (Some(name), Some(offset)) = (rela_text_name_sh, rela_text_offset) {
        push_shdr(
            name,
            4,
            0,
            0,
            offset,
            rela_text.len() as u64,
            symtab_idx,
            1, // link=.symtab, info=.text index
            8,
            24,
        );
    }
    // sh[symtab_idx]: .symtab
    let first_global = 2u32; // 0=null, 1=section; globals start at 2
    push_shdr(
        symtab_name_sh,
        2,
        0,
        0,
        symtab_offset,
        symtab.len() as u64,
        strtab_idx,
        first_global,
        8,
        24,
    );
    // sh[strtab_idx]: .strtab
    push_shdr(
        strtab_name_sh,
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
    // sh[last]: .shstrtab
    let shstrtab_idx = strtab_idx + 1;
    push_shdr(
        shstrtab_name_sh,
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

    let e_shnum: u16 = if rela_text.is_empty() { 5 } else { 6 };
    let e_shstrndx: u16 = shstrtab_idx as u16;

    // Fill ELF header.
    bytes[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
    bytes[4] = 2; // EI_CLASS = ELFCLASS64
    bytes[5] = 1; // EI_DATA  = ELFDATA2LSB
    bytes[6] = 1; // EI_VERSION
    bytes[7] = 0; // EI_OSABI = ELFOSABI_NONE
    push_u16_into(&mut bytes[16..18], 1); // e_type  = ET_REL
    push_u16_into(&mut bytes[18..20], 0x00B7); // e_machine = EM_AARCH64
    push_u32_into(&mut bytes[20..24], 1); // e_version
    push_u64_into(&mut bytes[24..32], 0); // e_entry
    push_u64_into(&mut bytes[32..40], 0); // e_phoff
    push_u64_into(&mut bytes[40..48], shoff); // e_shoff
    push_u32_into(&mut bytes[48..52], 0); // e_flags
    push_u16_into(&mut bytes[52..54], 64); // e_ehsize
    push_u16_into(&mut bytes[54..56], 0); // e_phentsize
    push_u16_into(&mut bytes[56..58], 0); // e_phnum
    push_u16_into(&mut bytes[58..60], 64); // e_shentsize
    push_u16_into(&mut bytes[60..62], e_shnum); // e_shnum
    push_u16_into(&mut bytes[62..64], e_shstrndx); // e_shstrndx

    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Task 5: AArch64 Mach-O object emission
// ---------------------------------------------------------------------------

/// Emit a Mach-O 64-bit relocatable object for the given AArch64 module.
///
/// The emitted file:
/// - starts with Mach-O 64-bit LE magic `0xFEEDFACF` = `[0xCF, 0xFA, 0xED, 0xFE]`
/// - `cputype` at offset 4 = `0x0100000C` (CPU_TYPE_ARM64)
/// - `cpusubtype` at offset 8 = `0x00000000` (CPU_SUBTYPE_ARM64_ALL)
pub fn emit_aarch64_macho_object_bytes(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    let (text_bytes, symbol_table, relocs) =
        lower_executable_krir_to_aarch64_object_inner(module, target)?;

    let undefined_names: Vec<String> = {
        let mut names: BTreeSet<String> = BTreeSet::new();
        for (_, sym) in &relocs {
            names.insert(sym.clone());
        }
        names.into_iter().collect()
    };

    // Build string table: leading null + "_name\0" for every symbol.
    let mut strtab = vec![0u8];
    let mut name_offsets: BTreeMap<String, u32> = BTreeMap::new();
    for (name, _, _) in &symbol_table {
        let off = strtab.len() as u32;
        let prefixed = format!("_{}", name);
        strtab.extend_from_slice(prefixed.as_bytes());
        strtab.push(0);
        name_offsets.insert(name.clone(), off);
    }
    for name in &undefined_names {
        let off = strtab.len() as u32;
        let prefixed = format!("_{}", name);
        strtab.extend_from_slice(prefixed.as_bytes());
        strtab.push(0);
        name_offsets.insert(name.clone(), off);
    }

    // nlist_64 entries (16 bytes each).
    let mut symtab_bytes: Vec<u8> = Vec::new();
    let mut sym_index: BTreeMap<String, u32> = BTreeMap::new();
    let mut idx = 0u32;
    for (name, offset, _size) in &symbol_table {
        let strx = *name_offsets.get(name).expect("defined sym in strtab");
        push_u32_le(&mut symtab_bytes, strx);
        symtab_bytes.push(0x0F); // N_SECT | N_EXT
        symtab_bytes.push(1); // section ordinal 1 = __text
        push_u16_le(&mut symtab_bytes, 0);
        push_u64_le(&mut symtab_bytes, *offset as u64);
        sym_index.insert(name.clone(), idx);
        idx += 1;
    }
    for name in &undefined_names {
        let strx = *name_offsets.get(name).expect("undef sym in strtab");
        push_u32_le(&mut symtab_bytes, strx);
        symtab_bytes.push(0x01); // N_EXT | N_UNDF
        symtab_bytes.push(0); // NO_SECT
        push_u16_le(&mut symtab_bytes, 0);
        push_u64_le(&mut symtab_bytes, 0);
        sym_index.insert(name.clone(), idx);
        idx += 1;
    }
    let nsyms = idx;

    // Relocation entries (8 bytes each).
    // ARM64_RELOC_BRANCH26 = 2; extern=1, pcrel=1, length=2 → lower byte = 0xD2 (same as x86 branch)
    let mut reloc_bytes: Vec<u8> = Vec::new();
    for (patch_offset, target_sym) in &relocs {
        push_u32_le(&mut reloc_bytes, *patch_offset);
        let sidx = *sym_index.get(target_sym).expect("reloc sym in index");
        let r_info = (sidx << 8) | 0xD2;
        push_u32_le(&mut reloc_bytes, r_info);
    }
    let nreloc = relocs.len() as u32;

    // File layout (same structure as x86_64 Mach-O):
    // [0]    mach_header_64 (32)
    // [32]   LC_SEGMENT_64 + section_64 (152)
    // [184]  LC_SYMTAB (24)
    // [208]  text (padded to 4)
    // [208+P] relocations
    // [...]  nlist_64 symbol table
    // [...]  string table
    let text_offset: u32 = 208;
    let text_len = text_bytes.len() as u32;
    let text_padded = (text_len + 3) & !3u32;
    let reloc_offset: u32 = if nreloc == 0 {
        0
    } else {
        text_offset + text_padded
    };
    let sym_offset: u32 = text_offset + text_padded + reloc_bytes.len() as u32;
    let str_offset: u32 = sym_offset + symtab_bytes.len() as u32;
    let sizeofcmds: u32 = 152 + 24;

    let mut out: Vec<u8> = Vec::new();

    // mach_header_64 (32 bytes)
    push_u32_le(&mut out, 0xFEED_FACF); // MH_MAGIC_64
    push_u32_le(&mut out, 0x0100_000C); // CPU_TYPE_ARM64
    push_u32_le(&mut out, 0x0000_0000); // CPU_SUBTYPE_ARM64_ALL
    push_u32_le(&mut out, 0x0000_0001); // MH_OBJECT
    push_u32_le(&mut out, 2); // ncmds
    push_u32_le(&mut out, sizeofcmds);
    push_u32_le(&mut out, 0x0000_2000); // MH_SUBSECTIONS_VIA_SYMBOLS
    push_u32_le(&mut out, 0); // reserved

    // LC_SEGMENT_64 (cmdsize=152)
    push_u32_le(&mut out, 0x0000_0019); // LC_SEGMENT_64
    push_u32_le(&mut out, 152);
    out.extend_from_slice(b"__TEXT\0\0\0\0\0\0\0\0\0\0");
    push_u64_le(&mut out, 0); // vmaddr
    push_u64_le(&mut out, text_len as u64); // vmsize
    push_u64_le(&mut out, text_offset as u64); // fileoff
    push_u64_le(&mut out, text_len as u64); // filesize
    push_u32_le(&mut out, 7); // maxprot
    push_u32_le(&mut out, 5); // initprot
    push_u32_le(&mut out, 1); // nsects
    push_u32_le(&mut out, 0); // flags

    // section_64 for __text (80 bytes)
    out.extend_from_slice(b"__text\0\0\0\0\0\0\0\0\0\0");
    out.extend_from_slice(b"__TEXT\0\0\0\0\0\0\0\0\0\0");
    push_u64_le(&mut out, 0); // addr
    push_u64_le(&mut out, text_len as u64); // size
    push_u32_le(&mut out, text_offset); // offset in file
    push_u32_le(&mut out, 2); // align (log2 → 2^2 = 4)
    push_u32_le(&mut out, reloc_offset); // reloff
    push_u32_le(&mut out, nreloc); // nreloc
    push_u32_le(&mut out, 0x8000_0400); // flags
    push_u32_le(&mut out, 0); // reserved1
    push_u32_le(&mut out, 0); // reserved2
    push_u32_le(&mut out, 0); // reserved3

    // LC_SYMTAB (24 bytes)
    push_u32_le(&mut out, 0x0000_0002); // LC_SYMTAB
    push_u32_le(&mut out, 24);
    push_u32_le(&mut out, sym_offset);
    push_u32_le(&mut out, nsyms);
    push_u32_le(&mut out, str_offset);
    push_u32_le(&mut out, strtab.len() as u32);

    // Text bytes (padded to 4)
    out.extend_from_slice(&text_bytes);
    while out.len() < (text_offset + text_padded) as usize {
        out.push(0);
    }

    // Relocation entries
    out.extend_from_slice(&reloc_bytes);

    // Symbol table
    out.extend_from_slice(&symtab_bytes);

    // String table
    out.extend_from_slice(&strtab);

    Ok(out)
}

// ---------------------------------------------------------------------------
// Task 6: AArch64 COFF object emission
// ---------------------------------------------------------------------------

/// Emit a COFF relocatable object for the given AArch64 module (Windows ARM64).
///
/// The emitted file:
/// - `Machine` at offset 0–1 = `0xAA64` (IMAGE_FILE_MACHINE_ARM64, little-endian)
pub fn emit_aarch64_coff_object_bytes(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    let (text_bytes, symbol_table, relocs) =
        lower_executable_krir_to_aarch64_object_inner(module, target)?;

    let undefined_names: Vec<String> = {
        let mut names: BTreeSet<String> = BTreeSet::new();
        for (_, sym) in &relocs {
            names.insert(sym.clone());
        }
        names.into_iter().collect()
    };

    let mut strtab_strings: Vec<u8> = Vec::new();
    let mut name_strtab_offsets: BTreeMap<String, u32> = BTreeMap::new();

    // Number of symbols: 1 (.text section) + defined + undefined.
    let num_syms = 1 + symbol_table.len() + undefined_names.len();

    let mut sym_index: BTreeMap<String, u32> = BTreeMap::new();
    for (i, (name, _, _)) in symbol_table.iter().enumerate() {
        sym_index.insert(name.clone(), (1 + i) as u32);
    }
    for (i, name) in undefined_names.iter().enumerate() {
        sym_index.insert(name.clone(), (1 + symbol_table.len() + i) as u32);
    }

    // IMAGE_RELOCATION entries (10 bytes each).
    // IMAGE_REL_ARM64_BRANCH26 = 0x0003
    let mut reloc_bytes: Vec<u8> = Vec::new();
    for (patch_offset, target_sym) in &relocs {
        push_u32_le(&mut reloc_bytes, *patch_offset);
        let sidx = *sym_index.get(target_sym).expect("reloc sym in index");
        push_u32_le(&mut reloc_bytes, sidx);
        push_u16_le(&mut reloc_bytes, 0x0003); // IMAGE_REL_ARM64_BRANCH26
    }
    let nrelocs = relocs.len() as u16;

    // File layout.
    let text_raw_offset: u32 = 20 + 40; // COFF header + section header = 60
    let text_len = text_bytes.len() as u32;
    let text_padded = (text_len + 3) & !3u32;
    let reloc_ptr: u32 = text_raw_offset + text_padded;
    let sym_table_ptr: u32 = reloc_ptr + reloc_bytes.len() as u32;

    let mut out: Vec<u8> = Vec::new();

    // IMAGE_FILE_HEADER (20 bytes)
    push_u16_le(&mut out, 0xAA64); // Machine = ARM64
    push_u16_le(&mut out, 1); // NumberOfSections
    push_u32_le(&mut out, 0); // TimeDateStamp
    push_u32_le(&mut out, sym_table_ptr); // PointerToSymbolTable
    push_u32_le(&mut out, num_syms as u32); // NumberOfSymbols
    push_u16_le(&mut out, 0); // SizeOfOptionalHeader
    push_u16_le(&mut out, 0); // Characteristics

    // IMAGE_SECTION_HEADER for .text (40 bytes)
    out.extend_from_slice(b".text\0\0\0");
    push_u32_le(&mut out, 0); // VirtualSize
    push_u32_le(&mut out, 0); // VirtualAddress
    push_u32_le(&mut out, text_padded); // SizeOfRawData
    push_u32_le(&mut out, text_raw_offset); // PointerToRawData
    push_u32_le(&mut out, if nrelocs == 0 { 0 } else { reloc_ptr }); // PointerToRelocations
    push_u32_le(&mut out, 0); // PointerToLinenumbers
    push_u16_le(&mut out, nrelocs); // NumberOfRelocations
    push_u16_le(&mut out, 0); // NumberOfLinenumbers
    push_u32_le(&mut out, 0x60500020); // Characteristics

    // Text bytes (padded to 4)
    out.extend_from_slice(&text_bytes);
    while out.len() < (text_raw_offset + text_padded) as usize {
        out.push(0);
    }

    // Relocation entries
    out.extend_from_slice(&reloc_bytes);

    // Symbol table.
    // Section symbol (.text)
    let section_name_buf = {
        let mut b = [0u8; 8];
        b[..5].copy_from_slice(b".text");
        b
    };
    push_coff_sym(&mut out, section_name_buf, 0, 1, 0, 0x03); // STATIC

    // Defined function symbols
    for (name, offset, _size) in &symbol_table {
        let name_buf = coff_encode_name(name, &mut strtab_strings, &mut name_strtab_offsets);
        push_coff_sym(
            &mut out, name_buf, *offset, 1, 0x0020, 0x02, // EXTERNAL
        );
    }

    // Undefined function symbols
    for name in &undefined_names {
        let name_buf = coff_encode_name(name, &mut strtab_strings, &mut name_strtab_offsets);
        push_coff_sym(&mut out, name_buf, 0, 0, 0x0020, 0x02);
    }

    // String table
    let strtab_total_size = (4 + strtab_strings.len()) as u32;
    push_u32_le(&mut out, strtab_total_size);
    out.extend_from_slice(&strtab_strings);

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{
        AArch64AsmFunction, AArch64AsmInstruction, AArch64AsmModule, AArch64IntegerRegister,
        BackendTargetContract, BackendTargetId, CallEdge, CompilerOwnedCodeSection,
        CompilerOwnedFixupKind, CompilerOwnedObject, CompilerOwnedObjectFixup,
        CompilerOwnedObjectHeader, CompilerOwnedObjectKind, CompilerOwnedObjectSymbol,
        CompilerOwnedObjectSymbolDefinition, CompilerOwnedObjectSymbolKind, Ctx, Eff,
        ExecutableBlock, ExecutableExternDecl, ExecutableFacts, ExecutableFunction,
        ExecutableKrirModule, ExecutableOp, ExecutableSignature, ExecutableTerminator,
        ExecutableValue, ExecutableValueType, FunctionAttrs, FutureScalarReturnConvention,
        IntegerRegister, MmioScalarType, TargetEndian, X86_64CoffFunctionSymbol,
        X86_64CoffRelocatableObject, X86_64IntegerRegister, X86_64MachOFunctionSymbol,
        X86_64MachORelocatableObject, emit_aarch64_asm_text, emit_compiler_owned_object_bytes,
        emit_x86_64_asm_text, emit_x86_64_coff_bytes, emit_x86_64_macho_object_bytes,
        emit_x86_64_object_bytes, export_compiler_owned_object_to_x86_64_asm,
        export_compiler_owned_object_to_x86_64_elf, lower_executable_krir_to_aarch64_asm,
        lower_executable_krir_to_compiler_owned_object, lower_executable_krir_to_x86_64_asm,
        lower_executable_krir_to_x86_64_object,
        validate_compiler_owned_object_for_x86_64_asm_export,
        validate_compiler_owned_object_linear_subset, validate_x86_64_object_linear_subset,
    };
    use serde_json::json;
    #[cfg(all(unix, target_arch = "x86_64"))]
    use std::os::unix::fs::PermissionsExt;
    use std::{
        collections::BTreeSet,
        fs,
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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

    #[cfg(all(unix, target_arch = "x86_64"))]
    fn run_executable_smoke(path: &Path) -> Option<std::process::Output> {
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(path)
                .expect("read linked artifact metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("mark linked artifact executable");
        }

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
            ..Default::default()
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
            ..Default::default()
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
        };

        assert_eq!(
            module.validate(),
            Err("executable KRIR function 'entry' entry block 'missing' is missing".to_string())
        );
    }

    #[test]
    fn executable_krir_validation_accepts_typed_params() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                name: "entry".to_string(),
                is_extern: false,
                signature: ExecutableSignature {
                    params: vec![ExecutableValueType::U8, ExecutableValueType::U64],
                    result: ExecutableValueType::Unit,
                },
                facts: ExecutableFacts {
                    ctx_ok: vec![],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                },
                entry_block: "entry".to_string(),
                blocks: vec![ExecutableBlock {
                    label: "entry".to_string(),
                    ops: vec![],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
            }],
            extern_declarations: vec![],
            call_edges: vec![],
            static_strings: Vec::new(),
        };

        assert_eq!(module.validate(), Ok(()));
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
                IntegerRegister::X86_64(X86_64IntegerRegister::Rdi),
                IntegerRegister::X86_64(X86_64IntegerRegister::Rsi),
                IntegerRegister::X86_64(X86_64IntegerRegister::Rdx),
                IntegerRegister::X86_64(X86_64IntegerRegister::Rcx),
                IntegerRegister::X86_64(X86_64IntegerRegister::R8),
                IntegerRegister::X86_64(X86_64IntegerRegister::R9),
            ]
        );
        assert_eq!(
            contract.future_scalar_return.registers(),
            vec![IntegerRegister::X86_64(X86_64IntegerRegister::Rax)]
        );
    }

    #[test]
    fn x86_64_sysv_target_contract_validation_rejects_overlapping_saved_sets() {
        let mut contract = BackendTargetContract::x86_64_sysv();
        contract
            .callee_saved
            .push(IntegerRegister::X86_64(X86_64IntegerRegister::Rax));

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
            .retain(|reg| *reg != IntegerRegister::X86_64(X86_64IntegerRegister::Rax));
        contract
            .caller_saved
            .retain(|reg| *reg != IntegerRegister::X86_64(X86_64IntegerRegister::Rax));
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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

    #[cfg_attr(not(target_arch = "x86_64"), ignore)]
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
            static_strings: Vec::new(),
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

    #[cfg_attr(not(target_arch = "x86_64"), ignore)]
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
            static_strings: Vec::new(),
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

    #[cfg_attr(not(target_arch = "x86_64"), ignore)]
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
            static_strings: Vec::new(),
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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
            static_strings: Vec::new(),
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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
            static_strings: Vec::new(),
        };
        let resolver = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("ext")],
            extern_declarations: vec![],
            call_edges: vec![],
            static_strings: Vec::new(),
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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
            static_strings: Vec::new(),
        };
        let resolver = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("ext")],
            extern_declarations: vec![],
            call_edges: vec![],
            static_strings: Vec::new(),
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
            static_strings: Vec::new(),
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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
            static_strings: Vec::new(),
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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
            static_strings: Vec::new(),
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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
            static_strings: Vec::new(),
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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
            static_strings: Vec::new(),
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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
            static_strings: Vec::new(),
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

    #[cfg(all(unix, target_arch = "x86_64"))]
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
            static_strings: Vec::new(),
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

    #[cfg_attr(not(target_arch = "x86_64"), ignore)]
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
            static_strings: Vec::new(),
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

    #[test]
    fn frame_encoding_uses_imm8_for_small_frames() {
        use crate::{emit_add_rsp, emit_sub_rsp, rsp_adj_encoded_len};
        // frame_size = 8 (stack cell) ≤ 127 → 4-byte SUB RSP / ADD RSP
        let mut buf = Vec::new();
        emit_sub_rsp(&mut buf, 8);
        assert_eq!(buf, &[0x48, 0x83, 0xEC, 0x08], "imm8 SUB RSP");
        let mut buf = Vec::new();
        emit_add_rsp(&mut buf, 8);
        assert_eq!(buf, &[0x48, 0x83, 0xC4, 0x08], "imm8 ADD RSP");
        assert_eq!(rsp_adj_encoded_len(8), 4);
    }

    #[test]
    fn frame_encoding_uses_imm32_for_large_frames() {
        use crate::{emit_add_rsp, emit_sub_rsp, rsp_adj_encoded_len};
        // frame_size = 128 > 127 → 7-byte SUB RSP / ADD RSP
        let mut buf = Vec::new();
        emit_sub_rsp(&mut buf, 128);
        assert_eq!(
            buf,
            &[0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00],
            "imm32 SUB RSP"
        );
        let mut buf = Vec::new();
        emit_add_rsp(&mut buf, 128);
        assert_eq!(
            buf,
            &[0x48, 0x81, 0xC4, 0x80, 0x00, 0x00, 0x00],
            "imm32 ADD RSP"
        );
        assert_eq!(rsp_adj_encoded_len(128), 7);
    }

    #[test]
    fn frame_encoding_imm32_boundary_at_127() {
        use crate::{emit_sub_rsp, rsp_adj_encoded_len};
        // 127 = max imm8 value
        let mut buf = Vec::new();
        emit_sub_rsp(&mut buf, 127);
        assert_eq!(buf, &[0x48, 0x83, 0xEC, 0x7F], "127 fits in imm8");
        assert_eq!(rsp_adj_encoded_len(127), 4);
        assert_eq!(rsp_adj_encoded_len(128), 7);
    }

    #[test]
    fn coff_object_bytes_start_with_amd64_machine() {
        let object = X86_64CoffRelocatableObject {
            text_bytes: vec![0xC3],
            function_symbols: vec![X86_64CoffFunctionSymbol {
                name: "entry".to_string(),
                offset: 0,
                size: 1,
            }],
            undefined_function_symbols: vec![],
            relocations: vec![],
        };
        let bytes = emit_x86_64_coff_bytes(&object);
        // Machine field = 0x8664 (AMD64) in little-endian
        assert_eq!(
            &bytes[0..2],
            &[0x64, 0x86],
            "must start with AMD64 machine type"
        );
    }

    #[test]
    fn macho_object_bytes_start_with_magic() {
        let object = X86_64MachORelocatableObject {
            text_bytes: vec![0xC3], // single `ret`
            function_symbols: vec![X86_64MachOFunctionSymbol {
                name: "entry".to_string(),
                offset: 0,
                size: 1,
            }],
            undefined_function_symbols: vec![],
            relocations: vec![],
        };
        let bytes = emit_x86_64_macho_object_bytes(&object);
        assert_eq!(
            &bytes[0..4],
            &[0xCF, 0xFA, 0xED, 0xFE],
            "must start with MH_MAGIC_64"
        );
    }

    #[test]
    fn krir_unsafe_markers_round_trip() {
        use super::{Ctx, Function, FunctionAttrs, KrirOp};
        let f = Function {
            name: "test".to_string(),
            is_extern: false,
            params: vec![],
            ctx_ok: vec![Ctx::Thread],
            eff_used: vec![],
            caps_req: vec![],
            attrs: FunctionAttrs::default(),
            ops: vec![KrirOp::UnsafeEnter, KrirOp::UnsafeExit],
        };
        assert!(
            matches!(f.ops[0], KrirOp::UnsafeEnter),
            "first op must be UnsafeEnter"
        );
        assert!(
            matches!(f.ops[1], KrirOp::UnsafeExit),
            "second op must be UnsafeExit"
        );
    }

    #[test]
    fn raw_ptr_load_op_exists() {
        use super::{KrirOp, MmioScalarType, MmioValueExpr};
        let op = KrirOp::RawPtrLoad {
            ty: MmioScalarType::U32,
            addr_slot: "p".to_string(),
            out_slot: "v".to_string(),
        };
        let op2 = KrirOp::RawPtrStore {
            ty: MmioScalarType::U32,
            addr_slot: "p".to_string(),
            value: MmioValueExpr::IntLiteral {
                value: "42".to_string(),
            },
        };
        match op {
            KrirOp::RawPtrLoad { .. } => {}
            _ => panic!("wrong variant"),
        }
        match op2 {
            KrirOp::RawPtrStore { .. } => {}
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn krbo_emit_and_parse_round_trip() {
        // Use host-native arch and a trivial 4-byte instruction sequence.
        #[cfg(target_arch = "aarch64")]
        let (host_arch, code_bytes) = (
            super::KRBO_ARCH_AARCH64,
            vec![0xc0u8, 0x03, 0x5f, 0xd6], // ret
        );
        #[cfg(not(target_arch = "aarch64"))]
        let (host_arch, code_bytes) = (
            super::KRBO_ARCH_X86_64,
            vec![0x90u8, 0x90, 0xC3, 0x00], // nop nop ret pad
        );
        let header_bytes = super::emit_krbo_bytes_raw_arch(&code_bytes, 0, host_arch);
        assert_eq!(&header_bytes[0..4], b"KRBO", "magic wrong");
        assert_eq!(header_bytes[4], 1, "version wrong");
        assert_eq!(header_bytes[5], host_arch, "arch wrong");
        assert_eq!(header_bytes[6], 0, "reserved[0] wrong");
        assert_eq!(header_bytes[7], 0, "reserved[1] wrong");
        let entry_off = u32::from_le_bytes(header_bytes[8..12].try_into().unwrap());
        let code_len = u32::from_le_bytes(header_bytes[12..16].try_into().unwrap());
        assert_eq!(entry_off, 0);
        assert_eq!(code_len, 4);
        assert_eq!(&header_bytes[16..], &code_bytes[..]);
        let hdr = super::parse_krbo_header(&header_bytes).unwrap();
        assert_eq!(hdr.entry_offset, 0);
        assert_eq!(hdr.code_length, 4);
    }

    #[test]
    fn krbo_parse_rejects_bad_magic() {
        let mut bad = vec![0u8; 20];
        bad[0..4].copy_from_slice(b"NOPE");
        bad[4] = 1;
        bad[5] = 0x01;
        bad[12..16].copy_from_slice(&4u32.to_le_bytes());
        assert!(super::parse_krbo_header(&bad).is_err());
    }

    #[test]
    fn krbo_parse_rejects_bad_version() {
        let mut bytes = vec![0u8; 20];
        bytes[0..4].copy_from_slice(b"KRBO");
        bytes[4] = 99;
        bytes[5] = 0x01;
        bytes[12..16].copy_from_slice(&4u32.to_le_bytes());
        assert!(super::parse_krbo_header(&bytes).is_err());
    }

    #[test]
    fn krbo_parse_rejects_empty_code() {
        let mut bytes = vec![0u8; 16];
        bytes[0..4].copy_from_slice(b"KRBO");
        bytes[4] = 1;
        bytes[5] = 0x01;
        // code_length stays 0
        assert!(super::parse_krbo_header(&bytes).is_err());
    }

    #[test]
    fn krbo_parse_rejects_entry_out_of_range() {
        let mut bytes = vec![0u8; 20];
        bytes[0..4].copy_from_slice(b"KRBO");
        bytes[4] = 1;
        bytes[5] = 0x01;
        bytes[8..12].copy_from_slice(&100u32.to_le_bytes()); // entry_offset=100
        bytes[12..16].copy_from_slice(&4u32.to_le_bytes()); // code_length=4 → out of range
        assert!(super::parse_krbo_header(&bytes).is_err());
    }

    #[test]
    fn aarch64_sysv_contract_validates() {
        let contract = BackendTargetContract::aarch64_sysv();
        assert_eq!(contract.validate(), Ok(()));
    }

    #[test]
    fn aarch64_macho_contract_validates() {
        let contract = BackendTargetContract::aarch64_macho();
        assert_eq!(contract.validate(), Ok(()));
    }

    #[test]
    fn aarch64_win_contract_validates() {
        let contract = BackendTargetContract::aarch64_win();
        assert_eq!(contract.validate(), Ok(()));
    }

    #[test]
    fn integer_register_ord_for_btreeset() {
        use std::collections::BTreeSet;
        let mut set: BTreeSet<IntegerRegister> = BTreeSet::new();
        set.insert(IntegerRegister::AArch64(AArch64IntegerRegister::X0));
        set.insert(IntegerRegister::X86_64(X86_64IntegerRegister::Rax));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn future_return_x0_registers() {
        let regs = FutureScalarReturnConvention::IntegerX0.registers();
        assert_eq!(
            regs,
            vec![IntegerRegister::AArch64(AArch64IntegerRegister::X0)]
        );
    }

    #[test]
    fn aarch64_asm_text_smoke() {
        let module = AArch64AsmModule {
            section: ".text",
            functions: vec![AArch64AsmFunction {
                symbol: "entry".to_string(),
                uses_saved_value_slot: false,
                n_stack_cells: 0,
                n_params: 0,
                instructions: vec![
                    AArch64AsmInstruction::Call {
                        symbol: "print".to_string(),
                    },
                    AArch64AsmInstruction::Ret,
                ],
            }],
        };
        let text = emit_aarch64_asm_text(&module);
        assert!(text.contains("entry:"), "missing label in:\n{}", text);
        assert!(text.contains("bl print"), "missing bl in:\n{}", text);
        assert!(text.contains("ret"), "missing ret in:\n{}", text);
    }

    #[test]
    fn aarch64_asm_text_mmio_widths() {
        let module = AArch64AsmModule {
            section: ".text",
            functions: vec![AArch64AsmFunction {
                symbol: "mmio_test".to_string(),
                uses_saved_value_slot: false,
                n_stack_cells: 0,
                n_params: 0,
                instructions: vec![
                    AArch64AsmInstruction::MmioRead {
                        ty: MmioScalarType::U8,
                        addr: 0x4000_0000,
                        capture_value: false,
                    },
                    AArch64AsmInstruction::MmioRead {
                        ty: MmioScalarType::U32,
                        addr: 0x4000_0004,
                        capture_value: false,
                    },
                    AArch64AsmInstruction::MmioRead {
                        ty: MmioScalarType::U64,
                        addr: 0x4000_0008,
                        capture_value: false,
                    },
                    AArch64AsmInstruction::Ret,
                ],
            }],
        };
        let text = emit_aarch64_asm_text(&module);
        assert!(text.contains("ldrb w0,"), "missing ldrb/w0 in:\n{}", text);
        assert!(text.contains("ldr w0,"), "missing ldr/w0 in:\n{}", text);
        assert!(text.contains("ldr x0,"), "missing ldr/x0 in:\n{}", text);
    }

    #[test]
    fn aarch64_asm_lowering_smoke() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                name: "entry".to_string(),
                is_extern: false,
                signature: ExecutableSignature {
                    params: vec![],
                    result: ExecutableValueType::Unit,
                },
                facts: ExecutableFacts {
                    ctx_ok: vec![],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                },
                entry_block: "b0".to_string(),
                blocks: vec![ExecutableBlock {
                    label: "b0".to_string(),
                    ops: vec![ExecutableOp::Call {
                        callee: "print".to_string(),
                    }],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
            }],
            extern_declarations: vec![ExecutableExternDecl {
                name: "print".to_string(),
            }],
            call_edges: vec![],
            static_strings: Vec::new(),
        };
        let target = BackendTargetContract::aarch64_sysv();
        let result = lower_executable_krir_to_aarch64_asm(&module, &target);
        assert!(result.is_ok(), "lowering failed: {:?}", result.err());
        let asm_module = result.unwrap();
        assert_eq!(asm_module.functions.len(), 1);
        assert_eq!(asm_module.functions[0].symbol, "entry");
        assert!(!asm_module.functions[0].instructions.is_empty());
    }

    fn minimal_aarch64_module() -> ExecutableKrirModule {
        ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![ExecutableFunction {
                name: "entry".to_string(),
                is_extern: false,
                signature: ExecutableSignature {
                    params: vec![],
                    result: ExecutableValueType::Unit,
                },
                facts: ExecutableFacts {
                    ctx_ok: vec![],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: FunctionAttrs::default(),
                },
                entry_block: "b0".to_string(),
                blocks: vec![ExecutableBlock {
                    label: "b0".to_string(),
                    ops: vec![],
                    terminator: ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                }],
            }],
            extern_declarations: vec![],
            call_edges: vec![],
            static_strings: Vec::new(),
        }
    }

    #[test]
    fn aarch64_elf_object_smoke() {
        use super::{BackendTargetContract, emit_aarch64_elf_object_bytes};
        let module = minimal_aarch64_module();
        let target = BackendTargetContract::aarch64_sysv();
        let result = emit_aarch64_elf_object_bytes(&module, &target);
        assert!(result.is_ok(), "{:?}", result.err());
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..4], b"\x7fELF");
        // e_machine at ELF offset 18-19: EM_AARCH64 = 0x00B7 (LE)
        assert_eq!(
            u16::from_le_bytes([bytes[18], bytes[19]]),
            0x00B7,
            "expected EM_AARCH64 (0x00B7) at offset 18"
        );
    }

    #[test]
    fn aarch64_macho_object_smoke() {
        use super::{BackendTargetContract, emit_aarch64_macho_object_bytes};
        let module = minimal_aarch64_module();
        let target = BackendTargetContract::aarch64_macho();
        let result = emit_aarch64_macho_object_bytes(&module, &target);
        assert!(result.is_ok(), "{:?}", result.err());
        let bytes = result.unwrap();
        // Mach-O 64-bit LE magic: 0xFEEDFACF = [0xCF, 0xFA, 0xED, 0xFE]
        assert_eq!(
            &bytes[0..4],
            &[0xCF, 0xFA, 0xED, 0xFE],
            "expected MH_MAGIC_64"
        );
        // cputype at offset 4: CPU_TYPE_ARM64 = 0x0100000C (LE)
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            0x0100_000C,
            "expected CPU_TYPE_ARM64 (0x0100000C) at offset 4"
        );
    }

    #[test]
    fn aarch64_coff_object_smoke() {
        use super::{BackendTargetContract, emit_aarch64_coff_object_bytes};
        let module = minimal_aarch64_module();
        let target = BackendTargetContract::aarch64_win();
        let result = emit_aarch64_coff_object_bytes(&module, &target);
        assert!(result.is_ok(), "{:?}", result.err());
        let bytes = result.unwrap();
        // COFF Machine at offset 0: IMAGE_FILE_MACHINE_ARM64 = 0xAA64 (LE)
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            0xAA64,
            "expected IMAGE_FILE_MACHINE_ARM64 (0xAA64) at offset 0"
        );
    }

    #[test]
    fn krbofat_roundtrip() {
        use super::{
            KRBO_FAT_ARCH_AARCH64, KRBO_FAT_ARCH_X86_64, KRBO_FAT_MAGIC, emit_krbofat_bytes,
            parse_krbofat_slice,
        };
        let x86_slice = b"KRBO\x01\x01\x00\x00\x00\x00\x00\x00_x86_fake_code_padding_".to_vec();
        let arm_slice = b"KRBO\x01\x02\x00\x00\x00\x00\x00\x00_arm_fake_code_padding_".to_vec();
        let fat = emit_krbofat_bytes(&[
            (KRBO_FAT_ARCH_X86_64, x86_slice.clone()),
            (KRBO_FAT_ARCH_AARCH64, arm_slice.clone()),
        ])
        .expect("emit failed");

        assert_eq!(&fat[0..8], &KRBO_FAT_MAGIC);

        let x86_back =
            parse_krbofat_slice(&fat, KRBO_FAT_ARCH_X86_64, None).expect("x86 slice missing");
        assert_eq!(x86_back, x86_slice);

        let arm_back =
            parse_krbofat_slice(&fat, KRBO_FAT_ARCH_AARCH64, None).expect("arm64 slice missing");
        assert_eq!(arm_back, arm_slice);
    }

    #[test]
    fn krbofat_fat_first_detection() {
        use super::{KRBO_FAT_ARCH_X86_64, emit_krbofat_bytes, parse_krbo_header};
        let x86_slice = b"KRBO\x01\x01\x00\x00\x04\x00\x00\x00\x04\x00\x00\x00\xc3___".to_vec();
        let fat = emit_krbofat_bytes(&[(KRBO_FAT_ARCH_X86_64, x86_slice)]).expect("emit failed");
        // A fat binary must NOT parse as single-arch
        let result = parse_krbo_header(&fat);
        assert!(
            result.is_err(),
            "fat binary must not parse as single-arch krbo"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("fat") || err.contains("KRBOFAT"),
            "error should mention fat format: {}",
            err
        );
    }

    #[test]
    fn krbofat_missing_arch_error() {
        use super::{
            KRBO_FAT_ARCH_AARCH64, KRBO_FAT_ARCH_X86_64, emit_krbofat_bytes, parse_krbofat_slice,
        };
        let arm_slice = b"KRBO\x01\x02\x00\x00\x04\x00\x00\x00_arm_fake".to_vec();
        let fat = emit_krbofat_bytes(&[(KRBO_FAT_ARCH_AARCH64, arm_slice)]).expect("emit failed");
        let result = parse_krbofat_slice(&fat, KRBO_FAT_ARCH_X86_64, Some("test.krbo"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("x86_64"),
            "error should name the missing arch: {}",
            err
        );
        assert!(
            err.contains("test.krbo"),
            "error should include filename: {}",
            err
        );
    }
}
