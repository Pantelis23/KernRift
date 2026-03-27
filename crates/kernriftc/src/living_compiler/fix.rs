//! Auto-fix support for `--fix` mode.
//!
//! Handles the `try_tail_call` fix: insert `tail ` before the bare call in the
//! last statement of each function body.
//!
//! # Approach
//!
//! The parser AST (`Stmt`, `Expr::Call`) does not carry per-statement byte
//! offsets — only `FnAst` carries a `SourceNote` for the function header.
//! Walking the AST therefore cannot produce the precise byte offset needed to
//! insert text.  We fall back to a **token-aware text-scan approach**:
//!
//! 1. Sanitize the source: replace string-literal contents and comments with
//!    ASCII spaces of equal byte length.  This prevents injection of `tail`
//!    into comments or strings that happen to look like calls.  Because
//!    replacement is in-place (same byte count), all byte offsets remain valid.
//! 2. Iterate over each line of the sanitized source, tracking brace depth to
//!    locate function bodies.
//! 3. Inside each function body, call `last_call_col` on each line.  That
//!    helper splits on `;` at paren-depth 0 so it correctly handles multiple
//!    call statements on one line (e.g. `a(); b()`) and returns the column of
//!    the *last* bare call start.
//! 4. After the closing `}` of a body, emit a `FixSite` for the last recorded
//!    call column — which points into the *original* source because sanitization
//!    preserves byte lengths.

/// A single fixable site.
#[derive(Debug)]
pub(crate) struct FixSite {
    /// Byte offset in the source where `tail ` should be inserted.
    pub(crate) insert_before: usize,
}

/// Apply all fix sites to `source` and return the patched string.
///
/// Sites are applied in **reverse byte-offset order** so that inserting text
/// at an earlier site does not shift the offsets of later sites.
pub(crate) fn apply_fixes(source: &str, sites: &[FixSite]) -> String {
    let mut result = source.to_string();
    let mut sorted: Vec<_> = sites.iter().collect();
    sorted.sort_by(|a, b| b.insert_before.cmp(&a.insert_before));
    for site in sorted {
        result.insert_str(site.insert_before, "tail ");
    }
    result
}

/// Produce a line-by-line unified diff between `before` and `after` for display.
///
/// Each changed line is emitted as a `@@ -N +N @@` hunk followed by the
/// removed (`-`) and added (`+`) lines.  Unchanged lines are omitted.
pub(crate) fn unified_diff(path: &str, before: &str, after: &str) -> String {
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();
    let mut out = String::new();
    out.push_str(&format!("--- {}\n", path));
    out.push_str(&format!("+++ {} (fixed)\n", path));
    let max = before_lines.len().max(after_lines.len());
    for i in 0..max {
        let b = before_lines.get(i).copied().unwrap_or("");
        let a = after_lines.get(i).copied().unwrap_or("");
        if b != a {
            out.push_str(&format!("@@ -{n} +{n} @@\n", n = i + 1));
            out.push_str(&format!("-{}\n", b));
            out.push_str(&format!("+{}\n", a));
        }
    }
    out
}

