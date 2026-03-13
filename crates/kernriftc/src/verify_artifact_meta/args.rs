#[derive(Debug)]
pub(crate) struct VerifyArtifactMetaArgs {
    pub(crate) artifact_path: String,
    pub(crate) metadata_path: String,
    pub(crate) format: VerifyArtifactMetaFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifyArtifactMetaFormat {
    Text,
    Json,
}

impl VerifyArtifactMetaFormat {
    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "invalid verify-artifact-meta mode: unsupported --format '{}' (expected 'text' or 'json')",
                other
            )),
        }
    }
}

pub(crate) fn parse_verify_artifact_meta_args(
    args: &[String],
) -> Result<VerifyArtifactMetaArgs, String> {
    let mut format = VerifyArtifactMetaFormat::Text;
    let mut format_set = false;
    let mut positionals = Vec::<String>::new();

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--format" => {
                if format_set {
                    return Err("invalid verify-artifact-meta mode: duplicate --format".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err(
                        "invalid verify-artifact-meta mode: --format requires 'text' or 'json'"
                            .to_string(),
                    );
                }
                format = VerifyArtifactMetaFormat::parse(&args[idx])?;
                format_set = true;
            }
            arg if arg.starts_with('-') => {
                return Err(format!(
                    "invalid verify-artifact-meta mode: unexpected argument '{}'",
                    arg
                ));
            }
            arg => {
                positionals.push(arg.to_string());
            }
        }
        idx += 1;
    }

    if positionals.len() != 2 {
        return Err(
            "invalid verify-artifact-meta mode: expected <artifact> <meta.json>".to_string(),
        );
    }

    Ok(VerifyArtifactMetaArgs {
        artifact_path: positionals.remove(0),
        metadata_path: positionals.remove(0),
        format,
    })
}
