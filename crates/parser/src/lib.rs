#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceNote {
    pub byte_offset: usize,
    pub line: usize,
    pub column: usize,
    pub line_text: String,
}

impl SourceNote {
    pub fn from_source(src: &str, byte_offset: usize) -> Self {
        let clamped = byte_offset.min(src.len());
        let prefix = &src[..clamped];
        let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
        let line_start = prefix.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let line_end = src[clamped..]
            .find('\n')
            .map(|idx| clamped + idx)
            .unwrap_or(src.len());
        let line_text = src[line_start..line_end].trim_end_matches('\r').to_string();
        let column = src[line_start..clamped].chars().count() + 1;
        Self {
            byte_offset: clamped,
            line,
            column,
            line_text,
        }
    }
}

pub fn format_source_diagnostic(source: &SourceNote, message: &str, help: Option<&str>) -> String {
    let mut rendered = format!("{} at {}:{}", message, source.line, source.column);
    rendered.push('\n');
    rendered.push_str(&format!("  {} | {}", source.line, source.line_text));
    if let Some(help) = help {
        rendered.push('\n');
        rendered.push_str(&format!("  = help: {}", help));
    }
    rendered
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawAttr {
    pub name: String,
    pub args: Option<String>,
    pub source: SourceNote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioScalarType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    F16,
    Bool,
    Char,
}

impl MmioScalarType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::U8 => "uint8",
            Self::U16 => "uint16",
            Self::U32 => "uint32",
            Self::U64 => "uint64",
            Self::I8 => "int8",
            Self::I16 => "int16",
            Self::I32 => "int32",
            Self::I64 => "int64",
            Self::F32 => "float32",
            Self::F64 => "float64",
            Self::F16 => "float16",
            Self::Bool => "bool",
            Self::Char => "char",
        }
    }

    /// Returns the underlying unsigned integer storage type for use in KRIR lowering.
    /// - Signed integers (I8/I16/I32/I64) → same-width unsigned (U8/U16/U32/U64)
    /// - Bool, Char → U8
    /// - F16 → U16 (storage as raw 16-bit value)
    /// - F32, F64 → returned as-is (no integer equivalent; KRIR float lowering handles these)
    /// - Unsigned integers → identity
    pub fn storage_type(self) -> Self {
        match self {
            Self::I8 => Self::U8,
            Self::I16 => Self::U16,
            Self::I32 => Self::U32,
            Self::I64 => Self::U64,
            Self::Bool | Self::Char => Self::U8,
            Self::F16 => Self::U16,
            other => other,
        }
    }

    pub fn byte_size(self) -> u8 {
        match self {
            Self::U8 | Self::I8 | Self::Bool | Self::Char => 1,
            Self::U16 | Self::I16 | Self::F16 => 2,
            Self::U32 | Self::I32 | Self::F32 => 4,
            Self::U64 | Self::I64 | Self::F64 => 8,
        }
    }

    pub fn is_signed(self) -> bool {
        matches!(self, Self::I8 | Self::I16 | Self::I32 | Self::I64)
    }

    pub fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64 | Self::F16)
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim() {
            "uint8" | "u8" | "byte" => Ok(Self::U8),
            "uint16" | "u16" => Ok(Self::U16),
            "uint32" | "u32" => Ok(Self::U32),
            "uint64" | "u64" | "addr" => Ok(Self::U64),
            "int8" | "i8" => Ok(Self::I8),
            "int16" | "i16" => Ok(Self::I16),
            "int32" | "i32" => Ok(Self::I32),
            "int64" | "i64" => Ok(Self::I64),
            "float32" | "f32" => Ok(Self::F32),
            "float64" | "f64" => Ok(Self::F64),
            "float16" | "f16" => Ok(Self::F16),
            "bool" => Ok(Self::Bool),
            "char" => Ok(Self::Char),
            other => Err(format!("unknown type '{}'", other)),
        }
    }
}

/// Binary operator kinds — precedence enforced by the Pratt parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    Add,
    Sub, // arithmetic
    And,
    Or,
    Xor,
    Shl,
    Shr, // bitwise
    Mul,
    Div,
    Rem, // arithmetic (deferred V1 — parser accepts, HIR rejects)
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge, // comparison
    LogAnd,
    LogOr, // logical
}

/// Unary operator kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOpKind {
    Not,    // logical not  `!`
    BitNot, // bitwise not  `~`
    Neg,    // arithmetic negation `-`
}

/// A full expression — replaces the old `MmioValueExpr` (which was Ident | IntLiteral only).
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    IntLiteral(u64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    CharLiteral(u8),
    StringLiteral(String), // for `print` intrinsic
    Ident(String),         // local variable or param slot
    DeviceField {
        // `UART0.Status`
        device: String,
        field: String,
    },
    SliceLen(String), // `buf.len`
    Call {
        // `get_status()`
        callee: String,
        args: Vec<Expr>,
    },
    BinOp {
        op: BinOpKind,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnOp {
        op: UnOpKind,
        operand: Box<Expr>,
    },
    /// `@syscall(nr, arg0, arg1, ...)` — generic syscall intrinsic.
    Syscall {
        args: Vec<Expr>, // first element is nr, rest are syscall args (up to 6)
    },
    /// `buf[index]` — read element from a slice parameter.
    SliceIndex {
        slice: String,
        index: Box<Expr>,
    },
}

/// A register inside a `device` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRegDecl {
    pub name: String,
    pub offset: String,
    pub ty: MmioScalarType,
    pub access: MmioRegAccess,
}

/// `device NAME at ADDR { ... }` — replaces `mmio NAME = ADDR` + `mmio_reg` pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDecl {
    pub name: String,
    pub base_addr: String,
    pub registers: Vec<DeviceRegDecl>,
    pub source: SourceNote,
}

/// Assignment target — either a local variable or a device register field.
#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Ident(String),                                 // local variable
    DeviceField { device: String, field: String }, // UART0.Data = b
}

/// Arithmetic operation for `cell_<op><T>` statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// A function parameter type — either a scalar or a fat-pointer slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamTy {
    /// Single integer value (u8 / u16 / u32 / u64).
    Scalar(MmioScalarType),
    /// Fat-pointer slice `[T]`: passed as (ptr: u64, len: u64) pair under SysV ABI.
    Slice(MmioScalarType),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MmioAddrExpr {
    Ident(String),
    IntLiteral(String),
    IdentPlusOffset { base: String, offset: String },
}