/// Replace string-literal contents and comments with ASCII spaces, preserving
/// all byte lengths and newlines so that every byte offset in the returned
/// string maps 1-to-1 to the same offset in `src`.
///
/// Rules:
/// - `// ...` — replaced from `//` through the end of the line (newline kept).
/// - `/* ... */` — replaced with spaces; embedded newlines are preserved so
///   that line-number accounting remains stable.
/// - `"..."` — content replaced with spaces; escape sequences are blanked too;
///   embedded newlines are preserved.
fn sanitize_source(src: &str) -> String {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut out: Vec<u8> = bytes.to_vec();
    let mut i = 0;
    while i < len {
        // Line comment: // … \n
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            out[i] = b' ';
            out[i + 1] = b' ';
            i += 2;
            while i < len && bytes[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
            // leave the '\n' as-is
            // Block comment: /* … */
        } else if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            out[i] = b' ';
            out[i + 1] = b' ';
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                // Preserve newlines so line accounting stays correct.
                if bytes[i] != b'\n' {
                    out[i] = b' ';
                }
                i += 1;
            }
            if i + 1 < len {
                out[i] = b' ';
                out[i + 1] = b' ';
                i += 2;
            }
        // String literal: "…"
        } else if bytes[i] == b'"' {
            out[i] = b' ';
            i += 1;
            while i < len && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < len {
                    if bytes[i] != b'\n' {
                        out[i] = b' ';
                    }
                    i += 1;
                    if i < len && bytes[i] != b'\n' {
                        out[i] = b' ';
                    }
                } else if bytes[i] != b'\n' {
                    out[i] = b' ';
                }
                i += 1;
            }
            if i < len {
                out[i] = b' ';
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    // SAFETY: we only wrote ASCII bytes (spaces) over ASCII bytes; any UTF-8
    // multi-byte sequences are left intact or were inside a string/comment and
    // their non-\n bytes are replaced with single-byte spaces — which may
    // produce invalid UTF-8 in theory, but sanitize_source is only ever called
    // on KernRift source which is ASCII.  Use lossy conversion as a safe fallback.
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Return the byte column (offset within `line`) where the **last** bare call
/// statement starts, or `None` if the line does not contain one.
///
/// A "bare call statement" is an identifier not preceded by `=` at the same
/// statement level, followed immediately by `(`.  Keywords (`fn`, `let`, `if`,
/// `while`, `return`, `tail`) are excluded.
///
/// To handle multiple statements on one line (e.g. `a(); b()`), the function
/// splits on `;` at paren-depth 0 and checks segments from right to left,
/// including segments that end with a trailing `;` (e.g. `bar();` yields the
/// segment `"    bar()"` before the `;`, not the empty part after it).
fn last_call_col(line: &str) -> Option<usize> {
    const KEYWORDS: &[&str] = &["fn", "let", "if", "while", "return", "tail", "for", "loop"];

    let bytes = line.as_bytes();
    let len = bytes.len();

    // Collect (start, end) byte ranges for each `;`-separated segment at depth 0.
    let mut depth: usize = 0;
    let mut seg_start: usize = 0;
    let mut segments: Vec<(usize, usize)> = Vec::new();

    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b';' if depth == 0 => {
                segments.push((seg_start, i));
                seg_start = i + 1;
            }
            _ => {}
        }
    }
    segments.push((seg_start, len)); // trailing segment (empty for `foo();`)

    // Walk from rightmost to leftmost; return the first segment that is a call.
    for &(start, end) in segments.iter().rev() {
        if start >= end {
            continue;
        }
        let seg = &line[start..end];
        let seg_trimmed = seg.trim_start();
        if seg_trimmed.is_empty() {
            continue;
        }

        let first = match seg_trimmed.chars().next() {
            Some(c) => c,
            None => continue,
        };
        if !first.is_ascii_alphabetic() && first != '_' {
            continue;
        }

        let ident_end = seg_trimmed
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(seg_trimmed.len());
        let ident = &seg_trimmed[..ident_end];

        if KEYWORDS.contains(&ident) {
            continue;
        }

        // After the identifier, optional whitespace, then must be `(`.
        let mut found = false;
        for ch in seg_trimmed[ident_end..].chars() {
            match ch {
                '(' => {
                    found = true;
                    break;
                }
                ' ' | '\t' => {}
                _ => break,
            }
        }
        if found {
            let leading = seg.len() - seg_trimmed.len();
            return Some(start + leading);
        }
    }
    None
}

/// Scan `source` for tail-call fix sites.
///
/// For each function body found via brace matching, the byte offset of the
/// last bare-call statement is recorded as a `FixSite`.
///
/// Returns an empty `Vec` if no eligible sites are found — the caller already
/// gates on `suggestions.iter().any(|m| m.id == "try_tail_call")`.
pub(crate) fn find_fix_sites(source: &str) -> Vec<FixSite> {
    // Sanitize first: blank out string contents and comments so we never
    // accidentally detect a call-like pattern inside them.  Byte offsets in
    // the sanitized string are identical to those in `source`.
    let sanitized = sanitize_source(source);
    find_fix_sites_on(&sanitized)
}

