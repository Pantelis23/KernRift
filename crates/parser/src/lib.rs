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

    fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "u8" => Ok(Self::U8),
            "u16" => Ok(Self::U16),
            "u32" => Ok(Self::U32),
            "u64" => Ok(Self::U64),
            other => Err(format!(
                "unsupported mmio element type '{}'; expected one of: u8, u16, u32, u64",
                other
            )),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Call(String),
    CallCapture {
        callee: String,
        slot: String,
    },
    Critical(Vec<Stmt>),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnAst {
    pub name: String,
    pub is_extern: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModuleAst {
    pub module_caps: Vec<String>,
    pub mmio_bases: Vec<MmioBaseDecl>,
    pub mmio_registers: Vec<MmioRegisterDecl>,
    pub items: Vec<FnAst>,
}

pub fn parse_module(src: &str) -> Result<ModuleAst, Vec<String>> {
    Parser::new(src).parse_module()
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
                    "expected 'fn', 'mmio', 'mmio_reg', or @module_caps(...) at item boundary",
                );
                self.recover_to_next_module_item();
                continue;
            }

            let Some(name) = self.parse_ident() else {
                self.error_here("expected function name after 'fn'");
                self.recover_to_next_module_item();
                continue;
            };

            self.skip_ws_comments();
            if !self.consume_char('(') || !self.consume_char(')') {
                self.error_here("expected empty parameter list '()' in KR0");
                self.recover_to_next_module_item();
                continue;
            }

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

            let stmt_start = self.pos;
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
                    self.error_here("expected ';' terminating statement");
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
                return None;
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
            if rest.starts_with("fn")
                || rest.starts_with("extern")
                || rest.starts_with("mmio_reg")
                || rest.starts_with("mmio")
                || rest.starts_with('@')
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

    Ok(None)
}

fn parse_mmio_addr_operand(raw: &str) -> Result<MmioAddrExpr, String> {
    let operand = raw.trim();
    if is_ident_token(operand) {
        return Ok(MmioAddrExpr::Ident(operand.to_string()));
    }
    if is_int_literal_token(operand) {
        return Ok(MmioAddrExpr::IntLiteral(operand.to_string()));
    }

    if operand.matches('+').count() == 1 {
        let (base, offset) = operand.split_once('+').expect("single plus");
        let base = base.trim();
        let offset = offset.trim();
        if is_ident_token(base) && is_int_literal_token(offset) {
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
    if is_ident_token(operand) {
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
        SourceNote, Stmt, format_source_diagnostic, int_literal_numeric_value, parse_module,
        split_csv_allow_trailing_comma,
    };
    use proptest::prelude::*;

    fn diagnostic_at(src: &str, byte_offset: usize, message: &str) -> String {
        format_source_diagnostic(&SourceNote::from_source(src, byte_offset), message, None)
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
                    value: MmioValueExpr::Ident {
                        name: "value".to_string(),
                    },
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
        assert_eq!(
            err,
            vec![diagnostic_at(
                src,
                13,
                "unsupported mmio element type 'u128'; expected one of: u8, u16, u32, u64",
            )]
        );
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
