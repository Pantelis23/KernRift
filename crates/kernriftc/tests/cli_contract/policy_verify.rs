#[test]
fn policy_denies_raw_mmio_by_default_when_configured() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let contracts_path = write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-deny");
    let policy_path = write_temp_policy_file("raw-mmio-deny", "[kernel]\nallow_raw_mmio = false\n");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec!["policy: KERNEL_RAW_MMIO_FORBID: raw_mmio is not allowed"]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allows_raw_mmio_when_explicitly_enabled() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let contracts_path = write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-allow");
    let policy_path = write_temp_policy_file("raw-mmio-allow", "[kernel]\nallow_raw_mmio = true\n");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allows_structured_mmio_when_raw_mmio_is_denied() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_register_declared.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-structured-pass");
    let policy_path = write_temp_policy_file(
        "raw-mmio-structured-pass",
        "[kernel]\nallow_raw_mmio = false\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_enforces_raw_mmio_site_limit() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_bypass_register_checks.kr");
    let contracts_path = write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-site-limit");
    let policy_path = write_temp_policy_file(
        "raw-mmio-site-limit",
        "[kernel]\nallow_raw_mmio = true\nmax_raw_mmio_sites = 1\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_RAW_MMIO_SITE_LIMIT: raw_mmio_sites_count 2 exceeds allowed maximum 1"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_enforces_raw_mmio_symbol_allowlist() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_two_symbols.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-symbol-allowlist");
    let policy_path = write_temp_policy_file(
        "raw-mmio-symbol-allowlist",
        "[kernel]\nallow_raw_mmio_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec!["policy: KERNEL_RAW_MMIO_SYMBOL_ALLOWLIST: raw_mmio symbol 'helper' is not allowed"]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allows_non_irq_raw_mmio_when_irq_raw_mmio_is_forbidden() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-non-irq-pass");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-non-irq-pass",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_denies_raw_mmio_in_irq_context() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-direct");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-direct",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_FORBID: raw_mmio is not allowed in irq context (via entry)"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allows_structured_mmio_in_irq_when_only_irq_raw_mmio_is_forbidden() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("irq_ctx_chain.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-structured-pass");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-structured-pass",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_denies_irq_reachable_helper_that_uses_raw_mmio() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-helper");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-helper",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_FORBID: raw_mmio is not allowed in irq context (via helper)"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_formats_irq_raw_mmio_forbid_with_multihop_contract_path() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let emitted_contracts =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-path-helper");
    let mut contracts: Value =
        serde_json::from_str(&fs::read_to_string(&emitted_contracts).expect("contracts text"))
            .expect("contracts json");
    let helper = contracts["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols")
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper["ctx_path_provenance"] = json!([{
        "ctx": "irq",
        "path": ["entry", "helper"]
    }]);
    let contracts_path = write_temp_contracts_file("raw-mmio-irq-forbid-path-helper", &contracts);
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-path-helper",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_FORBID: raw_mmio is not allowed in irq context (via entry -> helper)"
        ]
    );

    fs::remove_file(&emitted_contracts).ok();
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_irq_raw_mmio_forbid_is_structured_and_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-evidence-direct");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-evidence-direct",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_FORBID: raw_mmio is not allowed in irq context (via entry)",
            "evidence: symbol=entry",
            "evidence: irq_path=[entry]",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_irq_raw_mmio_forbid_multihop_path_is_structured() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let emitted_contracts =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-evidence-multihop");
    let mut contracts: Value =
        serde_json::from_str(&fs::read_to_string(&emitted_contracts).expect("contracts text"))
            .expect("contracts json");
    let helper = contracts["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols")
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper["ctx_path_provenance"] = json!([{
        "ctx": "irq",
        "path": ["entry", "dispatch", "helper"]
    }]);
    let contracts_path =
        write_temp_contracts_file("raw-mmio-irq-forbid-evidence-multihop", &contracts);
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-evidence-multihop",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_FORBID: raw_mmio is not allowed in irq context (via entry -> dispatch -> helper)",
            "evidence: symbol=helper",
            "evidence: irq_path=[entry,dispatch,helper]",
        ]
    );

    fs::remove_file(&emitted_contracts).ok();
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_irq_raw_mmio_site_limit_ignores_non_irq_raw_mmio() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_bypass_register_checks.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-site-limit-non-irq-pass");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-site-limit-non-irq-pass",
        "[kernel]\nallow_raw_mmio = true\nmax_raw_mmio_sites_in_irq = 0\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_enforces_irq_raw_mmio_site_limit_for_direct_irq_usage() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-site-limit-direct");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-site-limit-direct",
        "[kernel]\nallow_raw_mmio = true\nmax_raw_mmio_sites_in_irq = 0\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SITE_LIMIT: irq raw_mmio_sites_count 1 exceeds allowed maximum 0"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_enforces_irq_raw_mmio_site_limit_for_irq_reachable_helper() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-site-limit-helper");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-site-limit-helper",
        "[kernel]\nallow_raw_mmio = true\nmax_raw_mmio_sites_in_irq = 0\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SITE_LIMIT: irq raw_mmio_sites_count 1 exceeds allowed maximum 0"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_irq_raw_mmio_site_limit_does_not_affect_structured_irq_mmio() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("irq_ctx_chain.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-site-limit-structured");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-site-limit-structured",
        "[kernel]\nallow_raw_mmio = true\nmax_raw_mmio_sites_in_irq = 0\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_irq_raw_mmio_symbol_allowlist_ignores_non_irq_raw_mmio() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_two_symbols.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &fixture,
        "raw-mmio-irq-symbol-allowlist-non-irq-pass",
    );
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-non-irq-pass",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"nobody\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allows_irq_raw_mmio_when_symbol_is_allowlisted() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-symbol-allowlist-pass");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-pass",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_denies_irq_raw_mmio_when_symbol_is_not_allowlisted() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &fixture,
        "raw-mmio-irq-symbol-allowlist-direct-deny",
    );
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-direct-deny",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"helper\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST: irq raw_mmio symbol 'entry' is not allowed (via entry)"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_denies_irq_reachable_raw_mmio_helper_when_not_allowlisted() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &fixture,
        "raw-mmio-irq-symbol-allowlist-helper-deny",
    );
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-helper-deny",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST: irq raw_mmio symbol 'helper' is not allowed (via helper)"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_formats_irq_raw_mmio_symbol_allowlist_with_deep_contract_path() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let emitted_contracts =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-symbol-allowlist-deep-path");
    let mut contracts: Value =
        serde_json::from_str(&fs::read_to_string(&emitted_contracts).expect("contracts text"))
            .expect("contracts json");
    let helper = contracts["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols")
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper["ctx_path_provenance"] = json!([{
        "ctx": "irq",
        "path": ["entry", "dispatch", "helper"]
    }]);
    let contracts_path =
        write_temp_contracts_file("raw-mmio-irq-symbol-allowlist-deep-path", &contracts);
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-deep-path",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST: irq raw_mmio symbol 'helper' is not allowed (via entry -> dispatch -> helper)"
        ]
    );

    fs::remove_file(&emitted_contracts).ok();
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_irq_raw_mmio_symbol_allowlist_is_structured_and_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &fixture,
        "raw-mmio-irq-symbol-allowlist-evidence-helper",
    );
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-evidence-helper",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST: irq raw_mmio symbol 'helper' is not allowed (via helper)",
            "evidence: symbol=helper",
            "evidence: irq_path=[helper]",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_json_irq_raw_mmio_forbid_is_exact_and_structured() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-json-direct");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-json-direct",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "json policy mode must not write stderr: {}",
        stderr
    );
    assert_json_transport(&stdout, &stderr, "kernrift_policy_violations_v1");
    assert_eq!(
        stdout,
        concat!(
            "{\n",
            "  \"schema_version\": \"kernrift_policy_violations_v1\",\n",
            "  \"result\": \"deny\",\n",
            "  \"exit_code\": 1,\n",
            "  \"violations\": [\n",
            "    {\n",
            "      \"rule\": \"KERNEL_IRQ_RAW_MMIO_FORBID\",\n",
            "      \"family\": \"effect\",\n",
            "      \"message\": \"raw_mmio is not allowed in irq context (via entry)\",\n",
            "      \"evidence\": [\n",
            "        {\n",
            "          \"kind\": \"scalar\",\n",
            "          \"key\": \"symbol\",\n",
            "          \"value\": \"entry\"\n",
            "        },\n",
            "        {\n",
            "          \"kind\": \"list\",\n",
            "          \"key\": \"irq_path\",\n",
            "          \"values\": [\n",
            "            \"entry\"\n",
            "          ]\n",
            "        }\n",
            "      ]\n",
            "    }\n",
            "  ]\n",
            "}\n"
        )
    );
    let json: Value = serde_json::from_str(&stdout).expect("policy json");
    validate_policy_violations_schema(&json);
    assert_eq!(
        object_keys(&json),
        BTreeSet::from([
            "exit_code".to_string(),
            "result".to_string(),
            "schema_version".to_string(),
            "violations".to_string(),
        ]),
        "policy json envelope drifted"
    );
    let violation = json["violations"][0].as_object().expect("violation object");
    assert_eq!(
        violation.keys().cloned().collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "evidence".to_string(),
            "family".to_string(),
            "message".to_string(),
            "rule".to_string(),
        ]),
        "policy json violation shape drifted"
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_json_irq_raw_mmio_allowlist_deep_path_is_exact_and_structured() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let emitted_contracts =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-allowlist-json-deep");
    let mut contracts: Value =
        serde_json::from_str(&fs::read_to_string(&emitted_contracts).expect("contracts text"))
            .expect("contracts json");
    let helper = contracts["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols")
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper["ctx_path_provenance"] = json!([{
        "ctx": "irq",
        "path": ["entry", "dispatch", "helper"]
    }]);
    let contracts_path = write_temp_contracts_file("raw-mmio-irq-allowlist-json-deep", &contracts);
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-allowlist-json-deep",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "json policy mode must not write stderr: {}",
        stderr
    );
    assert_json_transport(&stdout, &stderr, "kernrift_policy_violations_v1");
    assert_eq!(
        stdout,
        concat!(
            "{\n",
            "  \"schema_version\": \"kernrift_policy_violations_v1\",\n",
            "  \"result\": \"deny\",\n",
            "  \"exit_code\": 1,\n",
            "  \"violations\": [\n",
            "    {\n",
            "      \"rule\": \"KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST\",\n",
            "      \"family\": \"effect\",\n",
            "      \"message\": \"irq raw_mmio symbol 'helper' is not allowed (via entry -> dispatch -> helper)\",\n",
            "      \"evidence\": [\n",
            "        {\n",
            "          \"kind\": \"scalar\",\n",
            "          \"key\": \"symbol\",\n",
            "          \"value\": \"helper\"\n",
            "        },\n",
            "        {\n",
            "          \"kind\": \"list\",\n",
            "          \"key\": \"irq_path\",\n",
            "          \"values\": [\n",
            "            \"entry\",\n",
            "            \"dispatch\",\n",
            "            \"helper\"\n",
            "          ]\n",
            "        }\n",
            "      ]\n",
            "    }\n",
            "  ]\n",
            "}\n"
        )
    );
    let json: Value = serde_json::from_str(&stdout).expect("policy json");
    validate_policy_violations_schema(&json);
    let evidence = json["violations"][0]["evidence"]
        .as_array()
        .expect("evidence array");
    assert_eq!(
        evidence[0]["kind"],
        json!("scalar"),
        "scalar evidence variant drifted"
    );
    assert_eq!(
        evidence[1]["kind"],
        json!("list"),
        "list evidence variant drifted"
    );

    fs::remove_file(&emitted_contracts).ok();
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_json_schema_accepts_scalar_list_and_empty_list_evidence_variants() {
    let compiled = compile_policy_violations_schema();

    let scalar_and_list = json!({
        "schema_version": "kernrift_policy_violations_v1",
        "result": "deny",
        "exit_code": 1,
        "violations": [{
            "rule": "KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST",
            "family": "effect",
            "message": "irq raw_mmio symbol 'helper' is not allowed (via entry -> helper)",
            "evidence": [
                { "kind": "scalar", "key": "symbol", "value": "helper" },
                { "kind": "list", "key": "irq_path", "values": ["entry", "helper"] }
            ]
        }]
    });
    if let Err(errors) = compiled.validate(&scalar_and_list) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "scalar/list policy evidence must validate against schema: {}",
            details.join(" | ")
        );
    }

    let empty_list = json!({
        "schema_version": "kernrift_policy_violations_v1",
        "result": "deny",
        "exit_code": 1,
        "violations": [{
            "rule": "KERNEL_RAW_MMIO_SYMBOL_ALLOWLIST",
            "family": "effect",
            "message": "raw_mmio symbol 'helper' is not allowed",
            "evidence": [
                { "kind": "list", "key": "irq_path", "values": [] }
            ]
        }]
    });
    if let Err(errors) = compiled.validate(&empty_list) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "empty-list policy evidence must validate against schema: {}",
            details.join(" | ")
        );
    }
}

