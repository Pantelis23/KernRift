# KernRift Examples

Run all commands from the repo root.

## 01 `01_happy_check.kr` (happy path check)

```bash
cargo run -q -p kernriftc -- check examples/01_happy_check.kr
```

Expected: exit `0`.

## 02 `02_parse_error_missing_semicolon.kr` (parse error)

```bash
cargo run -q -p kernriftc -- check examples/02_parse_error_missing_semicolon.kr
```

Expected: exit `1` with parser error (`expected ';' terminating statement ...`).

## 03 `03_semantic_unresolved_symbol.kr` (semantic/HIR error)

```bash
cargo run -q -p kernriftc -- check examples/03_semantic_unresolved_symbol.kr
```

Expected: exit `1` with undefined symbol diagnostic.

## 04 `04_verify_policy_pass.kr` (policy pass)

```bash
cargo run -q -p kernriftc -- \
  check --policy examples/policy/pass.toml examples/04_verify_policy_pass.kr
```

Expected: exit `0`.

## 05 `05_verify_fail_max_lock_depth.kr` (policy fail: max lock depth)

```bash
TMP="$(mktemp -d)"
cargo run -q -p kernriftc -- \
  check --contracts-out "$TMP/contracts.json" examples/05_verify_fail_max_lock_depth.kr
cargo run -q -p kernriftc -- \
  policy --policy examples/policy/fail_max_depth.toml --contracts "$TMP/contracts.json"
```

Expected: second command exits `1` with `policy: LIMIT_MAX_LOCK_DEPTH`.

## 06 `06_verify_fail_no_yield_spans.kr` (policy fail: no-yield spans)

```bash
TMP="$(mktemp -d)"
cargo run -q -p kernriftc -- \
  check --contracts-out "$TMP/contracts.json" examples/06_verify_fail_no_yield_spans.kr
cargo run -q -p kernriftc -- \
  policy --policy examples/policy/fail_no_yield.toml --contracts "$TMP/contracts.json"
```

Expected: second command exits `1` with `policy: NO_YIELD_UNBOUNDED`.

## 07 `07_contracts_schema_version_invalid.kr` (schema/version invalid)

```bash
TMP="$(mktemp -d)"
cargo run -q -p kernriftc -- \
  check --contracts-out "$TMP/contracts.json" --hash-out "$TMP/contracts.sha256" examples/07_contracts_schema_version_invalid.kr
sed -i 's/kernrift_contracts_v1/kernrift_contracts_v999/' "$TMP/contracts.json"
sha256sum "$TMP/contracts.json" | awk '{print $1}' > "$TMP/contracts.sha256"
cargo run -q -p kernriftc -- \
  verify --contracts "$TMP/contracts.json" --hash "$TMP/contracts.sha256"
```

Expected: verify exits `2` (schema/version validation failure).

## 08 `08_contracts_utf8_invalid.kr` (contracts UTF-8 requirement)

```bash
TMP="$(mktemp -d)"
# create non-UTF-8 payload directly
printf '\xff\xfe\xfd' > "$TMP/contracts.bin"
sha256sum "$TMP/contracts.bin" | awk '{print $1}' > "$TMP/contracts.sha256"
cargo run -q -p kernriftc -- \
  verify --contracts "$TMP/contracts.bin" --hash "$TMP/contracts.sha256"
```

Expected: verify exits `2` with UTF-8 decode failure.

## 09 `09_staged_outputs_no_write_on_deny.kr` (staged output behavior)

```bash
TMP="$(mktemp -d)"
cargo run -q -p kernriftc -- \
  check --contracts-out "$TMP/contracts.json" --hash-out "$TMP/contracts.sha256" examples/09_staged_outputs_no_write_on_deny.kr
ls -1 "$TMP"
```

Expected: check exits `1`, and no outputs are written (`contracts.json`, `contracts.sha256`, or `*.kernriftc.tmp.*`).

## 10 `10_edge_comments_whitespace.kr` (edge case: comments/whitespace)

```bash
cargo run -q -p kernriftc -- check examples/10_edge_comments_whitespace.kr
```

Expected: exit `0`.
