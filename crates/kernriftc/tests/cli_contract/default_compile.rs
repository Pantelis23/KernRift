#[test]
fn default_compile_produces_executable_in_cwd() {
    let tmp = std::env::temp_dir().join(format!(
        "kernriftc_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();

    let hello_kr = repo_root().join("hello.kr");

    Command::cargo_bin("kernriftc")
        .unwrap()
        .current_dir(&tmp)
        .arg(hello_kr.to_str().unwrap())
        .assert()
        .success();

    assert!(
        tmp.join("hello.krbo").exists(),
        "expected hello.krbo in CWD after kernriftc hello.kr"
    );

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn default_compile_extra_args_gives_clear_error() {
    Command::cargo_bin("kernriftc")
        .unwrap()
        .arg(repo_root().join("hello.kr").to_str().unwrap())
        .arg("--extra")
        .assert()
        .failure()
        .stderr(predicates::str::contains("unexpected arguments after source file"));
}

#[test]
fn unknown_subcommand_gives_clear_error() {
    Command::cargo_bin("kernriftc")
        .unwrap()
        .arg("notacommand")
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown subcommand"));
}

#[test]
fn wrong_extension_gives_clear_error() {
    Command::cargo_bin("kernriftc")
        .unwrap()
        .arg("config.toml")
        .assert()
        .failure()
        .stderr(predicates::str::contains("expected a .kr source file"));
}

#[test]
fn unknown_flag_gives_clear_error() {
    Command::cargo_bin("kernriftc")
        .unwrap()
        .arg("--unknownflag")
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown flag"));
}