#[test]
fn policy_json_rejects_invalid_contracts_without_emitting_json() {
    let root = repo_root();
    let policy_path = write_temp_policy_file("policy-json-invalid-contracts", "[kernel]\n");
    let contracts_path = unique_temp_output_path("policy-json-invalid-contracts", "json");
    fs::write(&contracts_path, "{}").expect("write malformed contracts");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stdout.is_empty(),
        "policy json invalid input must not emit stdout payload: {:?}",
        stdout
    );
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![format!(
            "failed to decode contracts bundle '{}': missing string field 'schema_version'",
            contracts_path.display()
        )]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_json_rejects_missing_policy_file_without_emitting_json() {
    let root = repo_root();
    let contracts_fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &contracts_fixture,
        "policy-json-missing-policy-file",
    );
    let missing_policy_path = unique_temp_output_path("policy-json-missing-policy-file", "toml");
    fs::remove_file(&missing_policy_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(missing_policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stdout.is_empty(),
        "policy json invalid input must not emit stdout payload: {:?}",
        stdout
    );
    assert!(
        stderr.lines().next().is_some_and(|line| line.starts_with(&format!(
            "failed to read policy '{}':",
            missing_policy_path.display()
        ))),
        "expected missing-policy read error, got: {stderr}"
    );

    fs::remove_file(&contracts_path).ok();
}

