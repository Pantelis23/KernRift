use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;
use assert_cmd::assert::Assert;
use assert_cmd::cargo::cargo_bin_cmd;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use ed25519_dalek::SigningKey;
use jsonschema::JSONSchema;
use predicates::str::contains;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const CONTRACTS_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_contracts_v1.schema.json");
const CONTRACTS_SCHEMA_V2: &str =
    include_str!("../../../docs/schemas/kernrift_contracts_v2.schema.json");
const POLICY_VIOLATIONS_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_policy_violations_v1.schema.json");
const INSPECT_ARTIFACT_SCHEMA_V2: &str =
    include_str!("../../../docs/schemas/kernrift_inspect_artifact_v2.schema.json");
const VERIFY_ARTIFACT_META_SCHEMA_V2: &str =
    include_str!("../../../docs/schemas/kernrift_verify_artifact_meta_v2.schema.json");
const CANONICAL_FINDINGS_SCHEMA_V2: &str =
    include_str!("../../../docs/schemas/kernrift_canonical_findings_v2.schema.json");
const CANONICAL_EDIT_PLAN_SCHEMA_V2: &str =
    include_str!("../../../docs/schemas/kernrift_canonical_edit_plan_v2.schema.json");
const CANONICAL_FIX_RESULT_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_canonical_fix_result_v1.schema.json");
const CANONICAL_FIX_PREVIEW_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_canonical_fix_preview_v1.schema.json");
const VERIFY_REPORT_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_verify_report_v1.schema.json");
const ADAPTIVE_OS_CONTEXT_TEXT: &str = include_str!("../../../docs/ADAPTIVE_OS_CONTEXT.md");
const ARCHITECTURE_DOC_TEXT: &str = include_str!("../../../docs/ARCHITECTURE.md");
const KERNEL_PROFILE_NOTES_TEXT: &str =
    include_str!("../../../docs/design/kernel_profile_pr1_notes.md");
const KR0_KR3_PLAN_TEXT: &str = include_str!("../../../docs/KR0_KR3_PLAN.md");
const KR0_AUTHORING_REFERENCE_TEXT: &str =
    include_str!("../../../docs/spec/kr0-canonical-authoring-reference.md");
const KRIR_SPEC_TEXT: &str = include_str!("../../../docs/spec/krir-v0.1.md");
const KERNEL_PROFILE_SPEC_TEXT: &str = include_str!("../../../docs/spec/kernel_profile.md");
const README_TEXT: &str = include_str!("../../../README.md");
const FULL_SERIAL_WRAPPER_TEXT: &str = include_str!("../../../tools/validation/full_serial.sh");
const LOCAL_SAFE_WRAPPER_TEXT: &str = include_str!("../../../tools/validation/local_safe.sh");

#[path = "cli_contract/schema_support.rs"]
mod schema_support;
#[path = "cli_contract/support.rs"]
mod support;

use schema_support::*;
use support::*;

include!("cli_contract/meta_docs.rs");
include!("cli_contract/artifact_cli.rs");
include!("cli_contract/frontend_surface.rs");
include!("cli_contract/canonical_check.rs");
include!("cli_contract/canonical_fix.rs");
include!("cli_contract/canonical_schema.rs");
include!("cli_contract/governance_cli.rs");
include!("cli_contract/canonical_migrate_preview.rs");
include!("cli_contract/migrate_preview_surface.rs");
include!("cli_contract/frontend_contracts.rs");
include!("cli_contract/policy_verify.rs");
