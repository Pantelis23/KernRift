# KernRift Syntax Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current low-level KernRift syntax with a clean C/Python hybrid surface language while keeping the KRIR IR and all safety verification passes unchanged.

**Architecture:** New tokenizer + parser produces the existing `ModuleAst`/`FnAst` structure with extensions (new Stmt variants, Expr nodes, DeviceDecl, return types). The HIR lowering is extended to handle the new AST nodes, translating them to the same KRIR ops the backend already knows. Loop support requires new KRIR ops and backend label/jump infrastructure.

**Tech Stack:** Rust, existing `crates/parser`, `crates/hir`, `crates/krir`. No new crates. All tests via `cargo test` from the repo root.

**Spec:** `docs/superpowers/specs/2026-03-21-syntax-redesign-design.md`

---

## Codebase Notes (read before implementing)

- **`KrirStmt` does not exist** — use `Stmt` (from `crates/parser/src/lib.rs`) for parser-level statements. KRIR-level statements are `KrirOp` variants in `crates/krir/src/lib.rs`. Code samples in Tasks 7–12 that reference `KrirStmt::*` should be read as the equivalent `Stmt::*` or `KrirOp::*` variant.
- **`SlotId` does not exist** — all slot references in KRIR are `String`.
- **`compile_module(src)`** does not exist — use `compile_source(src: &str) -> Result<KrirModule, Vec<String>>` from `kernriftc`.
- **`validate_op_sequence`** does not exist — use the actual validator on a full `ExecutableKrirModule`.
- **`emit_simple_while_loop()` and `asm_to_text()`** are test helpers — define them inline in the test using the existing `lower_executable_krir_to_x86_64_asm` and `emit_x86_64_asm_text` functions from `crates/krir/src/lib.rs`.
- **`process_escape_sequences`** — the lexer already decodes escapes in `StrLit` tokens. Use `.as_bytes()` directly; null-terminator offset = number of bytes in the decoded string.
- **`*`, `/`, `%`** — per spec §17, these are deferred (reserved tokens, emit error). Task 13 is a simple "emit error" task, not a full implementation.
- **Floats** (`float32`, `float64`) — ARE in scope for V1 per spec §2.7. Task 14 (SSE2 float ops) is correct.

---

## File Map

| File | Change |
|------|--------|
| `crates/parser/src/lib.rs` | Full rewrite of lexer + parser. Keep `SourceNote`, `format_source_diagnostic`, `split_csv*`, `MmioRegAccess`, `MmioBaseDecl`, `MmioRegisterDecl`, `ConstDecl`, `EnumDecl`/`EnumVariant`, `StructDecl`/`StructField`, `PercpuDecl`, `ModuleAst`, `FnAst`. Extend `MmioScalarType`. Add `TypeKind`, `Expr`, `DeviceDecl`, `DeviceRegDecl`. Add new `Stmt` variants. Rename `spinlocks` → `locks`. Add `return_ty` to `FnAst`. Rewrite tokenizer and all `parse_*` functions. |
| `crates/hir/src/lib.rs` | Add device symbol resolution pass. Add `Expr` lowering (arithmetic, comparison, field access). Add new `Stmt` lowering paths (VarDecl, Assign, If, While, For, Return, Break, Continue, Print). Add `return_ty` to function lowering. |
| `crates/krir/src/lib.rs` | Add `LoopBegin`/`LoopEnd`/`LoopBreak`/`LoopContinue` to `KrirOp`/`ExecutableOp`. Add `F32`/`F64`/`F16` to `MmioScalarType`. Add `Mul`/`Div`/`Mod` to `ArithOp`. Add signed ops. Add `Label`/`Jmp`/`JmpIfZero`/`JmpIfNonZero` to `X86_64AsmInstruction`. Add REL32 encoding to object byte encoder. Relax single-block constraint for loop-containing functions. |
| `tests/must_pass/*.kr` | Rewrite all files to new syntax after parser is complete. |
| `tests/must_fail/*.kr` | Rewrite all files to new syntax. |
| `crates/kernriftc/tests/kr3_contract.rs` | Update inline .kr snippets to new syntax. |
| `docs/LANGUAGE.md` | Full rewrite for new syntax (final task). |

---

## Task 1: Extend `MmioScalarType` with float and new integer widths

**Files:**
- Modify: `crates/parser/src/lib.rs:48-87`

These are new type variants needed before anything else. All downstream code is updated in the same task.

- [ ] **Step 1: Write a failing test**

Add to `crates/parser/src/lib.rs` bottom:
```rust
#[cfg(test)]
mod type_tests {
    use super::*;
    #[test]
    fn scalar_type_roundtrip() {
        assert_eq!(MmioScalarType::parse("float32"), Ok(MmioScalarType::F32));
        assert_eq!(MmioScalarType::parse("float64"), Ok(MmioScalarType::F64));
        assert_eq!(MmioScalarType::parse("float16"), Ok(MmioScalarType::F16));
        assert_eq!(MmioScalarType::parse("uint8"),   Ok(MmioScalarType::U8));
        assert_eq!(MmioScalarType::parse("uint32"),  Ok(MmioScalarType::U32));
        assert_eq!(MmioScalarType::parse("int8"),    Ok(MmioScalarType::I8));
        assert_eq!(MmioScalarType::parse("int32"),   Ok(MmioScalarType::I32));
        assert_eq!(MmioScalarType::parse("bool"),    Ok(MmioScalarType::Bool));
        assert_eq!(MmioScalarType::parse("char"),    Ok(MmioScalarType::Char));
        assert_eq!(MmioScalarType::parse("byte"),    Ok(MmioScalarType::U8));
        assert_eq!(MmioScalarType::parse("addr"),    Ok(MmioScalarType::U64));
    }
}
```

- [ ] **Step 2: Run test — expect FAIL**
```bash
cargo test -p parser scalar_type_roundtrip 2>&1 | tail -5
```

- [ ] **Step 3: Extend `MmioScalarType`**

Replace the existing enum and `parse`/`as_str`/`byte_size` impls:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioScalarType {
    U8, U16, U32, U64,       // unsigned integers
    I8, I16, I32, I64,       // signed (stored as unsigned bitwise equiv)
    F32, F64, F16,            // floats (F16 = storage only)
    Bool,                     // lowers to U8, values 0/1
    Char,                     // lowers to U8, ASCII 0-127
}

impl MmioScalarType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::U8  => "uint8",  Self::U16 => "uint16",
            Self::U32 => "uint32", Self::U64 => "uint64",
            Self::I8  => "int8",   Self::I16 => "int16",
            Self::I32 => "int32",  Self::I64 => "int64",
            Self::F32 => "float32",Self::F64 => "float64",
            Self::F16 => "float16",
            Self::Bool => "bool",  Self::Char => "char",
        }
    }

    /// Returns the underlying storage type (signed types use unsigned storage).
    pub fn storage_type(self) -> Self {
        match self {
            Self::I8  => Self::U8,  Self::I16 => Self::U16,
            Self::I32 => Self::U32, Self::I64 => Self::U64,
            Self::Bool | Self::Char => Self::U8,
            Self::F16 => Self::U16,
            other => other,
        }
    }

    pub fn byte_size(self) -> u8 {
        match self {
            Self::U8  | Self::I8  | Self::Bool | Self::Char => 1,
            Self::U16 | Self::I16 | Self::F16               => 2,
            Self::U32 | Self::I32 | Self::F32               => 4,
            Self::U64 | Self::I64 | Self::F64               => 8,
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
            "uint8"  | "u8"   | "byte" => Ok(Self::U8),
            "uint16" | "u16"           => Ok(Self::U16),
            "uint32" | "u32"           => Ok(Self::U32),
            "uint64" | "u64"  | "addr" => Ok(Self::U64),
            "int8"   | "i8"            => Ok(Self::I8),
            "int16"  | "i16"           => Ok(Self::I16),
            "int32"  | "i32"           => Ok(Self::I32),
            "int64"  | "i64"           => Ok(Self::I64),
            "float32"| "f32"           => Ok(Self::F32),
            "float64"| "f64"           => Ok(Self::F64),
            "float16"| "f16"           => Ok(Self::F16),
            "bool"                     => Ok(Self::Bool),
            "char"                     => Ok(Self::Char),
            other => Err(format!("unknown type '{}'", other)),
        }
    }
}
```

- [ ] **Step 4: Fix all match exhaustiveness errors**
```bash
cargo build -p parser 2>&1 | grep "error\[" | head -20
```
Fix each arm that doesn't cover new variants. Old code that matches on `MmioScalarType` needs `I8 | I16 | I32 | I64 | F32 | F64 | F16 | Bool | Char` arms — map to nearest existing behavior (e.g. treat as `U8`/`U64` via `storage_type()`).

- [ ] **Step 5: Run test — expect PASS**
```bash
cargo test -p parser scalar_type_roundtrip
```

- [ ] **Step 6: Run full test suite**
```bash
cargo test 2>&1 | tail -10
```

- [ ] **Step 7: Commit**
```bash
git add crates/parser/src/lib.rs
git commit -m "feat(parser): extend MmioScalarType with int/float/bool/char/signed variants"
```

---

## Task 2: Add `Expr` AST and `DeviceDecl` types

**Files:**
- Modify: `crates/parser/src/lib.rs` — add new types after line 155

These are the new AST nodes that the parser will produce. Adding them now (before the parser rewrite) lets HIR code reference them cleanly.

- [ ] **Step 1: Write a failing compilation test**

Add to the test module:
```rust
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
        source: SourceNote { byte_offset: 0, line: 1, column: 1, line_text: String::new() },
    };
}
```

- [ ] **Step 2: Run — expect FAIL (types don't exist yet)**
```bash
cargo test -p parser expr_ast_compiles 2>&1 | head -10
```

- [ ] **Step 3: Add the new types** after line 155 in `crates/parser/src/lib.rs`:

```rust
/// Binary operator kinds — precedence enforced by the Pratt parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    Add, Sub,                              // arithmetic
    And, Or, Xor, Shl, Shr,               // bitwise
    Mul, Div, Rem,                         // arithmetic (deferred V1 — parser accepts, HIR rejects)
    Eq, Ne, Lt, Gt, Le, Ge,               // comparison
    LogAnd, LogOr,                         // logical
}