#[test]
fn check_json_policy_irq_raw_mmio_forbid_matches_policy_json_contract_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path = unique_temp_output_path("check-policy-json-direct-contracts", "json");
    let denied_contracts_path =
        unique_temp_output_path("check-policy-json-direct-denied-contracts", "json");
    let policy_path = write_temp_policy_file(
        "check-policy-json-direct",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&denied_contracts_path).ok();

    let mut emit_cmd: Command = cargo_bin_cmd!("kernriftc");
    emit_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    emit_cmd.assert().success();

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let policy_assert = policy_cmd.assert().failure().code(1);
    let policy_stdout =
        String::from_utf8(policy_assert.get_output().stdout.clone()).expect("policy stdout utf8");
    let policy_stderr =
        String::from_utf8(policy_assert.get_output().stderr.clone()).expect("policy stderr utf8");
    assert!(
        policy_stderr.is_empty(),
        "policy json mode must not write stderr: {}",
        policy_stderr
    );

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(denied_contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    let check_assert = check_cmd.assert().failure().code(1);
    let check_stdout =
        String::from_utf8(check_assert.get_output().stdout.clone()).expect("check stdout utf8");
    let check_stderr =
        String::from_utf8(check_assert.get_output().stderr.clone()).expect("check stderr utf8");
    assert!(
        check_stderr.is_empty(),
        "check json policy deny must not write stderr: {}",
        check_stderr
    );
    assert_json_transport(
        &check_stdout,
        &check_stderr,
        "kernrift_policy_violations_v1",
    );
    assert_eq!(
        check_stdout, policy_stdout,
        "check json policy deny must reuse exact policy JSON envelope"
    );
    let json: Value = serde_json::from_str(&check_stdout).expect("policy json");
    validate_policy_violations_schema(&json);
    assert!(
        !denied_contracts_path.exists(),
        "contracts output should not be written when policy denies in json mode"
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&denied_contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_json_policy_irq_raw_mmio_allowlist_helper_path_matches_policy_json_contract_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let contracts_path = unique_temp_output_path("check-policy-json-helper-contracts", "json");
    let policy_path = write_temp_policy_file(
        "check-policy-json-helper",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );
    fs::remove_file(&contracts_path).ok();

    let mut emit_cmd: Command = cargo_bin_cmd!("kernriftc");
    emit_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    emit_cmd.assert().success();

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let policy_assert = policy_cmd.assert().failure().code(1);
    let policy_stdout =
        String::from_utf8(policy_assert.get_output().stdout.clone()).expect("policy stdout utf8");
    let policy_stderr =
        String::from_utf8(policy_assert.get_output().stderr.clone()).expect("policy stderr utf8");
    assert!(
        policy_stderr.is_empty(),
        "policy json mode must not write stderr: {}",
        policy_stderr
    );

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    let check_assert = check_cmd.assert().failure().code(1);
    let check_stdout =
        String::from_utf8(check_assert.get_output().stdout.clone()).expect("check stdout utf8");
    let check_stderr =
        String::from_utf8(check_assert.get_output().stderr.clone()).expect("check stderr utf8");
    assert!(
        check_stderr.is_empty(),
        "check json policy deny must not write stderr: {}",
        check_stderr
    );
    assert_json_transport(
        &check_stdout,
        &check_stderr,
        "kernrift_policy_violations_v1",
    );
    assert_eq!(
        check_stdout, policy_stdout,
        "check json policy deny must reuse exact policy JSON envelope"
    );
    let json: Value = serde_json::from_str(&check_stdout).expect("policy json");
    validate_policy_violations_schema(&json);
    assert_eq!(
        json["violations"][0]["message"],
        json!("irq raw_mmio symbol 'helper' is not allowed (via helper)")
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_json_policy_rejects_missing_policy_file_without_emitting_json() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let missing_policy_path =
        unique_temp_output_path("check-policy-json-missing-policy-file", "toml");
    fs::remove_file(&missing_policy_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(missing_policy_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stdout.is_empty(),
        "check json policy invalid input must not emit stdout payload: {:?}",
        stdout
    );
    assert!(
        stderr.lines().next().is_some_and(|line| line.starts_with(&format!(
            "failed to read policy '{}':",
            missing_policy_path.display()
        ))),
        "expected missing-policy read error, got: {stderr}"
    );
}

#[test]
fn policy_irq_raw_mmio_symbol_allowlist_does_not_affect_structured_irq_mmio() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("irq_ctx_chain.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &fixture,
        "raw-mmio-irq-symbol-allowlist-structured-pass",
    );
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-structured-pass",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_with_policy_denies_raw_mmio_and_suppresses_contract_output() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let contracts_path = unique_temp_output_path("check-policy-raw-mmio-deny-contracts", "json");
    let policy_path = write_temp_policy_file(
        "check-policy-raw-mmio-deny",
        "[kernel]\nallow_raw_mmio = false\n",
    );
    fs::remove_file(&contracts_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec!["policy: KERNEL_RAW_MMIO_FORBID: raw_mmio is not allowed"]
    );
    assert!(
        !contracts_path.exists(),
        "contracts output should not be written when raw mmio policy denies"
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_uses_ctx_reachable_from_contract_facts() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("irq_ctx_chain.kr");
    let policy_path = root.join("policies").join("kernel.toml");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-ctx-src-{}.contracts.json", ts));
    let mutated_fail_path = std::env::temp_dir().join(format!(
        "kernrift-policy-ctx-mut-fail-{}.contracts.json",
        ts
    ));
    let mutated_pass_path = std::env::temp_dir().join(format!(
        "kernrift-policy-ctx-mut-pass-{}.contracts.json",
        ts
    ));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&mutated_fail_path).ok();
    fs::remove_file(&mutated_pass_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    let symbols = contracts_json["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols array");
    let helper = symbols
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper["eff_transitive"] = json!(["alloc"]);
    helper["eff_provenance"] = json!([{
        "effect":"alloc",
        "provenance":{
            "direct": false,
            "via_callee": [],
            "via_extern": []
        }
    }]);
    fs::write(
        &mutated_fail_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated fail contracts");

    let mut policy_fail_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_fail_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_fail_path.as_os_str());
    let fail_assert = policy_fail_cmd.assert().failure().code(1);
    let fail_stderr = String::from_utf8(fail_assert.get_output().stderr.clone()).expect("stderr");
    assert!(
        fail_stderr.contains("policy: KERNEL_IRQ_ALLOC: function 'helper'"),
        "expected helper to fail when ctx_reachable includes irq, got:\n{}",
        fail_stderr
    );

    let mut contracts_json_pass = contracts_json.clone();
    let symbols_pass = contracts_json_pass["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols array");
    let helper_pass = symbols_pass
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper_pass["ctx_reachable"] = json!([]);
    fs::write(
        &mutated_pass_path,
        serde_json::to_string(&contracts_json_pass).expect("contracts json text"),
    )
    .expect("write mutated pass contracts");

    let mut policy_pass_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_pass_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_pass_path.as_os_str());
    policy_pass_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&mutated_fail_path).ok();
    fs::remove_file(&mutated_pass_path).ok();
}

#[test]
fn policy_uses_critical_report_violations_without_reconstruction() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_region_yield.kr");
    let policy_path = root.join("policies").join("kernel.toml");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!(
        "kernrift-policy-critical-src-{}.contracts.json",
        ts
    ));
    let mutated_path = std::env::temp_dir().join(format!(
        "kernrift-policy-critical-mut-{}.contracts.json",
        ts
    ));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&mutated_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut policy_fail_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_fail_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let fail_assert = policy_fail_cmd.assert().failure().code(1);
    let fail_stderr = String::from_utf8(fail_assert.get_output().stderr.clone()).expect("stderr");
    assert!(
        fail_stderr.contains("policy: KERNEL_CRITICAL_REGION_YIELD:"),
        "expected critical region deny from emitted report facts, got:\n{}",
        fail_stderr
    );

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    contracts_json["report"]["critical"]["violations"] = json!([]);
    fs::write(
        &mutated_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated critical contracts");

    let mut policy_pass_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_pass_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_path.as_os_str());
    policy_pass_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&mutated_path).ok();
}

