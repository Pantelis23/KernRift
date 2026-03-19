use emit::ContractsSchema as EmitContractsSchema;
use kernriftc::SurfaceProfile;

#[derive(Debug)]
pub(crate) struct PolicyArgs {
    pub(crate) policy_path: String,
    pub(crate) contracts_path: String,
    pub(crate) evidence: bool,
    pub(crate) format: PolicyOutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PolicyOutputFormat {
    Text,
    Json,
}

impl PolicyOutputFormat {
    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "invalid policy mode: unsupported --format '{}' (expected 'text' or 'json')",
                other
            )),
        }
    }
}

#[derive(Debug)]
pub(crate) struct InspectArgs {
    pub(crate) contracts_path: String,
}

#[derive(Debug)]
pub(crate) struct InspectReportArgs {
    pub(crate) report_path: String,
    pub(crate) format: PolicyOutputFormat,
}

#[derive(Debug)]
pub(crate) struct CheckArgs {
    pub(crate) path: Option<String>,
    pub(crate) stdin: bool,
    pub(crate) surface: SurfaceProfile,
    pub(crate) canonical: bool,
    pub(crate) format: PolicyOutputFormat,
    pub(crate) profile: Option<CheckProfile>,
    pub(crate) contracts_schema: Option<ContractsSchemaArg>,
    pub(crate) contracts_out: Option<String>,
    pub(crate) policy_path: Option<String>,
    pub(crate) hash_out: Option<String>,
    pub(crate) sign_key_path: Option<String>,
    pub(crate) sig_out: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckProfile {
    Kernel,
}

impl CheckProfile {
    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "kernel" => Ok(Self::Kernel),
            other => Err(format!(
                "invalid check mode: unknown profile '{}', expected 'kernel'",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContractsSchemaArg {
    V1,
    V2,
}

impl ContractsSchemaArg {
    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        match raw.to_ascii_lowercase().as_str() {
            "v1" => Ok(Self::V1),
            "v2" => Ok(Self::V2),
            other => Err(format!(
                "invalid check mode: unknown contracts schema '{}', expected 'v1' or 'v2'",
                other
            )),
        }
    }

    pub(crate) fn to_emit_schema(self) -> EmitContractsSchema {
        match self {
            Self::V1 => EmitContractsSchema::V1,
            Self::V2 => EmitContractsSchema::V2,
        }
    }
}

#[derive(Debug)]
pub(crate) struct VerifyArgs {
    pub(crate) contracts_path: String,
    pub(crate) hash_path: String,
    pub(crate) sig_path: Option<String>,
    pub(crate) pubkey_path: Option<String>,
    pub(crate) report_path: Option<String>,
}

pub(crate) fn parse_check_args(args: &[String]) -> Result<CheckArgs, String> {
    let mut surface = SurfaceProfile::Stable;
    let mut saw_surface = false;
    let mut canonical = false;
    let mut stdin = false;
    let mut format = PolicyOutputFormat::Text;
    let mut format_set = false;
    let mut profile = None::<CheckProfile>;
    let mut contracts_schema = None::<ContractsSchemaArg>;
    let mut contracts_out = None::<String>;
    let mut policy_path = None::<String>;
    let mut hash_out = None::<String>;
    let mut sign_key_path = None::<String>;
    let mut sig_out = None::<String>;
    let mut positionals = Vec::<String>::new();

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--surface" => {
                if saw_surface {
                    return Err("invalid check mode: duplicate --surface".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --surface requires a value".to_string());
                };
                surface = SurfaceProfile::parse(value)
                    .map_err(|err| format!("invalid check mode: {}", err))?;
                saw_surface = true;
                idx += 2;
            }
            "--canonical" => {
                if canonical {
                    return Err("invalid check mode: duplicate --canonical".to_string());
                }
                canonical = true;
                idx += 1;
            }
            "--stdin" => {
                if stdin {
                    return Err("invalid check mode: duplicate --stdin".to_string());
                }
                stdin = true;
                idx += 1;
            }
            "--profile" => {
                if profile.is_some() {
                    return Err("invalid check mode: duplicate --profile".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --profile requires a value".to_string());
                };
                profile = Some(CheckProfile::parse(value)?);
                idx += 2;
            }
            "--format" => {
                if format_set {
                    return Err("invalid check mode: duplicate --format".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid check mode: --format requires 'text' or 'json'".to_string()
                    );
                };
                format = PolicyOutputFormat::parse(value)
                    .map_err(|err| err.replacen("invalid policy mode", "invalid check mode", 1))?;
                format_set = true;
                idx += 2;
            }
            "--contracts-schema" => {
                if contracts_schema.is_some() {
                    return Err("invalid check mode: duplicate --contracts-schema".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid check mode: --contracts-schema requires a value".to_string()
                    );
                };
                contracts_schema = Some(ContractsSchemaArg::parse(value)?);
                idx += 2;
            }
            "--contracts-out" => {
                if contracts_out.is_some() {
                    return Err("invalid check mode: duplicate --contracts-out".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid check mode: --contracts-out requires a file path".to_string()
                    );
                };
                contracts_out = Some(value.clone());
                idx += 2;
            }
            "--policy" => {
                if policy_path.is_some() {
                    return Err("invalid check mode: duplicate --policy".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --policy requires a file path".to_string());
                };
                policy_path = Some(value.clone());
                idx += 2;
            }
            "--hash-out" => {
                if hash_out.is_some() {
                    return Err("invalid check mode: duplicate --hash-out".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --hash-out requires a file path".to_string());
                };
                hash_out = Some(value.clone());
                idx += 2;
            }
            "--sign-ed25519" => {
                if sign_key_path.is_some() {
                    return Err("invalid check mode: duplicate --sign-ed25519".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid check mode: --sign-ed25519 requires a key file path".to_string(),
                    );
                };
                sign_key_path = Some(value.clone());
                idx += 2;
            }
            "--sig-out" => {
                if sig_out.is_some() {
                    return Err("invalid check mode: duplicate --sig-out".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --sig-out requires a file path".to_string());
                };
                sig_out = Some(value.clone());
                idx += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("invalid check mode: unknown flag '{}'", other));
            }
            _ => {
                positionals.push(args[idx].clone());
                idx += 1;
            }
        }
    }

    if stdin && !canonical {
        return Err("invalid check mode: --stdin requires --canonical".to_string());
    }

    if stdin && !positionals.is_empty() {
        return Err(
            "invalid check mode: --stdin cannot be combined with an input file".to_string(),
        );
    }

    if !stdin && positionals.len() != 1 {
        return Err("invalid check mode: expected exactly one <file.kr> input".to_string());
    }

    if canonical
        && (profile.is_some()
            || contracts_schema.is_some()
            || contracts_out.is_some()
            || policy_path.is_some()
            || hash_out.is_some()
            || sign_key_path.is_some()
            || sig_out.is_some())
    {
        return Err(
            "invalid check mode: --canonical cannot be combined with --profile, --contracts-schema, --contracts-out, --policy, --hash-out, --sign-ed25519, or --sig-out"
                .to_string(),
        );
    }

    if sign_key_path.is_some() ^ sig_out.is_some() {
        return Err(
            "invalid check mode: --sign-ed25519 and --sig-out must be provided together"
                .to_string(),
        );
    }

    Ok(CheckArgs {
        path: if stdin {
            None
        } else {
            Some(positionals.remove(0))
        },
        stdin,
        surface,
        canonical,
        format,
        profile,
        contracts_schema,
        contracts_out,
        policy_path,
        hash_out,
        sign_key_path,
        sig_out,
    })
}

pub(crate) fn parse_policy_args(args: &[String]) -> Result<PolicyArgs, String> {
    let mut policy_path = None::<String>;
    let mut contracts_path = None::<String>;
    let mut evidence = false;
    let mut format = PolicyOutputFormat::Text;
    let mut format_set = false;

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--format" => {
                if format_set {
                    return Err("invalid policy mode: duplicate --format".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err(
                        "invalid policy mode: --format requires 'text' or 'json'".to_string()
                    );
                }
                format = PolicyOutputFormat::parse(&args[idx])?;
                format_set = true;
            }
            "--policy" => {
                if policy_path.is_some() {
                    return Err("invalid policy mode: duplicate --policy".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err("invalid policy mode: --policy requires a file path".to_string());
                }
                policy_path = Some(args[idx].clone());
            }
            "--contracts" => {
                if contracts_path.is_some() {
                    return Err("invalid policy mode: duplicate --contracts".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err("invalid policy mode: --contracts requires a file path".to_string());
                }
                contracts_path = Some(args[idx].clone());
            }
            "--evidence" => {
                if evidence {
                    return Err("invalid policy mode: duplicate --evidence".to_string());
                }
                evidence = true;
            }
            _ => {
                return Err(format!(
                    "invalid policy mode: unexpected argument '{}'",
                    args[idx]
                ));
            }
        }
        idx += 1;
    }

    let Some(policy_path) = policy_path else {
        return Err("invalid policy mode: missing --policy".to_string());
    };
    let Some(contracts_path) = contracts_path else {
        return Err("invalid policy mode: missing --contracts".to_string());
    };

    Ok(PolicyArgs {
        policy_path,
        contracts_path,
        evidence,
        format,
    })
}

pub(crate) fn parse_inspect_args(args: &[String]) -> Result<InspectArgs, String> {
    let mut contracts_path = None::<String>;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--contracts" => {
                if contracts_path.is_some() {
                    return Err("invalid inspect mode: duplicate --contracts".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err(
                        "invalid inspect mode: --contracts requires a file path".to_string()
                    );
                }
                contracts_path = Some(args[idx].clone());
            }
            other => {
                return Err(format!(
                    "invalid inspect mode: unexpected argument '{}'",
                    other
                ));
            }
        }
        idx += 1;
    }

    let Some(contracts_path) = contracts_path else {
        return Err("invalid inspect mode: missing --contracts".to_string());
    };

    Ok(InspectArgs { contracts_path })
}

pub(crate) fn parse_inspect_report_args(args: &[String]) -> Result<InspectReportArgs, String> {
    let mut report_path = None::<String>;
    let mut format = PolicyOutputFormat::Text;
    let mut format_set = false;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--format" => {
                if format_set {
                    return Err("invalid inspect-report mode: duplicate --format".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err(
                        "invalid inspect-report mode: --format requires 'text' or 'json'"
                            .to_string(),
                    );
                }
                format = PolicyOutputFormat::parse(&args[idx]).map_err(|err| {
                    err.replacen("invalid policy mode", "invalid inspect-report mode", 1)
                })?;
                format_set = true;
            }
            "--report" => {
                if report_path.is_some() {
                    return Err("invalid inspect-report mode: duplicate --report".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err(
                        "invalid inspect-report mode: --report requires a file path".to_string()
                    );
                }
                report_path = Some(args[idx].clone());
            }
            other => {
                return Err(format!(
                    "invalid inspect-report mode: unexpected argument '{}'",
                    other
                ));
            }
        }
        idx += 1;
    }