/// Unary operator kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOpKind {
    Not,   // logical not  `!`
    BitNot,// bitwise not  `~`
    Neg,   // arithmetic negation `-`
}

/// A full expression — replaces the old `MmioValueExpr` (which was Ident | IntLiteral only).
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    IntLiteral(u64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    CharLiteral(u8),
    StringLiteral(String),              // for `print` intrinsic
    Ident(String),                      // local variable or param slot
    DeviceField {                       // `UART0.Status`
        device: String,
        field: String,
    },
    SliceLen(String),                   // `buf.len`
    Call {                              // `get_status()`
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
```

Also update `ModuleAst` to add devices and rename spinlocks:
```rust
pub struct ModuleAst {
    pub module_caps: Vec<String>,
    pub mmio_bases: Vec<MmioBaseDecl>,
    pub mmio_registers: Vec<MmioRegisterDecl>,
    pub devices: Vec<DeviceDecl>,          // NEW
    pub constants: Vec<ConstDecl>,
    pub enums: Vec<EnumDecl>,
    pub structs: Vec<StructDecl>,
    pub locks: Vec<String>,                // RENAMED from spinlocks
    pub percpu_vars: Vec<PercpuDecl>,
    pub items: Vec<FnAst>,
}
```

And update `FnAst`:
```rust
pub struct FnAst {
    pub name: String,
    pub is_extern: bool,
    pub params: Vec<(String, ParamTy)>,
    pub return_ty: Option<MmioScalarType>,  // NEW — None = void
    pub attrs: Vec<RawAttr>,
    pub body: Vec<Stmt>,
    pub source: SourceNote,
}
```

And add new `Stmt` variants at the end of the `Stmt` enum:
```rust
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
        else_body: Vec<Stmt>,     // empty = no else
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
```

Also add `AssignTarget`:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Ident(String),                // local variable
    DeviceField { device: String, field: String },  // UART0.Data = b
}
```

- [ ] **Step 4: Fix all downstream compilation errors** — `ModuleAst::spinlocks` renamed to `locks`, `FnAst` needs `return_ty` initialised everywhere it's constructed.
```bash
cargo build 2>&1 | grep "error\[" | head -30
```
For each error: add `return_ty: None` to FnAst constructors; change `spinlocks` to `locks` in HIR code.

- [ ] **Step 5: Run tests — expect PASS**
```bash
cargo test 2>&1 | tail -5
```

- [ ] **Step 6: Commit**
```bash
git add crates/parser/src/lib.rs crates/hir/src/lib.rs
git commit -m "feat(parser): add Expr, DeviceDecl, new Stmt variants, return_ty to FnAst"
```

---

## Task 3: New tokenizer

**Files:**
- Modify: `crates/parser/src/lib.rs` — replace the current character-level `Parser` struct with a two-phase tokenizer + parser

The current parser processes characters directly. The new syntax is complex enough to need a proper token stream. This task adds the tokenizer; the next tasks rebuild the parser on top of it.

- [ ] **Step 1: Write tokenizer tests**

```rust
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
        assert!(t.iter().any(|t| matches!(&t.kind, TokenKind::Ident(s) if s == "entry")));
    }

    #[test]
    fn lex_type_keywords() {
        let t = tok("uint32 int8 float32 bool char byte addr string");
        let types: Vec<_> = t.iter()
            .filter_map(|t| if let TokenKind::TypeKw(ty) = &t.kind { Some(*ty) } else { None })
            .collect();
        assert_eq!(types, vec![
            MmioScalarType::U32, MmioScalarType::I8, MmioScalarType::F32,
            MmioScalarType::Bool, MmioScalarType::Char, MmioScalarType::U8,
            MmioScalarType::U64, // addr -> U64; string is special
        ]);
    }

    #[test]
    fn lex_int_literals() {
        let t = tok("42 0xFF 0b1010");
        let ints: Vec<u64> = t.iter()
            .filter_map(|t| if let TokenKind::IntLit(n) = t.kind { Some(n) } else { None })
            .collect();
        assert_eq!(ints, vec![42, 255, 10]);
    }

    #[test]
    fn lex_char_literal_escapes() {
        let t = tok(r"'\n' '\t' '\xFF' 'A'");
        let chars: Vec<u8> = t.iter()
            .filter_map(|t| if let TokenKind::CharLit(c) = t.kind { Some(c) } else { None })
            .collect();
        assert_eq!(chars, vec![b'\n', b'\t', 0xFF, b'A']);
    }

    #[test]
    fn lex_float_literal() {
        let t = tok("3.14 0.5 1.0e-3");
        assert_eq!(t.iter().filter(|t| matches!(t.kind, TokenKind::FloatLit(_))).count(), 3);
    }

    #[test]
    fn lex_string_literal_escapes() {
        let t = tok(r#""Hello\nWorld\xFF""#);
        let s: Vec<_> = t.iter()
            .filter_map(|t| if let TokenKind::StrLit(s) = &t.kind { Some(s.clone()) } else { None })
            .collect();
        assert_eq!(s[0], "Hello\nWorld\xFF");
    }

    #[test]
    fn lex_operators_and_punctuation() {
        let t = tok("+= -= &= |= ^= <<= >>= .. ..= ->");
        let ops: Vec<_> = t.iter().map(|t| &t.kind).collect();
        assert!(ops.iter().any(|o| matches!(o, TokenKind::PlusEq)));
        assert!(ops.iter().any(|o| matches!(o, TokenKind::DotDot)));
        assert!(ops.iter().any(|o| matches!(o, TokenKind::DotDotEq)));
        assert!(ops.iter().any(|o| matches!(o, TokenKind::Arrow)));
    }

    #[test]
    fn lex_no_semicolons_needed() {
        // newlines are NOT statement terminators — the lexer just ignores them
        let t = tok("uint32 x = 5\nuint32 y = 10");
        assert_eq!(t.iter().filter(|t| matches!(t.kind, TokenKind::TypeKw(_))).count(), 2);
    }
}
```

- [ ] **Step 2: Run — expect FAIL**
```bash
cargo test -p parser lexer_tests 2>&1 | head -5
```

- [ ] **Step 3: Add the `Token`/`TokenKind`/`Lexer` types** just before the existing `Parser` struct.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Fn, Extern, Return, Break, Continue,
    If, Else, While, For, In,
    Const, Struct, Enum, Device, At, Lock, Percpu,
    Acquire, Release, Critical, Unsafe, Yieldpoint,
    Print, Raw_write, Raw_read,
    True, False,
    // Type keywords — carry the resolved type
    TypeKw(MmioScalarType),
    StringKw,       // `string` type keyword (special: []char)
    // Literals
    IntLit(u64),
    FloatLit(f64),
    CharLit(u8),
    StrLit(String),
    // Identifier
    Ident(String),
    // Punctuation
    LBrace, RBrace, LParen, RParen, LBracket, RBracket,
    Comma, Colon, Semicolon, Dot, DotDot, DotDotEq,
    Arrow,          // `->`
    // Operators
    Plus, Minus, Star, Slash, Percent,
    Amp, Pipe, Caret, Tilde, Bang,
    Shl, Shr,
    Eq, EqEq, BangEq, Lt, Gt, LtEq, GtEq,
    AmpAmp, PipePipe,
    // Compound assignment
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    AmpEq, PipeEq, CaretEq, ShlEq, ShrEq,
    // Attributes
    At_sign,       // `@` before attribute names
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
            if is_eof { break; }
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
            // Skip whitespace (including newlines — not significant)
            while self.peek_char().map(|c| c.is_whitespace()).unwrap_or(false) {
                self.advance();
            }
            // Skip // line comments
            if self.src[self.pos..].starts_with("//") {
                while self.peek_char().map(|c| c != '\n').unwrap_or(false) {
                    self.advance();
                }
                continue;
            }
            // Skip /* block comments */
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
        // opening `"` already consumed
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
        // opening `'` already consumed
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
            Some('n')  => Ok('\n'),
            Some('r')  => Ok('\r'),
            Some('t')  => Ok('\t'),
            Some('b')  => Ok('\x08'),
            Some('a')  => Ok('\x07'),
            Some('f')  => Ok('\x0C'),
            Some('v')  => Ok('\x0B'),
            Some('\\') => Ok('\\'),
            Some('\'') => Ok('\''),
            Some('"')  => Ok('"'),
            Some('0')  => Ok('\0'),
            Some('x')  => {
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
            Some('@') => TokenKind::At_sign,
            Some('"') => TokenKind::StrLit(self.lex_string_literal()?),
            Some('\'') => TokenKind::CharLit(self.lex_char_literal()?),
            Some('.') => {
                if self.src[self.pos..].starts_with(".=") { self.pos += 2; TokenKind::DotDotEq }
                else if self.src[self.pos..].starts_with('.') { self.pos += 1; TokenKind::DotDot }
                else { TokenKind::Dot }
            }
            Some('-') => {
                if self.src[self.pos..].starts_with('>') { self.pos += 1; TokenKind::Arrow }
                else if self.src[self.pos..].starts_with('=') { self.pos += 1; TokenKind::MinusEq }
                else { TokenKind::Minus }
            }
            Some('+') => if self.src[self.pos..].starts_with('=') { self.pos += 1; TokenKind::PlusEq  } else { TokenKind::Plus  },
            Some('*') => if self.src[self.pos..].starts_with('=') { self.pos += 1; TokenKind::StarEq  } else { TokenKind::Star  },
            Some('/') => if self.src[self.pos..].starts_with('=') { self.pos += 1; TokenKind::SlashEq } else { TokenKind::Slash },
            Some('%') => if self.src[self.pos..].starts_with('=') { self.pos += 1; TokenKind::PercentEq } else { TokenKind::Percent },
            Some('^') => if self.src[self.pos..].starts_with('=') { self.pos += 1; TokenKind::CaretEq } else { TokenKind::Caret },
            Some('!') => if self.src[self.pos..].starts_with('=') { self.pos += 1; TokenKind::BangEq  } else { TokenKind::Bang  },
            Some('=') => if self.src[self.pos..].starts_with('=') { self.pos += 1; TokenKind::EqEq    } else { TokenKind::Eq    },
            Some('<') => {
                // `<<=` must be checked before `<<` and `<=`
                if self.src[self.pos..].starts_with("<<=") { self.pos += 3; TokenKind::ShlEq }
                else if self.src[self.pos..].starts_with("<=") { self.pos += 2; TokenKind::LtEq }
                else if self.src[self.pos..].starts_with('<') { self.pos += 1; TokenKind::Shl }
                else { TokenKind::Lt }
            }
            Some('>') => {
                if self.src[self.pos..].starts_with(">=") { self.pos += 2; TokenKind::GtEq }
                else if self.src[self.pos..].starts_with(">>=") { self.pos += 3; TokenKind::ShrEq }
                else if self.src[self.pos..].starts_with('>') { self.pos += 1; TokenKind::Shr }
                else { TokenKind::Gt }
            }
            Some('&') => {
                if self.src[self.pos..].starts_with('&') { self.pos += 1; TokenKind::AmpAmp }
                else if self.src[self.pos..].starts_with('=') { self.pos += 1; TokenKind::AmpEq }
                else { TokenKind::Amp }
            }
            Some('|') => {
                if self.src[self.pos..].starts_with('|') { self.pos += 1; TokenKind::PipePipe }
                else if self.src[self.pos..].starts_with('=') { self.pos += 1; TokenKind::PipeEq }
                else { TokenKind::Pipe }
            }
            Some(c) if c.is_ascii_digit() => {
                let start = self.pos - 1;
                // hex: 0x...
                if c == '0' && self.src[self.pos..].starts_with('x') {
                    self.pos += 1;
                    while self.peek_char().map(|c| c.is_ascii_hexdigit()).unwrap_or(false) { self.advance(); }
                    let s = &self.src[start..self.pos];
                    let n = u64::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())?;
                    TokenKind::IntLit(n)
                }
                // binary: 0b...
                else if c == '0' && self.src[self.pos..].starts_with('b') {
                    self.pos += 1;
                    while self.peek_char().map(|c| c == '0' || c == '1').unwrap_or(false) { self.advance(); }
                    let s = &self.src[start..self.pos];
                    let n = u64::from_str_radix(&s[2..], 2).map_err(|e| e.to_string())?;
                    TokenKind::IntLit(n)
                }
                // decimal or float
                else {
                    while self.peek_char().map(|c| c.is_ascii_digit()).unwrap_or(false) { self.advance(); }
                    let is_float = self.peek_char() == Some('.')
                        && self.src[self.pos+1..].chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
                    if is_float {
                        self.advance(); // consume '.'
                        while self.peek_char().map(|c| c.is_ascii_digit()).unwrap_or(false) { self.advance(); }
                        // optional exponent
                        if self.peek_char() == Some('e') || self.peek_char() == Some('E') {
                            self.advance();
                            if self.peek_char() == Some('-') || self.peek_char() == Some('+') { self.advance(); }
                            while self.peek_char().map(|c| c.is_ascii_digit()).unwrap_or(false) { self.advance(); }
                        }
                        let f: f64 = self.src[start..self.pos].parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;
                        TokenKind::FloatLit(f)
                    } else {
                        let n: u64 = self.src[start..self.pos].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                        TokenKind::IntLit(n)
                    }
                }
            }
            Some(c) if c.is_alphabetic() || c == '_' => {
                let start = self.pos - 1;
                while self.peek_char().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false) { self.advance(); }
                let word = &self.src[start..self.pos];
                match word {
                    "fn"        => TokenKind::Fn,
                    "extern"    => TokenKind::Extern,
                    "return"    => TokenKind::Return,
                    "break"     => TokenKind::Break,
                    "continue"  => TokenKind::Continue,
                    "if"        => TokenKind::If,
                    "else"      => TokenKind::Else,
                    "while"     => TokenKind::While,
                    "for"       => TokenKind::For,
                    "in"        => TokenKind::In,
                    "const"     => TokenKind::Const,
                    "struct"    => TokenKind::Struct,
                    "enum"      => TokenKind::Enum,
                    "device"    => TokenKind::Device,
                    "at"        => TokenKind::At,
                    "lock"      => TokenKind::Lock,
                    "percpu"    => TokenKind::Percpu,
                    "acquire"   => TokenKind::Acquire,
                    "release"   => TokenKind::Release,
                    "critical"  => TokenKind::Critical,
                    "unsafe"    => TokenKind::Unsafe,
                    "yieldpoint"=> TokenKind::Yieldpoint,
                    "print"     => TokenKind::Print,
                    "true"      => TokenKind::True,
                    "false"     => TokenKind::False,
                    "string"    => TokenKind::StringKw,
                    _ => match MmioScalarType::parse(word) {
                        Ok(ty)  => TokenKind::TypeKw(ty),
                        Err(_)  => TokenKind::Ident(word.to_string()),
                    }
                }
            }
            Some(c) => return Err(format!("unexpected character '{}'", c)),
        };
        Ok(Token { kind, source: note })
    }
}
```

- [ ] **Step 4: Run lexer tests — expect PASS**
```bash
cargo test -p parser lexer_tests 2>&1 | tail -10
```

- [ ] **Step 5: Run full suite**
```bash
cargo test 2>&1 | tail -5
```

- [ ] **Step 6: Commit**
```bash
git add crates/parser/src/lib.rs
git commit -m "feat(parser): new tokenizer with full KernRift surface syntax token set"
```

---

## Task 4: Pratt expression parser

**Files:**
- Modify: `crates/parser/src/lib.rs` — add expression parser on top of the token stream

The new parser uses a token-stream `Parser` struct (different from the old character-level one). This task adds expression parsing using a Pratt (top-down operator precedence) approach.

- [ ] **Step 1: Write expression parser tests**

```rust
#[cfg(test)]
mod expr_tests {
    use super::*;