#[test]
fn contracts_v2_critical_report_includes_transitive_violation_details() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_region_yield.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path =
        std::env::temp_dir().join(format!("kernrift-contracts-v2-critical-report-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);
    let violations = json["report"]["critical"]["violations"]
        .as_array()
        .expect("critical violations");
    assert!(
        violations.iter().any(|v| v["function"] == "entry"
            && v["effect"] == "yield"
            && v["provenance"]["direct"] == Value::Bool(false)
            && v["provenance"]["via_callee"] == json!(["helper"])
            && v["provenance"]["via_extern"] == json!([])),
        "expected transitive critical-region yield violation in report, got {:?}",
        violations
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_facts_attrs_include_critical_flag() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_attr.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!("kernrift-contracts-v2-critical-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let text = fs::read_to_string(&out_path).expect("read contracts output");
    let json: Value = serde_json::from_str(&text).expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let critical_symbol = symbols
        .iter()
        .find(|sym| sym["name"] == "critical_entry")
        .expect("critical_entry symbol must exist");
    assert_eq!(critical_symbol["attrs"]["critical"], Value::Bool(true));

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_effect_counters_track_alloc_and_block_sites() {
    let root = repo_root();
    let alloc_fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_alloc_site.kr");
    let block_fixture = root
        .join("tests")
        .join("must_pass")
        .join("blockpoint_thread.kr");

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let alloc_out = std::env::temp_dir().join(format!("kernrift-contracts-v2-alloc-{}.json", ts));
    let block_out = std::env::temp_dir().join(format!("kernrift-contracts-v2-block-{}.json", ts));
    fs::remove_file(&alloc_out).ok();
    fs::remove_file(&block_out).ok();

    let mut alloc_cmd: Command = cargo_bin_cmd!("kernriftc");
    alloc_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(alloc_out.as_os_str())
        .arg(alloc_fixture.as_os_str());
    alloc_cmd.assert().success();

    let mut block_cmd: Command = cargo_bin_cmd!("kernriftc");
    block_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(block_out.as_os_str())
        .arg(block_fixture.as_os_str());
    block_cmd.assert().success();

    let alloc_json: Value =
        serde_json::from_str(&fs::read_to_string(&alloc_out).expect("alloc contracts"))
            .expect("alloc json");
    let block_json: Value =
        serde_json::from_str(&fs::read_to_string(&block_out).expect("block contracts"))
            .expect("block json");
    validate_contracts_schema_v2(&alloc_json);
    validate_contracts_schema_v2(&block_json);

    assert!(
        alloc_json["report"]["effects"]["alloc_sites_count"]
            .as_u64()
            .expect("alloc count")
            >= 1
    );
    assert!(
        block_json["report"]["effects"]["block_sites_count"]
            .as_u64()
            .expect("block count")
            >= 1
    );

    fs::remove_file(&alloc_out).ok();
    fs::remove_file(&block_out).ok();
}

#[test]
fn contracts_v2_facts_include_transitive_effects() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("transitive_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path =
        std::env::temp_dir().join(format!("kernrift-contracts-v2-eff-transitive-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let helper = symbols
        .iter()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    let entry = symbols
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("entry symbol");

    assert_eq!(
        helper["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        helper["eff_provenance"],
        json!([{
            "effect": "alloc",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(
        entry["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        entry["eff_provenance"],
        json!([{
            "effect": "alloc",
            "provenance": {
                "direct": false,
                "via_callee": ["helper"],
                "via_extern": []
            }
        }])
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_facts_include_transitive_capabilities() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("transitive_caps.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path =
        std::env::temp_dir().join(format!("kernrift-contracts-v2-cap-transitive-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let helper = symbols
        .iter()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    let isr = symbols
        .iter()
        .find(|sym| sym["name"] == "isr")
        .expect("isr symbol");

    assert_eq!(
        helper["caps_transitive"],
        Value::Array(vec![Value::String("PhysMap".to_string())])
    );
    assert_eq!(
        helper["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(
        isr["caps_transitive"],
        Value::Array(vec![Value::String("PhysMap".to_string())])
    );
    assert_eq!(
        isr["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": true,
                "via_callee": ["helper"],
                "via_extern": []
            }
        }])
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_transitive_capabilities_include_extern_stubs() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("transitive_caps_extern.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!(
        "kernrift-contracts-v2-cap-extern-transitive-{}.json",
        ts
    ));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let map_io = symbols
        .iter()
        .find(|sym| sym["name"] == "map_io")
        .expect("map_io symbol");
    let helper = symbols
        .iter()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    let isr = symbols
        .iter()
        .find(|sym| sym["name"] == "isr")
        .expect("isr symbol");

    assert_eq!(map_io["is_extern"], Value::Bool(true));
    assert_eq!(
        map_io["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(
        helper["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": ["map_io"]
            }
        }])
    );
    assert_eq!(
        isr["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": true,
                "via_callee": ["helper"],
                "via_extern": ["map_io"]
            }
        }])
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_transitive_effects_include_eff_attr_annotations() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("transitive_alloc_attr.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!(
        "kernrift-contracts-v2-eff-attr-transitive-{}.json",
        ts
    ));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let helper = symbols
        .iter()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    let entry = symbols
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("entry symbol");

    assert_eq!(
        helper["eff_used"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        helper["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        entry["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_transitive_effects_include_extern_eff_stubs() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("transitive_alloc_extern.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!(
        "kernrift-contracts-v2-eff-extern-transitive-{}.json",
        ts
    ));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let kmalloc = symbols
        .iter()
        .find(|sym| sym["name"] == "kmalloc")
        .expect("kmalloc symbol");
    let entry = symbols
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("entry symbol");

    assert_eq!(kmalloc["is_extern"], Value::Bool(true));
    assert_eq!(
        kmalloc["eff_used"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        kmalloc["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        kmalloc["eff_provenance"],
        json!([{
            "effect": "alloc",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(
        entry["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        entry["eff_provenance"],
        json!([{
            "effect": "alloc",
            "provenance": {
                "direct": false,
                "via_callee": [],
                "via_extern": ["kmalloc"]
            }
        }])
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_facts_include_ctx_reachable_transitive_irq() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("irq_ctx_chain.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path =
        std::env::temp_dir().join(format!("kernrift-contracts-v2-ctx-reach-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);
    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let helper = symbols
        .iter()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    assert!(
        helper["ctx_reachable"]
            .as_array()
            .expect("ctx_reachable array")
            .iter()
            .any(|ctx| ctx == "irq"),
        "expected helper ctx_reachable to include irq"
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_ctx_reachable_transitive_irq_drives_policy() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_alloc_transitive.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_IRQ_ALLOC: function 'helper'"),
        "expected transitive KERNEL_IRQ_ALLOC for helper, got:\n{}",
        stderr
    );
}

#[test]
fn policy_kernel_forbid_yield_in_irq_is_artifact_driven_and_deterministic() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-yield-irq-src-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-yield-irq-{}.toml", ts));
    let mutated_path =
        std::env::temp_dir().join(format!("kernrift-policy-yield-irq-mut-{}.json", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&mutated_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_yield_in_irq = true
"#,
    )
    .expect("write policy");

    let mut pass_cmd: Command = cargo_bin_cmd!("kernriftc");
    pass_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    pass_cmd.assert().success();

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    let symbols = contracts_json["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols array");
    assert!(
        symbols.len() >= 2,
        "expected at least two symbols in contracts"
    );
    symbols[0]["name"] = json!("helper");
    symbols[0]["ctx_reachable"] = json!(["irq"]);
    symbols[0]["eff_transitive"] = json!(["yield"]);
    symbols[0]["eff_provenance"] = json!([{
        "effect": "yield",
        "provenance": {
            "direct": true,
            "via_callee": [],
            "via_extern": []
        }
    }]);
    symbols[1]["name"] = json!("isr");
    symbols[1]["ctx_reachable"] = json!(["irq"]);
    symbols[1]["eff_transitive"] = json!(["yield"]);
    symbols[1]["eff_provenance"] = json!([{
        "effect": "yield",
        "provenance": {
            "direct": false,
            "via_callee": ["helper"],
            "via_extern": []
        }
    }]);
    fs::write(
        &mutated_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    let mut fail_cmd: Command = cargo_bin_cmd!("kernriftc");
    fail_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_path.as_os_str());
    let fail_assert = fail_cmd.assert().failure().code(1);
    let fail_stderr = String::from_utf8(fail_assert.get_output().stderr.clone()).expect("stderr");
    let lines = fail_stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_YIELD: function 'helper' is irq-reachable and uses yield effect (direct=true, via_callee=[], via_extern=[])",
            "policy: KERNEL_IRQ_YIELD: function 'isr' is irq-reachable and uses yield effect (direct=false, via_callee=[helper], via_extern=[])",
        ],
        "expected deterministic irq yield violations, got:\n{}",
        fail_stderr
    );

    for symbol in contracts_json["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols array")
    {
        symbol["eff_transitive"] = json!([]);
        symbol["eff_provenance"] = json!([]);
    }
    fs::write(
        &mutated_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    let mut clear_pass_cmd: Command = cargo_bin_cmd!("kernriftc");
    clear_pass_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_path.as_os_str());
    clear_pass_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&mutated_path).ok();
}

#[test]
fn contracts_v2_semantic_fields_coexist_and_validate_schema() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!("kernrift-contracts-v2-abi-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let entry = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols")
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("entry symbol");
    assert_eq!(entry["ctx_reachable"], json!(["irq"]));
    assert_eq!(
        entry["ctx_provenance"],
        json!([{
            "ctx": "irq",
            "sources": ["entry"]
        }])
    );
    assert_eq!(
        entry["ctx_path_provenance"],
        json!([{
            "ctx": "irq",
            "path": ["entry"]
        }])
    );
    assert_eq!(entry["eff_transitive"], json!(["alloc"]));
    assert_eq!(
        entry["eff_provenance"],
        json!([{
            "effect": "alloc",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(entry["caps_transitive"], json!(["PhysMap"]));
    assert_eq!(
        entry["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(json["report"]["critical"]["violations"], json!([]));

    fs::remove_file(&out_path).ok();
}


#[test]
fn policy_passes_for_compliant_contracts() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-pass-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-pass-{}.toml", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 2

[locks]
forbid_edges = [["RunQueueLock", "SchedLock"]]

[caps]
allow_module = []
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    policy_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_fails_with_deterministic_ordered_errors() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-fail-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-fail-{}.toml", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1",
            "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_caps_allow_module_rejects_disallowed_caps() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-caps-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-caps-{}.toml", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[caps]
allow_module = ["IoPort"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains(
            "policy: CAP_MODULE_ALLOWLIST: module capability 'PhysMap' is not in allow_module"
        ),
        "expected caps allowlist violation, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_kernel_forbid_caps_in_irq_is_artifact_driven_and_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_transitive.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-irq-src-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-cap-irq-{}.toml", ts));
    let mutated_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-irq-mut-{}.json", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&mutated_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut fail_cmd: Command = cargo_bin_cmd!("kernriftc");
    fail_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let fail_assert = fail_cmd.assert().failure().code(1);
    let fail_stderr = String::from_utf8(fail_assert.get_output().stderr.clone()).expect("stderr");
    let lines = fail_stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_CAP_FORBID: function 'helper' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[])",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'isr' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[helper], via_extern=[])",
        ],
        "expected deterministic capability violations, got:\n{}",
        fail_stderr
    );

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    let symbols = contracts_json["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols array");
    for symbol in symbols {
        symbol["caps_transitive"] = json!([]);
        symbol["caps_provenance"] = json!([]);
    }
    fs::write(
        &mutated_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    let mut pass_cmd: Command = cargo_bin_cmd!("kernriftc");
    pass_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_path.as_os_str());
    pass_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&mutated_path).ok();
}

#[test]
fn policy_kernel_allow_caps_in_irq_overrides_forbid() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_transitive.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-allow-override-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-allow-override-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
allow_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    policy_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_kernel_non_listed_irq_capability_is_allowed() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_unlisted.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-unlisted-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-unlisted-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
allow_caps_in_irq = []
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    policy_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_kernel_forbid_caps_in_irq_via_extern_is_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_extern.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-extern-src-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-cap-extern-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_CAP_FORBID: function 'helper' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[map_io])",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'isr' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[helper], via_extern=[map_io])",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'map_io' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[])",
        ],
        "expected deterministic extern capability propagation violations, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_without_evidence_keeps_output_exactly_unchanged() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-no-evidence-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-no-evidence-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_alloc_in_irq = true
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_ALLOC: function 'entry' is irq-reachable and uses alloc effect (direct=true, via_callee=[], via_extern=[])"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_irq_effect_is_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-irq-effect-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-irq-effect-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_alloc_in_irq = true
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_ALLOC: function 'entry' is irq-reachable and uses alloc effect (direct=true, via_callee=[], via_extern=[])",
            "evidence: symbol=entry",
            "evidence: effect=alloc",
            "evidence: direct=true",
            "evidence: via_callee=[]",
            "evidence: via_extern=[]",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_irq_capability_is_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_transitive.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-irq-cap-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-irq-cap-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_CAP_FORBID: function 'helper' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[])",
            "evidence: symbol=helper",
            "evidence: capability=PhysMap",
            "evidence: direct=true",
            "evidence: via_callee=[]",
            "evidence: via_extern=[]",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'isr' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[helper], via_extern=[])",
            "evidence: symbol=isr",
            "evidence: capability=PhysMap",
            "evidence: direct=true",
            "evidence: via_callee=[helper]",
            "evidence: via_extern=[]",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_critical_region_is_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-critical-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-critical-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    inject_single_critical_violation(&mut contracts_json, "entry", "alloc", true, &[], &[]);
    fs::write(
        &contracts_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_effects_in_critical = ["alloc"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_CRITICAL_REGION_ALLOC: function 'entry' uses alloc effect in critical region (direct=true, via_callee=[], via_extern=[])",
            "evidence: function=entry",
            "evidence: effect=alloc",
            "evidence: direct=true",
            "evidence: via_callee=[]",
            "evidence: via_extern=[]",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_blocks_follow_deterministic_violation_order() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-order-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-order-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    inject_single_critical_violation(&mut contracts_json, "entry", "alloc", true, &[], &[]);
    fs::write(
        &contracts_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]

[kernel]
forbid_alloc_in_irq = true
forbid_effects_in_critical = ["alloc"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_CRITICAL_REGION_ALLOC: function 'entry' uses alloc effect in critical region (direct=true, via_callee=[], via_extern=[])",
            "evidence: function=entry",
            "evidence: effect=alloc",
            "evidence: direct=true",
            "evidence: via_callee=[]",
            "evidence: via_extern=[]",
            "policy: KERNEL_IRQ_ALLOC: function 'entry' is irq-reachable and uses alloc effect (direct=true, via_callee=[], via_extern=[])",
            "evidence: symbol=entry",
            "evidence: effect=alloc",
            "evidence: direct=true",
            "evidence: via_callee=[]",
            "evidence: via_extern=[]",
            "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1",
            "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_kernel_capability_rule_requires_contracts_v2() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-v1-cap-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-v1-cap-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
allow_caps_in_irq = ["IoPort"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_POLICY_REQUIRES_V2: kernel policy rules require contracts schema 'kernrift_contracts_v2'"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_kernel_irq_yield_rule_requires_contracts_v2() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-v1-irq-yield-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-v1-irq-yield-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_yield_in_irq = true
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_POLICY_REQUIRES_V2: kernel policy rules require contracts schema 'kernrift_contracts_v2'"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_outputs_cross_family_violations_in_deterministic_order() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-families-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-families-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    inject_single_critical_violation(&mut contracts_json, "entry", "alloc", true, &[], &[]);
    fs::write(
        &contracts_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]

[kernel]
forbid_alloc_in_irq = true
forbid_effects_in_critical = ["alloc"]
forbid_caps_in_irq = ["IoPort", "PhysMap"]
allow_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_CRITICAL_REGION_ALLOC: function 'entry' uses alloc effect in critical region (direct=true, via_callee=[], via_extern=[])",
            "policy: KERNEL_IRQ_ALLOC: function 'entry' is irq-reachable and uses alloc effect (direct=true, via_callee=[], via_extern=[])",
            "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1",
            "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present",
        ],
        "cross-family ordering must be deterministic, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_catalog_rank_order_is_deterministic_across_families() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-rank-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-rank-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    inject_single_critical_violation(&mut contracts_json, "entry", "alloc", true, &[], &[]);
    fs::write(
        &contracts_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]

[caps]
allow_module = ["IoPort"]

[kernel]
forbid_alloc_in_irq = true
forbid_effects_in_critical = ["alloc"]
forbid_caps_in_irq = ["PhysMap"]
allow_caps_in_irq = []
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: CAP_MODULE_ALLOWLIST: module capability 'PhysMap' is not in allow_module",
            "policy: KERNEL_CRITICAL_REGION_ALLOC: function 'entry' uses alloc effect in critical region (direct=true, via_callee=[], via_extern=[])",
            "policy: KERNEL_IRQ_ALLOC: function 'entry' is irq-reachable and uses alloc effect (direct=true, via_callee=[], via_extern=[])",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'entry' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[])",
            "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1",
            "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present",
        ],
        "catalog rank ordering must be deterministic, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_bad_parse_exits_with_invalid_input_code() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-bad-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-bad-{}.toml", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(&policy_path, "[limits\nmax_lock_depth = 1\n").expect("write bad policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    policy_cmd.assert().failure().code(2);

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allow_caps_in_irq_empty_entry_exits_with_invalid_input_code() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-allow-cap-empty-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-allow-cap-empty-{}.toml", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
allow_caps_in_irq = [""]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("allow_caps_in_irq entries must be non-empty strings"),
        "expected allow_caps_in_irq parse validation error, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_with_policy_pass_writes_contracts_out() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-check-policy-pass-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-check-policy-pass-{}.toml", ts));

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 2

[locks]
forbid_edges = [["RunQueueLock", "SchedLock"]]
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let text = fs::read_to_string(&contracts_path).expect("read contracts output");
    assert!(
        !text.is_empty(),
        "contracts output should be written on pass"
    );
    let json: Value = serde_json::from_str(&text).expect("contracts json");
    validate_contracts_schema(&json);

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_with_policy_fail_does_not_write_file() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-check-policy-fail-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-check-policy-fail-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().failure().code(1);

    assert!(
        !contracts_path.exists(),
        "contracts output should not be written when policy denies"
    );

    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_with_policy_fail_has_deterministic_lines() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path = std::env::temp_dir().join(format!("kernrift-check-policy-lines-{}.toml", ts));

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1",
            "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present",
        ]
    );

    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_pass_writes_contracts_hash_sig_and_verifies() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-p84-contracts-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-p84-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-p84-sig-{}.sig", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-p84-policy-{}.toml", ts));
    let secret_path = std::env::temp_dir().join(format!("kernrift-p84-secret-{}.hex", ts));
    let pubkey_path = std::env::temp_dir().join(format!("kernrift-p84-pubkey-{}.hex", ts));

    write_test_keypair(&secret_path, &pubkey_path);
    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 2
"#,
    )
    .expect("write policy");

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let contracts_bytes = fs::read(&contracts_path).expect("read contracts");
    let expected_hash = sha256_hex(&contracts_bytes);
    let got_hash = fs::read_to_string(&hash_path).expect("read hash");
    assert_eq!(
        got_hash.trim(),
        expected_hash,
        "hash file must match contracts bytes"
    );
    let sig_text = fs::read_to_string(&sig_path).expect("read sig");
    assert!(!sig_text.trim().is_empty(), "sig file should not be empty");

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--sig")
        .arg(sig_path.as_os_str())
        .arg("--pubkey")
        .arg(pubkey_path.as_os_str());
    verify_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}

#[test]
fn check_policy_deny_writes_nothing_even_if_hash_sig_flags_present() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-p84-deny-contracts-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-p84-deny-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-p84-deny-sig-{}.sig", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-p84-deny-policy-{}.toml", ts));
    let secret_path = std::env::temp_dir().join(format!("kernrift-p84-deny-secret-{}.hex", ts));
    let pubkey_path = std::env::temp_dir().join(format!("kernrift-p84-deny-pubkey-{}.hex", ts));
    write_test_keypair(&secret_path, &pubkey_path);

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1
"#,
    )
    .expect("write policy");

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().failure().code(1);

    assert!(
        !contracts_path.exists(),
        "contracts file must not be written on deny"
    );
    assert!(!hash_path.exists(), "hash file must not be written on deny");
    assert!(!sig_path.exists(), "sig file must not be written on deny");

    fs::remove_file(&policy_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}

#[test]
fn check_invalid_key_exits_2_and_writes_nothing() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-p84-badkey-contracts-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-p84-badkey-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-p84-badkey-sig-{}.sig", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-p84-badkey-policy-{}.toml", ts));
    let bad_secret_path =
        std::env::temp_dir().join(format!("kernrift-p84-badkey-secret-{}.hex", ts));

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 2
"#,
    )
    .expect("write policy");
    fs::write(&bad_secret_path, "zz\n").expect("write bad key");

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(bad_secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().failure().code(2);

    assert!(
        !contracts_path.exists(),
        "contracts file must not be written on invalid key"
    );
    assert!(
        !hash_path.exists(),
        "hash file must not be written on invalid key"
    );
    assert!(
        !sig_path.exists(),
        "sig file must not be written on invalid key"
    );

    fs::remove_file(&policy_path).ok();
    fs::remove_file(&bad_secret_path).ok();
}

#[test]
fn verify_rejects_mismatched_hash_or_sig() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-p84-vrf-contracts-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-p84-vrf-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-p84-vrf-sig-{}.sig", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-p84-vrf-policy-{}.toml", ts));
    let secret_path = std::env::temp_dir().join(format!("kernrift-p84-vrf-secret-{}.hex", ts));
    let pubkey_path = std::env::temp_dir().join(format!("kernrift-p84-vrf-pubkey-{}.hex", ts));

    write_test_keypair(&secret_path, &pubkey_path);
    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 2
