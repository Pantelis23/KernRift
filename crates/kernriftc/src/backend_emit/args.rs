use kernriftc::{BackendArtifactKind, SurfaceProfile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackendEmitArgs {
    pub(crate) surface: SurfaceProfile,
    pub(crate) kind: BackendArtifactKind,
    pub(crate) output_path: String,
    pub(crate) meta_output_path: Option<String>,
    pub(crate) input_path: String,
}

pub(crate) fn parse_backend_emit_args(
    kind: &str,
    args: &[String],
    surface: SurfaceProfile,
) -> Result<BackendEmitArgs, String> {
    let kind =
        BackendArtifactKind::parse(kind).map_err(|err| format!("invalid emit mode: {}", err))?;
    let mut output_path = None::<String>;
    let mut meta_output_path = None::<String>;
    let mut positionals = Vec::<String>::new();

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "-o" => {
                if output_path.is_some() {
                    return Err("invalid emit mode: duplicate -o".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid emit mode: -o requires a file path".to_string());
                };
                output_path = Some(value.clone());
                idx += 2;
            }
            "--meta-out" => {
                if meta_output_path.is_some() {
                    return Err("invalid emit mode: duplicate --meta-out".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid emit mode: --meta-out requires a file path".to_string());
                };
                meta_output_path = Some(value.clone());
                idx += 2;
            }
            other if other.starts_with('-') => {
                return Err(format!("invalid emit mode: unknown flag '{}'", other));
            }
            other => {
                positionals.push(other.to_string());
                idx += 1;
            }
        }
    }

    let Some(output_path) = output_path else {
        return Err("invalid emit mode: missing -o <output-path>".to_string());
    };

    if meta_output_path.is_some()
        && matches!(
            kind,
            BackendArtifactKind::Asm
                | BackendArtifactKind::ElfExecutable
                | BackendArtifactKind::StaticLib
        )
    {
        return Err(format!(
            "invalid emit mode: --meta-out is unsupported for '{}'",
            kind.as_str()
        ));
    }

    if positionals.len() != 1 {
        return Err("invalid emit mode: expected exactly one <file.kr> input".to_string());
    }

    Ok(BackendEmitArgs {
        surface,
        kind,
        output_path,
        meta_output_path,
        input_path: positionals.pop().expect("exactly one positional"),
    })
}