    fn parse_expr(src: &str) -> Expr {
        let tokens = Lexer::new(src).collect_all().unwrap();
        let mut p = TokParser::new(tokens);
        p.parse_expr(0).unwrap()
    }

    #[test]
    fn parse_int_literal()   { assert_eq!(parse_expr("42"), Expr::IntLiteral(42)); }
    #[test]
    fn parse_bool_literal()  { assert_eq!(parse_expr("true"), Expr::BoolLiteral(true)); }
    #[test]
    fn parse_char_literal()  { assert_eq!(parse_expr("'A'"), Expr::CharLiteral(b'A')); }
    #[test]
    fn parse_ident()         { assert!(matches!(parse_expr("status"), Expr::Ident(s) if s == "status")); }
    #[test]
    fn parse_add()           {
        assert!(matches!(parse_expr("a + b"), Expr::BinOp { op: BinOpKind::Add, .. }));
    }
    #[test]
    fn parse_precedence()    {
        // a + b & c  should be (a + b) & c because + has higher precedence than &
        let e = parse_expr("a + b & c");
        assert!(matches!(e, Expr::BinOp { op: BinOpKind::And, .. }));
    }
    #[test]
    fn parse_device_field()  {
        let e = parse_expr("UART0.Status");
        assert!(matches!(e, Expr::DeviceField { device, field } if device == "UART0" && field == "Status"));
    }
    #[test]
    fn parse_fn_call_expr()  {
        let e = parse_expr("get_status()");
        assert!(matches!(e, Expr::Call { callee, .. } if callee == "get_status"));
    }
    #[test]
    fn parse_unary_not()     {
        assert!(matches!(parse_expr("!flag"), Expr::UnOp { op: UnOpKind::Not, .. }));
    }
}
```

- [ ] **Step 2: Run — expect FAIL**
```bash
cargo test -p parser expr_tests 2>&1 | head -5
```

- [ ] **Step 3: Add `TokParser` struct and Pratt expression parser**

Add after the `Lexer` impl:
```rust
/// Token-stream parser. Replaces the old character-level `Parser`.
pub struct TokParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl TokParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(self.tokens.last().unwrap())
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() - 1 { self.pos += 1; }
        t
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token, String> {
        let t = self.peek();
        if std::mem::discriminant(&t.kind) == std::mem::discriminant(kind) {
            Ok(self.advance())
        } else {
            Err(format!(
                "expected {:?} but found {:?} at {}:{}",
                kind, t.kind, t.source.line, t.source.column
            ))
        }
    }

    fn at(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) { self.advance(); true } else { false }
    }

    /// Pratt expression parser. `min_bp` is the minimum binding power.
    pub fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        // --- prefix ---
        let mut lhs = match self.advance().kind.clone() {
            TokenKind::IntLit(n)    => Expr::IntLiteral(n),
            TokenKind::FloatLit(f)  => Expr::FloatLiteral(f),
            TokenKind::CharLit(c)   => Expr::CharLiteral(c),
            TokenKind::StrLit(s)    => Expr::StringLiteral(s),
            TokenKind::True         => Expr::BoolLiteral(true),
            TokenKind::False        => Expr::BoolLiteral(false),
            TokenKind::Bang         => {
                let (_, rbp) = prefix_bp(UnOpKind::Not);
                let e = self.parse_expr(rbp)?;
                Expr::UnOp { op: UnOpKind::Not, operand: Box::new(e) }
            }
            TokenKind::Tilde        => {
                let (_, rbp) = prefix_bp(UnOpKind::BitNot);
                let e = self.parse_expr(rbp)?;
                Expr::UnOp { op: UnOpKind::BitNot, operand: Box::new(e) }
            }
            TokenKind::Minus        => {
                let (_, rbp) = prefix_bp(UnOpKind::Neg);
                let e = self.parse_expr(rbp)?;
                Expr::UnOp { op: UnOpKind::Neg, operand: Box::new(e) }
            }
            TokenKind::LParen       => {
                let e = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                e
            }
            TokenKind::Ident(name)  => {
                if self.eat(&TokenKind::Dot) {
                    // UART0.Status or buf.len
                    let field = match self.advance().kind.clone() {
                        TokenKind::Ident(f) => f,
                        other => return Err(format!("expected field name, got {:?}", other)),
                    };
                    if field == "len" {
                        Expr::SliceLen(name)
                    } else {
                        Expr::DeviceField { device: name, field }
                    }
                } else if self.eat(&TokenKind::LParen) {
                    let mut args = Vec::new();
                    while !self.at(&TokenKind::RParen) {
                        args.push(self.parse_expr(0)?);
                        if !self.eat(&TokenKind::Comma) { break; }
                    }
                    self.expect(&TokenKind::RParen)?;
                    Expr::Call { callee: name, args }
                } else {
                    Expr::Ident(name)
                }
            }
            other => return Err(format!("expected expression, got {:?}", other)),
        };

        // --- infix ---
        loop {
            let op = match token_to_binop(&self.peek().kind) {
                Some(op) => op,
                None => break,
            };
            let (lbp, rbp) = infix_bp(op);
            if lbp < min_bp { break; }
            self.advance();
            let rhs = self.parse_expr(rbp)?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }
}