impl MmioAddrExpr {
    pub fn as_source(&self) -> String {
        match self {
            Self::Ident(name) => name.clone(),
            Self::IntLiteral(value) => value.clone(),
            Self::IdentPlusOffset { base, offset } => format!("{base} + {offset}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MmioValueExpr {
    Ident(String),
    IntLiteral(String),
}

impl MmioValueExpr {
    pub fn as_source(&self) -> String {
        match self {
            Self::Ident(name) => name.clone(),
            Self::IntLiteral(value) => value.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Call(String),
    CallCapture {
        callee: String,
        slot: String,
    },
    Critical(Vec<Stmt>),
    Unsafe(Vec<Stmt>),
    YieldPoint,
    AllocPoint,
    BlockPoint,
    Acquire(String),
    Release(String),
    ReturnSlot {
        slot: String,
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
    StackCell {
        ty: MmioScalarType,
        cell: String,
    },
    CellStore {
        ty: MmioScalarType,
        cell: String,
        value: MmioValueExpr,
    },
    CellLoad {
        ty: MmioScalarType,
        cell: String,
        slot: String,
    },
    CellArithImm {
        ty: MmioScalarType,
        cell: String,
        op: ArithOp,
        imm: u64,
    },
    /// `slot_add/sub/and/or/xor/shl/shr<T>(dst, src)` — two-source slot arithmetic.
    CellArithSlot {
        ty: MmioScalarType,
        dst: String,
        src: String,
        op: ArithOp,
    },
    CallWithArgs {
        callee: String,
        args: Vec<MmioValueExpr>,
    },
    /// `tail_call(callee[, args...])` — jump to `callee` discarding the current frame.
    TailCall {
        callee: String,
        args: Vec<MmioValueExpr>,
    },
    MmioRead {
        ty: MmioScalarType,
        addr: MmioAddrExpr,
        capture: Option<String>,
    },
    MmioWrite {
        ty: MmioScalarType,
        addr: MmioAddrExpr,
        value: MmioValueExpr,
    },
    RawMmioRead {
        ty: MmioScalarType,
        addr: MmioAddrExpr,
        capture: Option<String>,
    },
    RawMmioWrite {
        ty: MmioScalarType,
        addr: MmioAddrExpr,
        value: MmioValueExpr,
    },
    /// `slice_len(slice, slot)` — loads the length component of a `[T]` param into `slot`.
    SliceLen {
        slice: String,
        slot: String,
    },
    /// `slice_ptr(slice, slot)` — loads the pointer component of a `[T]` param into `slot`.
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
    /// Load through raw pointer: `*(addr_var as TYPE) -> out_var`
    PtrLoad {
        ty: MmioScalarType,
        addr_var: String,
        out_var: String,
    },
    /// Store through raw pointer: `*(addr_var as TYPE) = value`
    PtrStore {
        ty: MmioScalarType,
        addr_var: String,
        value: Box<Expr>,
    },

    // ---- NEW SURFACE SYNTAX STATEMENTS ----
    /// `TypeKind name = expr` or `TypeKind name`
    VarDecl {
        ty: MmioScalarType,
        name: String,
        init: Option<Expr>,
    },
    /// `name = expr`
    Assign {
        target: AssignTarget,
        value: Expr,
    },
    /// `name op= expr`
    CompoundAssign {
        target: AssignTarget,
        op: BinOpKind,
        value: Expr,
    },
    /// `if cond { then } else { else_ }`
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>, // empty = no else
    },
    /// `while cond { body }`
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    /// `for i in start..end { body }`
    For {
        var: String,
        start: Expr,
        end: Expr,
        inclusive: bool,
        body: Vec<Stmt>,
    },
    /// `return expr` or bare `return`
    Return(Option<Expr>),
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `print("...")` — compiler intrinsic
    Print(String),
    /// `fn_name(args...)` as a statement (call whose return value is discarded)
    ExprStmt(Expr),
    /// `asm!(NAME)` — emit a named kernel intrinsic instruction. Only valid inside an unsafe block.
    InlineAsm(KernelIntrinsic),
    /// `@syscall(nr, arg0, arg1, ...)` used as a statement (return value discarded).
    SyscallStmt {
        args: Vec<Expr>,
    },
    /// `buf[index] = value` — write element to a slice parameter.
    SliceIndexWrite {
        slice: String,
        index: Box<Expr>,
        value: Expr,
    },
}

/// Named no-argument x86-64 kernel instructions that can be emitted with `asm!(NAME)`.
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl KernelIntrinsic {
    /// Parse a name (case-insensitive) into a `KernelIntrinsic`.
    pub fn parse_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "cli" => Some(Self::Cli),
            "sti" => Some(Self::Sti),
            "hlt" => Some(Self::Hlt),
            "nop" => Some(Self::Nop),
            "mfence" => Some(Self::Mfence),
            "sfence" => Some(Self::Sfence),
            "lfence" => Some(Self::Lfence),
            "wbinvd" => Some(Self::Wbinvd),
            "pause" => Some(Self::Pause),
            "int3" => Some(Self::Int3),
            "cpuid" => Some(Self::Cpuid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnAst {
    pub name: String,
    pub is_extern: bool,
    pub params: Vec<(String, ParamTy)>,
    pub return_ty: Option<MmioScalarType>, // None = void
    pub attrs: Vec<RawAttr>,
    pub body: Vec<Stmt>,
    pub source: SourceNote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MmioBaseDecl {
    pub name: String,
    pub addr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MmioRegisterDecl {
    pub base: String,
    pub name: String,
    pub offset: String,
    pub ty: MmioScalarType,
    pub access: MmioRegAccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstDecl {
    pub name: String,
    pub ty: MmioScalarType,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDecl {
    pub name: String,
    pub ty: MmioScalarType,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructField {
    pub name: String,
    pub ty: MmioScalarType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
}

impl StructDecl {
    /// Returns the byte offset of the named field within this struct, or `None`
    /// if the field does not exist. Layout is C-style: fields are packed in
    /// declaration order with no padding (explicit, deterministic).
    pub fn field_offset(&self, field_name: &str) -> Option<u64> {
        let mut offset: u64 = 0;
        for field in &self.fields {
            if field.name == field_name {
                return Some(offset);
            }
            offset += field.ty.byte_size() as u64;
        }
        None
    }

    /// Total byte size of the struct (sum of all field sizes, no padding).
    pub fn byte_size(&self) -> u64 {
        self.fields.iter().map(|f| f.ty.byte_size() as u64).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PercpuDecl {
    pub name: String,
    pub ty: MmioScalarType,
}

/// Module-level mutable static variable: `static TYPE NAME = LITERAL`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticVarDecl {
    pub name: String,
    pub ty: MmioScalarType,
    pub init_value: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModuleAst {
    pub module_caps: Vec<String>,
    pub mmio_bases: Vec<MmioBaseDecl>,
    pub mmio_registers: Vec<MmioRegisterDecl>,
    pub devices: Vec<DeviceDecl>, // NEW
    pub constants: Vec<ConstDecl>,
    pub enums: Vec<EnumDecl>,
    pub structs: Vec<StructDecl>,
    /// Lock class declarations: `spinlock NAME;`
    pub locks: Vec<String>, // RENAMED from spinlocks
    /// Per-cpu variable declarations: `percpu NAME: T;`
    pub percpu_vars: Vec<PercpuDecl>,
    /// Static (module-level mutable) variable declarations: `static TYPE NAME = LITERAL`
    pub static_vars: Vec<StaticVarDecl>,
    pub items: Vec<FnAst>,
    /// Optional per-file profile declaration: `#lang stable` or `#lang experimental`.
    /// `None` means no directive present; the caller's default profile applies.
    pub lang_profile: Option<String>,
    /// Optional numeric version declaration: `#lang 1.0`.
    /// `None` means no version directive present.
    pub lang_version: Option<(u32, u32)>,
}

pub fn parse_module(src: &str) -> Result<ModuleAst, Vec<String>> {
    let tokens = match Lexer::new(src).collect_all() {
        Ok(t) => t,
        Err(_) => {
            // Lex error (e.g. old syntax chars) — try old parser
            return Parser::new(src).parse_module();
        }
    };
    match TokParser::new(tokens).parse_module() {
        ok @ Ok(_) => ok,
        Err(new_errs) => {
            // TokParser failed — try old character-level parser as fallback
            match Parser::new(src).parse_module() {
                ok @ Ok(_) => ok,
                // Both failed: prefer TokParser errors for new-syntax files (they are
                // more contextual). Fall back to old-parser errors when TokParser hit
                // a construct it doesn't understand — signalled by "at top level"
                // (unknown module item, e.g. `mmio`) or "old-syntax intrinsic"
                // (typed intrinsic call like `mmio_write<u32>`).
                Err(_)
                    if !new_errs.is_empty()
                        && !new_errs.iter().any(|e| {
                            e.contains("at top level") || e.contains("old-syntax intrinsic")
                        }) =>
                {
                    Err(new_errs)
                }
                Err(old_errs) => Err(old_errs),
            }
        }
    }
}

pub fn split_csv(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0_i32;

    for ch in input.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                let piece = cur.trim();
                if !piece.is_empty() {
                    out.push(piece.to_string());
                }
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }

    let piece = cur.trim();
    if !piece.is_empty() {
        out.push(piece.to_string());
    }

    out
}

pub fn split_csv_allow_trailing_comma(input: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0_i32;
    let mut saw_top_level_comma = false;

    for ch in input.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                saw_top_level_comma = true;
                let piece = cur.trim();
                if piece.is_empty() {
                    return Err("expected list element before ','".to_string());
                }
                out.push(piece.to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }

    let piece = cur.trim();
    if piece.is_empty() {
        if input.trim().is_empty() {
            return Ok(Vec::new());
        }
        if saw_top_level_comma {
            return Ok(out);
        }
        return Ok(Vec::new());
    }

    out.push(piece.to_string());
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// Lexer — token-stream interface (new; replaces character-level parser in Task 5)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Fn,
    Extern,
    Return,
    Break,
    Continue,
    If,
    Else,
    While,
    For,
    In,
    Const,
    Struct,
    Enum,
    Device,
    At,
    Lock,
    Percpu,
    Static,
    Acquire,
    Release,
    Critical,
    Unsafe,
    Yieldpoint,
    Print,
    RawWrite,
    RawRead,
    True,
    False,
    // Type keywords — carry the resolved type
    TypeKw(MmioScalarType),
    StringKw, // `string` type keyword (special: []char)
    // Literals
    IntLit(u64),
    FloatLit(f64),
    CharLit(u8),
    StrLit(String),
    // Identifier
    Ident(String),
    // Punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Dot,
    DotDot,
    DotDotEq,
    Arrow, // `->`
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Bang,
    Shl,
    Shr,
    Eq,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    AmpAmp,
    PipePipe,
    // Compound assignment
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    AmpEq,
    PipeEq,
    CaretEq,
    ShlEq,
    ShrEq,
    // Attributes
    AtSign, // `@` before attribute names
    // Directive prefix
    Hash, // `#` for #lang directives
    // End of file
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub source: SourceNote,
}

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    pub fn collect_all(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = matches!(tok.kind, TokenKind::Eof);
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.peek_char().map(|c| c.is_whitespace()).unwrap_or(false) {
                self.advance();
            }
            if self.src[self.pos..].starts_with("//") {
                while self.peek_char().map(|c| c != '\n').unwrap_or(false) {
                    self.advance();
                }
                continue;
            }
            if self.src[self.pos..].starts_with("/*") {
                self.pos += 2;
                while self.pos < self.src.len() {
                    if self.src[self.pos..].starts_with("*/") {
                        self.pos += 2;
                        break;
                    }
                    self.advance();
                }
                continue;
            }
            break;
        }
    }

    fn source_note(&self) -> SourceNote {
        SourceNote::from_source(self.src, self.pos)
    }

    fn lex_string_literal(&mut self) -> Result<String, String> {
        let mut s = String::new();
        loop {
            match self.advance() {
                None | Some('\n') => return Err("unterminated string literal".into()),
                Some('"') => break,
                Some('\\') => s.push(self.lex_escape()?),
                Some(c) => s.push(c),
            }
        }
        Ok(s)
    }

    fn lex_char_literal(&mut self) -> Result<u8, String> {
        let ch = match self.advance() {
            None => return Err("unterminated char literal".into()),
            Some('\\') => self.lex_escape()?,
            Some(c) => c,
        };
        match self.advance() {
            Some('\'') => Ok(ch as u8),
            _ => Err("char literal must be exactly one character".into()),
        }
    }

    fn lex_escape(&mut self) -> Result<char, String> {
        match self.advance() {
            Some('n') => Ok('\n'),
            Some('r') => Ok('\r'),
            Some('t') => Ok('\t'),
            Some('b') => Ok('\x08'),
            Some('a') => Ok('\x07'),
            Some('f') => Ok('\x0C'),
            Some('v') => Ok('\x0B'),
            Some('\\') => Ok('\\'),
            Some('\'') => Ok('\''),
            Some('"') => Ok('"'),
            Some('0') => Ok('\0'),
            Some('x') => {
                let h1 = self.advance().ok_or("expected hex digit after \\x")?;
                let h2 = self.advance().ok_or("expected two hex digits after \\x")?;
                let hex = format!("{}{}", h1, h2);
                let byte = u8::from_str_radix(&hex, 16)
                    .map_err(|_| format!("invalid hex escape \\x{}", hex))?;
                Ok(byte as char)
            }
            Some(c) => Err(format!("unknown escape sequence '\\{}'", c)),
            None => Err("unexpected end of file in escape".into()),
        }
    }

    fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace_and_comments();
        let note = self.source_note();
        let kind = match self.advance() {
            None => TokenKind::Eof,
            Some('{') => TokenKind::LBrace,
            Some('}') => TokenKind::RBrace,
            Some('(') => TokenKind::LParen,
            Some(')') => TokenKind::RParen,
            Some('[') => TokenKind::LBracket,
            Some(']') => TokenKind::RBracket,
            Some(',') => TokenKind::Comma,
            Some(':') => TokenKind::Colon,
            Some(';') => TokenKind::Semicolon,
            Some('~') => TokenKind::Tilde,
            Some('@') => TokenKind::AtSign,
            Some('"') => TokenKind::StrLit(self.lex_string_literal()?),
            Some('\'') => TokenKind::CharLit(self.lex_char_literal()?),
            Some('.') => {
                if self.src[self.pos..].starts_with(".=") {
                    self.pos += 2;
                    TokenKind::DotDotEq
                } else if self.src[self.pos..].starts_with('.') {
                    self.pos += 1;
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }
            Some('-') => {
                if self.src[self.pos..].starts_with('>') {
                    self.pos += 1;
                    TokenKind::Arrow
                } else if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }
            Some('+') => {
                if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            Some('*') => {
                if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            Some('/') => {
                if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            Some('%') => {
                if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::PercentEq
                } else {
                    TokenKind::Percent
                }
            }
            Some('^') => {
                if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::CaretEq
                } else {
                    TokenKind::Caret
                }
            }
            Some('!') => {
                if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            Some('=') => {
                if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::EqEq
                } else {
                    TokenKind::Eq
                }
            }
            Some('<') => {
                // After consuming '<':
                // - remaining "<=": input was "<<=", produce ShlEq (advance 2 more)
                // - remaining "=":  input was "<=",  produce LtEq  (advance 1 more)
                // - remaining "<":  input was "<<",  produce Shl   (advance 1 more)
                // - otherwise:      input was "<",   produce Lt
                if self.src[self.pos..].starts_with("<=") {
                    self.pos += 2;
                    TokenKind::ShlEq
                } else if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::LtEq
                } else if self.src[self.pos..].starts_with('<') {
                    self.pos += 1;
                    TokenKind::Shl
                } else {
                    TokenKind::Lt
                }
            }
            Some('>') => {
                // After consuming '>':
                // - remaining ">=": input was ">>=", produce ShrEq (advance 2 more)
                // - remaining "=":  input was ">=",  produce GtEq  (advance 1 more)
                // - remaining ">":  input was ">>",  produce Shr   (advance 1 more)
                // - otherwise:      input was ">",   produce Gt
                if self.src[self.pos..].starts_with(">=") {
                    self.pos += 2;
                    TokenKind::ShrEq
                } else if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::GtEq
                } else if self.src[self.pos..].starts_with('>') {
                    self.pos += 1;
                    TokenKind::Shr
                } else {
                    TokenKind::Gt
                }
            }
            Some('&') => {
                if self.src[self.pos..].starts_with('&') {
                    self.pos += 1;
                    TokenKind::AmpAmp
                } else if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::AmpEq
                } else {
                    TokenKind::Amp
                }
            }
            Some('|') => {
                if self.src[self.pos..].starts_with('|') {
                    self.pos += 1;
                    TokenKind::PipePipe
                } else if self.src[self.pos..].starts_with('=') {
                    self.pos += 1;
                    TokenKind::PipeEq
                } else {
                    TokenKind::Pipe
                }
            }
            Some(c) if c.is_ascii_digit() => {
                let start = self.pos - 1;
                if c == '0' && self.src[self.pos..].starts_with('x') {
                    self.pos += 1;
                    while self
                        .peek_char()
                        .map(|c| c.is_ascii_hexdigit())
                        .unwrap_or(false)
                    {
                        self.advance();
                    }
                    let s = &self.src[start..self.pos];
                    let n = u64::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())?;
                    TokenKind::IntLit(n)
                } else if c == '0' && self.src[self.pos..].starts_with('b') {
                    self.pos += 1;
                    while self
                        .peek_char()
                        .map(|c| c == '0' || c == '1')
                        .unwrap_or(false)
                    {
                        self.advance();
                    }
                    let s = &self.src[start..self.pos];
                    let n = u64::from_str_radix(&s[2..], 2).map_err(|e| e.to_string())?;
                    TokenKind::IntLit(n)
                } else {
                    while self
                        .peek_char()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                    {
                        self.advance();
                    }
                    // Float if followed by '.' then digit
                    let is_float = self.peek_char() == Some('.')
                        && self.src[self.pos + 1..]
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false);
                    if is_float {
                        self.advance(); // consume '.'
                        while self
                            .peek_char()
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                        {
                            self.advance();
                        }
                        if self.peek_char() == Some('e') || self.peek_char() == Some('E') {
                            self.advance();
                            if self.peek_char() == Some('-') || self.peek_char() == Some('+') {
                                self.advance();
                            }
                            while self
                                .peek_char()
                                .map(|c| c.is_ascii_digit())
                                .unwrap_or(false)
                            {
                                self.advance();
                            }
                        }
                        let f: f64 = self.src[start..self.pos]
                            .parse()
                            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
                        TokenKind::FloatLit(f)
                    } else {
                        let n: u64 = self.src[start..self.pos]
                            .parse()
                            .map_err(|e: std::num::ParseIntError| e.to_string())?;
                        TokenKind::IntLit(n)
                    }
                }
            }
            Some(c) if c.is_alphabetic() || c == '_' => {
                let start = self.pos - c.len_utf8();
                while self
                    .peek_char()
                    .map(|c| c.is_alphanumeric() || c == '_')
                    .unwrap_or(false)
                {
                    self.advance();
                }
                let word = &self.src[start..self.pos];
                match word {
                    "fn" => TokenKind::Fn,
                    "extern" => TokenKind::Extern,
                    "return" => TokenKind::Return,
                    "break" => TokenKind::Break,
                    "continue" => TokenKind::Continue,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "while" => TokenKind::While,
                    "for" => TokenKind::For,
                    "in" => TokenKind::In,
                    "const" => TokenKind::Const,
                    "struct" => TokenKind::Struct,
                    "enum" => TokenKind::Enum,
                    "device" => TokenKind::Device,
                    "at" => TokenKind::At,
                    "lock" => TokenKind::Lock,
                    "percpu" => TokenKind::Percpu,
                    "static" => TokenKind::Static,
                    "acquire" => TokenKind::Acquire,
                    "release" => TokenKind::Release,
                    "critical" => TokenKind::Critical,
                    "unsafe" => TokenKind::Unsafe,
                    "yieldpoint" => TokenKind::Yieldpoint,
                    "print" => TokenKind::Print,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "string" => TokenKind::StringKw,
                    _ => match MmioScalarType::parse(word) {
                        Ok(ty) => TokenKind::TypeKw(ty),
                        Err(_) => TokenKind::Ident(word.to_string()),
                    },
                }
            }
            Some('#') => TokenKind::Hash,
            Some(c) => return Err(format!("unexpected character '{}'", c)),
        };
        Ok(Token { kind, source: note })
    }
}

/// Token-stream parser. Replaces the old character-level `Parser` (in Task 5).
pub struct TokParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl TokParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or(self.tokens.last().unwrap())
    }

    pub fn peek_at(&self, offset: usize) -> &Token {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx]
    }

    pub fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    pub fn expect_kind(&mut self, kind: &TokenKind) -> Result<Token, String> {
        let t = self.peek().clone();
        if std::mem::discriminant(&t.kind) == std::mem::discriminant(kind) {
            self.advance();
            Ok(t)
        } else {
            Err(format!(
                "expected '{}' but found '{}' at {}:{}",
                token_kind_to_str(kind),
                token_kind_to_str(&t.kind),
                t.source.line,
                t.source.column
            ))
        }
    }

    pub fn at(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    pub fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Pratt expression parser. `min_bp` is the minimum binding power (0 = parse all).
    pub fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        // --- prefix ---
        let expr_src = self.peek().source.clone();
        let mut lhs = match self.advance().kind.clone() {
            TokenKind::IntLit(n) => Expr::IntLiteral(n),
            TokenKind::FloatLit(f) => Expr::FloatLiteral(f),
            TokenKind::CharLit(c) => Expr::CharLiteral(c),
            TokenKind::StrLit(s) => Expr::StringLiteral(s),
            TokenKind::True => Expr::BoolLiteral(true),
            TokenKind::False => Expr::BoolLiteral(false),
            TokenKind::Bang => {
                let (_, rbp) = prefix_bp(UnOpKind::Not);
                let e = self.parse_expr(rbp)?;
                Expr::UnOp {
                    op: UnOpKind::Not,
                    operand: Box::new(e),
                }
            }
            TokenKind::Tilde => {
                let (_, rbp) = prefix_bp(UnOpKind::BitNot);
                let e = self.parse_expr(rbp)?;
                Expr::UnOp {
                    op: UnOpKind::BitNot,
                    operand: Box::new(e),
                }
            }
            TokenKind::Minus => {
                let (_, rbp) = prefix_bp(UnOpKind::Neg);
                let e = self.parse_expr(rbp)?;
                Expr::UnOp {
                    op: UnOpKind::Neg,
                    operand: Box::new(e),
                }
            }
            TokenKind::LParen => {
                let e = self.parse_expr(0)?;
                self.expect_kind(&TokenKind::RParen)?;
                e
            }
            TokenKind::Ident(name) => {
                if self.eat(&TokenKind::Dot) {
                    // UART0.Status  or  buf.len
                    let field = match self.advance().kind.clone() {
                        TokenKind::Ident(f) => f,
                        other => {
                            return Err(format!(
                                "expected field name after '.', got '{}'",
                                token_kind_to_str(&other)
                            ));
                        }
                    };
                    if field == "len" {
                        Expr::SliceLen(name)
                    } else {
                        Expr::DeviceField {
                            device: name,
                            field,
                        }
                    }
                } else if self.eat(&TokenKind::LParen) {
                    let mut args = Vec::new();
                    while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                        args.push(self.parse_expr(0)?);
                        if self.at(&TokenKind::RParen) || self.at(&TokenKind::Eof) {
                            break;
                        }
                        if !self.eat(&TokenKind::Comma) {
                            let tok = self.peek();
                            return Err(format_source_diagnostic(
                                &tok.source.clone(),
                                &format!(
                                    "expected ',' or ')' after argument, got '{}'",
                                    token_kind_to_str(&tok.kind.clone())
                                ),
                                Some("add a ',' between arguments"),
                            ));
                        }
                    }
                    self.expect_kind(&TokenKind::RParen)?;
                    Expr::Call { callee: name, args }
                } else if self.eat(&TokenKind::LBracket) {
                    let index = self.parse_expr(0)?;
                    self.expect_kind(&TokenKind::RBracket)?;
                    Expr::SliceIndex {
                        slice: name,
                        index: Box::new(index),
                    }
                } else {
                    Expr::Ident(name)
                }
            }
            // @syscall(nr, arg0, arg1, ...) — syscall intrinsic in expression position
            TokenKind::AtSign => {
                // `@` already consumed; expect `syscall` `(` args... `)`
                match self.peek().kind.clone() {
                    TokenKind::Ident(ref id) if id == "syscall" => {
                        self.advance(); // consume `syscall`
                        self.expect_kind(&TokenKind::LParen)?;
                        let mut args = Vec::new();
                        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                            args.push(self.parse_expr(0)?);
                            if self.at(&TokenKind::RParen) || self.at(&TokenKind::Eof) {
                                break;
                            }
                            if !self.eat(&TokenKind::Comma) {
                                let tok = self.peek();
                                return Err(format_source_diagnostic(
                                    &tok.source.clone(),
                                    &format!(
                                        "expected ',' or ')' after @syscall argument, got '{}'",
                                        token_kind_to_str(&tok.kind.clone())
                                    ),
                                    Some("add a ',' between arguments"),
                                ));
                            }
                        }
                        self.expect_kind(&TokenKind::RParen)?;
                        if args.is_empty() {
                            return Err(format_source_diagnostic(
                                &expr_src,
                                "@syscall requires at least 1 argument (the syscall number)",
                                None,
                            ));
                        }
                        if args.len() > 7 {
                            return Err(format_source_diagnostic(
                                &expr_src,
                                &format!(
                                    "@syscall accepts at most 7 arguments (nr + 6 args), got {}",
                                    args.len()
                                ),
                                None,
                            ));
                        }
                        Expr::Syscall { args }
                    }
                    _ => {
                        return Err(format_source_diagnostic(
                            &expr_src,
                            "expected 'syscall' after '@' in expression",
                            Some("use @syscall(nr, ...) for syscall intrinsics"),
                        ));
                    }
                }
            }
            other => {
                return Err(format_source_diagnostic(
                    &expr_src,
                    &format!("expected expression, got '{}'", token_kind_to_str(&other)),
                    None,
                ));
            }
        };

        // --- infix ---
        while let Some(op) = token_to_binop(&self.peek().kind) {
            let (lbp, rbp) = infix_bp(op);
            if lbp < min_bp {
                break;
            }
            // If we see `*` followed by `( ident as`, this is a PtrLoad/PtrStore
            // statement, not a multiplication. Stop consuming infix operators.
            if matches!(self.peek().kind, TokenKind::Star)
                && matches!(self.peek_at(1).kind, TokenKind::LParen)
                && matches!(self.peek_at(2).kind, TokenKind::Ident(_))
                && matches!(&self.peek_at(3).kind, TokenKind::Ident(kw) if kw == "as")
            {
                break;
            }
            self.advance();
            let rhs = self.parse_expr(rbp)?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    pub fn parse_module(&mut self) -> Result<ModuleAst, Vec<String>> {
        let mut module = ModuleAst::default();
        let mut errors: Vec<String> = Vec::new();
        let mut pending_attrs: Vec<RawAttr> = Vec::new();

        // Check for optional `#lang PROFILE` directive at the very start of the file.
        if self.at(&TokenKind::Hash) {
            let hash_src = self.peek().source.clone();
            self.advance(); // consume `#`
            match self.peek().kind.clone() {
                TokenKind::Ident(ref kw) if kw == "lang" => {
                    self.advance(); // consume `lang`
                    match self.peek().kind.clone() {
                        // Handle numeric version like `#lang 1.0` (lexed as a float literal)
                        TokenKind::FloatLit(v) => {
                            self.advance(); // consume the float token
                            let major = v.floor() as u32;
                            let minor = ((v - v.floor()) * 10.0).round() as u32;
                            module.lang_version = Some((major, minor));
                        }
                        // Handle profile words like `#lang stable`
                        TokenKind::Ident(profile_word) => {
                            self.advance(); // consume the profile word
                            module.lang_profile = Some(profile_word);
                        }
                        _ => {
                            errors.push(format_source_diagnostic(
                                &hash_src,
                                "#lang directive requires a profile name ('stable' or 'experimental') or version ('1.0')",
                                None,
                            ));
                        }
                    }
                }
                _ => {
                    errors.push(format_source_diagnostic(
                        &hash_src,
                        "unexpected '#' — did you mean '#lang stable' or '#lang experimental'?",
                        None,
                    ));
                }
            }
        }

        while !self.at(&TokenKind::Eof) {
            // Skip stray semicolons (backward compat)
            if self.eat(&TokenKind::Semicolon) {
                continue;
            }

            match self.peek().kind.clone() {
                // @annotation — may be module_caps or function annotation
                TokenKind::AtSign => match self.parse_attr_tok() {
                    Ok(attr) => {
                        if attr.name == "module_caps" {
                            let args_str = attr.args.as_deref().unwrap_or("");
                            match split_csv_allow_trailing_comma(args_str) {
                                Ok(caps) => {
                                    for cap in caps {
                                        module.module_caps.push(cap);
                                    }
                                }
                                Err(_) => {
                                    errors.push(format_source_diagnostic(
                                        &attr.source,
                                        "@module_caps(...) contains an empty capability entry",
                                        None,
                                    ));
                                }
                            }
                        } else {
                            pending_attrs.push(attr);
                        }
                    }
                    Err(e) => {
                        errors.push(e);
                        self.skip_to_next_item();
                    }
                },
                // fn
                TokenKind::Fn => {
                    self.advance();
                    match self.parse_fn_tok(std::mem::take(&mut pending_attrs), false) {
                        Ok(f) => module.items.push(f),
                        Err(e) => {
                            errors.push(e);
                            self.skip_to_next_item();
                        }
                    }
                }
                // extern fn
                TokenKind::Extern => {
                    self.advance();
                    if !self.eat(&TokenKind::Fn) {
                        errors.push(format!(
                            "expected 'fn' after 'extern' at {}:{}",
                            self.peek().source.line,
                            self.peek().source.column
                        ));
                        self.skip_to_next_item();
                        continue;
                    }
                    match self.parse_fn_tok(std::mem::take(&mut pending_attrs), true) {
                        Ok(f) => module.items.push(f),
                        Err(e) => {
                            errors.push(e);
                            self.skip_to_next_item();
                        }
                    }
                }
                // device NAME at ADDR { ... }
                TokenKind::Device => {
                    self.advance();
                    match self.parse_device_tok() {
                        Ok(d) => module.devices.push(d),
                        Err(e) => {
                            errors.push(e);
                            self.skip_to_next_item();
                        }
                    }
                }
                // lock NAME
                TokenKind::Lock => {
                    self.advance();
                    match self.peek().kind.clone() {
                        TokenKind::Ident(name) => {
                            self.advance();
                            module.locks.push(name);
                        }
                        _ => errors.push(format!(
                            "expected lock name at {}:{}",
                            self.peek().source.line,
                            self.peek().source.column
                        )),
                    }
                    self.eat(&TokenKind::Semicolon);
                }
                // const TYPE NAME = VALUE
                TokenKind::Const => {
                    self.advance();
                    match self.parse_const_tok() {
                        Ok(c) => module.constants.push(c),
                        Err(e) => {
                            errors.push(e);
                            self.skip_to_next_item();
                        }
                    }
                }
                // percpu NAME : TYPE
                TokenKind::Percpu => {
                    self.advance();
                    match self.parse_percpu_tok() {
                        Ok(p) => module.percpu_vars.push(p),
                        Err(e) => {
                            errors.push(e);
                            self.skip_to_next_item();
                        }
                    }
                }
                // static TYPE NAME = LITERAL
                TokenKind::Static => {
                    self.advance();
                    match self.parse_static_var_tok() {
                        Ok(s) => module.static_vars.push(s),
                        Err(e) => {
                            errors.push(e);
                            self.skip_to_next_item();
                        }
                    }
                }
                other => {
                    errors.push(format!(
                        "unexpected '{}' at top level ({}:{})",
                        token_kind_to_str(&other),
                        self.peek().source.line,
                        self.peek().source.column
                    ));
                    self.skip_to_next_item();
                }
            }
        }

        if errors.is_empty() {
            Ok(module)
        } else {
            Err(errors)
        }
    }

    /// Parse `@name(args)` — the leading `@` has not been consumed.
    fn parse_attr_tok(&mut self) -> Result<RawAttr, String> {
        let note = self.peek().source.clone();
        self.expect_kind(&TokenKind::AtSign)?;
        let name = match self.advance().kind.clone() {
            TokenKind::Ident(n) => n,
            other => {
                return Err(format!(
                    "expected attribute name, got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        let args = if self.eat(&TokenKind::LParen) {
            let mut depth = 1i32;
            let mut s = String::new();
            loop {
                match self.advance().kind.clone() {
                    TokenKind::LParen => {
                        depth += 1;
                        s.push('(');
                    }
                    TokenKind::RParen => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        s.push(')');
                    }
                    TokenKind::Eof => return Err("unterminated attribute".into()),
                    kind => {
                        s.push_str(&token_kind_to_str(&kind));
                    }
                }
            }
            Some(s)
        } else {
            None
        };
        self.eat(&TokenKind::Semicolon);
        Ok(RawAttr {
            name,
            args,
            source: note,
        })
    }

    /// Parse `fn name(params) -> ty { body }` — `fn` keyword already consumed.
    fn parse_fn_tok(&mut self, attrs: Vec<RawAttr>, is_extern: bool) -> Result<FnAst, String> {
        let source = self.peek().source.clone();
        let name = match self.advance().kind.clone() {
            TokenKind::Ident(n) => n,
            other => {
                return Err(format!(
                    "expected function name, got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        self.expect_kind(&TokenKind::LParen)?;
        let params = self.parse_param_list_tok()?;
        self.expect_kind(&TokenKind::RParen)?;

        // Optional `-> type`
        let return_ty = if self.eat(&TokenKind::Arrow) {
            let ty_src = self.peek().source.clone();
            match self.advance().kind.clone() {
                TokenKind::TypeKw(ty) => Some(ty),
                TokenKind::LBrace => {
                    return Err(format_source_diagnostic(
                        &ty_src,
                        "expected return type after '->'; valid types: u8, u16, u32, u64, bool",
                        Some("add a return type here, e.g. `-> u64`"),
                    ));
                }
                other => {
                    return Err(format_source_diagnostic(
                        &ty_src,
                        &format!(
                            "expected return type after '->', got '{}'; valid types: u8, u16, u32, u64, bool",
                            token_kind_to_str(&other)
                        ),
                        None,
                    ));
                }
            }
        } else {
            None
        };

        if is_extern {
            self.eat(&TokenKind::Semicolon);
            return Ok(FnAst {
                name,
                is_extern: true,
                params,
                return_ty,
                attrs,
                body: Vec::new(),
                source,
            });
        }

        self.expect_kind(&TokenKind::LBrace)?;
        let body = self.parse_block_tok()?;
        Ok(FnAst {
            name,
            is_extern: false,
            params,
            return_ty,
            attrs,
            body,
            source,
        })
    }

    /// Parse parameter list: `TYPE name, ...` — inside `(` and `)`.
    fn parse_param_list_tok(&mut self) -> Result<Vec<(String, ParamTy)>, String> {
        let mut params = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
            let is_slice = self.eat(&TokenKind::LBracket);
            let ty = match self.advance().kind.clone() {
                TokenKind::TypeKw(t) => t,
                other => {
                    return Err(format!(
                        "expected parameter type, got '{}'",
                        token_kind_to_str(&other)
                    ));
                }
            };
            if is_slice {
                self.expect_kind(&TokenKind::RBracket)?;
            }
            let name = match self.advance().kind.clone() {
                TokenKind::Ident(n) => n,
                other => {
                    return Err(format!(
                        "expected parameter name, got '{}'",
                        token_kind_to_str(&other)
                    ));
                }
            };
            let param_ty = if is_slice {
                ParamTy::Slice(ty)
            } else {
                ParamTy::Scalar(ty)
            };
            params.push((name, param_ty));
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        Ok(params)
    }

    /// Parse a block body `stmt* }` — `{` already consumed.
    fn parse_block_tok(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.eat(&TokenKind::Semicolon) {
                continue;
            }
            if self.at(&TokenKind::RBrace) {
                break;
            }
            match self.parse_stmt_tok() {
                Ok(s) => stmts.push(s),
                Err(e) => return Err(e),
            }
            self.eat(&TokenKind::Semicolon);
        }
        self.expect_kind(&TokenKind::RBrace)?;
        Ok(stmts)
    }

    /// Parse a single statement.
    fn parse_stmt_tok(&mut self) -> Result<Stmt, String> {
        let note = self.peek().source.clone();
        match self.peek().kind.clone() {
            // Variable declaration: `TypeKw name = expr` or `TypeKw name`
            TokenKind::TypeKw(ty) => {
                self.advance();
                let name = match self.advance().kind.clone() {
                    TokenKind::Ident(n) => n,
                    other => {
                        return Err(format!(
                            "expected variable name after type, got '{}'",
                            token_kind_to_str(&other)
                        ));
                    }
                };
                let init = if self.eat(&TokenKind::Eq) {
                    Some(self.parse_expr(0)?)
                } else {
                    None
                };
                Ok(Stmt::VarDecl { ty, name, init })
            }
            // if expr { body } else { body }
            TokenKind::If => {
                self.advance();
                if self.at(&TokenKind::LBrace) {
                    return Err(format_source_diagnostic(
                        &note,
                        "expected condition after 'if', found '{'",
                        Some("add a boolean expression before the '{', e.g. `if x > 0 {`"),
                    ));
                }
                let cond = self.parse_expr(0)?;
                self.expect_kind(&TokenKind::LBrace)?;
                let then_body = self.parse_block_tok()?;
                let else_body = if self.eat(&TokenKind::Else) {
                    self.expect_kind(&TokenKind::LBrace)?;
                    self.parse_block_tok()?
                } else {
                    Vec::new()
                };
                Ok(Stmt::If {
                    cond,
                    then_body,
                    else_body,
                })
            }
            // while expr { body }
            TokenKind::While => {
                self.advance();
                let cond = self.parse_expr(0)?;
                self.expect_kind(&TokenKind::LBrace)?;
                let body = self.parse_block_tok()?;
                Ok(Stmt::While { cond, body })
            }
            // for var in start..end { body }
            TokenKind::For => {
                self.advance();
                let var = match self.advance().kind.clone() {
                    TokenKind::Ident(n) => n,
                    other => {
                        return Err(format!(
                            "expected loop variable, got '{}'",
                            token_kind_to_str(&other)
                        ));
                    }
                };
                self.expect_kind(&TokenKind::In)?;
                let start = self.parse_expr(0)?;
                let inclusive = if self.eat(&TokenKind::DotDotEq) {
                    true
                } else {
                    self.expect_kind(&TokenKind::DotDot)?;
                    false
                };
                let end = self.parse_expr(0)?;
                self.expect_kind(&TokenKind::LBrace)?;
                let body = self.parse_block_tok()?;
                Ok(Stmt::For {
                    var,
                    start,
                    end,
                    inclusive,
                    body,
                })
            }
            // return expr?
            TokenKind::Return => {
                self.advance();
                if self.at(&TokenKind::RBrace)
                    || self.at(&TokenKind::Semicolon)
                    || self.at(&TokenKind::Eof)
                {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.parse_expr(0)?)))
                }
            }
            TokenKind::Break => {
                self.advance();
                Ok(Stmt::Break)
            }
            TokenKind::Continue => {
                self.advance();
                Ok(Stmt::Continue)
            }
            // print("string")
            TokenKind::Print => {
                self.advance();
                self.expect_kind(&TokenKind::LParen)?;
                let s = match self.advance().kind.clone() {
                    TokenKind::StrLit(s) => s,
                    other => {
                        return Err(format!(
                            "print() requires a string literal, got '{}'",
                            token_kind_to_str(&other)
                        ));
                    }
                };
                self.expect_kind(&TokenKind::RParen)?;
                Ok(Stmt::Print(s))
            }
            // acquire(NAME) / release(NAME)
            TokenKind::Acquire => {
                self.advance();
                self.expect_kind(&TokenKind::LParen)?;
                let name = match self.advance().kind.clone() {
                    TokenKind::Ident(n) => n,
                    other => {
                        return Err(format!(
                            "expected lock name, got '{}'",
                            token_kind_to_str(&other)
                        ));
                    }
                };
                self.expect_kind(&TokenKind::RParen)?;
                Ok(Stmt::Acquire(name))
            }
            TokenKind::Release => {
                self.advance();
                self.expect_kind(&TokenKind::LParen)?;
                let name = match self.advance().kind.clone() {
                    TokenKind::Ident(n) => n,
                    other => {
                        return Err(format!(
                            "expected lock name, got '{}'",
                            token_kind_to_str(&other)
                        ));
                    }
                };
                self.expect_kind(&TokenKind::RParen)?;
                Ok(Stmt::Release(name))
            }
            // critical { } / unsafe { }
            TokenKind::Critical => {
                self.advance();
                self.expect_kind(&TokenKind::LBrace)?;
                let body = self.parse_block_tok()?;
                Ok(Stmt::Critical(body))
            }
            TokenKind::Unsafe => {
                self.advance();
                self.expect_kind(&TokenKind::LBrace)?;
                let body = self.parse_block_tok()?;
                Ok(Stmt::Unsafe(body))
            }
            // yieldpoint
            TokenKind::Yieldpoint => {
                self.advance();
                Ok(Stmt::YieldPoint)
            }
            // Ident — assignment, compound-assign, field-assign, or call
            // Also handles `raw_write` and `raw_read` which are tokenized as Ident
            TokenKind::Ident(name) => {
                // Catch common mistake: `let` is not a keyword in KernRift.
                if name == "let" {
                    return Err(format_source_diagnostic(
                        &note,
                        "KernRift has no `let` keyword; declare variables with their type, e.g. `u64 x = ...`",
                        None,
                    ));
                }
                // Check for asm!(NAME) — kernel intrinsic instruction.
                if name == "asm" && matches!(self.peek_at(1).kind, TokenKind::Bang) {
                    self.advance(); // consume `asm`
                    self.advance(); // consume `!`
                    self.expect_kind(&TokenKind::LParen)?;
                    let intr_name = match self.advance().kind.clone() {
                        TokenKind::Ident(n) => n,
                        other => {
                            return Err(format!(
                                "expected intrinsic name after asm!(, got '{}'",
                                token_kind_to_str(&other)
                            ));
                        }
                    };
                    self.expect_kind(&TokenKind::RParen)?;
                    match KernelIntrinsic::parse_name(&intr_name) {
                        Some(intr) => return Ok(Stmt::InlineAsm(intr)),
                        None => {
                            return Err(format!(
                                "unknown kernel intrinsic '{}'; supported: cli, sti, hlt, nop, mfence, sfence, lfence, wbinvd, pause, int3, cpuid",
                                intr_name
                            ));
                        }
                    }
                }
                // Check for raw_write<T>(addr, val) / raw_read<T>(addr, cap)
                if name == "raw_write" || name == "raw_read" {
                    // handled below after advance
                }
                self.advance();
                // raw intrinsics: raw_write<T>(addr, val) and raw_read<T>(addr, cap)
                if name == "raw_write" {
                    let ty = self.parse_type_param_tok()?;
                    self.expect_kind(&TokenKind::LParen)?;
                    let addr_expr = self.parse_mmio_addr_expr_tok()?;
                    self.expect_kind(&TokenKind::Comma)?;
                    let val_expr = self.parse_mmio_value_expr_tok()?;
                    self.expect_kind(&TokenKind::RParen)?;
                    return Ok(Stmt::RawMmioWrite {
                        ty,
                        addr: addr_expr,
                        value: val_expr,
                    });
                }
                if name == "raw_read" {
                    let ty = self.parse_type_param_tok()?;
                    self.expect_kind(&TokenKind::LParen)?;
                    let addr_expr = self.parse_mmio_addr_expr_tok()?;
                    let capture = if self.eat(&TokenKind::Comma) {
                        Some(match self.advance().kind.clone() {
                            TokenKind::Ident(n) => n,
                            other => {
                                return Err(format!(
                                    "expected capture slot, got '{}'",
                                    token_kind_to_str(&other)
                                ));
                            }
                        })
                    } else {
                        None
                    };
                    self.expect_kind(&TokenKind::RParen)?;
                    return Ok(Stmt::RawMmioRead {
                        ty,
                        addr: addr_expr,
                        capture,
                    });
                }
                // Check next token for assignment variants
                match self.peek().kind.clone() {
                    // name[expr] = expr — slice index write
                    TokenKind::LBracket => {
                        self.advance(); // consume [
                        let index = self.parse_expr(0)?;
                        self.expect_kind(&TokenKind::RBracket)?;
                        // Must be followed by = (assignment)
                        self.expect_kind(&TokenKind::Eq)?;
                        let value = self.parse_expr(0)?;
                        Ok(Stmt::SliceIndexWrite {
                            slice: name,
                            index: Box::new(index),
                            value,
                        })
                    }
                    // name = expr
                    TokenKind::Eq => {
                        self.advance();
                        let value = self.parse_expr(0)?;
                        Ok(Stmt::Assign {
                            target: AssignTarget::Ident(name),
                            value,
                        })
                    }
                    // name op= expr
                    ref k if compound_assign_op(k).is_some() => {
                        let op = compound_assign_op(&self.advance().kind.clone()).unwrap();
                        let value = self.parse_expr(0)?;
                        Ok(Stmt::CompoundAssign {
                            target: AssignTarget::Ident(name),
                            op,
                            value,
                        })
                    }
                    // name.field = expr
                    TokenKind::Dot => {
                        self.advance();
                        let field = match self.advance().kind.clone() {
                            TokenKind::Ident(f) => f,
                            other => {
                                return Err(format!(
                                    "expected field name after '.', got '{}'",
                                    token_kind_to_str(&other)
                                ));
                            }
                        };
                        self.expect_kind(&TokenKind::Eq)?;
                        let value = self.parse_expr(0)?;
                        Ok(Stmt::Assign {
                            target: AssignTarget::DeviceField {
                                device: name,
                                field,
                            },
                            value,
                        })
                    }
                    // name(args)
                    TokenKind::LParen => {
                        self.advance();
                        let mut args = Vec::new();
                        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                            args.push(self.parse_expr(0)?);
                            if self.at(&TokenKind::RParen) || self.at(&TokenKind::Eof) {
                                break;
                            }
                            if !self.eat(&TokenKind::Comma) {
                                let tok = self.peek();
                                return Err(format_source_diagnostic(
                                    &tok.source.clone(),
                                    &format!(
                                        "expected ',' or ')' after argument, got '{}'",
                                        token_kind_to_str(&tok.kind.clone())
                                    ),
                                    Some("add a ',' between arguments"),
                                ));
                            }
                        }
                        self.expect_kind(&TokenKind::RParen)?;
                        // Map well-known old-syntax zero-arg intrinsics to their Stmt variants.
                        if args.is_empty() {
                            match name.as_str() {
                                "allocpoint" => return Ok(Stmt::AllocPoint),
                                "blockpoint" => return Ok(Stmt::BlockPoint),
                                "yieldpoint" => return Ok(Stmt::YieldPoint),
                                _ => {}
                            }
                        }
                        // Reject known old-syntax intrinsic names so TokParser falls back
                        // to the old character-level parser which handles them correctly.
                        match name.as_str() {
                            "tail_call"
                            | "call_with_args"
                            | "call_capture"
                            | "return_slot"
                            | "branch_if_zero"
                            | "branch_if_eq"
                            | "branch_if_mask_nonzero"
                            | "mmio_read"
                            | "mmio_write"
                            | "raw_mmio_read"
                            | "raw_mmio_write"
                            | "stack_cell"
                            | "cell_store"
                            | "cell_load"
                            | "cell_add"
                            | "cell_sub"
                            | "cell_and"
                            | "cell_or"
                            | "cell_xor"
                            | "cell_shl"
                            | "cell_shr"
                            | "slot_add"
                            | "slot_sub"
                            | "slot_and"
                            | "slot_or"
                            | "slot_xor"
                            | "slot_shl"
                            | "slot_shr"
                            | "slice_len"
                            | "slice_ptr"
                            | "percpu_read"
                            | "percpu_write" => {
                                return Err(format!(
                                    "old-syntax intrinsic '{}' — use old parser",
                                    name
                                ));
                            }
                            _ => {}
                        }
                        if args.is_empty() {
                            Ok(Stmt::Call(name))
                        } else {
                            Ok(Stmt::ExprStmt(Expr::Call { callee: name, args }))
                        }
                    }
                    // `mmio UART0 = 0x1000;` — old-syntax device declaration used
                    // inside a function body. Return the signal so the old parser runs and
                    // produces "mmio declarations are only allowed at module scope".
                    TokenKind::Ident(_) if matches!(name.as_str(), "mmio" | "mmio_reg") => {
                        Err(format!("old-syntax intrinsic '{}' — use old parser", name))
                    }
                    // old-syntax intrinsic with a type parameter: `mmio_write<u32>(...)`.
                    // Returning "old-syntax intrinsic" ensures the old parser is preferred.
                    TokenKind::Lt
                        if matches!(
                            name.as_str(),
                            "mmio_read"
                                | "mmio_write"
                                | "raw_mmio_read"
                                | "raw_mmio_write"
                                | "stack_cell"
                                | "cell_store"
                                | "cell_load"
                                | "cell_add"
                                | "cell_sub"
                                | "cell_and"
                                | "cell_or"
                                | "cell_xor"
                                | "cell_shl"
                                | "cell_shr"
                                | "slot_add"
                                | "slot_sub"
                                | "slot_and"
                                | "slot_or"
                                | "slot_xor"
                                | "slot_shl"
                                | "slot_shr"
                                | "percpu_read"
                                | "percpu_write"
                        ) =>
                    {
                        Err(format!("old-syntax intrinsic '{}' — use old parser", name))
                    }
                    other => Err(format_source_diagnostic(
                        &note,
                        &format!(
                            "unexpected '{}' after '{}'; expected '=', '(', or '.'",
                            token_kind_to_str(&other),
                            name
                        ),
                        None,
                    )),
                }
            }
            // @syscall(nr, arg0, arg1, ...) — syscall intrinsic as statement
            TokenKind::AtSign => {
                // peek ahead: if `@` is followed by `syscall` `(`, parse as syscall stmt
                if matches!(self.peek_at(1).kind, TokenKind::Ident(ref id) if id == "syscall")
                    && matches!(self.peek_at(2).kind, TokenKind::LParen)
                {
                    self.advance(); // consume `@`
                    self.advance(); // consume `syscall`
                    self.advance(); // consume `(`
                    let mut args = Vec::new();
                    while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                        args.push(self.parse_expr(0)?);
                        if self.at(&TokenKind::RParen) || self.at(&TokenKind::Eof) {
                            break;
                        }
                        if !self.eat(&TokenKind::Comma) {
                            let tok = self.peek();
                            return Err(format_source_diagnostic(
                                &tok.source.clone(),
                                &format!(
                                    "expected ',' or ')' after @syscall argument, got '{}'",
                                    token_kind_to_str(&tok.kind.clone())
                                ),
                                Some("add a ',' between arguments"),
                            ));
                        }
                    }
                    self.expect_kind(&TokenKind::RParen)?;
                    if args.is_empty() {
                        return Err(format_source_diagnostic(
                            &note,
                            "@syscall requires at least 1 argument (the syscall number)",
                            None,
                        ));
                    }
                    if args.len() > 7 {
                        return Err(format_source_diagnostic(
                            &note,
                            &format!(
                                "@syscall accepts at most 7 arguments (nr + 6 args), got {}",
                                args.len()
                            ),
                            None,
                        ));
                    }
                    Ok(Stmt::SyscallStmt { args })
                } else {
                    Err(format_source_diagnostic(
                        &note,
                        "unexpected '@' in statement position; only @syscall(...) is supported here",
                        Some("use @syscall(nr, ...) for syscall intrinsics"),
                    ))
                }
            }
            // *(addr_var as TYPE) -> out_var   — PtrLoad
            // *(addr_var as TYPE) = expr       — PtrStore
            TokenKind::Star => {
                self.advance();
                self.expect_kind(&TokenKind::LParen)?;
                let addr_var = match self.advance().kind.clone() {
                    TokenKind::Ident(n) => n,
                    other => {
                        return Err(format!(
                            "expected identifier after *(, got '{}'",
                            token_kind_to_str(&other)
                        ));
                    }
                };
                // expect `as`
                match self.advance().kind.clone() {
                    TokenKind::Ident(kw) if kw == "as" => {}
                    other => {
                        return Err(format!(
                            "expected 'as' in ptr deref cast, got '{}'",
                            token_kind_to_str(&other)
                        ));
                    }
                }
                let ty = match self.advance().kind.clone() {
                    TokenKind::TypeKw(t) => t,
                    other => {
                        return Err(format!(
                            "expected type in ptr deref cast, got '{}'",
                            token_kind_to_str(&other)
                        ));
                    }
                };
                self.expect_kind(&TokenKind::RParen)?;
                // discriminate on -> vs =
                if self.eat(&TokenKind::Arrow) {
                    let out_var = match self.advance().kind.clone() {
                        TokenKind::Ident(n) => n,
                        other => {
                            return Err(format!(
                                "expected output slot after ->, got '{}'",
                                token_kind_to_str(&other)
                            ));
                        }
                    };
                    Ok(Stmt::PtrLoad {
                        ty,
                        addr_var,
                        out_var,
                    })
                } else {
                    self.expect_kind(&TokenKind::Eq)?;
                    let value = self.parse_expr(0)?;
                    Ok(Stmt::PtrStore {
                        ty,
                        addr_var,
                        value: Box::new(value),
                    })
                }
            }
            other => Err(format_source_diagnostic(
                &note,
                &format!(
                    "unexpected '{}'; expected a statement",
                    token_kind_to_str(&other)
                ),
                None,
            )),
        }
    }

    /// Parse `<TYPE>` type parameter for raw_write/raw_read intrinsics.
    fn parse_type_param_tok(&mut self) -> Result<MmioScalarType, String> {
        self.expect_kind(&TokenKind::Lt)?;
        let ty = match self.advance().kind.clone() {
            TokenKind::TypeKw(t) => t,
            other => {
                return Err(format!(
                    "expected type in <>, got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        self.expect_kind(&TokenKind::Gt)?;
        Ok(ty)
    }

    /// Parse an MMIO address expression.
    fn parse_mmio_addr_expr_tok(&mut self) -> Result<MmioAddrExpr, String> {
        match self.peek().kind.clone() {
            TokenKind::IntLit(n) => {
                self.advance();
                Ok(MmioAddrExpr::IntLiteral(format!("0x{:X}", n)))
            }
            TokenKind::Ident(base) => {
                self.advance();
                if self.eat(&TokenKind::Plus) {
                    match self.advance().kind.clone() {
                        TokenKind::IntLit(offset) => Ok(MmioAddrExpr::IdentPlusOffset {
                            base,
                            offset: format!("0x{:X}", offset),
                        }),
                        other => Err(format!(
                            "expected integer offset, got '{}'",
                            token_kind_to_str(&other)
                        )),
                    }
                } else {
                    Ok(MmioAddrExpr::Ident(base))
                }
            }
            other => Err(format!(
                "expected MMIO address expression, got '{}'",
                token_kind_to_str(&other)
            )),
        }
    }

    /// Parse an MMIO value expression.
    fn parse_mmio_value_expr_tok(&mut self) -> Result<MmioValueExpr, String> {
        match self.advance().kind.clone() {
            TokenKind::IntLit(n) => Ok(MmioValueExpr::IntLiteral(format!("0x{:X}", n))),
            TokenKind::Ident(n) => Ok(MmioValueExpr::Ident(n)),
            other => Err(format!(
                "expected value expression, got '{}'",
                token_kind_to_str(&other)
            )),
        }
    }

    /// Parse `device NAME at ADDR { regs... }` — `device` keyword already consumed.
    fn parse_device_tok(&mut self) -> Result<DeviceDecl, String> {
        let source = self.peek().source.clone();
        let name = match self.advance().kind.clone() {
            TokenKind::Ident(n) => n,
            other => {
                return Err(format!(
                    "expected device name, got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        self.expect_kind(&TokenKind::At)?;
        let base_addr = match self.advance().kind.clone() {
            TokenKind::IntLit(n) => format!("0x{:X}", n),
            TokenKind::Ident(s) => s,
            other => {
                return Err(format!(
                    "expected device base address, got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        self.expect_kind(&TokenKind::LBrace)?;
        let mut registers = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.eat(&TokenKind::Semicolon) {
                continue;
            }
            if self.at(&TokenKind::RBrace) {
                break;
            }
            let reg_name = match self.advance().kind.clone() {
                TokenKind::Ident(n) => n,
                other => {
                    return Err(format!(
                        "expected register name, got '{}'",
                        token_kind_to_str(&other)
                    ));
                }
            };
            self.expect_kind(&TokenKind::At)?;
            let offset = match self.advance().kind.clone() {
                TokenKind::IntLit(n) => format!("0x{:X}", n),
                TokenKind::Ident(s) => s,
                other => {
                    return Err(format!(
                        "expected register offset, got '{}'",
                        token_kind_to_str(&other)
                    ));
                }
            };
            self.expect_kind(&TokenKind::Colon)?;
            let ty = match self.advance().kind.clone() {
                TokenKind::TypeKw(t) => t,
                other => {
                    return Err(format!(
                        "expected register type, got '{}'",
                        token_kind_to_str(&other)
                    ));
                }
            };
            let access = match self.advance().kind.clone() {
                TokenKind::Ident(s) => match s.as_str() {
                    "rw" => MmioRegAccess::Rw,
                    "ro" => MmioRegAccess::Ro,
                    "wo" => MmioRegAccess::Wo,
                    other => return Err(format!("expected 'rw', 'ro', or 'wo', got '{}'", other)),
                },
                other => {
                    return Err(format!(
                        "expected access mode, got '{}'",
                        token_kind_to_str(&other)
                    ));
                }
            };
            self.eat(&TokenKind::Semicolon);
            registers.push(DeviceRegDecl {
                name: reg_name,
                offset,
                ty,
                access,
            });
        }
        self.expect_kind(&TokenKind::RBrace)?;
        Ok(DeviceDecl {
            name,
            base_addr,
            registers,
            source,
        })
    }

    /// Parse `const TYPE NAME = VALUE` — `const` keyword already consumed.
    fn parse_const_tok(&mut self) -> Result<ConstDecl, String> {
        // Support both `TYPE NAME = VALUE` (new) and `NAME : TYPE = VALUE` (old fallback)
        let (name, ty) = if let TokenKind::TypeKw(t) = self.peek().kind.clone() {
            self.advance();
            let name = match self.advance().kind.clone() {
                TokenKind::Ident(n) => n,
                other => {
                    return Err(format!(
                        "expected constant name, got '{}'",
                        token_kind_to_str(&other)
                    ));
                }
            };
            (name, t)
        } else {
            let name = match self.advance().kind.clone() {
                TokenKind::Ident(n) => n,
                other => {
                    return Err(format!(
                        "expected constant name, got '{}'",
                        token_kind_to_str(&other)
                    ));
                }
            };
            (name, MmioScalarType::U64)
        };
        self.expect_kind(&TokenKind::Eq)?;
        let value = match self.advance().kind.clone() {
            TokenKind::IntLit(n) => format!("{}", n),
            other => {
                return Err(format!(
                    "expected constant value, got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        self.eat(&TokenKind::Semicolon);
        Ok(ConstDecl { name, ty, value })
    }

    /// Parse `percpu NAME : TYPE` — `percpu` keyword already consumed.
    fn parse_percpu_tok(&mut self) -> Result<PercpuDecl, String> {
        let name = match self.advance().kind.clone() {
            TokenKind::Ident(n) => n,
            other => {
                return Err(format!(
                    "expected percpu variable name, got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        self.expect_kind(&TokenKind::Colon)?;
        let ty = match self.advance().kind.clone() {
            TokenKind::TypeKw(t) => t,
            other => {
                return Err(format!(
                    "expected type after ':', got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        self.eat(&TokenKind::Semicolon);
        Ok(PercpuDecl { name, ty })
    }

    /// Parse `static TYPE NAME = LITERAL` — `static` keyword already consumed.
    fn parse_static_var_tok(&mut self) -> Result<StaticVarDecl, String> {
        let ty = match self.peek().kind.clone() {
            TokenKind::TypeKw(t) => {
                self.advance();
                t
            }
            other => {
                return Err(format!(
                    "expected type after 'static', got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        let name = match self.advance().kind.clone() {
            TokenKind::Ident(n) => n,
            other => {
                return Err(format!(
                    "expected static variable name, got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        self.expect_kind(&TokenKind::Eq)?;
        let init_value = match self.advance().kind.clone() {
            TokenKind::IntLit(n) => n,
            other => {
                return Err(format!(
                    "expected integer literal for static initializer, got '{}'",
                    token_kind_to_str(&other)
                ));
            }
        };
        self.eat(&TokenKind::Semicolon);
        Ok(StaticVarDecl {
            name,
            ty,
            init_value,
        })
    }

    /// Skip to the next top-level item on error.
    fn skip_to_next_item(&mut self) {
        while !self.at(&TokenKind::Eof) {
            match self.peek().kind {
                TokenKind::Fn
                | TokenKind::Device
                | TokenKind::Lock
                | TokenKind::Const
                | TokenKind::Percpu
                | TokenKind::Static
                | TokenKind::Extern
                | TokenKind::AtSign => break,
                _ => {
                    self.advance();
                }
            }
        }
    }
}

fn token_to_binop(kind: &TokenKind) -> Option<BinOpKind> {
    Some(match kind {
        TokenKind::Plus => BinOpKind::Add,
        TokenKind::Minus => BinOpKind::Sub,
        TokenKind::Star => BinOpKind::Mul,
        TokenKind::Slash => BinOpKind::Div,
        TokenKind::Percent => BinOpKind::Rem,
        TokenKind::Amp => BinOpKind::And,
        TokenKind::Pipe => BinOpKind::Or,
        TokenKind::Caret => BinOpKind::Xor,
        TokenKind::Shl => BinOpKind::Shl,
        TokenKind::Shr => BinOpKind::Shr,
        TokenKind::EqEq => BinOpKind::Eq,
        TokenKind::BangEq => BinOpKind::Ne,
        TokenKind::Lt => BinOpKind::Lt,
        TokenKind::Gt => BinOpKind::Gt,
        TokenKind::LtEq => BinOpKind::Le,
        TokenKind::GtEq => BinOpKind::Ge,
        TokenKind::AmpAmp => BinOpKind::LogAnd,
        TokenKind::PipePipe => BinOpKind::LogOr,
        _ => return None,
    })
}

/// Returns (left_bp, right_bp) for infix operators.
/// Higher bp = tighter binding. Left-associative: rbp = lbp + 1.
fn infix_bp(op: BinOpKind) -> (u8, u8) {
    match op {
        BinOpKind::LogOr => (10, 11),
        BinOpKind::LogAnd => (20, 21),
        BinOpKind::Or => (30, 31),
        BinOpKind::Xor => (40, 41),
        BinOpKind::And => (50, 51),
        BinOpKind::Eq | BinOpKind::Ne => (60, 61),
        BinOpKind::Lt | BinOpKind::Gt | BinOpKind::Le | BinOpKind::Ge => (70, 71),
        BinOpKind::Shl | BinOpKind::Shr => (80, 81),
        BinOpKind::Add | BinOpKind::Sub => (90, 91),
        BinOpKind::Mul | BinOpKind::Div | BinOpKind::Rem => (100, 101),
    }
}

fn prefix_bp(op: UnOpKind) -> ((), u8) {
    match op {
        UnOpKind::Not | UnOpKind::BitNot | UnOpKind::Neg => ((), 110),
    }
}

/// Map a compound-assignment token to its BinOpKind (returns None if not a compound-assign).
fn compound_assign_op(kind: &TokenKind) -> Option<BinOpKind> {
    Some(match kind {
        TokenKind::PlusEq => BinOpKind::Add,
        TokenKind::MinusEq => BinOpKind::Sub,
        TokenKind::AmpEq => BinOpKind::And,
        TokenKind::PipeEq => BinOpKind::Or,
        TokenKind::CaretEq => BinOpKind::Xor,
        TokenKind::ShlEq => BinOpKind::Shl,
        TokenKind::ShrEq => BinOpKind::Shr,
        TokenKind::StarEq => BinOpKind::Mul,
        TokenKind::SlashEq => BinOpKind::Div,
        TokenKind::PercentEq => BinOpKind::Rem,
        _ => return None,
    })
}

/// Reconstruct approximate source text for a token kind (used in diagnostic messages).
fn token_kind_to_str(kind: &TokenKind) -> String {
    match kind {
        // Literals & identifier
        TokenKind::Ident(s) => s.clone(),
        TokenKind::IntLit(n) => n.to_string(),
        TokenKind::FloatLit(f) => f.to_string(),
        TokenKind::CharLit(c) => format!("'{}'", *c as char),
        TokenKind::StrLit(s) => format!("\"{}\"", s),
        TokenKind::TypeKw(ty) => ty.as_str().into(),
        // Keywords
        TokenKind::Fn => "fn".into(),
        TokenKind::Extern => "extern".into(),
        TokenKind::Return => "return".into(),
        TokenKind::Break => "break".into(),
        TokenKind::Continue => "continue".into(),
        TokenKind::If => "if".into(),
        TokenKind::Else => "else".into(),
        TokenKind::While => "while".into(),
        TokenKind::For => "for".into(),
        TokenKind::In => "in".into(),
        TokenKind::Const => "const".into(),
        TokenKind::Struct => "struct".into(),
        TokenKind::Enum => "enum".into(),
        TokenKind::Device => "device".into(),
        TokenKind::At => "at".into(),
        TokenKind::Lock => "lock".into(),
        TokenKind::Percpu => "percpu".into(),
        TokenKind::Static => "static".into(),
        TokenKind::Acquire => "acquire".into(),
        TokenKind::Release => "release".into(),
        TokenKind::Critical => "critical".into(),
        TokenKind::Unsafe => "unsafe".into(),
        TokenKind::Yieldpoint => "yieldpoint".into(),
        TokenKind::Print => "print".into(),
        TokenKind::RawWrite => "raw_write".into(),
        TokenKind::RawRead => "raw_read".into(),
        TokenKind::True => "true".into(),
        TokenKind::False => "false".into(),
        TokenKind::StringKw => "string".into(),
        // Punctuation
        TokenKind::LBrace => "{".into(),
        TokenKind::RBrace => "}".into(),
        TokenKind::LParen => "(".into(),
        TokenKind::RParen => ")".into(),
        TokenKind::LBracket => "[".into(),
        TokenKind::RBracket => "]".into(),
        TokenKind::Comma => ",".into(),
        TokenKind::Colon => ":".into(),
        TokenKind::Semicolon => ";".into(),
        TokenKind::Dot => ".".into(),
        TokenKind::DotDot => "..".into(),
        TokenKind::DotDotEq => "..=".into(),
        TokenKind::Arrow => "->".into(),
        // Operators
        TokenKind::Plus => "+".into(),
        TokenKind::Minus => "-".into(),
        TokenKind::Star => "*".into(),
        TokenKind::Slash => "/".into(),
        TokenKind::Percent => "%".into(),
        TokenKind::Amp => "&".into(),
        TokenKind::Pipe => "|".into(),
        TokenKind::Caret => "^".into(),
        TokenKind::Tilde => "~".into(),
        TokenKind::Bang => "!".into(),
        TokenKind::Shl => "<<".into(),
        TokenKind::Shr => ">>".into(),
        TokenKind::Eq => "=".into(),
        TokenKind::EqEq => "==".into(),
        TokenKind::BangEq => "!=".into(),
        TokenKind::Lt => "<".into(),
        TokenKind::Gt => ">".into(),
        TokenKind::LtEq => "<=".into(),
        TokenKind::GtEq => ">=".into(),
        TokenKind::AmpAmp => "&&".into(),
        TokenKind::PipePipe => "||".into(),
        // Compound assignment
        TokenKind::PlusEq => "+=".into(),
        TokenKind::MinusEq => "-=".into(),
        TokenKind::StarEq => "*=".into(),
        TokenKind::SlashEq => "/=".into(),
        TokenKind::PercentEq => "%=".into(),
        TokenKind::AmpEq => "&=".into(),
        TokenKind::PipeEq => "|=".into(),
        TokenKind::CaretEq => "^=".into(),
        TokenKind::ShlEq => "<<=".into(),
        TokenKind::ShrEq => ">>=".into(),
        // Misc
        TokenKind::AtSign => "@".into(),
        TokenKind::Hash => "#".into(),
        TokenKind::Eof => "<end of file>".into(),
    }
}

/// Minimal CSV splitter for annotation args (splits on top-level commas only).
#[allow(dead_code)]
fn split_csv_simple(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for ch in s.chars() {
        match ch {
            '(' | '[' => {
                depth += 1;
                cur.push(ch);
            }
            ')' | ']' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                let t = cur.trim().to_string();
                if !t.is_empty() {
                    out.push(t);
                }
                cur = String::new();
            }
            c => cur.push(c),
        }
    }
    let t = cur.trim().to_string();
    if !t.is_empty() {
        out.push(t);
    }
    out
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
    errors: Vec<String>,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            errors: Vec::new(),
        }
    }

    fn parse_module(mut self) -> Result<ModuleAst, Vec<String>> {
        let mut module = ModuleAst::default();
        let mut mmio_names = std::collections::BTreeSet::new();
        let mut mmio_register_names = std::collections::BTreeSet::new();
        let mut spinlock_names = std::collections::BTreeSet::new();
        let mut percpu_names = std::collections::BTreeSet::new();

        while self.skip_ws_comments() {
            if self.eof() {
                break;
            }
            let item_start = self.pos;

            if self.consume_keyword("mmio") {
                match self.parse_mmio_base_decl() {
                    Ok(decl) => {
                        if !mmio_names.insert(decl.name.clone()) {
                            self.errors
                                .push(format!("duplicate mmio base '{}'", decl.name));
                        } else {
                            module.mmio_bases.push(decl);
                        }
                    }
                    Err(msg) => {
                        self.error_here(&msg);
                        self.recover_to_next_module_item();
                    }
                }
                continue;
            }

            if self.consume_keyword("mmio_reg") {
                match self.parse_mmio_reg_decl() {
                    Ok(decl) => {
                        let full_name = format!("{}.{}", decl.base, decl.name);
                        if !mmio_register_names.insert(full_name.clone()) {
                            self.errors
                                .push(format!("duplicate mmio register '{}'", full_name));
                        } else {
                            module.mmio_registers.push(decl);
                        }
                    }
                    Err(msg) => {
                        self.error_here(&msg);
                        self.recover_to_next_module_item();
                    }
                }
                continue;
            }

            if self.consume_keyword("const") {
                match self.parse_const_decl() {
                    Ok(decl) => module.constants.push(decl),
                    Err(msg) => {
                        self.error_here(&msg);
                        self.recover_to_next_module_item();
                    }
                }
                continue;
            }

            if self.consume_keyword("enum") {
                match self.parse_enum_decl() {
                    Ok(decl) => module.enums.push(decl),
                    Err(msg) => {
                        self.error_here(&msg);
                        self.recover_to_next_module_item();
                    }
                }
                continue;
            }

            if self.consume_keyword("struct") {
                match self.parse_struct_decl() {
                    Ok(decl) => module.structs.push(decl),
                    Err(msg) => {
                        self.error_here(&msg);
                        self.recover_to_next_module_item();
                    }
                }
                continue;
            }

            if self.consume_keyword("spinlock") {
                self.skip_ws_comments();
                match self.parse_ident() {
                    Some(name) => {
                        self.skip_ws_comments();
                        if !self.consume_char(';') {
                            self.error_here("expected ';' after spinlock declaration");
                            self.recover_to_next_module_item();
                        } else if !spinlock_names.insert(name.clone()) {
                            self.errors
                                .push(format!("duplicate spinlock declaration '{}'", name));
                        } else {
                            module.locks.push(name);
                        }
                    }
                    None => {
                        self.error_here("expected lock class name after 'spinlock'");
                        self.recover_to_next_module_item();
                    }
                }
                continue;
            }

            if self.consume_keyword("percpu") {
                self.skip_ws_comments();
                match self.parse_percpu_decl() {
                    Ok(decl) => {
                        if !percpu_names.insert(decl.name.clone()) {
                            self.errors
                                .push(format!("duplicate percpu declaration '{}'", decl.name));
                        } else {
                            module.percpu_vars.push(decl);
                        }
                    }
                    Err(msg) => {
                        self.error_here(&msg);
                        self.recover_to_next_module_item();
                    }
                }
                continue;
            }

            if self.consume_keyword("static") {
                self.skip_ws_comments();
                match self.parse_static_var_decl() {
                    Ok(decl) => module.static_vars.push(decl),
                    Err(msg) => {
                        self.error_here(&msg);
                        self.recover_to_next_module_item();
                    }
                }
                continue;
            }

            let mut attrs = Vec::new();
            let mut is_extern = false;

            loop {
                self.skip_ws_comments();
                if self.consume_keyword("extern") {
                    is_extern = true;
                    continue;
                }
                if self.peek_char() == Some('@') {
                    match self.parse_attr() {
                        Some(attr) => attrs.push(attr),
                        None => break,
                    }
                    continue;
                }
                break;
            }

            self.skip_ws_comments();

            if attrs.len() == 1 && attrs[0].name == "module_caps" && !is_extern {
                if !self.consume_char(';') {
                    self.error_here("expected ';' after @module_caps(...) directive");
                    self.recover_to_next_module_item();
                    continue;
                }
                let caps = match attrs[0]
                    .args
                    .as_deref()
                    .map(split_csv_allow_trailing_comma)
                    .transpose()
                {
                    Ok(Some(caps)) => caps,
                    Ok(None) => Vec::new(),
                    Err(_) => {
                        self.errors.push(format_source_diagnostic(
                            &attrs[0].source,
                            "@module_caps(...) contains an empty capability entry",
                            None,
                        ));
                        self.recover_to_next_module_item();
                        continue;
                    }
                };
                module.module_caps = caps;
                continue;
            }

            if !self.consume_keyword("fn") {
                self.error_here(
                    "expected 'fn', 'mmio', 'mmio_reg', 'const', 'enum', 'struct', 'spinlock', 'percpu', 'static', or @module_caps(...) at item boundary",
                );
                self.recover_to_next_module_item();
                continue;
            }

            let Some(name) = self.parse_ident() else {
                self.error_here("expected function name after 'fn'");
                self.recover_to_next_module_item();
                continue;
            };

            let params = match self.parse_fn_param_list() {
                Some(params) => params,
                None => {
                    self.recover_to_next_module_item();
                    continue;
                }
            };

            if is_extern {
                self.skip_ws_comments();
                if !self.consume_char(';') {
                    self.error_here("expected ';' after extern declaration");
                    self.recover_to_next_module_item();
                    continue;
                }
                module.items.push(FnAst {
                    name,
                    is_extern: true,
                    params,
                    return_ty: None,
                    attrs,
                    body: Vec::new(),
                    source: self.source_note(item_start),
                });
                continue;
            }

            self.skip_ws_comments();
            if !self.consume_char('{') {
                self.error_here("expected '{' to start function body");
                self.recover_to_next_module_item();
                continue;
            }

            let body = self.parse_body();
            module.items.push(FnAst {
                name,
                is_extern: false,
                params,
                return_ty: None,
                attrs,
                body,
                source: self.source_note(item_start),
            });
        }

        if self.errors.is_empty() {
            Ok(module)
        } else {
            Err(self.errors)
        }
    }

    fn parse_mmio_base_decl(&mut self) -> Result<MmioBaseDecl, String> {
        let Some(name) = self.parse_ident() else {
            return Err("expected mmio base name after 'mmio'".to_string());
        };

        self.skip_ws_comments();
        if !self.consume_char('=') {
            return Err(format!(
                "invalid mmio base declaration for '{}': expected '='",
                name
            ));
        }

        self.skip_ws_comments();
        let value_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == ';' {
                break;
            }
            self.pos += ch.len_utf8();
        }

        if self.eof() {
            return Err(format!(
                "invalid mmio base declaration for '{}': expected ';'",
                name
            ));
        }

        let value = self.src[value_start..self.pos].trim().to_string();
        if !self.consume_char(';') {
            return Err(format!(
                "invalid mmio base declaration for '{}': expected ';'",
                name
            ));
        }

        if !is_int_literal_token(&value) {
            return Err(format!(
                "invalid mmio base declaration for '{}': expected integer literal",
                name
            ));
        }

        Ok(MmioBaseDecl { name, addr: value })
    }

    fn parse_mmio_reg_decl(&mut self) -> Result<MmioRegisterDecl, String> {
        let Some(base) = self.parse_ident() else {
            return Err("expected mmio register base name after 'mmio_reg'".to_string());
        };

        self.skip_ws_comments();
        if !self.consume_char('.') {
            return Err(format!(
                "invalid mmio register declaration for '{}': expected '.'",
                base
            ));
        }

        let Some(name) = self.parse_ident() else {
            return Err(format!(
                "invalid mmio register declaration for '{}': expected register name",
                base
            ));
        };

        let full_name = format!("{}.{}", base, name);

        self.skip_ws_comments();
        if !self.consume_char('=') {
            return Err(format!(
                "invalid mmio register declaration for '{}': expected '='",
                full_name
            ));
        }

        self.skip_ws_comments();
        let offset_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == ':' || ch == ';' {
                break;
            }
            self.pos += ch.len_utf8();
        }

        if self.eof() {
            return Err(format!(
                "invalid mmio register declaration for '{}': expected ':'",
                full_name
            ));
        }

        let offset = self.src[offset_start..self.pos].trim().to_string();
        if !self.consume_char(':') {
            return Err(format!(
                "invalid mmio register declaration for '{}': expected ':'",
                full_name
            ));
        }
        if !is_int_literal_token(&offset) {
            return Err(format!(
                "invalid mmio register declaration for '{}': expected integer literal offset",
                full_name
            ));
        }

        self.skip_ws_comments();
        let Some(ty_raw) = self.parse_ident() else {
            return Err(format!(
                "invalid mmio register declaration for '{}': expected type",
                full_name
            ));
        };
        let ty = match MmioScalarType::parse(&ty_raw) {
            Ok(ty) => ty,
            Err(_) => {
                return Err(format!(
                    "invalid mmio register declaration for '{}': unsupported type '{}'",
                    full_name, ty_raw
                ));
            }
        };

        self.skip_ws_comments();
        let Some(access_raw) = self.parse_ident() else {
            return Err(format!(
                "invalid mmio register declaration for '{}': expected access",
                full_name
            ));
        };
        let access = match access_raw.to_ascii_lowercase().as_str() {
            "ro" => MmioRegAccess::Ro,
            "wo" => MmioRegAccess::Wo,
            "rw" => MmioRegAccess::Rw,
            _ => {
                return Err(format!(
                    "invalid mmio register declaration for '{}': unsupported access '{}'",
                    full_name, access_raw
                ));
            }
        };

        self.skip_ws_comments();
        if !self.consume_char(';') {
            return Err(format!(
                "invalid mmio register declaration for '{}': expected ';'",
                full_name
            ));
        }

        Ok(MmioRegisterDecl {
            base,
            name,
            offset,
            ty,
            access,
        })
    }

    // Parse `const NAME: type = value;` (the 'const' keyword has already been consumed).
    fn parse_const_decl(&mut self) -> Result<ConstDecl, String> {
        self.skip_ws_comments();
        let Some(name) = self.parse_ident() else {
            return Err("expected constant name after 'const'".to_string());
        };
        self.skip_ws_comments();
        if !self.consume_char(':') {
            return Err(format!(
                "invalid const declaration '{}': expected ':'",
                name
            ));
        }
        self.skip_ws_comments();
        let Some(ty_raw) = self.parse_ident() else {
            return Err(format!(
                "invalid const declaration '{}': expected type",
                name
            ));
        };
        let ty = MmioScalarType::parse(&ty_raw).map_err(|_| {
            format!(
                "invalid const declaration '{}': unsupported type '{}'",
                name, ty_raw
            )
        })?;
        self.skip_ws_comments();
        if !self.consume_char('=') {
            return Err(format!(
                "invalid const declaration '{}': expected '='",
                name
            ));
        }
        self.skip_ws_comments();
        let value_start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == ';' {
                break;
            }
            self.pos += ch.len_utf8();
        }
        if self.eof() {
            return Err(format!(
                "invalid const declaration '{}': expected ';'",
                name
            ));
        }
        let value = self.src[value_start..self.pos].trim().to_string();
        if !self.consume_char(';') {
            return Err(format!(
                "invalid const declaration '{}': expected ';'",
                name
            ));
        }
        if !is_int_literal_token(&value) {
            return Err(format!(
                "invalid const declaration '{}': expected integer literal, got '{}'",
                name, value
            ));
        }
        Ok(ConstDecl { name, ty, value })
    }

    fn parse_enum_decl(&mut self) -> Result<EnumDecl, String> {
        self.skip_ws_comments();
        let Some(name) = self.parse_ident() else {
            return Err("expected enum name after 'enum'".to_string());
        };
        self.skip_ws_comments();
        if !self.consume_char(':') {
            return Err(format!("invalid enum declaration '{}': expected ':'", name));
        }
        self.skip_ws_comments();
        let Some(ty_raw) = self.parse_ident() else {
            return Err(format!(
                "invalid enum declaration '{}': expected type",
                name
            ));
        };
        let ty = MmioScalarType::parse(&ty_raw).map_err(|_| {
            format!(
                "invalid enum declaration '{}': unsupported type '{}'",
                name, ty_raw
            )
        })?;
        self.skip_ws_comments();
        if !self.consume_char('{') {
            return Err(format!(
                "invalid enum declaration '{}': expected '{{'",
                name
            ));
        }
        let mut variants = Vec::new();
        loop {
            self.skip_ws_comments();
            if self.consume_char('}') {
                break;
            }
            if self.eof() {
                return Err(format!(
                    "invalid enum declaration '{}': unterminated body",
                    name
                ));
            }
            let Some(variant_name) = self.parse_ident() else {
                return Err(format!(
                    "invalid enum declaration '{}': expected variant name",
                    name
                ));
            };
            self.skip_ws_comments();
            if !self.consume_char('=') {
                return Err(format!(
                    "invalid enum declaration '{}': expected '=' after variant '{}'",
                    name, variant_name
                ));
            }
            self.skip_ws_comments();
            let value_start = self.pos;
            while let Some(ch) = self.peek_char() {
                if ch == ',' || ch == '}' {
                    break;
                }
                self.pos += ch.len_utf8();
            }
            let value = self.src[value_start..self.pos].trim().to_string();
            if !is_int_literal_token(&value) {
                return Err(format!(
                    "invalid enum declaration '{}': variant '{}' expected integer literal, got '{}'",
                    name, variant_name, value
                ));
            }
            variants.push(EnumVariant {
                name: variant_name,
                value,
            });
            self.skip_ws_comments();
            // optional trailing comma
            self.consume_char(',');
        }
        Ok(EnumDecl { name, ty, variants })
    }

    fn parse_struct_decl(&mut self) -> Result<StructDecl, String> {
        self.skip_ws_comments();
        let Some(name) = self.parse_ident() else {
            return Err("expected struct name after 'struct'".to_string());
        };
        self.skip_ws_comments();
        if !self.consume_char('{') {
            return Err(format!(
                "invalid struct declaration '{}': expected '{{'",
                name
            ));
        }
        let mut fields = Vec::new();
        loop {
            self.skip_ws_comments();
            if self.consume_char('}') {
                break;
            }
            if self.eof() {
                return Err(format!(
                    "invalid struct declaration '{}': unterminated body",
                    name
                ));
            }
            let Some(field_name) = self.parse_ident() else {
                return Err(format!(
                    "invalid struct declaration '{}': expected field name",
                    name
                ));
            };
            self.skip_ws_comments();
            if !self.consume_char(':') {
                return Err(format!(
                    "invalid struct declaration '{}': expected ':' after field '{}'",
                    name, field_name
                ));
            }
            self.skip_ws_comments();
            let Some(ty_raw) = self.parse_ident() else {
                return Err(format!(
                    "invalid struct declaration '{}': expected type for field '{}'",
                    name, field_name
                ));
            };
            let ty = MmioScalarType::parse(&ty_raw).map_err(|_| {
                format!(
                    "invalid struct declaration '{}': field '{}' has unsupported type '{}'",
                    name, field_name, ty_raw
                )
            })?;
            fields.push(StructField {
                name: field_name,
                ty,
            });
            self.skip_ws_comments();
            // optional trailing comma
            self.consume_char(',');
        }
        if fields.is_empty() {
            return Err(format!(
                "invalid struct declaration '{}': struct must have at least one field",
                name
            ));
        }
        Ok(StructDecl { name, fields })
    }

    fn parse_percpu_decl(&mut self) -> Result<PercpuDecl, String> {
        let Some(name) = self.parse_ident() else {
            return Err("expected per-cpu variable name after 'percpu'".to_string());
        };
        self.skip_ws_comments();
        if !self.consume_char(':') {
            return Err(format!(
                "invalid percpu declaration '{}': expected ':'",
                name
            ));
        }
        self.skip_ws_comments();
        let Some(ty_raw) = self.parse_ident() else {
            return Err(format!(
                "invalid percpu declaration '{}': expected type",
                name
            ));
        };
        let ty = MmioScalarType::parse(&ty_raw).map_err(|_| {
            format!(
                "invalid percpu declaration '{}': unsupported type '{}'",
                name, ty_raw
            )
        })?;
        self.skip_ws_comments();
        if !self.consume_char(';') {
            return Err(format!(
                "invalid percpu declaration '{}': expected ';'",
                name
            ));
        }
        Ok(PercpuDecl { name, ty })
    }

    /// Parse `static TYPE NAME = LITERAL` — `static` keyword already consumed.
    fn parse_static_var_decl(&mut self) -> Result<StaticVarDecl, String> {
        let Some(ty_raw) = self.parse_ident() else {
            return Err("expected type after 'static'".to_string());
        };
        let ty = MmioScalarType::parse(&ty_raw)
            .map_err(|_| format!("unsupported static variable type '{}'", ty_raw))?;
        self.skip_ws_comments();
        let Some(name) = self.parse_ident() else {
            return Err("expected static variable name".to_string());
        };
        self.skip_ws_comments();
        if !self.consume_char('=') {
            return Err(format!(
                "expected '=' after static variable name '{}'",
                name
            ));
        }
        self.skip_ws_comments();
        // Parse a decimal integer literal
        let start = self.pos;
        while self
            .peek_char()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(format!(
                "expected integer literal for static variable '{}' initializer",
                name
            ));
        }
        let lit_str = &self.src[start..self.pos];
        let init_value: u64 = lit_str.parse().map_err(|_| {
            format!(
                "invalid integer literal '{}' for static variable '{}'",
                lit_str, name
            )
        })?;
        self.skip_ws_comments();
        let _ = self.consume_char(';');
        Ok(StaticVarDecl {
            name,
            ty,
            init_value,
        })
    }

    fn parse_body(&mut self) -> Vec<Stmt> {
        let mut body = Vec::new();

        loop {
            self.skip_ws_comments();
            if self.eof() {
                self.error_here("unterminated function body; expected '}'");
                break;
            }

            if self.consume_char('}') {
                break;
            }

            if self.consume_keyword("critical") {
                self.skip_ws_comments();
                if !self.consume_char('{') {
                    self.error_here("expected '{' after 'critical'");
                    self.recover_to_next_item();
                    break;
                }
                let inner = self.parse_body();
                body.push(Stmt::Critical(inner));
                continue;
            }

            if self.consume_keyword("unsafe") {
                self.skip_ws_comments();
                if !self.consume_char('{') {
                    self.error_here("expected '{' after 'unsafe'");
                    self.recover_to_next_item();
                    break;
                }
                let inner = self.parse_body();
                body.push(Stmt::Unsafe(inner));
                continue;
            }

            let stmt_start = self.pos;
            let errors_before = self.errors.len();
            match self.read_statement_text() {
                Some(text) => {
                    if text.trim().is_empty() {
                        continue;
                    }
                    match parse_stmt(text.trim()) {
                        Ok(Some(stmt)) => body.push(stmt),
                        Ok(None) => {}
                        Err(msg) => self.errors.push(self.format_diagnostic_at(
                            stmt_start,
                            &msg,
                            None::<&str>,
                        )),
                    }
                }
                None => {
                    if self.errors.len() == errors_before {
                        self.error_here("expected ';' terminating statement");
                    }
                    self.recover_to_next_item();
                    break;
                }
            }
        }

        body
    }

    fn read_statement_text(&mut self) -> Option<String> {
        let mut out = String::new();
        let mut depth = 0_i32;

        while let Some(ch) = self.peek_char() {
            if ch == ';' && depth == 0 {
                self.pos += ch.len_utf8();
                return Some(out);
            }

            if ch == '}' && depth == 0 {
                // Treat `}` as an implicit statement terminator so that
                // the last statement in a block does not require a trailing `;`.
                // An empty `out` means we are at the end of the block with no
                // pending statement — signal that to the caller via None.
                return if out.trim().is_empty() {
                    None
                } else {
                    Some(out)
                };
            }

            if ch == '"' {
                let open_pos = self.pos;
                out.push('"');
                self.pos += 1;
                loop {
                    match self.peek_char() {
                        None | Some('\n') => {
                            self.errors.push(self.format_diagnostic_at(
                                open_pos,
                                "unterminated string literal",
                                Some("add a closing '\"' before the end of the line"),
                            ));
                            return None;
                        }
                        Some('"') => {
                            out.push('"');
                            self.pos += 1;
                            break;
                        }
                        Some('\\') => {
                            out.push('\\');
                            self.pos += 1;
                            if let Some(esc) = self.peek_char() {
                                out.push(esc);
                                self.pos += esc.len_utf8();
                            }
                        }
                        Some(c) => {
                            out.push(c);
                            self.pos += c.len_utf8();
                        }
                    }
                }
                continue;
            }

            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
            }

            out.push(ch);
            self.pos += ch.len_utf8();
        }

        None
    }

    fn parse_attr(&mut self) -> Option<RawAttr> {
        let attr_start = self.pos;
        if !self.consume_char('@') {
            return None;
        }

        let Some(name) = self.parse_ident() else {
            self.error_here("expected attribute name after '@'");
            return None;
        };

        self.skip_ws_comments();
        let args = if self.consume_char('(') {
            Some(self.read_balanced_parens())
        } else {
            None
        };

        Some(RawAttr {
            name,
            args,
            source: self.source_note(attr_start),
        })
    }

    fn read_balanced_parens(&mut self) -> String {
        let mut out = String::new();
        let mut depth = 1_i32;

        while let Some(ch) = self.peek_char() {
            self.pos += ch.len_utf8();
            if ch == '(' {
                depth += 1;
                out.push(ch);
                continue;
            }
            if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                out.push(ch);
                continue;
            }
            out.push(ch);
        }

        out
    }

    fn parse_ident(&mut self) -> Option<String> {
        self.skip_ws_comments();
        let mut chars = self.src[self.pos..].char_indices();
        let (_, first) = chars.next()?;
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return None;
        }

        let mut end = self.pos + first.len_utf8();
        for (idx, ch) in chars {
            if ch == '_' || ch.is_ascii_alphanumeric() {
                end = self.pos + idx + ch.len_utf8();
            } else {
                break;
            }
        }

        let ident = self.src[self.pos..end].to_string();
        self.pos = end;
        Some(ident)
    }

    fn skip_ws_comments(&mut self) -> bool {
        while !self.eof() {
            let rest = &self.src[self.pos..];
            if rest.starts_with("//") {
                while let Some(ch) = self.peek_char() {
                    self.pos += ch.len_utf8();
                    if ch == '\n' {
                        break;
                    }
                }
                continue;
            }

            let Some(ch) = self.peek_char() else {
                break;
            };
            if ch.is_whitespace() {
                self.pos += ch.len_utf8();
                continue;
            }
            break;
        }
        !self.eof()
    }

    fn consume_keyword(&mut self, kw: &str) -> bool {
        self.skip_ws_comments();
        let rest = &self.src[self.pos..];
        if !rest.starts_with(kw) {
            return false;
        }

        let next = rest[kw.len()..].chars().next();
        if matches!(next, Some(ch) if ch == '_' || ch.is_ascii_alphanumeric()) {
            return false;
        }

        self.pos += kw.len();
        true
    }

    fn consume_char(&mut self, expected: char) -> bool {
        self.skip_ws_comments();
        if self.peek_char() == Some(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn parse_fn_param_list(&mut self) -> Option<Vec<(String, ParamTy)>> {
        self.skip_ws_comments();
        if !self.consume_char('(') {
            self.error_here("expected '(' to start parameter list");
            return None;
        }
        self.skip_ws_comments();
        if self.consume_char(')') {
            return Some(vec![]);
        }
        let mut params = Vec::new();
        loop {
            self.skip_ws_comments();
            let Some(name) = self.parse_ident() else {
                self.error_here("expected parameter name");
                return None;
            };
            self.skip_ws_comments();
            if !self.consume_char(':') {
                self.error_here("expected ':' after parameter name");
                return None;
            }
            self.skip_ws_comments();
            // Slice type `[T]` or scalar type `u8/u16/u32/u64`.
            let ty = if self.peek_char() == Some('[') {
                self.pos += 1; // consume '['
                self.skip_ws_comments();
                let Some(elem_name) = self.parse_ident() else {
                    self.error_here("expected element type after '['");
                    return None;
                };
                let elem_ty = match MmioScalarType::parse(&elem_name) {
                    Ok(ty) => ty,
                    Err(msg) => {
                        self.error_here(&msg);
                        return None;
                    }
                };
                self.skip_ws_comments();
                if !self.consume_char(']') {
                    self.error_here("expected ']' to close slice type");
                    return None;
                }
                ParamTy::Slice(elem_ty)
            } else {
                let Some(ty_name) = self.parse_ident() else {
                    self.error_here("expected parameter type (u8, u16, u32, u64, or [T])");
                    return None;
                };
                match MmioScalarType::parse(&ty_name) {
                    Ok(ty) => ParamTy::Scalar(ty),
                    Err(msg) => {
                        self.error_here(&msg);
                        return None;
                    }
                }
            };
            params.push((name, ty));
            self.skip_ws_comments();
            if self.consume_char(')') {
                return Some(params);
            }
            if !self.consume_char(',') {
                self.error_here("expected ',' or ')' in parameter list");
                return None;
            }
        }
    }

    fn recover_to_next_item(&mut self) {
        while let Some(ch) = self.peek_char() {
            self.pos += ch.len_utf8();
            if ch == ';' || ch == '}' {
                break;
            }
        }
    }

    fn recover_to_next_module_item(&mut self) {
        while !self.eof() {
            self.skip_ws_comments();
            let rest = &self.src[self.pos..];
            // '@' is always a valid item boundary (single char, no word boundary needed).
            if rest.starts_with('@') {
                break;
            }
            // For keyword prefixes, require a proper word boundary (not followed by
            // '_' or alphanumeric). Without this check, e.g. "fnnonsense" would match
            // starts_with("fn") and loop forever since consume_keyword("fn") also
            // requires a word boundary and would reject it.
            let at_word_boundary = |kw: &str| -> bool {
                if !rest.starts_with(kw) {
                    return false;
                }
                let after = rest[kw.len()..].chars().next();
                !matches!(after, Some(c) if c == '_' || c.is_ascii_alphanumeric())
            };
            if at_word_boundary("mmio_reg")
                || at_word_boundary("mmio")
                || at_word_boundary("extern")
                || at_word_boundary("fn")
            {
                break;
            }
            let Some(ch) = self.peek_char() else {
                break;
            };
            self.pos += ch.len_utf8();
        }
    }

    fn error_here(&mut self, msg: &str) {
        self.errors
            .push(self.format_diagnostic_at(self.pos, msg, None::<&str>));
    }

    fn format_diagnostic_at(&self, byte_offset: usize, msg: &str, help: Option<&str>) -> String {
        format_source_diagnostic(&self.source_note(byte_offset), msg, help)
    }

    fn source_note(&self, byte_offset: usize) -> SourceNote {
        SourceNote::from_source(self.src, byte_offset)
    }

    fn eof(&self) -> bool {
        self.pos >= self.src.len()
    }
}

fn parse_stmt(stmt: &str) -> Result<Option<Stmt>, String> {
    if stmt.is_empty() {
        return Ok(None);
    }

    // asm!(NAME) — kernel intrinsic instruction.
    {
        let trimmed = stmt.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("asm!") {
            // Find the parenthesised argument.
            let rest = trimmed[4..].trim_start();
            if rest.starts_with('(') && rest.ends_with(')') {
                let intr_name = rest[1..rest.len() - 1].trim();
                return match KernelIntrinsic::parse_name(intr_name) {
                    Some(intr) => Ok(Some(Stmt::InlineAsm(intr))),
                    None => Err(format!(
                        "unknown kernel intrinsic '{}'; supported: cli, sti, hlt, nop, mfence, sfence, lfence, wbinvd, pause, int3, cpuid",
                        intr_name
                    )),
                };
            }
            return Err(format!(
                "malformed asm! invocation: expected asm!(NAME), got '{}'",
                trimmed
            ));
        }
    }

    if stmt.trim_start().to_ascii_lowercase().starts_with("mmio ") {
        return Err("mmio declarations are only allowed at module scope".to_string());
    }
    if stmt
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("mmio_reg ")
    {
        return Err("mmio_reg declarations are only allowed at module scope".to_string());
    }

    let (name, args) = parse_invocation(stmt)?;
    let lowered = name.to_ascii_lowercase();

    if lowered == "yieldpoint" {
        if !args.trim().is_empty() {
            return Err("yieldpoint() must have no arguments".to_string());
        }
        return Ok(Some(Stmt::YieldPoint));
    }

    if lowered == "allocpoint" {
        if !args.trim().is_empty() {
            return Err("allocpoint() must have no arguments".to_string());
        }
        return Ok(Some(Stmt::AllocPoint));
    }

    if lowered == "blockpoint" {
        if !args.trim().is_empty() {
            return Err("blockpoint() must have no arguments".to_string());
        }
        return Ok(Some(Stmt::BlockPoint));
    }

    if lowered == "acquire" {
        let lock = args.trim();
        if lock.is_empty() {
            return Err("acquire(lock_class) requires one lock class".to_string());
        }
        return Ok(Some(Stmt::Acquire(lock.to_string())));
    }

    if lowered == "release" {
        let lock = args.trim();
        if lock.is_empty() {
            return Err("release(lock_class) requires one lock class".to_string());
        }
        return Ok(Some(Stmt::Release(lock.to_string())));
    }

    if lowered == "call_capture" {
        let parts = split_csv(&args);
        if parts.len() != 2 {
            return Err(
                "call_capture(callee, slot) requires exactly two identifier arguments".to_string(),
            );
        }
        let callee = parse_branch_target_operand(parts[0].trim())?;
        let slot = parse_mmio_capture_operand(parts[1].trim())?;
        return Ok(Some(Stmt::CallCapture { callee, slot }));
    }

    if lowered == "call_with_args" {
        let parts = split_csv(&args);
        if parts.is_empty() {
            return Err(
                "call_with_args(callee[, arg, ...]) requires at least a callee".to_string(),
            );
        }
        let callee = parse_branch_target_operand(parts[0].trim())?;
        let call_args: Result<Vec<MmioValueExpr>, String> = parts[1..]
            .iter()
            .map(|a| parse_mmio_value_operand(a.trim()))
            .collect();
        return Ok(Some(Stmt::CallWithArgs {
            callee,
            args: call_args?,
        }));
    }

    if lowered == "tail_call" {
        let parts = split_csv(&args);
        if parts.is_empty() {
            return Err("tail_call(callee[, arg, ...]) requires at least a callee".to_string());
        }
        let callee = parse_branch_target_operand(parts[0].trim())?;
        let tail_args: Result<Vec<MmioValueExpr>, String> = parts[1..]
            .iter()
            .map(|a| parse_mmio_value_operand(a.trim()))
            .collect();
        return Ok(Some(Stmt::TailCall {
            callee,
            args: tail_args?,
        }));
    }

    if lowered == "return_slot" {
        let slot = parse_branch_slot_operand(args.trim())?;
        if slot.is_empty() {
            return Err("return_slot(slot) requires exactly one slot identifier".to_string());
        }
        if split_csv(&args).len() != 1 {
            return Err("return_slot(slot) requires exactly one slot identifier".to_string());
        }
        return Ok(Some(Stmt::ReturnSlot { slot }));
    }

    if lowered == "branch_if_zero" {
        let parts = split_csv(&args);
        if parts.len() != 3 {
            return Err(
                "branch_if_zero(slot, then_fn, else_fn) requires exactly three identifier arguments"
                    .to_string(),
            );
        }
        let slot = parse_branch_slot_operand(parts[0].trim())?;
        let then_callee = parse_branch_target_operand(parts[1].trim())?;
        let else_callee = parse_branch_target_operand(parts[2].trim())?;
        return Ok(Some(Stmt::BranchIfZero {
            slot,
            then_callee,
            else_callee,
        }));
    }

    if lowered == "branch_if_eq" {
        let parts = split_csv(&args);
        if parts.len() != 4 {
            return Err(
                "branch_if_eq(slot, literal, then_fn, else_fn) requires exactly four arguments"
                    .to_string(),
            );
        }
        let slot = parse_branch_slot_operand(parts[0].trim())?;
        let compare_value = parse_branch_literal_operand(parts[1].trim())?;
        let then_callee = parse_branch_target_operand(parts[2].trim())?;
        let else_callee = parse_branch_target_operand(parts[3].trim())?;
        return Ok(Some(Stmt::BranchIfEq {
            slot,
            compare_value,
            then_callee,
            else_callee,
        }));
    }

    if lowered == "branch_if_mask_nonzero" {
        let parts = split_csv(&args);
        if parts.len() != 4 {
            return Err(
                "branch_if_mask_nonzero(slot, mask, then_fn, else_fn) requires exactly four arguments"
                    .to_string(),
            );
        }
        let slot = parse_branch_slot_operand(parts[0].trim())?;
        let mask_value = parse_branch_literal_operand(parts[1].trim())?;
        let then_callee = parse_branch_target_operand(parts[2].trim())?;
        let else_callee = parse_branch_target_operand(parts[3].trim())?;
        return Ok(Some(Stmt::BranchIfMaskNonZero {
            slot,
            mask_value,
            then_callee,
            else_callee,
        }));
    }

    if let Some(stmt) = parse_typed_mmio_stmt(&name, &args)? {
        return Ok(Some(stmt));
    }

    Ok(Some(Stmt::Call(name)))
}

fn parse_typed_mmio_stmt(name: &str, args: &str) -> Result<Option<Stmt>, String> {
    let lowered = name.to_ascii_lowercase();
    if lowered == "mmio_read" {
        return Err("mmio_read() legacy form is unsupported; use mmio_read<T>(addr)".to_string());
    }
    if lowered == "mmio_write" {
        return Err(
            "mmio_write() legacy form is unsupported; use mmio_write<T>(addr, value)".to_string(),
        );
    }
    if lowered == "raw_mmio_read" {
        return Err(
            "raw_mmio_read() legacy form is unsupported; use raw_mmio_read<T>(addr)".to_string(),
        );
    }
    if lowered == "raw_mmio_write" {
        return Err(
            "raw_mmio_write() legacy form is unsupported; use raw_mmio_write<T>(addr, value)"
                .to_string(),
        );
    }
    if lowered == "stack_cell" {
        return Err("stack_cell() legacy form is unsupported; use stack_cell<T>(cell)".to_string());
    }
    if lowered == "cell_store" {
        return Err(
            "cell_store() legacy form is unsupported; use cell_store<T>(cell, value)".to_string(),
        );
    }
    if lowered == "cell_load" {
        return Err(
            "cell_load() legacy form is unsupported; use cell_load<T>(cell, slot)".to_string(),
        );
    }

    if let Some(ty) = parse_mmio_scalar_from_name(name, "stack_cell")? {
        let parts = split_csv(args);
        if parts.len() != 1 {
            return Err("stack_cell<T>(cell) requires exactly one cell identifier".to_string());
        }
        let cell = parse_mmio_capture_operand(parts[0].trim())?;
        return Ok(Some(Stmt::StackCell { ty, cell }));
    }

    if let Some(ty) = parse_mmio_scalar_from_name(name, "cell_store")? {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err(
                "cell_store<T>(cell, value) requires exactly two arguments: cell and value"
                    .to_string(),
            );
        }
        let cell = parse_mmio_capture_operand(parts[0].trim())?;
        let value = parse_mmio_value_operand(parts[1].trim())?;
        return Ok(Some(Stmt::CellStore { ty, cell, value }));
    }

    if let Some(ty) = parse_mmio_scalar_from_name(name, "cell_load")? {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err(
                "cell_load<T>(cell, slot) requires exactly two identifier arguments".to_string(),
            );
        }
        let cell = parse_mmio_capture_operand(parts[0].trim())?;
        let slot = parse_mmio_capture_operand(parts[1].trim())?;
        return Ok(Some(Stmt::CellLoad { ty, cell, slot }));
    }

    if let Some((ty, op)) = parse_cell_arith_from_name(name)? {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err(format!(
                "cell_{}<T>(cell, imm) requires exactly two arguments: cell name and integer literal",
                op.as_str()
            ));
        }
        let cell = parse_mmio_capture_operand(parts[0].trim())?;
        let imm_str = parts[1].trim();
        if !is_int_literal_token(imm_str) {
            return Err(format!(
                "cell_{}<T>(cell, imm): '{}' is not a valid integer literal",
                op.as_str(),
                imm_str
            ));
        }
        let imm = parse_integer_literal_u64(imm_str)?;
        return Ok(Some(Stmt::CellArithImm { ty, cell, op, imm }));
    }

    if let Some((ty, op)) = parse_slot_arith_from_name(name)? {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err(format!(
                "slot_{}<T>(dst, src) requires exactly two arguments: dst cell name and src cell name",
                op.as_str()
            ));
        }
        let dst = parse_mmio_capture_operand(parts[0].trim())?;
        let src = parse_mmio_capture_operand(parts[1].trim())?;
        return Ok(Some(Stmt::CellArithSlot { ty, dst, src, op }));
    }

    if let Some(ty) = parse_mmio_scalar_from_name(name, "mmio_read")? {
        let parts = split_csv(args);
        if parts.len() != 1 && parts.len() != 2 {
            return Err(
                "mmio_read<T>(addr[, slot]) requires one address argument and optional capture slot"
                    .to_string(),
            );
        }
        let addr = parse_mmio_addr_operand(parts[0].trim())?;
        let capture = if parts.len() == 2 {
            Some(parse_mmio_capture_operand(parts[1].trim())?)
        } else {
            None
        };
        return Ok(Some(Stmt::MmioRead { ty, addr, capture }));
    }

    if let Some(ty) = parse_mmio_scalar_from_name(name, "mmio_write")? {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err(
                "mmio_write<T>(addr, value) requires exactly two arguments: address and value"
                    .to_string(),
            );
        }
        let addr = parse_mmio_addr_operand(parts[0].trim())?;
        let value = parse_mmio_value_operand(parts[1].trim())?;
        return Ok(Some(Stmt::MmioWrite { ty, addr, value }));
    }

    if let Some(ty) = parse_mmio_scalar_from_name(name, "raw_mmio_read")? {
        let parts = split_csv(args);
        if parts.len() != 1 && parts.len() != 2 {
            return Err(
                "raw_mmio_read<T>(addr[, slot]) requires one address argument and optional capture slot"
                    .to_string(),
            );
        }
        let addr = parse_mmio_addr_operand(parts[0].trim())?;
        let capture = if parts.len() == 2 {
            Some(parse_mmio_capture_operand(parts[1].trim())?)
        } else {
            None
        };
        return Ok(Some(Stmt::RawMmioRead { ty, addr, capture }));
    }

    if let Some(ty) = parse_mmio_scalar_from_name(name, "raw_mmio_write")? {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err(
                "raw_mmio_write<T>(addr, value) requires exactly two arguments: address and value"
                    .to_string(),
            );
        }
        let addr = parse_mmio_addr_operand(parts[0].trim())?;
        let value = parse_mmio_value_operand(parts[1].trim())?;
        return Ok(Some(Stmt::RawMmioWrite { ty, addr, value }));
    }

    if lowered == "slice_len" {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err(
                "slice_len(slice, slot) requires exactly two identifier arguments".to_string(),
            );
        }
        let slice = parse_mmio_capture_operand(parts[0].trim())?;
        let slot = parse_mmio_capture_operand(parts[1].trim())?;
        return Ok(Some(Stmt::SliceLen { slice, slot }));
    }

    if lowered == "slice_ptr" {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err(
                "slice_ptr(slice, slot) requires exactly two identifier arguments".to_string(),
            );
        }
        let slice = parse_mmio_capture_operand(parts[0].trim())?;
        let slot = parse_mmio_capture_operand(parts[1].trim())?;
        return Ok(Some(Stmt::SlicePtr { slice, slot }));
    }

    if let Some(ty) = parse_mmio_scalar_from_name(name, "percpu_read")? {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err("percpu_read<T>(NAME, slot) requires exactly two arguments".to_string());
        }
        let var_name = parse_mmio_capture_operand(parts[0].trim())?;
        let slot = parse_mmio_capture_operand(parts[1].trim())?;
        return Ok(Some(Stmt::PercpuRead {
            ty,
            name: var_name,
            slot,
        }));
    }

    if let Some(ty) = parse_mmio_scalar_from_name(name, "percpu_write")? {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err("percpu_write<T>(NAME, value) requires exactly two arguments".to_string());
        }
        let var_name = parse_mmio_capture_operand(parts[0].trim())?;
        let value = parse_mmio_value_operand(parts[1].trim())?;
        return Ok(Some(Stmt::PercpuWrite {
            ty,
            name: var_name,
            value,
        }));
    }

    Ok(None)
}

fn parse_mmio_addr_operand(raw: &str) -> Result<MmioAddrExpr, String> {
    let operand = raw.trim();
    if is_ident_token(operand) || is_qualified_ident_token(operand) {
        return Ok(MmioAddrExpr::Ident(operand.to_string()));
    }
    if is_int_literal_token(operand) {
        return Ok(MmioAddrExpr::IntLiteral(operand.to_string()));
    }

    if operand.matches('+').count() == 1 {
        let (base, offset) = operand.split_once('+').expect("single plus");
        let base = base.trim();
        let offset = offset.trim();
        let base_ok = is_ident_token(base) || is_qualified_ident_token(base);
        let offset_ok = is_int_literal_token(offset) || is_qualified_ident_token(offset);
        if base_ok && offset_ok {
            return Ok(MmioAddrExpr::IdentPlusOffset {
                base: base.to_string(),
                offset: offset.to_string(),
            });
        }
    }

    Err(format!(
        "unsupported mmio address operand '{}'; expected identifier, integer literal, or identifier + integer literal",
        operand
    ))
}

fn parse_mmio_value_operand(raw: &str) -> Result<MmioValueExpr, String> {
    let operand = raw.trim();
    if is_ident_token(operand) || is_qualified_ident_token(operand) {
        return Ok(MmioValueExpr::Ident(operand.to_string()));
    }
    if is_int_literal_token(operand) {
        return Ok(MmioValueExpr::IntLiteral(operand.to_string()));
    }
    Err(format!(
        "unsupported mmio value operand '{}'; expected identifier or integer literal",
        operand
    ))
}

fn parse_mmio_capture_operand(raw: &str) -> Result<String, String> {
    let operand = raw.trim();
    if is_ident_token(operand) {
        return Ok(operand.to_string());
    }
    Err(format!(
        "'{}' is not a valid capture slot identifier",
        operand
    ))
}

fn parse_branch_slot_operand(raw: &str) -> Result<String, String> {
    let operand = raw.trim();
    if is_ident_token(operand) {
        return Ok(operand.to_string());
    }
    Err(format!(
        "'{}' is not a valid branch slot identifier",
        operand
    ))
}

fn parse_branch_target_operand(raw: &str) -> Result<String, String> {
    let operand = raw.trim();
    if is_ident_token(operand) {
        return Ok(operand.to_string());
    }
    Err(format!(
        "'{}' is not a valid branch target identifier",
        operand
    ))
}

fn parse_branch_literal_operand(raw: &str) -> Result<String, String> {
    let operand = raw.trim();
    if is_int_literal_token(operand) {
        return Ok(operand.to_string());
    }
    Err(format!(
        "'{}' is not a valid branch comparison literal",
        operand
    ))
}

pub fn int_literal_numeric_value(raw: &str) -> Option<u128> {
    if raw.starts_with("0x") || raw.starts_with("0X") {
        let suffix = raw[2..].replace('_', "");
        if suffix.is_empty() {
            return None;
        }
        return u128::from_str_radix(&suffix, 16).ok();
    }

    let digits = raw.replace('_', "");
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u128>().ok()
}

fn is_ident_token(raw: &str) -> bool {
    let mut chars = raw.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first != '_' && !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// Returns true for `IDENT::IDENT` (qualified enum variant path).
fn is_qualified_ident_token(raw: &str) -> bool {
    match raw.split_once("::") {
        Some((scope, variant)) => is_ident_token(scope) && is_ident_token(variant),
        None => false,
    }
}

fn is_int_literal_token(raw: &str) -> bool {
    if raw.starts_with("0x") || raw.starts_with("0X") {
        let suffix = &raw[2..];
        return !suffix.is_empty() && suffix.chars().all(|ch| ch == '_' || ch.is_ascii_hexdigit());
    }

    let mut saw_digit = false;
    for ch in raw.chars() {
        if ch == '_' {
            continue;
        }
        if ch.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        return false;
    }
    saw_digit
}

fn parse_integer_literal_u64(raw: &str) -> Result<u64, String> {
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        let clean: String = hex.chars().filter(|&c| c != '_').collect();
        u64::from_str_radix(&clean, 16)
            .map_err(|_| format!("'{}' is not a valid u64 integer literal", raw))
    } else {
        let clean: String = raw.chars().filter(|&c| c != '_').collect();
        clean
            .parse::<u64>()
            .map_err(|_| format!("'{}' is not a valid u64 integer literal", raw))
    }
}

fn parse_cell_arith_from_name(name: &str) -> Result<Option<(MmioScalarType, ArithOp)>, String> {
    const OPS: [(&str, ArithOp); 7] = [
        ("cell_add", ArithOp::Add),
        ("cell_sub", ArithOp::Sub),
        ("cell_and", ArithOp::And),
        ("cell_or", ArithOp::Or),
        ("cell_xor", ArithOp::Xor),
        ("cell_shl", ArithOp::Shl),
        ("cell_shr", ArithOp::Shr),
    ];
    for (base, op) in &OPS {
        if let Some(ty) = parse_mmio_scalar_from_name(name, base)? {
            return Ok(Some((ty, *op)));
        }
    }
    Ok(None)
}

fn parse_slot_arith_from_name(name: &str) -> Result<Option<(MmioScalarType, ArithOp)>, String> {
    const OPS: [(&str, ArithOp); 7] = [
        ("slot_add", ArithOp::Add),
        ("slot_sub", ArithOp::Sub),
        ("slot_and", ArithOp::And),
        ("slot_or", ArithOp::Or),
        ("slot_xor", ArithOp::Xor),
        ("slot_shl", ArithOp::Shl),
        ("slot_shr", ArithOp::Shr),
    ];
    for (base, op) in &OPS {
        if let Some(ty) = parse_mmio_scalar_from_name(name, base)? {
            return Ok(Some((ty, *op)));
        }
    }
    Ok(None)
}

fn parse_mmio_scalar_from_name(name: &str, base: &str) -> Result<Option<MmioScalarType>, String> {
    let lowered = name.to_ascii_lowercase();
    if !lowered.starts_with(base) {
        return Ok(None);
    }

    let suffix = &lowered[base.len()..];
    if suffix.starts_with('<') {
        if !suffix.ends_with('>') {
            return Err(format!(
                "malformed {} typed invocation '{}'; expected {}<T>()",
                base, name, base
            ));
        }
        if suffix.len() < 3 {
            return Err(format!(
                "malformed {} typed invocation '{}'; expected {}<T>()",
                base, name, base
            ));
        }
        let ty = &name[base.len() + 1..name.len() - 1];
        return MmioScalarType::parse(ty).map(Some);
    }

    // Keep non-mmio identifiers such as `mmio_reader()` as ordinary calls.
    if matches!(suffix.chars().next(), Some(ch) if ch == '_' || ch.is_ascii_alphanumeric()) {
        return Ok(None);
    }

    Err(format!(
        "malformed {} invocation '{}'; expected {}<T>(...)",
        base, name, base
    ))
}

fn parse_invocation(stmt: &str) -> Result<(String, String), String> {
    let open = stmt
        .find('(')
        .ok_or_else(|| format!("expected invocation syntax: '{}'", stmt))?;
    let close = stmt
        .rfind(')')
        .ok_or_else(|| format!("expected invocation syntax: '{}'", stmt))?;

    if close <= open {
        return Err(format!("invalid invocation syntax: '{}'", stmt));
    }

    let name = stmt[..open].trim();
    if name.is_empty() {
        return Err(format!("missing callee name in statement: '{}'", stmt));
    }

    let args = stmt[open + 1..close].trim().to_string();
    let trailing = stmt[close + 1..].trim();
    if !trailing.is_empty() {
        return Err(format!("unexpected trailing text in statement: '{}'", stmt));
    }

    Ok((name.to_string(), args))
}

#[cfg(test)]
mod tests {
    use super::{
        MmioAddrExpr, MmioBaseDecl, MmioRegAccess, MmioRegisterDecl, MmioScalarType, MmioValueExpr,
        ParamTy, SourceNote, Stmt, format_source_diagnostic, int_literal_numeric_value,
        parse_module, split_csv_allow_trailing_comma,
    };
    use proptest::prelude::*;

    fn diagnostic_at(src: &str, byte_offset: usize, message: &str) -> String {
        format_source_diagnostic(&SourceNote::from_source(src, byte_offset), message, None)
    }

    fn diagnostic_at_with_help(src: &str, byte_offset: usize, message: &str, help: &str) -> String {
        format_source_diagnostic(
            &SourceNote::from_source(src, byte_offset),
            message,
            Some(help),
        )
    }

    #[test]
    fn parse_critical_block_statement() {
        let src = r#"
        fn entry() {
          critical {
            helper();
          }
        }

        fn helper() {}
        "#;
        let ast = parse_module(src).expect("parse");
        let entry = ast
            .items
            .iter()
            .find(|item| item.name == "entry")
            .expect("entry function");
        assert_eq!(entry.body.len(), 1);
        match &entry.body[0] {
            Stmt::Critical(inner) => {
                assert_eq!(inner.len(), 1);
                assert_eq!(inner[0], Stmt::Call("helper".to_string()));
            }
            other => panic!("expected critical statement, got {:?}", other),
        }
    }

    #[test]
    fn parse_typed_mmio_statements() {
        let src = r#"
        fn entry() {
          mmio_read<u16>(uart0);
          mmio_write<u64>(uart0, value0);
        }
        "#;
        let ast = parse_module(src).expect("parse");
        let entry = ast
            .items
            .iter()
            .find(|item| item.name == "entry")
            .expect("entry function");
        assert_eq!(
            entry.body,
            vec![
                Stmt::MmioRead {
                    ty: MmioScalarType::U16,
                    addr: MmioAddrExpr::Ident("uart0".to_string()),
                    capture: None,
                },
                Stmt::MmioWrite {
                    ty: MmioScalarType::U64,
                    addr: MmioAddrExpr::Ident("uart0".to_string()),
                    value: MmioValueExpr::Ident("value0".to_string())
                }
            ]
        );
    }

    #[test]
    fn parse_typed_raw_mmio_statements() {
        let src = r#"
        fn entry() {
          raw_mmio_read<u16>(uart0 + 0x10);
          raw_mmio_write<u64>(0x1000, value0);
        }
        "#;
        let ast = parse_module(src).expect("parse");
        let entry = ast
            .items
            .iter()
            .find(|item| item.name == "entry")
            .expect("entry function");
        assert_eq!(
            entry.body,
            vec![
                Stmt::RawMmioRead {
                    ty: MmioScalarType::U16,
                    addr: MmioAddrExpr::IdentPlusOffset {
                        base: "uart0".to_string(),
                        offset: "0x10".to_string()
                    },
                    capture: None,
                },
                Stmt::RawMmioWrite {
                    ty: MmioScalarType::U64,
                    addr: MmioAddrExpr::IntLiteral("0x1000".to_string()),
                    value: MmioValueExpr::Ident("value0".to_string())
                }
            ]
        );
    }

    #[test]
    fn parse_typed_mmio_operands_are_structured() {
        let src = r#"
        fn entry() {
          mmio_read<u32>(0x1000);
          mmio_read<u32>(uart0 + 0x10);
          mmio_write<u8>(uart0 + 4, 0xff);
        }
        "#;
        let ast = parse_module(src).expect("parse");
        let entry = ast
            .items
            .iter()
            .find(|item| item.name == "entry")
            .expect("entry function");
        assert_eq!(
            entry.body,
            vec![
                Stmt::MmioRead {
                    ty: MmioScalarType::U32,
                    addr: MmioAddrExpr::IntLiteral("0x1000".to_string()),
                    capture: None,
                },
                Stmt::MmioRead {
                    ty: MmioScalarType::U32,
                    addr: MmioAddrExpr::IdentPlusOffset {
                        base: "uart0".to_string(),
                        offset: "0x10".to_string()
                    },
                    capture: None,
                },
                Stmt::MmioWrite {
                    ty: MmioScalarType::U8,
                    addr: MmioAddrExpr::IdentPlusOffset {
                        base: "uart0".to_string(),
                        offset: "4".to_string()
                    },
                    value: MmioValueExpr::IntLiteral("0xff".to_string())
                }
            ]
        );
    }

    #[test]
    fn parse_typed_mmio_read_capture_slots_are_structured() {
        let src = r#"
        fn entry() {
          mmio_read<u32>(uart0 + 0x10, status);
          raw_mmio_read<u16>(0x1010, sample);
        }
        "#;
        let ast = parse_module(src).expect("parse");
        let entry = ast
            .items
            .iter()
            .find(|item| item.name == "entry")
            .expect("entry function");
        assert_eq!(
            entry.body,
            vec![
                Stmt::MmioRead {
                    ty: MmioScalarType::U32,
                    addr: MmioAddrExpr::IdentPlusOffset {
                        base: "uart0".to_string(),
                        offset: "0x10".to_string(),
                    },
                    capture: Some("status".to_string()),
                },
                Stmt::RawMmioRead {
                    ty: MmioScalarType::U16,
                    addr: MmioAddrExpr::IntLiteral("0x1010".to_string()),
                    capture: Some("sample".to_string()),
                }
            ]
        );
    }

    #[test]
    fn parse_branch_if_eq_is_structured() {
        let src = r#"
        fn entry() {
          mmio_read<u32>(uart0 + 0x10, status);
          branch_if_eq(status, 0x20, ready, idle);
        }
        "#;
        let ast = parse_module(src).expect("parse");
        let entry = ast
            .items
            .iter()
            .find(|item| item.name == "entry")
            .expect("entry function");
        assert_eq!(
            entry.body,
            vec![
                Stmt::MmioRead {
                    ty: MmioScalarType::U32,
                    addr: MmioAddrExpr::IdentPlusOffset {
                        base: "uart0".to_string(),
                        offset: "0x10".to_string(),
                    },
                    capture: Some("status".to_string()),
                },
                Stmt::BranchIfEq {
                    slot: "status".to_string(),
                    compare_value: "0x20".to_string(),
                    then_callee: "ready".to_string(),
                    else_callee: "idle".to_string(),
                }
            ]
        );
    }

    #[test]
    fn parse_call_capture_and_return_slot_are_structured() {
        let src = r#"
        fn read_status() {
          mmio_read<u32>(uart0 + 0x10, status);
          return_slot(status);
        }

        fn entry() {
          call_capture(read_status, latest);
          branch_if_mask_nonzero(latest, 0x20, ready, idle);
        }
        "#;
        let ast = parse_module(src).expect("parse");
        let read_status = ast
            .items
            .iter()
            .find(|item| item.name == "read_status")
            .expect("read_status function");
        assert_eq!(
            read_status.body,
            vec![
                Stmt::MmioRead {
                    ty: MmioScalarType::U32,
                    addr: MmioAddrExpr::IdentPlusOffset {
                        base: "uart0".to_string(),
                        offset: "0x10".to_string(),
                    },
                    capture: Some("status".to_string()),
                },
                Stmt::ReturnSlot {
                    slot: "status".to_string(),
                }
            ]
        );
        let entry = ast
            .items
            .iter()
            .find(|item| item.name == "entry")
            .expect("entry function");
        assert_eq!(
            entry.body,
            vec![
                Stmt::CallCapture {
                    callee: "read_status".to_string(),
                    slot: "latest".to_string(),
                },
                Stmt::BranchIfMaskNonZero {
                    slot: "latest".to_string(),
                    mask_value: "0x20".to_string(),
                    then_callee: "ready".to_string(),
                    else_callee: "idle".to_string(),
                }
            ]
        );
    }

    #[test]
    fn parse_stack_cell_and_load_store_are_structured() {
        let src = r#"
        fn entry() {
          stack_cell<u32>(saved_status);
          mmio_read<u32>(uart0 + 0x10, value);
          cell_store<u32>(saved_status, value);
          cell_load<u32>(saved_status, value);
        }
        "#;
        let ast = parse_module(src).expect("parse");
        let entry = ast
            .items
            .iter()
            .find(|item| item.name == "entry")
            .expect("entry function");
        assert_eq!(
            entry.body,
            vec![
                Stmt::StackCell {
                    ty: MmioScalarType::U32,
                    cell: "saved_status".to_string(),
                },
                Stmt::MmioRead {
                    ty: MmioScalarType::U32,
                    addr: MmioAddrExpr::IdentPlusOffset {
                        base: "uart0".to_string(),
                        offset: "0x10".to_string(),
                    },
                    capture: Some("value".to_string()),
                },
                Stmt::CellStore {
                    ty: MmioScalarType::U32,
                    cell: "saved_status".to_string(),
                    value: MmioValueExpr::Ident("value".to_string()),
                },
                Stmt::CellLoad {
                    ty: MmioScalarType::U32,
                    cell: "saved_status".to_string(),
                    slot: "value".to_string(),
                }
            ]
        );
    }

    #[test]
    fn parse_branch_if_mask_nonzero_is_structured() {
        let src = r#"
        fn entry() {
          mmio_read<u32>(uart0 + 0x10, status);
          branch_if_mask_nonzero(status, 0x20, ready, idle);
        }
        "#;
        let ast = parse_module(src).expect("parse");
        let entry = ast
            .items
            .iter()
            .find(|item| item.name == "entry")
            .expect("entry function");
        assert_eq!(
            entry.body,
            vec![
                Stmt::MmioRead {
                    ty: MmioScalarType::U32,
                    addr: MmioAddrExpr::IdentPlusOffset {
                        base: "uart0".to_string(),
                        offset: "0x10".to_string(),
                    },
                    capture: Some("status".to_string()),
                },
                Stmt::BranchIfMaskNonZero {
                    slot: "status".to_string(),
                    mask_value: "0x20".to_string(),
                    then_callee: "ready".to_string(),
                    else_callee: "idle".to_string(),
                }
            ]
        );
    }

    #[test]
    fn parse_module_mmio_base_declarations() {
        let src = r#"
        mmio UART0 = 0x1000;
        mmio TIMER = 4096;
        fn entry() { mmio_read<u32>(UART0 + 0x10); }
        "#;
        let ast = parse_module(src).expect("parse");
        assert_eq!(
            ast.mmio_bases,
            vec![
                MmioBaseDecl {
                    name: "UART0".to_string(),
                    addr: "0x1000".to_string()
                },
                MmioBaseDecl {
                    name: "TIMER".to_string(),
                    addr: "4096".to_string()
                }
            ]
        );
    }

    #[test]
    fn parse_module_mmio_register_declarations() {
        let src = r#"
        mmio UART0 = 0x1000;
        mmio_reg UART0.DR = 0x00 : u32 rw;
        mmio_reg UART0.SR = 0x04 : u32 ro;
        fn entry() { mmio_read<u32>(UART0 + 0x04); }
        "#;
        let ast = parse_module(src).expect("parse");
        assert_eq!(
            ast.mmio_registers,
            vec![
                MmioRegisterDecl {
                    base: "UART0".to_string(),
                    name: "DR".to_string(),
                    offset: "0x00".to_string(),
                    ty: MmioScalarType::U32,
                    access: MmioRegAccess::Rw,
                },
                MmioRegisterDecl {
                    base: "UART0".to_string(),
                    name: "SR".to_string(),
                    offset: "0x04".to_string(),
                    ty: MmioScalarType::U32,
                    access: MmioRegAccess::Ro,
                }
            ]
        );
    }

    #[test]
    fn split_csv_allow_trailing_comma_accepts_single_and_multi_value_trailing_commas() {
        assert_eq!(
            split_csv_allow_trailing_comma("thread,").expect("single trailing comma"),
            vec!["thread".to_string()]
        );
        assert_eq!(
            split_csv_allow_trailing_comma("thread, boot,").expect("multi trailing comma"),
            vec!["thread".to_string(), "boot".to_string()]
        );
        assert_eq!(
            split_csv_allow_trailing_comma("").expect("empty list"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn split_csv_allow_trailing_comma_rejects_empty_entries() {
        for input in [",thread", "thread,,boot", "thread, ,boot"] {
            assert_eq!(
                split_csv_allow_trailing_comma(input),
                Err("expected list element before ','".to_string()),
                "input '{}' should reject empty list element",
                input
            );
        }
    }

    #[test]
    fn parse_module_caps_accepts_trailing_comma() {
        let src = "@module_caps(PhysMap,); fn entry() { }";
        let ast = parse_module(src).expect("module caps with trailing comma should parse");
        assert_eq!(ast.module_caps, vec!["PhysMap".to_string()]);
    }

    #[test]
    fn parse_module_caps_rejects_empty_entry_with_diagnostic() {
        let src = "@module_caps(PhysMap,, MmioRaw); fn entry() { }";
        let err = parse_module(src).expect_err("empty module cap entry should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                0,
                "@module_caps(...) contains an empty capability entry",
            )]
        );
    }

    #[test]
    fn parse_rejects_invalid_mmio_register_declaration_rhs() {
        let src = "mmio UART0 = 0x1000; mmio_reg UART0.DR = BASE + 4 : u32 rw;";
        let err = parse_module(src).expect_err("invalid register rhs should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                51,
                "invalid mmio register declaration for 'UART0.DR': expected integer literal offset",
            )]
        );
    }

    #[test]
    fn parse_rejects_invalid_mmio_register_type_or_access() {
        let invalid_type = "mmio UART0 = 0x1000; mmio_reg UART0.DR = 0x00 : u128 rw;";
        let err_type = parse_module(invalid_type).expect_err("invalid register type should fail");
        assert_eq!(
            err_type,
            vec![diagnostic_at(
                invalid_type,
                52,
                "invalid mmio register declaration for 'UART0.DR': unsupported type 'u128'",
            )]
        );

        let invalid_access = "mmio UART0 = 0x1000; mmio_reg UART0.DR = 0x00 : u32 xx;";
        let err_access =
            parse_module(invalid_access).expect_err("invalid register access should fail");
        assert_eq!(
            err_access,
            vec![diagnostic_at(
                invalid_access,
                54,
                "invalid mmio register declaration for 'UART0.DR': unsupported access 'xx'",
            )]
        );
    }

    #[test]
    fn parse_rejects_duplicate_mmio_register_declarations() {
        let src = "mmio UART0 = 0x1000; mmio_reg UART0.DR = 0x00 : u32 rw; mmio_reg UART0.DR = 0x08 : u32 ro;";
        let err = parse_module(src).expect_err("duplicate mmio register declarations should fail");
        assert_eq!(err, vec!["duplicate mmio register 'UART0.DR'".to_string()]);
    }

    #[test]
    fn parse_rejects_invalid_mmio_base_declaration_rhs() {
        let src = "mmio UART0 = BASE + 4;";
        let err = parse_module(src).expect_err("invalid rhs should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                22,
                "invalid mmio base declaration for 'UART0': expected integer literal",
            )]
        );
    }

    #[test]
    fn parse_rejects_duplicate_mmio_base_declarations() {
        let src = "mmio UART0 = 0x1000; mmio UART0 = 0x2000;";
        let err = parse_module(src).expect_err("duplicate declarations should fail");
        assert_eq!(err, vec!["duplicate mmio base 'UART0'".to_string()]);
    }

    #[test]
    fn parse_rejects_mmio_declaration_inside_function_body() {
        let src = "fn entry() { mmio UART0 = 0x1000; }";
        let err = parse_module(src).expect_err("nested declaration should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                13,
                "mmio declarations are only allowed at module scope",
            )]
        );
    }

    #[test]
    fn parse_rejects_mmio_register_declaration_inside_function_body() {
        let src = "fn entry() { mmio_reg UART0.DR = 0x00 : u32 rw; }";
        let err = parse_module(src).expect_err("nested register declaration should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                13,
                "mmio_reg declarations are only allowed at module scope",
            )]
        );
    }

    #[test]
    fn parse_rejects_return_type_missing_after_arrow() {
        let src = "fn foo() -> { }";
        let err = parse_module(src).expect_err("missing return type should fail");
        assert_eq!(
            err,
            vec![diagnostic_at_with_help(
                src,
                12,
                "expected return type after '->'; valid types: u8, u16, u32, u64, bool",
                "add a return type here, e.g. `-> u64`",
            )]
        );
    }

    #[test]
    fn parse_rejects_if_without_condition() {
        let src = "fn foo() { if { } }";
        let err = parse_module(src).expect_err("if without condition should fail");
        assert_eq!(
            err,
            vec![diagnostic_at_with_help(
                src,
                11,
                "expected condition after 'if', found '{'",
                "add a boolean expression before the '{', e.g. `if x > 0 {`",
            )]
        );
    }

    #[test]
    fn int_literal_numeric_value_normalizes_decimal_hex_and_underscores() {
        assert_eq!(int_literal_numeric_value("4"), Some(4));
        assert_eq!(int_literal_numeric_value("0x04"), Some(4));
        assert_eq!(int_literal_numeric_value("0X4"), Some(4));
        assert_eq!(int_literal_numeric_value("0_4"), Some(4));
        assert_eq!(int_literal_numeric_value("0x1_0"), Some(16));
    }

    #[test]
    fn int_literal_numeric_value_rejects_invalid_tokens() {
        assert_eq!(int_literal_numeric_value(""), None);
        assert_eq!(int_literal_numeric_value("0x"), None);
        assert_eq!(int_literal_numeric_value("foo"), None);
        assert_eq!(int_literal_numeric_value("0xgg"), None);
    }

    #[test]
    fn parse_rejects_legacy_zero_arg_mmio_forms() {
        let src = "fn entry() { mmio_read(); mmio_write(); }";
        let err = parse_module(src).expect_err("legacy zero-arg mmio should fail");
        assert_eq!(
            err,
            vec![
                diagnostic_at(
                    src,
                    13,
                    "mmio_read() legacy form is unsupported; use mmio_read<T>(addr)",
                ),
                diagnostic_at(
                    src,
                    26,
                    "mmio_write() legacy form is unsupported; use mmio_write<T>(addr, value)",
                )
            ]
        );
    }

    #[test]
    fn parse_rejects_legacy_zero_arg_raw_mmio_forms() {
        let src = "fn entry() { raw_mmio_read(); raw_mmio_write(); }";
        let err = parse_module(src).expect_err("legacy zero-arg raw mmio should fail");
        assert_eq!(
            err,
            vec![
                diagnostic_at(
                    src,
                    13,
                    "raw_mmio_read() legacy form is unsupported; use raw_mmio_read<T>(addr)",
                ),
                diagnostic_at(
                    src,
                    30,
                    "raw_mmio_write() legacy form is unsupported; use raw_mmio_write<T>(addr, value)",
                )
            ]
        );
    }

    #[test]
    fn parse_rejects_unsupported_typed_mmio_element() {
        let src = "fn entry() { mmio_read<u128>(); }";
        let err = parse_module(src).expect_err("invalid mmio element type should fail");
        assert_eq!(err, vec![diagnostic_at(src, 13, "unknown type 'u128'",)]);
    }

    #[test]
    fn parse_rejects_malformed_typed_mmio_invocation() {
        let src = "fn entry() { mmio_write<u32( ); }";
        let err = parse_module(src).expect_err("malformed mmio invocation should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                13,
                "malformed mmio_write typed invocation 'mmio_write<u32'; expected mmio_write<T>()",
            )]
        );
    }

    #[test]
    fn parse_rejects_typed_mmio_read_missing_address_argument() {
        let src = "fn entry() { mmio_read<u32>(); }";
        let err = parse_module(src).expect_err("missing mmio read arg should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                13,
                "mmio_read<T>(addr[, slot]) requires one address argument and optional capture slot",
            )]
        );
    }

    #[test]
    fn parse_rejects_typed_mmio_read_invalid_capture_slot_operand() {
        let src = "fn entry() { mmio_read<u32>(UART0, 0x10); }";
        let err = parse_module(src).expect_err("invalid capture slot should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                13,
                "'0x10' is not a valid capture slot identifier",
            )]
        );
    }

    #[test]
    fn parse_rejects_branch_if_eq_invalid_compare_literal_operand() {
        let src = "fn entry() { branch_if_eq(status, ready, on_ready, on_idle); }";
        let err = parse_module(src).expect_err("invalid branch compare literal should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                13,
                "'ready' is not a valid branch comparison literal",
            )]
        );
    }

    #[test]
    fn parse_rejects_branch_if_mask_nonzero_invalid_mask_literal_operand() {
        let src = "fn entry() { branch_if_mask_nonzero(status, ready, on_ready, on_idle); }";
        let err = parse_module(src).expect_err("invalid branch mask literal should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                13,
                "'ready' is not a valid branch comparison literal",
            )]
        );
    }

    #[test]
    fn parse_rejects_return_slot_invalid_operand() {
        let src = "fn entry() { return_slot(0x10); }";
        let err = parse_module(src).expect_err("invalid return slot should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                13,
                "'0x10' is not a valid branch slot identifier",
            )]
        );
    }

    #[test]
    fn parse_rejects_cell_load_invalid_slot_operand() {
        let src = "fn entry() { cell_load<u32>(saved_status, 0x10); }";
        let err = parse_module(src).expect_err("invalid cell_load slot should fail");
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                13,
                "'0x10' is not a valid capture slot identifier",
            )]
        );
    }

    #[test]
    fn parse_rejects_typed_mmio_write_wrong_arity() {
        let src_missing = "fn entry() { mmio_write<u32>(addr); }";
        let err_missing = parse_module(src_missing).expect_err("missing value arg should fail");
        assert_eq!(
            err_missing,
            vec![diagnostic_at(
                src_missing,
                13,
                "mmio_write<T>(addr, value) requires exactly two arguments: address and value",
            )]
        );

        let src_extra = "fn entry() { mmio_write<u32>(addr, value, extra); }";
        let err_extra = parse_module(src_extra).expect_err("extra value arg should fail");
        assert_eq!(
            err_extra,
            vec![diagnostic_at(
                src_extra,
                13,
                "mmio_write<T>(addr, value) requires exactly two arguments: address and value",
            )]
        );
    }

    #[test]
    fn parse_rejects_typed_raw_mmio_write_wrong_arity() {
        let src_missing = "fn entry() { raw_mmio_write<u32>(addr); }";
        let err_missing =
            parse_module(src_missing).expect_err("missing raw mmio value arg should fail");
        assert_eq!(
            err_missing,
            vec![diagnostic_at(
                src_missing,
                13,
                "raw_mmio_write<T>(addr, value) requires exactly two arguments: address and value",
            )]
        );

        let src_extra = "fn entry() { raw_mmio_write<u32>(addr, value, extra); }";
        let err_extra = parse_module(src_extra).expect_err("extra raw mmio value arg should fail");
        assert_eq!(
            err_extra,
            vec![diagnostic_at(
                src_extra,
                13,
                "raw_mmio_write<T>(addr, value) requires exactly two arguments: address and value",
            )]
        );
    }

    #[test]
    fn parse_rejects_unsupported_typed_mmio_operand_shapes() {
        let read_add = "fn entry() { mmio_read<u32>(a + b); }";
        let err_read_add = parse_module(read_add).expect_err("unsupported addr shape should fail");
        assert_eq!(
            err_read_add,
            vec![diagnostic_at(
                read_add,
                13,
                "unsupported mmio address operand 'a + b'; expected identifier, integer literal, or identifier + integer literal",
            )]
        );

        let read_chain = "fn entry() { mmio_read<u32>(a + 1 + 2); }";
        let err_read_chain =
            parse_module(read_chain).expect_err("unsupported chained addr shape should fail");
        assert_eq!(
            err_read_chain,
            vec![diagnostic_at(
                read_chain,
                13,
                "unsupported mmio address operand 'a + 1 + 2'; expected identifier, integer literal, or identifier + integer literal",
            )]
        );

        let write_value = "fn entry() { mmio_write<u32>(addr, a + b); }";
        let err_write_value =
            parse_module(write_value).expect_err("unsupported value shape should fail");
        assert_eq!(
            err_write_value,
            vec![diagnostic_at(
                write_value,
                13,
                "unsupported mmio value operand 'a + b'; expected identifier or integer literal",
            )]
        );
    }

    #[test]
    fn parse_slice_param_roundtrip() {
        let src = "fn process(data: [u8]) {}";
        let ast = parse_module(src).expect("parse");
        let f = &ast.items[0];
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].0, "data");
        assert_eq!(f.params[0].1, ParamTy::Slice(MmioScalarType::U8));
    }

    #[test]
    fn parse_mixed_scalar_and_slice_params() {
        let src = "fn copy(base: u64, data: [u32]) {}";
        let ast = parse_module(src).expect("parse");
        let f = &ast.items[0];
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].1, ParamTy::Scalar(MmioScalarType::U64));
        assert_eq!(f.params[1].1, ParamTy::Slice(MmioScalarType::U32));
    }

    #[test]
    fn parse_slice_len_stmt() {
        let src = "fn f(data: [u8]) { slice_len(data, len_val); }";
        let ast = parse_module(src).expect("parse");
        let body = &ast.items[0].body;
        assert_eq!(body.len(), 1);
        assert!(
            matches!(&body[0], Stmt::SliceLen { slice, slot } if slice == "data" && slot == "len_val")
        );
    }

    #[test]
    fn parse_slice_ptr_stmt() {
        let src = "fn f(data: [u8]) { slice_ptr(data, ptr_val); }";
        let ast = parse_module(src).expect("parse");
        let body = &ast.items[0].body;
        assert_eq!(body.len(), 1);
        assert!(
            matches!(&body[0], Stmt::SlicePtr { slice, slot } if slice == "data" && slot == "ptr_val")
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn parse_module_never_panics_on_small_random_input(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            let input = String::from_utf8_lossy(&bytes).to_string();
            let result = std::panic::catch_unwind(|| parse_module(&input));
            prop_assert!(result.is_ok());
        }
    }
}

