# Kernel Profile Example (MVP)

Run from repo root.

## Pass Case: `hello_irq.kr`

```bash
TMP="$(mktemp -d)"
cargo run -q -p kernriftc -- \
  check --profile kernel \
  --contracts-out "$TMP/contracts.json" \
  --hash-out "$TMP/contracts.sha256" \
  examples/kernel/hello_irq.kr

cargo run -q -p kernriftc -- \
  verify --contracts "$TMP/contracts.json" \
  --hash "$TMP/contracts.sha256" \
  --report "$TMP/verify.report.json"
```

Expected:
- `check` exits `0`
- contracts are emitted as `kernrift_contracts_v2`
- `verify` exits `0` and writes deterministic report JSON

## Deny Case: `critical_yield_bad.kr`

```bash
cargo run -q -p kernriftc -- check --profile kernel examples/kernel/critical_yield_bad.kr
```

Expected:
- exit `1`
- deterministic policy diagnostic with code `KERNEL_CRITICAL_YIELD`