    let Some(report_path) = report_path else {
        return Err("invalid inspect-report mode: missing --report".to_string());
    };

    Ok(InspectReportArgs {
        report_path,
        format,
    })
}

pub(crate) fn parse_verify_args(args: &[String]) -> Result<VerifyArgs, String> {
    let mut contracts_path = None::<String>;
    let mut hash_path = None::<String>;
    let mut sig_path = None::<String>;
    let mut pubkey_path = None::<String>;
    let mut report_path = None::<String>;

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--contracts" => {
                if contracts_path.is_some() {
                    return Err("invalid verify mode: duplicate --contracts".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --contracts requires a file path".to_string());
                };
                contracts_path = Some(value.clone());
                idx += 2;
            }
            "--hash" => {
                if hash_path.is_some() {
                    return Err("invalid verify mode: duplicate --hash".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --hash requires a file path".to_string());
                };
                hash_path = Some(value.clone());
                idx += 2;
            }
            "--sig" => {
                if sig_path.is_some() {
                    return Err("invalid verify mode: duplicate --sig".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --sig requires a file path".to_string());
                };
                sig_path = Some(value.clone());
                idx += 2;
            }
            "--pubkey" => {
                if pubkey_path.is_some() {
                    return Err("invalid verify mode: duplicate --pubkey".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --pubkey requires a file path".to_string());
                };
                pubkey_path = Some(value.clone());
                idx += 2;
            }
            "--report" => {
                if report_path.is_some() {
                    return Err("invalid verify mode: duplicate --report".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --report requires a file path".to_string());
                };
                report_path = Some(value.clone());
                idx += 2;
            }
            other => {
                return Err(format!("invalid verify mode: unknown token '{}'", other));
            }
        }
    }

    let Some(contracts_path) = contracts_path else {
        return Err("invalid verify mode: missing --contracts <contracts.json>".to_string());
    };
    let Some(hash_path) = hash_path else {
        return Err("invalid verify mode: missing --hash <contracts.sha256>".to_string());
    };
    if sig_path.is_some() ^ pubkey_path.is_some() {
        return Err(
            "invalid verify mode: --sig and --pubkey must be provided together".to_string(),
        );
    }

    Ok(VerifyArgs {
        contracts_path,
        hash_path,
        sig_path,
        pubkey_path,
        report_path,
    })
}
