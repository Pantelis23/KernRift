
fn kernriftc_migrate_bin() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // pop test binary name
    if p.ends_with("deps") {
        p.pop();
    }
    p.push("kernriftc");
    p
}

fn run_migrate_cmd(args: &[&str]) -> std::process::Output {
    std::process::Command::new(kernriftc_migrate_bin())
        .arg("migrate")
        .args(args)
        .output()
        .expect("failed to run kernriftc migrate")
}

#[test]
fn migrate_canonical_source_noops_cleanly() {
    let src = must_pass_fixture("basic.kr");
    let tmp = unique_temp_output_path("migrate-noop", "kr");
    fs::copy(&src, &tmp).unwrap();

    let out = run_migrate_cmd(&[tmp.to_str().unwrap()]);
    assert!(out.status.success(), "migrate should succeed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No migration needed"),
        "expected 'No migration needed', got: {stdout}"
    );
    assert_eq!(
        fixture_text(&tmp).replace("\r\n", "\n"),
        fixture_text(&src).replace("\r\n", "\n"),
        "file should be unchanged for canonical source"
    );
}

#[test]
fn migrate_dry_run_does_not_write() {
    let tmp = unique_temp_output_path("migrate-dry-run", "kr");
    // @thread_entry is a stable alias with migration_safe=true.
    fs::write(&tmp, "@thread_entry\nfn f() {}\n").unwrap();
    let original = fixture_text(&tmp).replace("\r\n", "\n");

    let out = run_migrate_cmd(&[tmp.to_str().unwrap(), "--dry-run"]);
    assert!(
        out.status.success(),
        "migrate --dry-run should succeed: {:?}",
        out
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("dry-run") || stdout.contains("Would migrate"),
        "expected dry-run notice, got: {stdout}"
    );
    assert_eq!(
        fixture_text(&tmp).replace("\r\n", "\n"),
        original,
        "--dry-run must not write the file"
    );
}
