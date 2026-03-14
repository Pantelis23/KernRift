#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawAttr {
    pub name: String,
    pub args: Option<String>,
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
pub enum Stmt {
    Call(String),
    Critical(Vec<Stmt>),
    YieldPoint,
    AllocPoint,
    BlockPoint,
    Acquire(String),
    Release(String),
    MmioRead {
        ty: MmioScalarType,
        addr: String,
    },
    MmioWrite {
        ty: MmioScalarType,
        addr: String,
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnAst {
    pub name: String,
    pub is_extern: bool,
    pub attrs: Vec<RawAttr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModuleAst {
    pub module_caps: Vec<String>,
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

        while self.skip_ws_comments() {
            if self.eof() {
                break;
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
                    self.recover_to_next_item();
                    continue;
                }
                let caps = attrs[0].args.as_deref().map(split_csv).unwrap_or_default();
                module.module_caps = caps;
                continue;
            }

            if !self.consume_keyword("fn") {
                self.error_here("expected 'fn' or @module_caps(...) at item boundary");
                self.recover_to_next_item();
                continue;
            }

            let Some(name) = self.parse_ident() else {
                self.error_here("expected function name after 'fn'");
                self.recover_to_next_item();
                continue;
            };

            self.skip_ws_comments();
            if !self.consume_char('(') || !self.consume_char(')') {
                self.error_here("expected empty parameter list '()' in KR0");
                self.recover_to_next_item();
                continue;
            }

            if is_extern {
                self.skip_ws_comments();
                if !self.consume_char(';') {
                    self.error_here("expected ';' after extern declaration");
                    self.recover_to_next_item();
                    continue;
                }
                module.items.push(FnAst {
                    name,
                    is_extern: true,
                    attrs,
                    body: Vec::new(),
                });
                continue;
            }

            self.skip_ws_comments();
            if !self.consume_char('{') {
                self.error_here("expected '{' to start function body");
                self.recover_to_next_item();
                continue;
            }

            let body = self.parse_body();
            module.items.push(FnAst {
                name,
                is_extern: false,
                attrs,
                body,
            });
        }

        if self.errors.is_empty() {
            Ok(module)
        } else {
            Err(self.errors)
        }
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

            match self.read_statement_text() {
                Some(text) => {
                    if text.trim().is_empty() {
                        continue;
                    }
                    match parse_stmt(text.trim()) {
                        Ok(Some(stmt)) => body.push(stmt),
                        Ok(None) => {}
                        Err(msg) => self.errors.push(format!("{} at byte {}", msg, self.pos)),
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

        Some(RawAttr { name, args })
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

    fn error_here(&mut self, msg: &str) {
        self.errors.push(format!("{} at byte {}", msg, self.pos));
    }

    fn eof(&self) -> bool {
        self.pos >= self.src.len()
    }
}

fn parse_stmt(stmt: &str) -> Result<Option<Stmt>, String> {
    if stmt.is_empty() {
        return Ok(None);
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

    if let Some(ty) = parse_mmio_scalar_from_name(name, "mmio_read")? {
        let parts = split_csv(args);
        if parts.len() != 1 {
            return Err("mmio_read<T>(addr) requires exactly one address argument".to_string());
        }
        return Ok(Some(Stmt::MmioRead {
            ty,
            addr: parts[0].trim().to_string(),
        }));
    }

    if let Some(ty) = parse_mmio_scalar_from_name(name, "mmio_write")? {
        let parts = split_csv(args);
        if parts.len() != 2 {
            return Err(
                "mmio_write<T>(addr, value) requires exactly two arguments: address and value"
                    .to_string(),
            );
        }
        return Ok(Some(Stmt::MmioWrite {
            ty,
            addr: parts[0].trim().to_string(),
            value: parts[1].trim().to_string(),
        }));
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
    use super::{MmioScalarType, Stmt, parse_module};
    use proptest::prelude::*;

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
                    addr: "uart0".to_string()
                },
                Stmt::MmioWrite {
                    ty: MmioScalarType::U64,
                    addr: "uart0".to_string(),
                    value: "value0".to_string()
                }
            ]
        );
    }

    #[test]
    fn parse_rejects_legacy_zero_arg_mmio_forms() {
        let src = "fn entry() { mmio_read(); mmio_write(); }";
        let err = parse_module(src).expect_err("legacy zero-arg mmio should fail");
        assert_eq!(
            err,
            vec![
                "mmio_read() legacy form is unsupported; use mmio_read<T>(addr) at byte 25"
                    .to_string(),
                "mmio_write() legacy form is unsupported; use mmio_write<T>(addr, value) at byte 39"
                    .to_string()
            ]
        );
    }

    #[test]
    fn parse_rejects_unsupported_typed_mmio_element() {
        let src = "fn entry() { mmio_read<u128>(); }";
        let err = parse_module(src).expect_err("invalid mmio element type should fail");
        assert_eq!(
            err,
            vec![
                "unsupported mmio element type 'u128'; expected one of: u8, u16, u32, u64 at byte 31"
                    .to_string()
            ]
        );
    }

    #[test]
    fn parse_rejects_malformed_typed_mmio_invocation() {
        let src = "fn entry() { mmio_write<u32( ); }";
        let err = parse_module(src).expect_err("malformed mmio invocation should fail");
        assert_eq!(
            err,
            vec![
                "malformed mmio_write typed invocation 'mmio_write<u32'; expected mmio_write<T>() at byte 31"
                    .to_string()
            ]
        );
    }

    #[test]
    fn parse_rejects_typed_mmio_read_missing_address_argument() {
        let src = "fn entry() { mmio_read<u32>(); }";
        let err = parse_module(src).expect_err("missing mmio read arg should fail");
        assert_eq!(
            err,
            vec!["mmio_read<T>(addr) requires exactly one address argument at byte 30".to_string()]
        );
    }

    #[test]
    fn parse_rejects_typed_mmio_write_wrong_arity() {
        let src_missing = "fn entry() { mmio_write<u32>(addr); }";
        let err_missing = parse_module(src_missing).expect_err("missing value arg should fail");
        assert_eq!(
            err_missing,
            vec![
                "mmio_write<T>(addr, value) requires exactly two arguments: address and value at byte 35"
                    .to_string()
            ]
        );

        let src_extra = "fn entry() { mmio_write<u32>(addr, value, extra); }";
        let err_extra = parse_module(src_extra).expect_err("extra value arg should fail");
        assert_eq!(
            err_extra,
            vec![
                "mmio_write<T>(addr, value) requires exactly two arguments: address and value at byte 49"
                    .to_string()
            ]
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
