use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(super) fn write_output_files(outputs: &[(String, String)]) -> Result<(), String> {
    if outputs.is_empty() {
        return Ok(());
    }

    let mut final_paths = BTreeSet::<&str>::new();
    for (path, _) in outputs {
        if !final_paths.insert(path.as_str()) {
            return Err(format!("duplicate output path '{}'", path));
        }
        if Path::new(path).exists() {
            return Err(format!(
                "refusing to overwrite existing output '{}'; remove it first",
                path
            ));
        }
    }

    let mut staged = Vec::<(String, String)>::new();
    for (idx, (path, payload)) in outputs.iter().enumerate() {
        let tmp = format!("{}.kernriftc.tmp.{}.{}", path, std::process::id(), idx);
        fs::write(Path::new(&tmp), payload).map_err(|e| {
            cleanup_temp_paths(&staged);
            format!("failed to stage output '{}': {}", path, e)
        })?;
        staged.push((tmp, path.clone()));
    }

    let mut committed = Vec::<String>::new();
    for (tmp, final_path) in &staged {
        if let Err(err) = fs::rename(Path::new(tmp), Path::new(final_path)) {
            cleanup_temp_paths(&staged);
            cleanup_final_paths(&committed);
            return Err(format!(
                "failed to commit output '{}' from '{}': {}",
                final_path, tmp, err
            ));
        }
        committed.push(final_path.clone());
    }

    Ok(())
}

fn cleanup_temp_paths(staged: &[(String, String)]) {
    for (tmp, _) in staged {
        let _ = fs::remove_file(Path::new(tmp));
    }
}

fn cleanup_final_paths(paths: &[String]) {
    for path in paths {
        let _ = fs::remove_file(Path::new(path));
    }
}
