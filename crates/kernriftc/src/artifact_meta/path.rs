use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn normalize_backend_artifact_input_path(input_path: &str) -> (String, &'static str) {
    let raw = input_path.to_string();
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(_) => return (raw, "raw"),
    };
    let input_abs = Path::new(input_path);
    let input_abs = if input_abs.is_absolute() {
        input_abs.to_path_buf()
    } else {
        cwd.join(input_abs)
    };
    let input_abs = match fs::canonicalize(input_abs) {
        Ok(input_abs) => input_abs,
        Err(_) => return (raw, "raw"),
    };
    let repo_root = match find_git_repo_root_from(&input_abs) {
        Some(repo_root) => repo_root,
        None => return (raw, "raw"),
    };
    let repo_root = match fs::canonicalize(repo_root) {
        Ok(repo_root) => repo_root,
        Err(_) => return (raw, "raw"),
    };

    match input_abs.strip_prefix(&repo_root) {
        Ok(relative) => (
            relative.to_string_lossy().replace('\\', "/"),
            "repo-relative",
        ),
        Err(_) => (raw, "raw"),
    }
}

fn find_git_repo_root_from(start: &Path) -> Option<PathBuf> {
    let anchor = if start.is_dir() {
        start
    } else {
        start.parent().unwrap_or(start)
    };
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(anchor)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let repo_root = stdout.trim();
    if repo_root.is_empty() {
        None
    } else {
        Some(PathBuf::from(repo_root))
    }
}
