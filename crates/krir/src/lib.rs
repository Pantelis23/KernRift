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
    pub instructions: Vec<X86_64AsmInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum X86_64AsmInstruction {
    Call { symbol: String },
    Ret,
}

pub fn validate_x86_64_linear_subset(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<(), String> {
    target.validate()?;
    if target.target_id != BackendTargetId::X86_64Sysv
        || target.arch != TargetArch::X86_64
        || target.abi != TargetAbi::Sysv
    {
        return Err("x86_64 asm lowering requires x86_64-sysv target contract".to_string());
    }

    module.validate()?;
    let function_names = module
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect::<BTreeSet<_>>();

    for function in &module.functions {
        if function.blocks.len() != 1 {
            return Err(format!(
                "x86_64 asm lowering requires exactly one block in function '{}'",
                function.name
            ));
        }
        let entry = &function.blocks[0];
        if entry.label != function.entry_block {
            return Err(format!(
                "x86_64 asm lowering requires entry block '{}' to be first in function '{}'",
                function.entry_block, function.name
            ));
        }
        for op in &entry.ops {
            match op {
                ExecutableOp::Call { callee } => {
                    if !function_names.contains(callee.as_str()) {
                        return Err(format!(
                            "x86_64 asm lowering requires defined direct call target '{}' in function '{}'",
                            callee, function.name
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn lower_executable_krir_to_x86_64_asm(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<X86_64AsmModule, String> {
    validate_x86_64_linear_subset(module, target)?;

    let mut canonical = module.clone();
    canonical.canonicalize();

    let functions = canonical
        .functions
        .into_iter()
        .map(|function| {
            let block = &function.blocks[0];
            let mut instructions = block
                .ops
                .iter()
                .map(|op| match op {
                    ExecutableOp::Call { callee } => X86_64AsmInstruction::Call {
                        symbol: callee.clone(),
                    },
                })
                .collect::<Vec<_>>();
            instructions.push(X86_64AsmInstruction::Ret);

            X86_64AsmFunction {
                symbol: function.name,
                instructions,
            }
        })
        .collect::<Vec<_>>();

    Ok(X86_64AsmModule {
        section: target.sections.text,
        functions,
    })
}

pub fn emit_x86_64_asm_text(module: &X86_64AsmModule) -> String {
    let mut out = String::new();
    out.push_str(module.section);
    out.push('\n');
    for function in &module.functions {
        out.push('\n');
        out.push_str(&function.symbol);
        out.push_str(":\n");
        for instruction in &function.instructions {
            match instruction {
                X86_64AsmInstruction::Call { symbol } => {
                    out.push_str("    call ");
                    out.push_str(symbol);
                    out.push('\n');
                }
                X86_64AsmInstruction::Ret => {
                    out.push_str("    ret\n");
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64ElfRelocatableObject {
    pub format: &'static str,
    pub text_section: &'static str,
    pub text_bytes: Vec<u8>,
    pub function_symbols: Vec<X86_64ElfFunctionSymbol>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64ElfFunctionSymbol {
    pub name: String,
    pub offset: u64,
    pub size: u64,
}

pub fn validate_x86_64_object_linear_subset(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<(), String> {
    validate_x86_64_linear_subset(module, target)
        .map_err(|err| err.replace("x86_64 asm lowering", "x86_64 object emission"))
}

pub fn lower_executable_krir_to_x86_64_object(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<X86_64ElfRelocatableObject, String> {
    validate_x86_64_object_linear_subset(module, target)?;

    let mut canonical = module.clone();
    canonical.canonicalize();

    let mut function_offsets = BTreeMap::new();
    let mut function_sizes = BTreeMap::new();
    let mut cursor = 0u64;
    for function in &canonical.functions {
        let block = &function.blocks[0];
        let size = (block.ops.len() as u64) * 5 + 1;
        function_offsets.insert(function.name.clone(), cursor);
        function_sizes.insert(function.name.clone(), size);
        cursor += size;
    }

    let mut text_bytes = Vec::with_capacity(cursor as usize);
    let mut function_symbols = Vec::with_capacity(canonical.functions.len());
    for function in &canonical.functions {
        let block = &function.blocks[0];
        let function_offset = *function_offsets
            .get(&function.name)
            .expect("function offset must exist");
        let function_size = *function_sizes
            .get(&function.name)
            .expect("function size must exist");
        let mut local_offset = 0u64;
        for op in &block.ops {
            match op {
                ExecutableOp::Call { callee } => {
                    let callee_offset = *function_offsets.get(callee).ok_or_else(|| {
                        format!(
                            "x86_64 object emission requires defined direct call target '{}' in function '{}'",
                            callee, function.name
                        )
                    })?;
                    let next_ip = function_offset + local_offset + 5;
                    let displacement = (callee_offset as i64) - (next_ip as i64);
                    let rel32 = i32::try_from(displacement).map_err(|_| {
                        format!(
                            "x86_64 object emission call displacement to '{}' in function '{}' does not fit rel32",
                            callee, function.name
                        )
                    })?;
                    text_bytes.push(0xE8);
                    text_bytes.extend_from_slice(&rel32.to_le_bytes());
                    local_offset += 5;
                }
            }
        }
        text_bytes.push(0xC3);
        function_symbols.push(X86_64ElfFunctionSymbol {
            name: function.name.clone(),
            offset: function_offset,
            size: function_size,
        });
    }

    Ok(X86_64ElfRelocatableObject {
        format: "elf64-relocatable",
        text_section: target.sections.text,
        text_bytes,
        function_symbols,
    })
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
    let mut strtab = vec![0u8];
    let mut name_offsets = BTreeMap::new();
    for symbol in &object.function_symbols {
        let offset = strtab.len() as u32;
        strtab.extend_from_slice(symbol.name.as_bytes());
        strtab.push(0);
        name_offsets.insert(symbol.name.clone(), offset);
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

    let mut shstrtab = vec![0u8];
    let text_name = shstrtab.len() as u32;
    shstrtab.extend_from_slice(object.text_section.as_bytes());
    shstrtab.push(0);
    let symtab_name = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".symtab\0");
    let strtab_name = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".strtab\0");
    let shstrtab_name = shstrtab.len() as u32;
    shstrtab.extend_from_slice(b".shstrtab\0");

    let mut bytes = vec![0u8; 64];
    let text_offset = append_with_alignment(&mut bytes, &object.text_bytes, 16) as u64;
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
    push_shdr(
        symtab_name,
        2,
        0,
        0,
        symtab_offset,
        symtab.len() as u64,
        3,
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
    push_u16_into(&mut bytes[60..62], 5);
    push_u16_into(&mut bytes[62..64], 4);

    bytes
}

#[cfg(test)]
mod tests {
    use super::{
        BackendTargetContract, CallEdge, Ctx, Eff, ExecutableBlock, ExecutableFacts,
        ExecutableFunction, ExecutableKrirModule, ExecutableOp, ExecutableSignature,
        ExecutableTerminator, ExecutableValue, ExecutableValueType, FunctionAttrs,
        FutureScalarReturnConvention, X86_64IntegerRegister, emit_x86_64_asm_text,
        emit_x86_64_object_bytes, lower_executable_krir_to_x86_64_asm,
        lower_executable_krir_to_x86_64_object, validate_x86_64_linear_subset,
        validate_x86_64_object_linear_subset,
    };
    use serde_json::json;
    use std::collections::BTreeSet;

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
            call_edges: vec![],
        };
        module.canonicalize();

        let asm =
            lower_executable_krir_to_x86_64_asm(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 asm");

        assert_eq!(emit_x86_64_asm_text(&asm), ".text\n\nentry:\n    ret\n");
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
            call_edges: vec![],
        };

        let asm =
            lower_executable_krir_to_x86_64_asm(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 asm");

        assert_eq!(
            emit_x86_64_asm_text(&asm),
            ".text\n\nalpha:\n    ret\n\nbeta:\n    ret\n\nentry:\n    call alpha\n    call beta\n    ret\n"
        );
    }

    #[test]
    fn executable_krir_lowers_functions_in_deterministic_symbol_order() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("zeta"), unit_return_function("alpha")],
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
            call_edges: vec![],
        };

        assert_eq!(
            validate_x86_64_linear_subset(&module, &BackendTargetContract::x86_64_sysv()),
            Err("x86_64 asm lowering requires exactly one block in function 'entry'".to_string())
        );
    }

    #[test]
    fn executable_krir_x86_64_lowering_rejects_undefined_direct_call_target() {
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
            call_edges: vec![],
        };

        assert_eq!(
            validate_x86_64_linear_subset(&module, &BackendTargetContract::x86_64_sysv()),
            Err(
                "x86_64 asm lowering requires defined direct call target 'missing' in function 'entry'"
                    .to_string()
            )
        );
    }

    #[test]
    fn executable_krir_emits_empty_function_to_deterministic_elf64_object() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("entry")],
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
            call_edges: vec![],
        };

        let object =
            lower_executable_krir_to_x86_64_object(&module, &BackendTargetContract::x86_64_sysv())
                .expect("lower x86_64 object");

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
    }

    #[test]
    fn executable_krir_emits_functions_in_deterministic_symbol_order_to_object() {
        let module = ExecutableKrirModule {
            module_caps: vec![],
            functions: vec![unit_return_function("zeta"), unit_return_function("alpha")],
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
    fn executable_krir_x86_64_object_emission_rejects_undefined_direct_call_target() {
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
            call_edges: vec![],
        };

        assert_eq!(
            validate_x86_64_object_linear_subset(&module, &BackendTargetContract::x86_64_sysv()),
            Err(
                "x86_64 object emission requires defined direct call target 'missing' in function 'entry'"
                    .to_string()
            )
        );
    }
}