"#,
    )
    .expect("write policy");

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &hash_path,
        "0000000000000000000000000000000000000000000000000000000000000000\n",
    )
    .expect("write tampered hash");
    let mut verify_hash_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_hash_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str());
    verify_hash_cmd.assert().failure().code(1);

    let contracts_bytes = fs::read(&contracts_path).expect("read contracts");
    fs::write(&hash_path, format!("{}\n", sha256_hex(&contracts_bytes))).expect("restore hash");
    let bad_sig = BASE64_STANDARD.encode([0_u8; 64]);
    fs::write(&sig_path, format!("{bad_sig}\n")).expect("tamper sig");
    let mut verify_sig_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_sig_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--sig")
        .arg(sig_path.as_os_str())
        .arg("--pubkey")
        .arg(pubkey_path.as_os_str());
    verify_sig_cmd.assert().failure().code(1);

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}

#[test]
fn check_refuses_overwriting_existing_outputs() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-p84-overwrite-{}.json", ts));

    fs::write(&contracts_path, "sentinel\n").expect("write sentinel");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().failure().code(2);

    let after = fs::read_to_string(&contracts_path).expect("read sentinel");
    assert_eq!(after, "sentinel\n", "existing output must remain untouched");

    fs::remove_file(&contracts_path).ok();
}