fn token_to_binop(kind: &TokenKind) -> Option<BinOpKind> {
    Some(match kind {
        TokenKind::Plus    => BinOpKind::Add,  TokenKind::Minus  => BinOpKind::Sub,
        TokenKind::Star    => BinOpKind::Mul,  TokenKind::Slash  => BinOpKind::Div,
        TokenKind::Percent => BinOpKind::Rem,
        TokenKind::Amp     => BinOpKind::And,  TokenKind::Pipe   => BinOpKind::Or,
        TokenKind::Caret   => BinOpKind::Xor,
        TokenKind::Shl     => BinOpKind::Shl,  TokenKind::Shr    => BinOpKind::Shr,
        TokenKind::EqEq    => BinOpKind::Eq,   TokenKind::BangEq => BinOpKind::Ne,
        TokenKind::Lt      => BinOpKind::Lt,   TokenKind::Gt     => BinOpKind::Gt,
        TokenKind::LtEq    => BinOpKind::Le,   TokenKind::GtEq   => BinOpKind::Ge,
        TokenKind::AmpAmp  => BinOpKind::LogAnd,
        TokenKind::PipePipe=> BinOpKind::LogOr,
        _ => return None,
    })
}

/// Returns (left_bp, right_bp) for infix operators.
/// Higher bp = tighter binding. Right-associative: rbp = lbp - 1.
fn infix_bp(op: BinOpKind) -> (u8, u8) {
    match op {
        BinOpKind::LogOr                                       => (10, 11),
        BinOpKind::LogAnd                                      => (20, 21),
        BinOpKind::Or                                          => (30, 31),
        BinOpKind::Xor                                         => (40, 41),
        BinOpKind::And                                         => (50, 51),
        BinOpKind::Eq | BinOpKind::Ne                          => (60, 61),
        BinOpKind::Lt | BinOpKind::Gt | BinOpKind::Le | BinOpKind::Ge => (70, 71),
        BinOpKind::Shl | BinOpKind::Shr                        => (80, 81),
        BinOpKind::Add | BinOpKind::Sub                        => (90, 91),
        BinOpKind::Mul | BinOpKind::Div | BinOpKind::Rem       => (100, 101),
    }
}

fn prefix_bp(op: UnOpKind) -> ((), u8) {
    match op {
        UnOpKind::Not | UnOpKind::BitNot | UnOpKind::Neg => ((), 110),
    }
}
```

- [ ] **Step 4: Run tests — expect PASS**
```bash
cargo test -p parser expr_tests 2>&1 | tail -10
```

- [ ] **Step 5: Commit**
```bash
git add crates/parser/src/lib.rs
git commit -m "feat(parser): Pratt expression parser with full operator precedence"
```

---

## Task 5: New statement and declaration parser (`parse_module` rewrite)

**Files:**
- Modify: `crates/parser/src/lib.rs` — rewrite `parse_module` and all `parse_*` helpers to use `TokParser`

This is the largest single task. The old `Parser` struct (character-level) is replaced by `TokParser`. The output is still `ModuleAst`.

- [ ] **Step 1: Write integration tests using new syntax**

Create `tests/must_pass/new_syntax_basic.kr`:
```kr
@module_caps(MmioRaw)

const uint32 TIMEOUT = 1000