#[cfg(test)]
mod type_tests {
    use super::*;
    #[test]
    fn scalar_type_roundtrip() {
        assert_eq!(MmioScalarType::parse("float32"), Ok(MmioScalarType::F32));
        assert_eq!(MmioScalarType::parse("float64"), Ok(MmioScalarType::F64));
        assert_eq!(MmioScalarType::parse("float16"), Ok(MmioScalarType::F16));
        assert_eq!(MmioScalarType::parse("uint8"), Ok(MmioScalarType::U8));
        assert_eq!(MmioScalarType::parse("uint32"), Ok(MmioScalarType::U32));
        assert_eq!(MmioScalarType::parse("int8"), Ok(MmioScalarType::I8));
        assert_eq!(MmioScalarType::parse("int32"), Ok(MmioScalarType::I32));
        assert_eq!(MmioScalarType::parse("bool"), Ok(MmioScalarType::Bool));
        assert_eq!(MmioScalarType::parse("char"), Ok(MmioScalarType::Char));
        assert_eq!(MmioScalarType::parse("byte"), Ok(MmioScalarType::U8));
        assert_eq!(MmioScalarType::parse("addr"), Ok(MmioScalarType::U64));
    }

    #[test]
    fn expr_ast_compiles() {
        // just checks the types exist and are usable
        let _e = Expr::IntLiteral(42);
        let _e2 = Expr::BinOp {
            op: BinOpKind::Add,
            lhs: Box::new(Expr::IntLiteral(1)),
            rhs: Box::new(Expr::IntLiteral(2)),
        };
        let _d = DeviceDecl {
            name: "UART0".to_string(),
            base_addr: "0x3F000000".to_string(),
            registers: vec![],
            source: SourceNote {
                byte_offset: 0,
                line: 1,
                column: 1,
                line_text: String::new(),
            },
        };
    }
}