#[test]
fn verify_rejects_schema_invalid_even_with_matching_hash() {
    let root = repo_root();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-p84-schema-invalid-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-p84-schema-invalid-{}.sha256", ts));

    let garbage = b"{\"not_contracts\":true}";
    fs::write(&contracts_path, garbage).expect("write garbage contracts");
    fs::write(&hash_path, format!("{}\n", sha256_hex(garbage))).expect("write matching hash");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str());
    cmd.assert().failure().code(2);

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
}

#[test]
fn verify_rejects_contracts_with_unknown_top_level_key() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-p84-unknown-contract-key-{}.json", ts));
    let hash_path =
        std::env::temp_dir().join(format!("kernrift-p84-unknown-contract-key-{}.sha256", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut contracts: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("read contracts"))
            .expect("contracts json");
    contracts
        .as_object_mut()
        .expect("contracts object")
        .insert("unexpected".to_string(), Value::Bool(true));
    let tampered = serde_json::to_string(&contracts).expect("serialize tampered contracts");
    fs::write(&contracts_path, tampered.as_bytes()).expect("write tampered contracts");
    fs::write(&hash_path, format!("{}\n", sha256_hex(tampered.as_bytes())))
        .expect("write matching tampered hash");

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str());
    verify_cmd
        .assert()
        .failure()
        .code(2)
        .stderr(contains("contracts schema validation failed"));

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
}