fn find_fix_sites_on(source: &str) -> Vec<FixSite> {
    let mut sites: Vec<FixSite> = Vec::new();

    let mut depth: usize = 0;
    let mut in_fn_body: bool = false;
    let mut fn_body_depth: usize = 0;
    let mut last_call_offset: Option<usize> = None;
    let mut prev_line_is_fn = false;
    let mut byte_offset: usize = 0;

    for line in source.lines() {
        let trimmed = line.trim();

        let opens = line.chars().filter(|&c| c == '{').count();
        let closes = line.chars().filter(|&c| c == '}').count();

        // Inside a function body: look for the last bare call on this line.
        if in_fn_body && depth > fn_body_depth {
            if let Some(col) = last_call_col(line) {
                last_call_offset = Some(byte_offset + col);
            }
        }

        depth = depth.saturating_add(opens).saturating_sub(closes);

        // `fn` on previous line, `{` on this line.
        if opens > 0 && prev_line_is_fn && !in_fn_body {
            in_fn_body = true;
            fn_body_depth = depth - 1;
        }

        // `fn foo() {` all on one line.
        let line_has_fn = trimmed.starts_with("fn ") || trimmed.contains(" fn ");
        if line_has_fn && opens > 0 && !in_fn_body {
            in_fn_body = true;
            fn_body_depth = depth - 1;
        }

        // Function body closed.
        if in_fn_body && depth <= fn_body_depth {
            if let Some(offset) = last_call_offset.take() {
                sites.push(FixSite {
                    insert_before: offset,
                });
            }
            in_fn_body = false;
            fn_body_depth = 0;
        }

        // Advance byte offset (`lines()` strips the newline).
        let raw = &source.as_bytes()[byte_offset..];
        let line_len = line.len();
        let newline_len = if raw.get(line_len) == Some(&b'\n') {
            1
        } else if raw.get(line_len) == Some(&b'\r') {
            2
        } else {
            0
        };
        byte_offset += line_len + newline_len;

        prev_line_is_fn = line_has_fn;
    }

    // Malformed source: body never closed — emit whatever we have.
    if in_fn_body {
        if let Some(offset) = last_call_offset.take() {
            sites.push(FixSite {
                insert_before: offset,
            });
        }
    }

    sites
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_fixes_inserts_tail() {
        let source = "fn foo() { bar() }";
        let sites = vec![FixSite { insert_before: 11 }];
        let result = apply_fixes(source, &sites);
        assert_eq!(result, "fn foo() { tail bar() }");
    }

    #[test]
    fn apply_fixes_reverse_order() {
        let source = "fn f() { a() } fn g() { b() }";
        let sites = vec![FixSite { insert_before: 9 }, FixSite { insert_before: 24 }];
        let result = apply_fixes(source, &sites);
        assert!(result.contains("tail a()"));
        assert!(result.contains("tail b()"));
    }

    #[test]
    fn find_fix_sites_single_fn() {
        let source = "fn foo() {\n    bar();\n}\n";
        let sites = find_fix_sites(source);
        assert_eq!(sites.len(), 1, "should find one fix site");
        let fixed = apply_fixes(source, &sites);
        assert!(fixed.contains("tail bar();"), "got: {:?}", fixed);
    }

    #[test]
    fn find_fix_sites_last_call_only() {
        let source = "fn foo() {\n    a();\n    b();\n    c();\n}\n";
        let sites = find_fix_sites(source);
        assert_eq!(sites.len(), 1);
        let fixed = apply_fixes(source, &sites);
        assert!(fixed.contains("tail c();"), "got: {:?}", fixed);
        assert!(!fixed.contains("tail a()"), "a() should not be fixed");
        assert!(!fixed.contains("tail b()"), "b() should not be fixed");
    }

    #[test]
    fn find_fix_sites_multi_fn() {
        let source = "fn foo() {\n    a();\n}\nfn bar() {\n    b();\n}\n";
        let sites = find_fix_sites(source);
        assert_eq!(sites.len(), 2, "one site per function");
        let fixed = apply_fixes(source, &sites);
        assert!(fixed.contains("tail a();"));
        assert!(fixed.contains("tail b();"));
    }

    #[test]
    fn find_fix_sites_empty_body() {
        let source = "fn foo() {\n}\n";
        let sites = find_fix_sites(source);
        assert_eq!(sites.len(), 0, "empty body has no fix site");
    }

    #[test]
    fn unified_diff_shows_changed_lines() {
        let before = "fn foo() {\n    bar();\n}\n";
        let after = "fn foo() {\n    tail bar();\n}\n";
        let diff = unified_diff("test.kr", before, after);
        assert!(diff.contains("--- test.kr"));
        assert!(diff.contains("+++ test.kr (fixed)"));
        assert!(diff.contains("-    bar();"));
        assert!(diff.contains("+    tail bar();"));
    }

    #[test]
    fn unified_diff_empty_for_identical() {
        let src = "fn foo() { }\n";
        let diff = unified_diff("x.kr", src, src);
        assert!(!diff.contains("@@"), "no hunks for identical input");
    }

    // ── new: sanitize_source ────────────────────────────────────────────────

    #[test]
    fn sanitize_blanks_line_comment() {
        let src = "foo(); // bar()\n";
        let san = sanitize_source(src);
        // comment content replaced with spaces, newline preserved
        // "foo(); " (7 bytes) + "// bar()" (8 bytes blanked) = "foo();         \n"
        assert_eq!(san, "foo();         \n");
    }

    #[test]
    fn sanitize_blanks_block_comment() {
        let src = "/* call() */ foo()";
        let san = sanitize_source(src);
        assert_eq!(san, "             foo()");
    }

    #[test]
    fn sanitize_blanks_string() {
        let src = "\"init()\" foo()";
        let san = sanitize_source(src);
        assert_eq!(san, "         foo()");
    }

    #[test]
    fn sanitize_preserves_newlines_in_block_comment() {
        let src = "/*\ninit()\n*/ foo()";
        let san = sanitize_source(src);
        // The two non-newline chars of "/*" and "*/" are blanked; inner newlines kept.
        assert!(san.contains('\n'), "newlines must be preserved");
        assert!(
            !san.contains("init()"),
            "call inside comment must be erased"
        );
        assert!(san.ends_with(" foo()"));
    }

    #[test]
    fn find_fix_sites_skips_call_in_string() {
        // A multiline string that contains what looks like a call should NOT
        // be treated as a fix site.
        let source = "fn foo() {\n    let x = \"bar()\";\n    baz();\n}\n";
        let sites = find_fix_sites(source);
        assert_eq!(sites.len(), 1);
        let fixed = apply_fixes(source, &sites);
        assert!(fixed.contains("tail baz();"), "got: {:?}", fixed);
        assert!(!fixed.contains("tail let"), "assignment must not be fixed");
    }

    #[test]
    fn find_fix_sites_skips_call_in_comment() {
        let source = "fn foo() {\n    // old_fn();\n    bar();\n}\n";
        let sites = find_fix_sites(source);
        assert_eq!(sites.len(), 1);
        let fixed = apply_fixes(source, &sites);
        assert!(fixed.contains("tail bar();"), "got: {:?}", fixed);
        assert!(!fixed.contains("tail //"), "comment must not be fixed");
    }

    #[test]
    fn find_fix_sites_two_calls_on_one_line() {
        // `a(); b()` — only b() is the tail call.
        let source = "fn foo() {\n    a(); b();\n}\n";
        let sites = find_fix_sites(source);
        assert_eq!(sites.len(), 1);
        let fixed = apply_fixes(source, &sites);
        assert!(fixed.contains("tail b();"), "got: {:?}", fixed);
        assert!(!fixed.contains("tail a()"), "a() must not be fixed");
    }

    // ── new: last_call_col ──────────────────────────────────────────────────

    #[test]
    fn last_call_col_single() {
        assert_eq!(last_call_col("    foo()"), Some(4));
    }

    #[test]
    fn last_call_col_two_on_line() {
        // Should return the column of `b`, not `a`.
        let line = "    a(); b()";
        let col = last_call_col(line).unwrap();
        assert_eq!(&line[col..col + 1], "b");
    }

    #[test]
    fn last_call_col_assignment_rejected() {
        assert_eq!(last_call_col("    let x = foo()"), None);
    }

    #[test]
    fn last_call_col_keyword_rejected() {
        assert_eq!(last_call_col("    fn foo()"), None);
        assert_eq!(last_call_col("    return bar()"), None);
    }
}