@ctx(boot)
fn entry() {
    uint32 x = 42
    x += 1
    if x > 0 {
        print("Hello, World!\n")
    }
}
```

Create `tests/must_pass/new_syntax_device.kr`:
```kr
@module_caps(Mmio)

device UART0 at 0x3F000000 {
    Data   at 0x00 : uint8  rw
    Status at 0x04 : uint32 ro
}

lock UartLock

@ctx(thread, boot)
fn uart_send(uint8 b) {
    acquire(UartLock)
    while UART0.Status == 0 {}
    UART0.Data = b
    release(UartLock)
}
```

- [ ] **Step 2: Run — expect FAIL (new syntax not parsed yet)**
```bash
cargo test 2>&1 | grep "new_syntax" | head -5
```

- [ ] **Step 3: Rewrite `parse_module` and add all statement parsing to `TokParser`**

Key functions to add to `TokParser`:
- `parse_module(&mut self) -> Result<ModuleAst, Vec<String>>` — top-level dispatch
- `parse_fn(&mut self, attrs: Vec<RawAttr>, is_extern: bool) -> Result<FnAst, String>` — parses `fn name(params) -> ty { body }`
- `parse_param_list(&mut self) -> Result<Vec<(String, ParamTy)>, String>` — `(uint32 x, uint8 b)`
- `parse_block(&mut self) -> Result<Vec<Stmt>, String>` — `{ stmts... }`
- `parse_stmt(&mut self) -> Result<Stmt, String>` — dispatches on first token
- `parse_device(&mut self) -> Result<DeviceDecl, String>` — `device NAME at ADDR { ... }`
- `parse_attrs(&mut self) -> Vec<RawAttr>` — collects `@ctx(...)` etc. before `fn`

Key parsing rules for `parse_stmt`:
```
token is TypeKw(_) or StringKw  → VarDecl
token is `if`                   → If
token is `while`                → While
token is `for`                  → For
token is `return`               → Return
token is `break`                → Break { }
token is `continue`             → Continue
token is `acquire`              → Acquire(ident)
token is `release`              → Release(ident)
token is `critical`             → Critical(block)
token is `unsafe`               → Unsafe(block)
token is `yieldpoint`           → YieldPoint
token is `print`                → Print(string_lit)
token is Ident, peek is `=`     → Assign
token is Ident, peek is compound-op → CompoundAssign
token is Ident, peek is `.`, peek+2 is `=` → Assign(DeviceField)
token is Ident                  → ExprStmt (function call)
```

Update `parse_module` public function to use `TokParser`:
```rust
pub fn parse_module(src: &str) -> Result<ModuleAst, Vec<String>> {
    let tokens = Lexer::new(src)
        .collect_all()
        .map_err(|e| vec![e])?;
    TokParser::new(tokens).parse_module()
}
```

- [ ] **Step 4: Run — expect tests to pass**
```bash
cargo test 2>&1 | tail -10
```

- [ ] **Step 5: Fix any failures**
```bash
cargo test 2>&1 | grep "FAILED"
```

- [ ] **Step 6: Commit**
```bash
git add crates/parser/src/lib.rs tests/must_pass/
git commit -m "feat(parser): rewrite parse_module using TokParser for new KernRift surface syntax"
```

---

## Task 6: HIR — type resolution and device symbol table

**Files:**
- Modify: `crates/hir/src/lib.rs`

The HIR needs to know about the new type names and resolve `DEVICE.REG` references to MMIO address expressions.

- [ ] **Step 1: Write a failing HIR test**

Add to `crates/kernriftc/tests/kr3_contract.rs` (or a new test file `crates/kernriftc/tests/new_syntax.rs`):
```rust
#[test]
fn device_block_lowers_to_mmio_decls() {
    let src = r#"
@module_caps(Mmio)
device UART0 at 0x3F000000 {
    Data   at 0x00 : uint8  rw
    Status at 0x04 : uint32 ro
}
@ctx(thread)
fn dummy() {}
"#;
    // Use compile_source from kernriftc (returns KrirModule directly, no wrapper)
    use kernriftc::compile_source;
    let module = compile_source(src).unwrap();
    assert!(module.mmio_bases.iter().any(|b| b.name == "UART0"));
    assert!(module.mmio_registers.iter().any(|r| r.name == "Status"));
}
```

- [ ] **Step 2: Run — expect FAIL**
```bash
cargo test device_block_lowers_to_mmio_decls 2>&1 | head -10
```

- [ ] **Step 3: Add device expansion pass to HIR lowering**

In `crates/hir/src/lib.rs`, add a pass that runs before the main lowering loop:

```rust
/// Expand `DeviceDecl` blocks into `MmioBaseDecl` + `MmioRegisterDecl` entries.
/// Also builds the device register map used for `DEVICE.REG` expression lowering.
fn expand_device_decls(ast: &ModuleAst) -> (Vec<MmioBaseDecl>, Vec<MmioRegisterDecl>) {
    let mut bases = ast.mmio_bases.clone();
    let mut regs  = ast.mmio_registers.clone();
    for dev in &ast.devices {
        bases.push(MmioBaseDecl {
            name: dev.name.clone(),
            addr: dev.base_addr.clone(),
        });
        for reg in &dev.registers {
            regs.push(MmioRegisterDecl {
                base:   dev.name.clone(),
                name:   reg.name.clone(),
                offset: reg.offset.clone(),
                ty:     reg.ty,
                access: reg.access,
            });
        }
    }
    (bases, regs)
}
```

Also build a `DeviceRegMap` type:
```rust
use std::collections::HashMap;
type DeviceRegMap = HashMap<(String, String), MmioRegisterDecl>;

fn build_device_reg_map(regs: &[MmioRegisterDecl]) -> DeviceRegMap {
    regs.iter()
        .map(|r| ((r.base.clone(), r.name.clone()), r.clone()))
        .collect()
}
```

Call `expand_device_decls` at the start of the HIR module lowering and pass `DeviceRegMap` through the lowering context.

- [ ] **Step 4: Run test — expect PASS**
```bash
cargo test device_block_lowers_to_mmio_decls
```

- [ ] **Step 5: Commit**
```bash
git add crates/hir/src/lib.rs crates/kernriftc/tests/
git commit -m "feat(hir): expand device blocks to MmioBaseDecl/MmioRegisterDecl; build device reg map"
```

---

## Task 7: HIR — expression lowering (arithmetic, comparisons)

**Files:**
- Modify: `crates/hir/src/lib.rs`

Lower `Expr` nodes to KRIR slot operations. A sub-expression allocates a fresh temporary slot and returns its name.

- [ ] **Step 1: Write failing test**

Create `tests/must_pass/expr_arithmetic.kr`:
```kr
@ctx(thread)
fn compute(uint32 a, uint32 b) -> uint32 {
    uint32 result = a + b
    result &= 0xFF
    return result
}
```
```bash
cargo test 2>&1 | grep "expr_arithmetic" | head -5
```

- [ ] **Step 2: Add `lower_expr` to HIR**

```rust
/// Lower an `Expr` to a KRIR slot, emitting statements into `stmts`.
/// Returns the slot name holding the result.
fn lower_expr(
    expr: &Expr,
    stmts: &mut Vec<KrirStmt>,
    ctx: &mut LowerCtx,
) -> Result<String, String> {
    match expr {
        Expr::IntLiteral(n) => {
            let slot = ctx.fresh_slot();
            stmts.push(KrirStmt::StackCell { ty: MmioScalarType::U64, cell: slot.clone() });
            stmts.push(KrirStmt::CellStore { ty: MmioScalarType::U64, cell: slot.clone(),
                value: MmioValueExpr::IntLiteral(n.to_string()) });
            Ok(slot)
        }
        Expr::BoolLiteral(b) => {
            let slot = ctx.fresh_slot();
            let n = if *b { 1u64 } else { 0u64 };
            stmts.push(KrirStmt::StackCell { ty: MmioScalarType::U8, cell: slot.clone() });
            stmts.push(KrirStmt::CellStore { ty: MmioScalarType::U8, cell: slot.clone(),
                value: MmioValueExpr::IntLiteral(n.to_string()) });
            Ok(slot)
        }
        Expr::Ident(name) => Ok(name.clone()), // already a slot
        Expr::DeviceField { device, field } => {
            // look up in DeviceRegMap
            let reg = ctx.device_regs.get(&(device.clone(), field.clone()))
                .ok_or_else(|| format!("unknown register '{}.{}'", device, field))?;
            if reg.access == MmioRegAccess::Wo {
                return Err(format!("register '{}.{}' is write-only", device, field));
            }
            let slot = ctx.fresh_slot();
            stmts.push(KrirStmt::StackCell { ty: reg.ty, cell: slot.clone() });
            stmts.push(KrirStmt::MmioRead {
                ty: reg.ty,
                addr: MmioAddrExpr::IdentPlusOffset { base: device.clone(), offset: reg.offset.clone() },
                capture: Some(slot.clone()),
            });
            Ok(slot)
        }
        Expr::BinOp { op, lhs, rhs } => {
            let l = lower_expr(lhs, stmts, ctx)?;
            let r = lower_expr(rhs, stmts, ctx)?;
            // deferred ops
            if matches!(op, BinOpKind::Mul | BinOpKind::Div | BinOpKind::Rem) {
                return Err("multiplication/division not yet supported (V1 limitation)".into());
            }
            if let Some(arith_op) = binop_to_arith(*op) {
                // emit cell_arith_slot: l op= r, result is l
                stmts.push(KrirStmt::CellArithSlot {
                    ty: MmioScalarType::U64, // type widening — proper type inference deferred
                    dst: l.clone(), src: r.clone(), op: arith_op,
                });
                Ok(l)
            } else {
                // comparison — produce a bool slot
                let slot = ctx.fresh_slot();
                stmts.push(KrirStmt::StackCell { ty: MmioScalarType::U8, cell: slot.clone() });
                stmts.push(KrirStmt::CompareIntoSlot {
                    op: *op, lhs: l, rhs: r, out: slot.clone(),
                });
                Ok(slot)
            }
        }
        _ => Err(format!("expression type not yet supported in HIR lowering")),
    }
}

