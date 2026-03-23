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
//! insert text.  We fall back to a **text-scan approach**:
//!
//! 1. Iterate over each line of the source.
//! 2. Track brace depth to detect function bodies.
//! 3. Inside each function body, record the byte offset of the start of every
//!    line that looks like a bare call (identifier immediately followed by `(`).
//! 4. After the closing `}` of the body, emit a `FixSite` for the *last* such
//!    call line found.
//!
//! This is intentionally conservative: it only fixes the last call in a
//! function, which is the canonical tail-call position, and it skips lines that
//! look like declarations, assignments, or keywords.

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

/// Returns true if `trimmed` looks like a bare call statement:
/// starts with an identifier character, then eventually hits `(` before any
/// `=`, `{`, `}`, or whitespace-separated keyword.
fn looks_like_call(trimmed: &str) -> bool {
    // Must start with a letter or underscore (identifier start).
    let first = match trimmed.chars().next() {
        Some(c) => c,
        None => return false,
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    // Walk until we hit `(` or a disqualifying character.
    for ch in trimmed.chars() {
        match ch {
            '(' => return true,
            // Assignment, block open/close, or end of statement without a call.
            '=' | '{' | '}' => return false,
            _ => {}
        }
    }
    false
}

/// Scan `source` for tail-call fix sites.
///
/// For each function body found via brace matching, the byte offset of the
/// last bare-call line is recorded as a `FixSite`.
///
/// Returns an empty `Vec` if no eligible sites are found — the caller already
/// gates on `suggestions.iter().any(|m| m.id == "try_tail_call")`.
pub(crate) fn find_fix_sites(source: &str) -> Vec<FixSite> {
    let mut sites: Vec<FixSite> = Vec::new();

    // Track brace depth and the byte offset where each function body opened.
    // We only enter "body tracking" mode once we see the `{` that opens a
    // function (after a line that contains `fn `).
    let mut depth: usize = 0;
    // Were we inside at least one brace level that was entered after a `fn` line?
    let mut in_fn_body: bool = false;
    // Depth at which the current function body opened.
    let mut fn_body_depth: usize = 0;
    // Last call site byte offset seen inside the current function body.
    let mut last_call_offset: Option<usize> = None;

    // We need the previous line to detect whether a `{` belongs to a function.
    let mut prev_line_is_fn = false;

    let mut byte_offset: usize = 0;

    for line in source.lines() {
        let trimmed = line.trim();

        // Count braces on this line.
        let opens = line.chars().filter(|&c| c == '{').count();
        let closes = line.chars().filter(|&c| c == '}').count();

        // If we're inside a function body, check for a call candidate.
        if in_fn_body && depth > fn_body_depth {
            // Skip comments and empty lines.
            let is_comment = trimmed.starts_with("//") || trimmed.starts_with("/*");
            if !trimmed.is_empty() && !is_comment && looks_like_call(trimmed) {
                // Record the byte offset of the call on this line.
                // We want to insert before the first non-whitespace character.
                let leading_spaces = line.len() - line.trim_start().len();
                last_call_offset = Some(byte_offset + leading_spaces);
            }
        }

        // Update depth.
        depth = depth.saturating_add(opens).saturating_sub(closes);

        // Detect function body open: a `{` on a line that follows a `fn ` line.
        if opens > 0 && prev_line_is_fn && !in_fn_body {
            in_fn_body = true;
            fn_body_depth = depth - 1; // depth before the `{` on this line
            // If both `fn` keyword and `{` are on the same line (e.g. `fn foo() {`),
            // fn_body_depth needs adjustment — but since we check prev_line_is_fn
            // (set from the *previous* iteration) this case also works when the
            // `fn ... {` is on one line: prev_line_is_fn is set from the fn-bearing
            // line and the current line holds the opening brace.
        }

        // Also handle the common case where `fn foo() {` is all on one line.
        // In that case, both the fn keyword and the `{` appear on the same line.
        // prev_line_is_fn would be false (set at *end* of this iteration), so
        // we handle it here.
        let line_has_fn = trimmed.starts_with("fn ") || trimmed.contains(" fn ");
        if line_has_fn && opens > 0 && !in_fn_body {
            in_fn_body = true;
            // The body was opened by the last `{` on this line.
            // depth already includes all opens/closes on this line.
            // The body's interior is at depth fn_body_depth + 1.
            fn_body_depth = depth - 1;
        }

        // Detect function body close.
        if in_fn_body && depth <= fn_body_depth {
            // We've exited the function body.
            if let Some(offset) = last_call_offset.take() {
                sites.push(FixSite {
                    insert_before: offset,
                });
            }
            in_fn_body = false;
            fn_body_depth = 0;
        }

        // Advance byte offset.  `lines()` strips the newline, so add 1 for `\n`.
        // For `\r\n` line endings add 2.  We detect this by checking the raw bytes.
        let raw_line_bytes = &source.as_bytes()[byte_offset..];
        let line_byte_len = line.len();
        // Check for CRLF
        let newline_len = if raw_line_bytes.get(line_byte_len) == Some(&b'\r') {
            2
        } else if raw_line_bytes.get(line_byte_len) == Some(&b'\n') {
            1
        } else {
            0 // last line with no trailing newline
        };
        byte_offset += line_byte_len + newline_len;

        // Update prev_line_is_fn for next iteration.
        prev_line_is_fn = line_has_fn;
    }

    // Handle a function whose body was never closed (malformed source).
    // Emit whatever last call we found.
    if in_fn_body && let Some(offset) = last_call_offset.take() {
        sites.push(FixSite {
            insert_before: offset,
        });
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
        // Only the last call in the body should be fixed.
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
}