#[test]
fn verify_report_schema_rejects_unknown_keys_and_invalid_result() {
    let compiled = compile_verify_report_schema();
    let mut valid = json!({
        "schema_version": "kernrift_verify_report_v1",
        "result": "pass",
        "inputs": {
            "contracts": "contracts.json",
            "hash": "contracts.sha256",
            "sig": null,
            "pubkey": null
        },
        "hash": {
            "expected_sha256": "0".repeat(64),
            "computed_sha256": "0".repeat(64),
            "matched": true
        },
        "contracts": {
            "utf8_valid": true,
            "schema_valid": true,
            "schema_version": "kernrift_contracts_v1"
        },
        "signature": {
            "checked": false,
            "valid": null
        },
        "diagnostics": []
    });
    if let Err(errors) = compiled.validate(&valid) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "expected valid verify report schema instance, got: {}",
            details.join(" | ")
        );
    }

    valid
        .as_object_mut()
        .expect("report object")
        .insert("unexpected".to_string(), Value::Bool(true));
    assert_schema_rejects(&compiled, &valid, "Additional properties are not allowed");

    let mut invalid_nested = json!({
        "schema_version": "kernrift_verify_report_v1",
        "result": "pass",
        "inputs": {
            "contracts": "contracts.json",
            "hash": "contracts.sha256",
            "sig": null,
            "pubkey": null,
            "extra": "nope"
        },
        "hash": {
            "expected_sha256": "0".repeat(64),
            "computed_sha256": "0".repeat(64),
            "matched": true
        },
        "contracts": {
            "utf8_valid": true,
            "schema_valid": true,
            "schema_version": "kernrift_contracts_v1"
        },
        "signature": {
            "checked": false,
            "valid": null
        },
        "diagnostics": []
    });
    assert_schema_rejects(
        &compiled,
        &invalid_nested,
        "Additional properties are not allowed",
    );

    invalid_nested["inputs"] = json!({
        "contracts": "contracts.json",
        "hash": "contracts.sha256",
        "sig": null,
        "pubkey": null
    });
    invalid_nested["result"] = Value::String("maybe".to_string());
    assert_schema_rejects(&compiled, &invalid_nested, "\"maybe\" is not one of");
}

#[test]
fn policy_rejects_unbounded_no_yield_spans() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("thread_no_yield_unbounded.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-no-yield-{}.toml", ts));
    fs::write(
        &policy_path,
        r#"
[limits]
forbid_unbounded_no_yield = true
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: NO_YIELD_UNBOUNDED: no_yield_spans 'helper' is unbounded",
            "policy: NO_YIELD_UNBOUNDED: no_yield_spans 'worker' is unbounded",
        ]
    );

    fs::remove_file(&policy_path).ok();
}

#[test]
fn verify_report_success_is_deterministic_and_path_stripped() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-vrf-report-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-vrf-report-{}.sha256", ts));
    let report_path = std::env::temp_dir().join(format!("kernrift-vrf-report-{}.report.json", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().success();

    let report_text = fs::read_to_string(&report_path).expect("read verify report");
    let report_json: Value = serde_json::from_str(&report_text).expect("verify report json");
    let keys = report_json
        .as_object()
        .expect("verify report object")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from([
            "contracts".to_string(),
            "diagnostics".to_string(),
            "hash".to_string(),
            "inputs".to_string(),
            "result".to_string(),
            "schema_version".to_string(),
            "signature".to_string(),
        ])
    );
    assert_eq!(
        report_json["schema_version"],
        Value::String("kernrift_verify_report_v1".to_string())
    );
    assert_eq!(report_json["result"], Value::String("pass".to_string()));
    assert_eq!(report_json["hash"]["matched"], Value::Bool(true));
    assert_eq!(
        report_json["diagnostics"],
        Value::Array(vec![]),
        "verify report diagnostics should be empty on success"
    );

    let contracts_name = contracts_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("contracts basename");
    let hash_name = hash_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("hash basename");
    let report_contracts_path = report_json["inputs"]["contracts"]
        .as_str()
        .expect("report contracts path");
    let report_hash_path = report_json["inputs"]["hash"]
        .as_str()
        .expect("report hash path");
    assert_eq!(report_contracts_path, contracts_name);
    assert_eq!(report_hash_path, hash_name);
    assert!(
        !report_contracts_path.contains('/'),
        "verify report should strip absolute paths"
    );
    assert!(
        !report_hash_path.contains('/'),
        "verify report should strip absolute paths"
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&report_path).ok();
}