fn binop_to_arith(op: BinOpKind) -> Option<ArithOp> {
    Some(match op {
        BinOpKind::Add => ArithOp::Add,   BinOpKind::Sub => ArithOp::Sub,
        BinOpKind::And => ArithOp::And,   BinOpKind::Or  => ArithOp::Or,
        BinOpKind::Xor => ArithOp::Xor,   BinOpKind::Shl => ArithOp::Shl,
        BinOpKind::Shr => ArithOp::Shr,
        _ => return None,
    })
}
```

- [ ] **Step 3: Run test — expect PASS**
```bash
cargo test 2>&1 | grep "expr_arithmetic"
```

- [ ] **Step 4: Commit**
```bash
git add crates/hir/src/lib.rs tests/must_pass/
git commit -m "feat(hir): lower Expr nodes to KRIR slot ops; arithmetic and DeviceField reads"
```

---

## Task 8: HIR — VarDecl, Assign, MMIO writes, print intrinsic

**Files:**
- Modify: `crates/hir/src/lib.rs`

- [ ] **Step 1: Write failing test**

Create `tests/must_pass/new_syntax_assign.kr`:
```kr
@module_caps(Mmio)

device UART0 at 0x3F000000 {
    Data   at 0x00 : uint8  rw
    Status at 0x04 : uint32 ro
}

@ctx(thread, boot)
fn send(uint8 b) {
    UART0.Data = b
}
```
```bash
cargo test 2>&1 | grep "new_syntax_assign" | head -5
```

- [ ] **Step 2: Lower `Stmt::VarDecl`, `Stmt::Assign`, `Stmt::CompoundAssign`, `Stmt::Print`**

In the main statement lowering loop add:
```rust
Stmt::VarDecl { ty, name, init } => {
    stmts.push(KrirStmt::StackCell { ty: ty.storage_type(), cell: name.clone() });
    if let Some(init_expr) = init {
        let src = lower_expr(init_expr, stmts, ctx)?;
        stmts.push(KrirStmt::CellArithSlot {
            ty: ty.storage_type(), dst: name.clone(), src, op: ArithOp::Or,
        }); // OR with 0 = copy
    }
}
Stmt::Assign { target, value } => match target {
    AssignTarget::Ident(name) => {
        let src = lower_expr(value, stmts, ctx)?;
        stmts.push(KrirStmt::CellArithSlot {
            ty: MmioScalarType::U64, dst: name.clone(), src, op: ArithOp::Or,
        });
    }
    AssignTarget::DeviceField { device, field } => {
        let reg = ctx.device_regs.get(&(device.clone(), field.clone()))
            .ok_or_else(|| format!("unknown register '{}.{}'", device, field))?
            .clone();
        if reg.access == MmioRegAccess::Ro {
            return Err(format!("register '{}.{}' is read-only", device, field));
        }
        let src = lower_expr(value, stmts, ctx)?;
        stmts.push(KrirStmt::MmioWrite {
            ty: reg.ty,
            addr: MmioAddrExpr::IdentPlusOffset { base: device.clone(), offset: reg.offset.clone() },
            value: MmioValueExpr::Ident(src),
        });
    }
}
Stmt::CompoundAssign { target, op, value } => {
    // Lower to: target = target op value
    let rhs = lower_expr(value, stmts, ctx)?;
    let arith = binop_to_arith(*op)
        .ok_or_else(|| "compound assign: unsupported operator".to_string())?;
    let lhs_name = match target {
        AssignTarget::Ident(n) => n.clone(),
        _ => return Err("compound assignment to device register not supported".into()),
    };
    stmts.push(KrirStmt::CellArithSlot {
        ty: MmioScalarType::U64, dst: lhs_name, src: rhs, op: arith,
    });
}
Stmt::Print(text) => {
    // Lower to sequence of RawMmioWrite<U8> to KERN_UART_BASE (0x10000000)
    const UART_BASE: u64 = 0x10000000;
    for (i, byte) in process_escape_sequences(text)?.iter().enumerate() {
        stmts.push(KrirStmt::RawMmioWrite {
            ty: MmioScalarType::U8,
            addr: MmioAddrExpr::IntLiteral(format!("{:#x}", UART_BASE + i as u64)),
            value: MmioValueExpr::IntLiteral(byte.to_string()),
        });
    }
    // null terminator
    let end = UART_BASE + text.len() as u64;
    stmts.push(KrirStmt::RawMmioWrite {
        ty: MmioScalarType::U8,
        addr: MmioAddrExpr::IntLiteral(format!("{:#x}", end)),
        value: MmioValueExpr::IntLiteral("0".to_string()),
    });
}
```

Add helper for escape processing:
```rust
fn process_escape_sequences(s: &str) -> Result<Vec<u8>, String> {
    // The lexer already processes escapes in StrLit tokens — s is already decoded.
    Ok(s.bytes().collect())
}
```

- [ ] **Step 3: Run tests**
```bash
cargo test 2>&1 | tail -10
```

- [ ] **Step 4: Commit**
```bash
git add crates/hir/src/lib.rs tests/must_pass/
git commit -m "feat(hir): lower VarDecl, Assign, CompoundAssign, DeviceField writes, print intrinsic"
```

---

## Task 9: HIR — if/else lowering

**Files:**
- Modify: `crates/hir/src/lib.rs`

- [ ] **Step 1: Write failing test**

Create `tests/must_pass/new_syntax_if.kr`:
```kr
@ctx(thread)
fn check(uint32 status) {
    if status == 0 {
        uint32 x = 1
    } else {
        uint32 x = 2
    }
}
```
```bash
cargo test 2>&1 | grep "new_syntax_if" | head -5
```

- [ ] **Step 2: Lower `Stmt::If`**

```rust
Stmt::If { cond, then_body, else_body } => {
    // 1. Lower condition into a slot
    let cond_slot = lower_expr(cond, stmts, ctx)?;
    // 2. Synthesize then/else functions
    let then_name = ctx.fresh_fn_name("__if_then");
    let else_name = if else_body.is_empty() {
        ctx.fresh_fn_name("__if_end")
    } else {
        ctx.fresh_fn_name("__if_else")
    };
    let end_name = ctx.fresh_fn_name("__if_end");
    // 3. Emit branch
    stmts.push(KrirStmt::BranchIfZero {
        slot: cond_slot,
        then_callee: else_name.clone(), // zero = false = else
        else_callee: then_name.clone(), // nonzero = true = then
    });
    // 4. Queue synthesized functions for later emission
    ctx.pending_fns.push(PendingFn {
        name: then_name,
        body: then_body.clone(),
        continuation: end_name.clone(),
    });
    if !else_body.is_empty() {
        ctx.pending_fns.push(PendingFn {
            name: else_name,
            body: else_body.clone(),
            continuation: end_name.clone(),
        });
    }
    ctx.pending_fns.push(PendingFn {
        name: end_name,
        body: vec![],
        continuation: String::new(), // terminal
    });
}
```

- [ ] **Step 3: Run test — expect PASS**
```bash
cargo test 2>&1 | grep "new_syntax_if"
```

- [ ] **Step 4: Commit**
```bash
git add crates/hir/src/lib.rs tests/must_pass/
git commit -m "feat(hir): lower if/else to BranchIfZero with synthesized continuation functions"
```

---

## Task 10: KRIR — add loop ops

**Files:**
- Modify: `crates/krir/src/lib.rs`

- [ ] **Step 1: Write a KRIR unit test**

```rust
#[test]
fn loop_ops_in_enum() {
    // Compile-time check: variants exist and are constructible
    let _a = KrirOp::LoopBegin;
    let _b = KrirOp::LoopEnd;
    let _c = KrirOp::LoopBreak;
    let _d = KrirOp::LoopContinue;
    let _e = KrirOp::CompareIntoSlot {
        op: CmpOp::Eq,
        lhs: "a".to_string(),
        rhs: "b".to_string(),
        out: "c".to_string(),
    };
    let _f = KrirOp::BranchIfZeroLoopBreak { slot: "x".to_string() };
}
```

- [ ] **Step 2: Run — expect FAIL**
```bash
cargo test -p krir loop_ops_validate 2>&1 | head -5
```

- [ ] **Step 3: Add loop ops and helper ops to `KrirOp` and `ExecutableOp`**

In `crates/krir/src/lib.rs`, add to the `KrirOp` enum:
```rust
LoopBegin,          // opens a loop scope
LoopEnd,            // jumps back to loop head
LoopBreak,          // exits innermost loop
LoopContinue,       // jumps to loop condition check
/// Evaluate `lhs op rhs` (comparison) and store 0 or 1 into `out` slot.
CompareIntoSlot { op: BinOpKind, lhs: String, rhs: String, out: String },
/// If slot == 0, break out of innermost loop.
BranchIfZeroLoopBreak { slot: String },
/// If slot != 0, break out of innermost loop.
BranchIfNonZeroLoopBreak { slot: String },
```

Add `BinOpKind` import from `parser` crate into `krir`, or re-define a `CmpOp` enum with `Eq, Ne, Lt, Gt, Le, Ge` variants to avoid the cross-crate dependency.

Add the same variants to `ExecutableOp`. Relax the `validate_executable_krir_linear_structure` check — functions containing `LoopBegin` are exempt from the single-block constraint.

- [ ] **Step 4: Run test — expect PASS**
```bash
cargo test -p krir loop_ops_validate
```

- [ ] **Step 5: Commit**
```bash
git add crates/krir/src/lib.rs
git commit -m "feat(krir): add LoopBegin/LoopEnd/LoopBreak/LoopContinue ops; relax single-block constraint"
```

---

## Task 11: Backend — label and intra-function jump support

**Files:**
- Modify: `crates/krir/src/lib.rs` — `X86_64AsmInstruction` enum, ASM emitter, object byte encoder

This is the highest-risk task. Read the existing `lower_executable_krir_to_x86_64_asm` and the `X86_64AsmInstruction` emit loop carefully before making changes.

- [ ] **Step 1: Write a backend test for a loop**

Build a minimal `ExecutableKrirModule` inline with loop ops and check the ASM text output. Use the existing `emit_x86_64_asm_text` function from `crates/krir/src/lib.rs`:

```rust
#[test]
fn loop_emits_labels_and_jumps() {
    // Build a minimal module with one function containing LoopBegin/LoopEnd
    let module = build_minimal_loop_module(); // define inline below
    let asm_text = emit_x86_64_asm_text(&module).unwrap();
    assert!(asm_text.contains("__loop_0_head:"),  "missing loop head label");
    assert!(asm_text.contains("jmp __loop_0_head"), "missing jmp back to head");
    assert!(asm_text.contains("__loop_0_end:"),   "missing loop end label");
}

