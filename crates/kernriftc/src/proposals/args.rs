#[derive(Debug)]
pub(crate) struct ProposalsArgs {
    pub(crate) validate: bool,
    pub(crate) promotion_readiness: bool,
    pub(crate) promote_feature: Option<String>,
    pub(crate) dry_run: bool,
    pub(crate) diff: bool,
}

pub(crate) fn parse_proposals_args(args: &[String]) -> Result<ProposalsArgs, String> {
    let mut validate = false;
    let mut promotion_readiness = false;
    let mut promote_feature = None::<String>;
    let mut dry_run = false;
    let mut diff = false;
    let mut idx = 0usize;
    while idx < args.len() {
        let arg = &args[idx];
        match arg.as_str() {
            "--validate" => {
                if validate {
                    return Err("invalid proposals mode: duplicate --validate".to_string());
                }
                if promotion_readiness || promote_feature.is_some() {
                    return Err(
                        "invalid proposals mode: unexpected argument '--validate'".to_string()
                    );
                }
                validate = true;
            }
            "--promotion-readiness" => {
                if promotion_readiness {
                    return Err(
                        "invalid proposals mode: duplicate --promotion-readiness".to_string()
                    );
                }
                if validate {
                    return Err(
                        "invalid proposals mode: unexpected argument '--promotion-readiness'"
                            .to_string(),
                    );
                }
                if promote_feature.is_some() {
                    return Err(
                        "invalid proposals mode: unexpected argument '--promotion-readiness'"
                            .to_string(),
                    );
                }
                promotion_readiness = true;
            }
            "--promote" => {
                if promote_feature.is_some() {
                    return Err("invalid proposals mode: duplicate --promote".to_string());
                }
                if validate || promotion_readiness {
                    return Err(
                        "invalid proposals mode: unexpected argument '--promote'".to_string()
                    );
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid proposals mode: --promote requires a feature id".to_string()
                    );
                };
                promote_feature = Some(value.clone());
                idx += 1;
            }
            "--dry-run" => {
                if dry_run {
                    return Err("invalid proposals mode: duplicate --dry-run".to_string());
                }
                dry_run = true;
            }
            "--diff" => {
                if diff {
                    return Err("invalid proposals mode: duplicate --diff".to_string());
                }
                diff = true;
            }
            other => {
                return Err(format!(
                    "invalid proposals mode: unexpected argument '{}'",
                    other
                ));
            }
        }
        idx += 1;
    }

    if dry_run && promote_feature.is_none() {
        return Err(
            "invalid proposals mode: --dry-run requires --promote <feature-id>".to_string(),
        );
    }
    if diff && promote_feature.is_none() {
        return Err("invalid proposals mode: --diff requires --promote <feature-id>".to_string());
    }

    Ok(ProposalsArgs {
        validate,
        promotion_readiness,
        promote_feature,
        dry_run,
        diff,
    })
}
