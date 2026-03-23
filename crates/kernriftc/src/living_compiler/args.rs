use kernriftc::SurfaceProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LivingCompilerFormat {
    Text,
    Json,
}

impl LivingCompilerFormat {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "invalid living-compiler mode: unsupported --format '{}' (expected 'text' or 'json')",
                other
            )),
        }
    }
}

/// Parsed arguments for `kernriftc lc` / `kernriftc living-compiler`.
#[derive(Debug, Clone)]
pub(crate) struct LivingCompilerArgs {
    /// Primary input file (always required).
    pub(crate) input_path: String,
    /// Optional second file for two-file diff mode.
    pub(crate) diff_after: Option<String>,
    pub(crate) surface: SurfaceProfile,
    pub(crate) format: LivingCompilerFormat,
    /// Enable CI mode: exit 1 if any suggestion fitness >= `ci_min_fitness`.
    pub(crate) ci: bool,
    /// Threshold for CI mode and display filter (default 50).
    pub(crate) ci_min_fitness: u8,
    /// True when `--min-fitness` was explicitly supplied (enables display filter without --ci).
    pub(crate) min_fitness_explicit: bool,
    /// Enable diff mode. Uses git HEAD when `diff_after` is None.
    pub(crate) diff: bool,
    /// Enable auto-fix mode (try_tail_call only).
    pub(crate) fix: bool,
    /// Print what would be fixed without writing.
    pub(crate) dry_run: bool,
    /// Apply fixes in place.
    pub(crate) write: bool,
}

pub(crate) fn parse_living_compiler_args(args: &[String]) -> Result<LivingCompilerArgs, String> {
    let mut surface = SurfaceProfile::Stable;
    let mut format = LivingCompilerFormat::Text;
    let mut positionals: Vec<String> = Vec::new();
    let mut ci = false;
    let mut ci_min_fitness: u8 = 50;
    let mut min_fitness_explicit = false;
    let mut diff = false;
    let mut fix = false;
    let mut dry_run = false;
    let mut write = false;

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--surface" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid living-compiler mode: --surface requires a value".to_string()
                    );
                };
                surface = SurfaceProfile::parse(value)
                    .map_err(|err| format!("invalid living-compiler mode: {}", err))?;
                idx += 2;
            }
            "--format" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid living-compiler mode: --format requires a value".to_string()
                    );
                };
                format = LivingCompilerFormat::parse(value)?;
                idx += 2;
            }
            "--ci" => {
                ci = true;
                idx += 1;
            }
            "--min-fitness" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid living-compiler mode: --min-fitness requires a value".to_string()
                    );
                };
                ci_min_fitness = value.parse::<u8>().map_err(|_| {
                    format!(
                        "invalid living-compiler mode: --min-fitness '{}' is not a valid 0-100 integer",
                        value
                    )
                })?;
                min_fitness_explicit = true;
                idx += 2;
            }
            "--diff" => {
                diff = true;
                idx += 1;
            }
            "--fix" => {
                fix = true;
                idx += 1;
            }
            "--dry-run" => {
                dry_run = true;
                idx += 1;
            }
            "--write" => {
                write = true;
                idx += 1;
            }
            other if other.starts_with('-') => {
                return Err(format!(
                    "invalid living-compiler mode: unknown flag '{}'",
                    other
                ));
            }
            other => {
                positionals.push(other.to_string());
                idx += 1;
            }
        }
    }

    // Validate flag combinations.
    if diff && fix {
        return Err("invalid living-compiler mode: --diff and --fix cannot be combined".to_string());
    }
    if fix && !dry_run && !write {
        return Err(
            "error: --fix requires --dry-run or --write".to_string()
        );
    }

    // Validate positional arguments.
    if diff {
        match positionals.len() {
            1 => {} // git-aware mode
            2 => {} // two-file mode
            _ => {
                return Err(
                    "invalid living-compiler mode: --diff requires 1 or 2 <file.kr> arguments".to_string()
                );
            }
        }
    } else if positionals.len() != 1 {
        return Err(
            "invalid living-compiler mode: expected exactly one <file.kr> input".to_string(),
        );
    }

    let mut pos_iter = positionals.into_iter();
    let input_path = pos_iter.next().expect("at least one positional");
    let diff_after = pos_iter.next();

    Ok(LivingCompilerArgs {
        input_path,
        diff_after,
        surface,
        format,
        ci,
        ci_min_fitness,
        min_fitness_explicit,
        diff,
        fix,
        dry_run,
        write,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> String { v.to_string() }

    #[test]
    fn parse_ci_flag() {
        let a = parse_living_compiler_args(&[s("--ci"), s("file.kr")]).unwrap();
        assert!(a.ci);
        assert_eq!(a.ci_min_fitness, 50);
    }

    #[test]
    fn parse_min_fitness_override() {
        let a = parse_living_compiler_args(&[s("--ci"), s("--min-fitness"), s("70"), s("file.kr")]).unwrap();
        assert_eq!(a.ci_min_fitness, 70);
        assert!(a.min_fitness_explicit);
    }

    #[test]
    fn parse_diff_one_file() {
        let a = parse_living_compiler_args(&[s("--diff"), s("file.kr")]).unwrap();
        assert!(a.diff);
        assert_eq!(a.input_path, "file.kr");
        assert!(a.diff_after.is_none());
    }

    #[test]
    fn parse_diff_two_files() {
        let a = parse_living_compiler_args(&[s("--diff"), s("before.kr"), s("after.kr")]).unwrap();
        assert!(a.diff);
        assert_eq!(a.input_path, "before.kr");
        assert_eq!(a.diff_after.as_deref(), Some("after.kr"));
    }

    #[test]
    fn parse_fix_write() {
        let a = parse_living_compiler_args(&[s("--fix"), s("--write"), s("file.kr")]).unwrap();
        assert!(a.fix);
        assert!(a.write);
        assert!(!a.dry_run);
    }

    #[test]
    fn fix_without_mode_is_error() {
        let e = parse_living_compiler_args(&[s("--fix"), s("file.kr")]).unwrap_err();
        assert!(e.contains("--dry-run or --write"), "got: {}", e);
    }

    #[test]
    fn diff_and_fix_is_error() {
        let e = parse_living_compiler_args(&[s("--diff"), s("--fix"), s("--write"), s("file.kr")]).unwrap_err();
        assert!(e.contains("cannot be combined"), "got: {}", e);
    }
}