fn build_minimal_loop_module() -> ExecutableKrirModule {
    // Construct the minimum valid ExecutableKrirModule using the builder/struct
    // that already exists in krir tests. Copy the pattern from existing tests in
    // crates/kernriftc/tests/kr3_contract.rs — compile a .kr source string that
    // contains a while loop using the new syntax, then return the module.
    use kernriftc::compile_source;
    compile_source("@ctx(thread) fn f() { while true { break } }").unwrap()
}
```

- [ ] **Step 2: Run — expect FAIL**
```bash
cargo test -p krir loop_emits_labels_and_jumps 2>&1 | head -5
```

- [ ] **Step 3: Add label/jump instruction variants**

In the `X86_64AsmInstruction` enum add:
```rust
Label(String),           // emits `name:` in ASM text; zero bytes in object
JmpLabel(String),        // emits `jmp name` / REL32 in object
JmpIfZeroLabel(String),  // emits `jz name` / REL32
JmpIfNonZeroLabel(String),// emits `jnz name` / REL32
```

Update the ASM text emitter to emit labels and jump mnemonics. Update the object byte encoder:
- `Label` → record position in a `label_map: HashMap<String, usize>`
- `JmpLabel` → emit `0xE9` + 4-byte placeholder; record a relocation entry
- After emitting all instructions: patch all relocations using `label_map`

- [ ] **Step 4: Lower `LoopBegin`/`LoopEnd`/`LoopBreak`/`LoopContinue` in the backend**

In `lower_executable_krir_to_x86_64_asm`, maintain a loop stack:
```rust
struct LoopFrame { head_label: String, end_label: String }
let mut loop_stack: Vec<LoopFrame> = Vec::new();
let mut loop_counter = 0usize;
```

For each op:
```rust
ExecutableOp::LoopBegin => {
    let head = format!("__loop_{}_head", loop_counter);
    let end  = format!("__loop_{}_end",  loop_counter);
    loop_counter += 1;
    instrs.push(X86_64AsmInstruction::Label(head.clone()));
    loop_stack.push(LoopFrame { head_label: head, end_label: end });
}
ExecutableOp::LoopEnd => {
    let frame = loop_stack.last().expect("LoopEnd without LoopBegin");
    instrs.push(X86_64AsmInstruction::JmpLabel(frame.head_label.clone()));
    let frame = loop_stack.pop().unwrap();
    instrs.push(X86_64AsmInstruction::Label(frame.end_label));
}
ExecutableOp::LoopBreak => {
    let end = loop_stack.last().expect("break outside loop").end_label.clone();
    instrs.push(X86_64AsmInstruction::JmpLabel(end));
}
ExecutableOp::LoopContinue => {
    let head = loop_stack.last().expect("continue outside loop").head_label.clone();
    instrs.push(X86_64AsmInstruction::JmpLabel(head));
}
```

- [ ] **Step 5: Run test — expect PASS**
```bash
cargo test -p krir loop_emits_labels_and_jumps
cargo test 2>&1 | tail -5
```

- [ ] **Step 6: Commit**
```bash
git add crates/krir/src/lib.rs
git commit -m "feat(krir): add label/jump instructions to x86_64 backend; lower loop ops to native jumps"
```

---

## Task 12: HIR — while and for loop lowering

**Files:**
- Modify: `crates/hir/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/must_pass/new_syntax_while.kr`:
```kr
@ctx(thread, boot)
fn wait_ready(uint32 base) {
    while base == 0 {
        base += 1
    }
}
```

Create `tests/must_pass/new_syntax_for.kr`:
```kr
@module_caps(MmioRaw)