#[test]
fn verify_report_records_hash_mismatch_deterministically() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-vrf-deny-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-vrf-deny-{}.sha256", ts));
    let report_path = std::env::temp_dir().join(format!("kernrift-vrf-deny-{}.report.json", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &hash_path,
        "0000000000000000000000000000000000000000000000000000000000000000\n",
    )
    .expect("tamper hash");

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().failure().code(1);

    let report_text = fs::read_to_string(&report_path).expect("read verify report");
    let report_json: Value = serde_json::from_str(&report_text).expect("verify report json");
    assert_eq!(report_json["result"], Value::String("deny".to_string()));
    let diagnostics = report_json["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|v| v.as_str().expect("diag string").to_string())
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics[0].starts_with("verify: HASH_MISMATCH:"),
        "unexpected diagnostics: {:?}",
        diagnostics
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&report_path).ok();
}

#[test]
fn verify_report_signature_pass_is_stable_and_path_stripped() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-{}.sig", ts));
    let report_path =
        std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-{}.report.json", ts));
    let secret_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-secret-{}.hex", ts));
    let pubkey_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-pubkey-{}.hex", ts));

    write_test_keypair(&secret_path, &pubkey_path);

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--sig")
        .arg(sig_path.as_os_str())
        .arg("--pubkey")
        .arg(pubkey_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().success();

    let report_text = fs::read_to_string(&report_path).expect("read verify report");
    let report_json: Value = serde_json::from_str(&report_text).expect("verify report json");

    assert_eq!(report_json["result"], Value::String("pass".to_string()));
    assert_eq!(report_json["signature"]["checked"], Value::Bool(true));
    assert_eq!(report_json["signature"]["valid"], Value::Bool(true));
    assert_eq!(
        report_json["diagnostics"],
        Value::Array(vec![]),
        "verify report diagnostics should be empty on signature pass"
    );

    let sig_name = sig_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("sig basename");
    let pubkey_name = pubkey_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("pubkey basename");
    let report_sig_path = report_json["inputs"]["sig"]
        .as_str()
        .expect("report sig path");
    let report_pubkey_path = report_json["inputs"]["pubkey"]
        .as_str()
        .expect("report pubkey path");
    assert_eq!(report_sig_path, sig_name);
    assert_eq!(report_pubkey_path, pubkey_name);
    assert!(
        !report_sig_path.contains('/'),
        "verify report should strip absolute signature path"
    );
    assert!(
        !report_pubkey_path.contains('/'),
        "verify report should strip absolute pubkey path"
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();
    fs::remove_file(&report_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}

#[test]
fn verify_report_signature_mismatch_records_deny_deterministically() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-contracts-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-sig-{}.sig", ts));
    let report_path =
        std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-report-{}.json", ts));
    let secret_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-secret-{}.hex", ts));
    let pubkey_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-pubkey-{}.hex", ts));

    write_test_keypair(&secret_path, &pubkey_path);

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &sig_path,
        format!("{}\n", BASE64_STANDARD.encode([0_u8; 64])),
    )
    .expect("tamper signature");

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--sig")
        .arg(sig_path.as_os_str())
        .arg("--pubkey")
        .arg(pubkey_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().failure().code(1);

    let report_text = fs::read_to_string(&report_path).expect("read verify report");
    let report_json: Value = serde_json::from_str(&report_text).expect("verify report json");
    assert_eq!(report_json["result"], Value::String("deny".to_string()));
    assert_eq!(report_json["hash"]["matched"], Value::Bool(true));
    assert_eq!(report_json["signature"]["checked"], Value::Bool(true));
    assert_eq!(report_json["signature"]["valid"], Value::Bool(false));

    let diagnostics = report_json["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|v| v.as_str().expect("diag string").to_string())
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics[0].starts_with("verify: SIG_MISMATCH:"),
        "unexpected diagnostics: {:?}",
        diagnostics
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();
    fs::remove_file(&report_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}

#[test]
fn verify_report_invalid_input_normalizes_diagnostic_paths() {
    let root = repo_root();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-vrf-missing-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-vrf-missing-{}.sha256", ts));
    let report_path = std::env::temp_dir().join(format!("kernrift-vrf-missing-{}.report.json", ts));

    fs::remove_file(&contracts_path).ok();
    fs::write(
        &hash_path,
        "0000000000000000000000000000000000000000000000000000000000000000\n",
    )
    .expect("write hash");

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().failure().code(2);

    let report_text = fs::read_to_string(&report_path).expect("read verify report");
    let report_json: Value = serde_json::from_str(&report_text).expect("verify report json");
    assert_eq!(
        report_json["result"],
        Value::String("invalid_input".to_string())
    );
    assert_eq!(report_json["signature"]["checked"], Value::Bool(false));
    assert_eq!(report_json["signature"]["valid"], Value::Null);
    assert_eq!(report_json["hash"]["matched"], Value::Bool(false));

    let contracts_name = contracts_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("contracts basename");
    let report_contracts_path = report_json["inputs"]["contracts"]
        .as_str()
        .expect("report contracts path");
    assert_eq!(report_contracts_path, contracts_name);

    let diagnostics = report_json["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|v| v.as_str().expect("diag string").to_string())
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics[0].starts_with(&format!("failed to read contracts '{}':", contracts_name)),
        "unexpected diagnostics: {:?}",
        diagnostics
    );
    assert!(
        !diagnostics[0].contains('/'),
        "diagnostic path should be normalized to basename"
    );

    fs::remove_file(&hash_path).ok();
    fs::remove_file(&report_path).ok();
}

#[test]
fn inspect_report_generated_signature_pass_summary_is_exact() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-contracts-{}.json", ts));
    let hash_path =
        std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-sig-{}.sig", ts));
    let report_path =
        std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-report-{}.json", ts));
    let secret_path =
        std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-secret-{}.hex", ts));
    let pubkey_path =
        std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-pubkey-{}.hex", ts));

    write_test_keypair(&secret_path, &pubkey_path);

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--sig")
        .arg(sig_path.as_os_str())
        .arg("--pubkey")
        .arg(pubkey_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().success();

    let mut inspect_cmd: Command = cargo_bin_cmd!("kernriftc");
    inspect_cmd
        .current_dir(&root)
        .arg("inspect-report")
        .arg("--report")
        .arg(report_path.as_os_str());
    let assert = inspect_cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let lines = stdout.lines().collect::<Vec<_>>();

    let sig_name = sig_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("sig basename");
    let pubkey_name = pubkey_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("pubkey basename");

    assert_eq!(lines[0], "schema: kernrift_verify_report_v1");
    assert_eq!(lines[1], "result: pass");
    assert!(lines.contains(&format!("sig: {}", sig_name).as_str()));
    assert!(lines.contains(&format!("pubkey: {}", pubkey_name).as_str()));
    assert!(lines.contains(&"checked: true"));
    assert!(lines.contains(&"valid: true"));
    assert!(lines.contains(&"diagnostics: 0"));

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();
    fs::remove_file(&report_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}