#[cfg(test)]
mod lexer_tests {
    use super::*;

    fn tok(src: &str) -> Vec<Token> {
        Lexer::new(src).collect_all().unwrap()
    }

    #[test]
    fn lex_keywords_and_idents() {
        let t = tok("fn entry() { }");
        assert!(t.iter().any(|t| matches!(t.kind, TokenKind::Fn)));
        assert!(
            t.iter()
                .any(|t| matches!(&t.kind, TokenKind::Ident(s) if s == "entry"))
        );
    }

    #[test]
    fn lex_type_keywords() {
        let t = tok("uint32 int8 float32 bool char byte addr string");
        let types: Vec<_> = t
            .iter()
            .filter_map(|t| {
                if let TokenKind::TypeKw(ty) = &t.kind {
                    Some(*ty)
                } else {
                    None
                }
            })
            .collect();
        // `string` produces StringKw (not TypeKw), so 7 entries, not 8
        assert_eq!(
            types,
            vec![
                MmioScalarType::U32,
                MmioScalarType::I8,
                MmioScalarType::F32,
                MmioScalarType::Bool,
                MmioScalarType::Char,
                MmioScalarType::U8,
                MmioScalarType::U64, // addr -> U64
            ]
        );
    }

    #[test]
    fn lex_int_literals() {
        let t = tok("42 0xFF 0b1010");
        let ints: Vec<u64> = t
            .iter()
            .filter_map(|t| {
                if let TokenKind::IntLit(n) = t.kind {
                    Some(n)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(ints, vec![42, 255, 10]);
    }

    #[test]
    fn lex_char_literal_escapes() {
        let t = tok(r"'\n' '\t' '\xFF' 'A'");
        let chars: Vec<u8> = t
            .iter()
            .filter_map(|t| {
                if let TokenKind::CharLit(c) = t.kind {
                    Some(c)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(chars, vec![b'\n', b'\t', 0xFF, b'A']);
    }

    #[test]
    fn lex_float_literal() {
        let t = tok("3.14 0.5 1.0e-3");
        assert_eq!(
            t.iter()
                .filter(|t| matches!(t.kind, TokenKind::FloatLit(_)))
                .count(),
            3
        );
    }

    #[test]
    fn lex_string_literal_escapes() {
        let t = tok(r#""Hello\nWorld\xFF""#);
        let s: Vec<_> = t
            .iter()
            .filter_map(|t| {
                if let TokenKind::StrLit(s) = &t.kind {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect();
        // \xFF in the lexer produces byte 0xFF cast to char → U+00FF (ÿ)
        assert_eq!(s[0], "Hello\nWorld\u{00FF}");
    }

    #[test]
    fn lex_operators_and_punctuation() {
        let t = tok("+= -= &= |= ^= <<= >>= .. ..= ->");
        let ops: Vec<_> = t.iter().map(|t| &t.kind).collect();
        assert!(ops.iter().any(|o| matches!(o, TokenKind::PlusEq)));
        assert!(ops.iter().any(|o| matches!(o, TokenKind::ShlEq)));
        assert!(ops.iter().any(|o| matches!(o, TokenKind::ShrEq)));
        assert!(ops.iter().any(|o| matches!(o, TokenKind::DotDot)));
        assert!(ops.iter().any(|o| matches!(o, TokenKind::DotDotEq)));
        assert!(ops.iter().any(|o| matches!(o, TokenKind::Arrow)));
    }

    #[test]
    fn lex_no_semicolons_needed() {
        // newlines are NOT statement terminators — the lexer just ignores them
        let t = tok("uint32 x = 5\nuint32 y = 10");
        assert_eq!(
            t.iter()
                .filter(|t| matches!(t.kind, TokenKind::TypeKw(_)))
                .count(),
            2
        );
    }

    #[test]
    fn lex_shift_comparison_operators() {
        // Verify <, <=, <<, <<= are all distinct
        let t = tok("< <= << <<=");
        let kinds: Vec<_> = t
            .iter()
            .filter(|t| !matches!(t.kind, TokenKind::Eof))
            .map(|t| &t.kind)
            .collect();
        assert!(kinds.iter().any(|k| matches!(k, TokenKind::Lt)));
        assert!(kinds.iter().any(|k| matches!(k, TokenKind::LtEq)));
        assert!(kinds.iter().any(|k| matches!(k, TokenKind::Shl)));
        assert!(kinds.iter().any(|k| matches!(k, TokenKind::ShlEq)));
    }
}

#[cfg(test)]
mod expr_tests {
    use super::*;

    fn parse_expr(src: &str) -> Expr {
        let tokens = Lexer::new(src).collect_all().unwrap();
        let mut p = TokParser::new(tokens);
        p.parse_expr(0).unwrap()
    }

    #[test]
    fn parse_int_literal() {
        assert_eq!(parse_expr("42"), Expr::IntLiteral(42));
    }
    #[test]
    fn parse_bool_literal() {
        assert_eq!(parse_expr("true"), Expr::BoolLiteral(true));
    }
    #[test]
    fn parse_char_literal() {
        assert_eq!(parse_expr("'A'"), Expr::CharLiteral(b'A'));
    }
    #[test]
    fn parse_ident() {
        assert!(matches!(parse_expr("status"), Expr::Ident(s) if s == "status"));
    }
    #[test]
    fn parse_add() {
        assert!(matches!(
            parse_expr("a + b"),
            Expr::BinOp {
                op: BinOpKind::Add,
                ..
            }
        ));
    }
    #[test]
    fn parse_precedence() {
        // a + b & c  should be (a + b) & c because + has higher precedence than &
        let e = parse_expr("a + b & c");
        assert!(matches!(
            e,
            Expr::BinOp {
                op: BinOpKind::And,
                ..
            }
        ));
    }
    #[test]
    fn parse_device_field() {
        let e = parse_expr("UART0.Status");
        assert!(
            matches!(e, Expr::DeviceField { ref device, ref field } if device == "UART0" && field == "Status")
        );
    }
    #[test]
    fn parse_fn_call_expr() {
        let e = parse_expr("get_status()");
        assert!(matches!(e, Expr::Call { ref callee, .. } if callee == "get_status"));
    }
    #[test]
    fn parse_unary_not() {
        assert!(matches!(
            parse_expr("!flag"),
            Expr::UnOp {
                op: UnOpKind::Not,
                ..
            }
        ));
    }
    #[test]
    fn parse_parens() {
        // (a + b) * c — parens override precedence
        let e = parse_expr("(a + b) * c");
        assert!(matches!(
            e,
            Expr::BinOp {
                op: BinOpKind::Mul,
                ..
            }
        ));
    }
    #[test]
    fn parse_comparison() {
        let e = parse_expr("x == 0");
        assert!(matches!(
            e,
            Expr::BinOp {
                op: BinOpKind::Eq,
                ..
            }
        ));
    }
    #[test]
    fn parse_logical_and() {
        let e = parse_expr("a && b");
        assert!(matches!(
            e,
            Expr::BinOp {
                op: BinOpKind::LogAnd,
                ..
            }
        ));
    }
}