@ctx(boot)
fn zero_buffer() {
    for i in 0..16 {
        raw_write<uint8>(0x10000000, 0)
    }
}
```

- [ ] **Step 2: Run — expect FAIL**
```bash
cargo test 2>&1 | grep "new_syntax_while\|new_syntax_for" | head -5
```

- [ ] **Step 3: Lower `Stmt::While` and `Stmt::For`**

```rust
Stmt::While { cond, body } => {
    // LoopBegin, condition check → LoopBreak if false, body, LoopEnd
    stmts.push(KrirStmt::LoopBegin);
    let cond_slot = lower_expr(cond, stmts, ctx)?;
    // if cond is zero (false), break
    stmts.push(KrirStmt::BranchIfZeroLoopBreak { slot: cond_slot });
    for s in body { lower_stmt(s, stmts, ctx)?; }
    stmts.push(KrirStmt::LoopEnd);
}
Stmt::For { var, start, end, inclusive, body } => {
    // var = start; LoopBegin; if var >= end → break; body; var += 1; LoopEnd
    let start_slot = lower_expr(start, stmts, ctx)?;
    stmts.push(KrirStmt::StackCell { ty: MmioScalarType::U32, cell: var.clone() });
    stmts.push(KrirStmt::CellArithSlot { ty: MmioScalarType::U32, dst: var.clone(), src: start_slot, op: ArithOp::Or });
    stmts.push(KrirStmt::LoopBegin);
    let end_slot = lower_expr(end, stmts, ctx)?;
    let cmp_slot = ctx.fresh_slot();
    stmts.push(KrirStmt::StackCell { ty: MmioScalarType::U8, cell: cmp_slot.clone() });
    let cmp_op = if *inclusive { BinOpKind::Gt } else { BinOpKind::Ge };
    stmts.push(KrirStmt::CompareIntoSlot { op: cmp_op, lhs: var.clone(), rhs: end_slot, out: cmp_slot.clone() });
    stmts.push(KrirStmt::BranchIfNonZeroLoopBreak { slot: cmp_slot });
    for s in body { lower_stmt(s, stmts, ctx)?; }
    let one = ctx.fresh_slot();
    stmts.push(KrirStmt::StackCell { ty: MmioScalarType::U32, cell: one.clone() });
    stmts.push(KrirStmt::CellStore { ty: MmioScalarType::U32, cell: one.clone(), value: MmioValueExpr::IntLiteral("1".into()) });
    stmts.push(KrirStmt::CellArithSlot { ty: MmioScalarType::U32, dst: var.clone(), src: one, op: ArithOp::Add });
    stmts.push(KrirStmt::LoopEnd);
}
Stmt::Break    => stmts.push(KrirStmt::LoopBreak),
Stmt::Continue => stmts.push(KrirStmt::LoopContinue),
```

- [ ] **Step 4: Run tests — expect PASS**
```bash
cargo test 2>&1 | grep "new_syntax_while\|new_syntax_for"
```

- [ ] **Step 5: Run full suite**
```bash
cargo test 2>&1 | tail -5
```

- [ ] **Step 6: Commit**
```bash
git add crates/hir/src/lib.rs tests/must_pass/
git commit -m "feat(hir): lower while/for loops and break/continue to KRIR loop ops"
```

---

## Task 13: HIR — Return statement lowering + emit error for deferred ops

**Files:**
- Modify: `crates/hir/src/lib.rs`

Two small items: lowering `return expr` and producing clear errors when `*`/`/`/`%` are used (deferred per spec §17).

- [ ] **Step 1: Write failing tests**

Create `tests/must_pass/new_syntax_return.kr`:
```kr
@ctx(thread)
fn get_val() -> uint32 {
    uint32 x = 42
    return x
}
```

Create `tests/must_fail/mul_not_supported.kr`:
```kr
@ctx(thread)
fn bad(uint32 x) -> uint32 {
    uint32 result = x * 4
    return result
}
```
Expected error: `multiplication/division not yet supported`

- [ ] **Step 2: Run — expect FAIL**
```bash
cargo test 2>&1 | grep "new_syntax_return\|mul_not_supported" | head -5
```

- [ ] **Step 3: Add Return lowering to HIR**

In the main statement lowering loop:
```rust
Stmt::Return(Some(expr)) => {
    let slot = lower_expr(expr, stmts, ctx)?;
    stmts.push(Stmt::ReturnSlot { slot });
}
Stmt::Return(None) => {
    // void return — emit nothing; function falls off end
}
```

Confirm that `BinOpKind::Mul | Div | Rem` arm in `lower_expr` already returns `Err(...)` per Task 7. If not, add it:
```rust
BinOpKind::Mul | BinOpKind::Div | BinOpKind::Rem => {
    return Err("multiplication/division not yet supported (deferred to V2)".into());
}
```

- [ ] **Step 4: Run — expect PASS for return, FAIL (with correct error) for mul**
```bash
cargo test 2>&1 | grep "new_syntax_return\|mul_not_supported"
```

- [ ] **Step 5: Commit**
```bash
git add crates/hir/src/lib.rs tests/must_pass/ tests/must_fail/
git commit -m "feat(hir): lower return statement; emit error for deferred mul/div/rem ops"
```

---

## Task 14: KRIR + Backend — float32/float64 (SSE2)

**Files:**
- Modify: `crates/krir/src/lib.rs`

- [ ] **Step 1: Write failing test**

Create `tests/must_pass/new_syntax_float.kr`:
```kr
@ctx(thread)
fn scale_f(float32 x) -> float32 {
    float32 result = x + 1.0
    return result
}
```

- [ ] **Step 2: Add `F32`/`F64` to KRIR scalar type and add float arith ops**

Add `FArithOp` enum:
```rust
pub enum FArithOp { FAdd, FSub, FMul, FDiv }
```

Add to `KrirOp`/`ExecutableOp`:
```rust
FloatArith { ty: MmioScalarType, op: FArithOp, dst: SlotId, src: SlotId },
```

In the x86_64 backend, float values are held in XMM registers. Add encoding for:
- `addss xmm0, xmm1` — `0xF3 0x0F 0x58 0xC1` (float32)
- `addsd xmm0, xmm1` — `0xF2 0x0F 0x58 0xC1` (float64)
- Similarly for `subss`/`subsd`, `mulss`/`mulsd`, `divss`/`divsd`
- `movss` / `movsd` for float stack cell load/store
- Float literals stored as 4/8-byte constants in `.data` section, loaded via `movss xmm0, [rip+rel]`

Float stack slots are 16-byte aligned (SSE requirement).

- [ ] **Step 3: Run test — expect PASS**
```bash
cargo test 2>&1 | grep "new_syntax_float"
```

- [ ] **Step 4: Commit**
```bash
git add crates/krir/src/lib.rs tests/must_pass/
git commit -m "feat(krir): add float32/float64 type and FArith ops with SSE2 x86_64 encoding"
```

---

## Task 15: Update all existing test files to new syntax

**Files:**
- Modify: `tests/must_pass/*.kr`, `tests/must_fail/*.kr`, `crates/kernriftc/tests/*.rs`

- [ ] **Step 1: Run full test suite and identify failures**
```bash
cargo test 2>&1 | grep "FAILED" > /tmp/failing_tests.txt
cat /tmp/failing_tests.txt
```

- [ ] **Step 2: Update each failing `.kr` file to new syntax**

Key mechanical translations:
| Old syntax | New syntax |
|-----------|-----------|
| `stack_cell<u32>(x);` | `uint32 x = 0` |
| `cell_store<u32>(x, val);` | `x = val` |
| `cell_load<u32>(x, slot);` | `uint32 slot = x` |
| `cell_add<u32>(dst, src);` | `dst += src` |
| `branch_if_zero(s, t, f);` | `if s == 0 { ... }` |
| `call_with_args(fn, a, b);` | `fn(a, b)` |
| `return_slot(x);` | `return x` |
| `spinlock NAME;` | `lock NAME` |
| `mmio NAME = ADDR;` + `mmio_reg NAME.REG = OFF : T ACCESS;` | `device NAME at ADDR { REG at OFF : T ACCESS }` |
| `u32` | `uint32` |
| `u8` | `uint8` |

- [ ] **Step 3: Run full suite — expect all PASS**
```bash
cargo test 2>&1 | tail -5
```

- [ ] **Step 4: Commit**
```bash
git add tests/ crates/kernriftc/tests/
git commit -m "test: update all .kr test files to new surface syntax"
```

---

## Task 16: Update `docs/LANGUAGE.md`

**Files:**
- Modify: `docs/LANGUAGE.md` — full rewrite

- [ ] **Step 1: Rewrite `docs/LANGUAGE.md`**

Cover the following sections, using content from the spec:
1. Quick Start (hello world in 5 lines)
2. Types (full table: int8–int64, uint8–uint64, float32/float64/float16, bool, char, byte, addr, string, slices, fixed arrays)
3. Variables and assignment
4. Operators and precedence table
5. Functions (parameters, return types)
6. Control flow (if/else, while, for, break, continue, return)
7. Structs and enums
8. Constants
9. Hardware access (device blocks, MMIO read/write, raw_write/raw_read)
10. Kernel safety annotations (@ctx, @eff, @caps, @noyield, @hook)
11. Locks, per-cpu variables
12. Extern functions
13. Critical sections and unsafe blocks
14. Module capabilities (@module_caps)
15. CLI reference (--emit, --target, --surface)
16. Full examples: hello world, UART driver, IRQ handler

- [ ] **Step 2: Verify examples in the docs compile**
```bash
# Try each code block from the doc as a .kr file
kernriftc check docs_example.kr
```

- [ ] **Step 3: Commit**
```bash
git add docs/LANGUAGE.md
git commit -m "docs: rewrite LANGUAGE.md for new KernRift surface syntax"
```

---

## Completion Checklist

- [ ] All `cargo test` pass
- [ ] `kernriftc --emit=krboexe -o hello hello.kr && ./hello` prints "Hello, World!\n" using new syntax
- [ ] Device block syntax compiles to valid ELF object
- [ ] While loop with variable modification compiles and runs correctly
- [ ] Float arithmetic compiles to SSE2 instructions
- [ ] `docs/LANGUAGE.md` reflects new syntax
- [ ] No old syntax (`stack_cell`, `branch_if_zero`, `cell_add`, `u32`, `spinlock`) in any `.kr` file
