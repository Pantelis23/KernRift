# KRIR Spec (MVP)

## Purpose

KRIR is the kernel-aware IR for KernRift. It carries semantic facts that must remain enforceable through optimization and lowering.

KRIR currently has two distinct roles:

- `KrirModule`: the existing analysis-first contract used for checks and deterministic artifact emission.
- `ExecutableKrirModule`: the minimal executable subset contract that future backend work must lower from.
- `BackendTargetContract`: the explicit target-machine contract that future executable KRIR lowering must target.
- `CompilerOwnedObject`: the primary internal machine-facing binary object artifact, derived from executable KRIR plus a target contract and preserving explicit symbols and fixups, including unresolved external call intent.
- `X86_64AsmModule`: a target-specific assembly/debug/reference model, derived from executable KRIR plus a target contract.
- `X86_64ElfRelocatableObject`: a downstream x86_64 ELF compatibility/export artifact derived from `CompilerOwnedObject`, including relocation/export data derived only from compiler-owned symbols and fixups rather than from direct KRIR lowering. Its symbol ordering, symbol indices, relocation ordering, and `.rela.text` metadata are deterministic compatibility facts, not hidden emitter accidents. When external ELF inspection and linker tools are available, this subset is smoke-checked against them for compatibility, including the smallest practical final-link flows and runtime execution smoke for those linked artifacts; those tools do not become compiler truth.

Between surface KernRift and executable KRIR, the compiler owns a separate canonical executable semantics boundary in HIR. Governed surface forms normalize there before any lowering to executable KRIR begins.

The executable subset is intentionally narrow. It is specified separately so backend work does not pretend the current fact-heavy analysis IR is already codegen-ready.

The first target contract is specified separately in `docs/spec/backend-target-model-x86_64-sysv-v0.1.md`. It defines target facts only; the contract itself still does not perform instruction selection, register allocation, or stack-frame lowering.

The first compiler-owned object subset is specified separately in `docs/spec/compiler-owned-object-linear-subset-v0.1.md`. It lowers only the current tiny executable subset to a deterministic compiler-owned binary object format with explicit symbols and fixups.

The first target-specific lowering subset is specified separately in `docs/spec/x86_64-asm-linear-subset-v0.1.md`. It lowers only the current tiny executable subset to deterministic textual x86_64 SysV-flavored assembly and is not the primary backend artifact.

The first ELF machine-facing compatibility/export subset is specified separately in `docs/spec/x86_64-object-linear-subset-v0.1.md`. It exports the current tiny compiler-owned object subset to a deterministic ELF64 relocatable object subset with explicit symbol-order, symbol-index, relocation-order, and `.rela.text` metadata guarantees. It is not the primary internal object contract. Compatibility smoke checks against standard ELF inspection tools, relocatable linker flows, the smallest practical final-link flows, and narrow runtime execution smoke verify acceptance of the emitted bytes without making those tools a second semantic source.

`kernriftc` can export the current backend artifacts directly:

- `kernriftc --surface stable --emit=krbo -o <output.krbo> --meta-out <output.json> <file.kr>`
- `kernriftc --surface stable --emit=elfobj -o <output.o> --meta-out <output.json> <file.kr>`
- `kernriftc --surface stable --emit=krbo -o <output.krbo> <file.kr>`
- `kernriftc --surface stable --emit=elfobj -o <output.o> <file.kr>`
- `kernriftc --emit=krbo -o <output.krbo> --meta-out <output.json> <file.kr>`
- `kernriftc --emit=elfobj -o <output.o> --meta-out <output.json> <file.kr>`
- `kernriftc --emit=krbo -o <output.krbo> <file.kr>`
- `kernriftc --emit=elfobj -o <output.o> <file.kr>`
- `kernriftc verify-artifact-meta <artifact> <meta.json>`

These are artifact-export and artifact-verification paths only. They participate in the same surface-aware CLI contract as the rest of `kernriftc`, while preserving stable-default behavior. Optional `--meta-out` writes deterministic automation/CI metadata derived from the emitted bytes and CLI inputs; it does not create a second backend truth path. When the resolved input path lies under the Git repo root, the sidecar records repo-relative source provenance for stable automation independent of invocation cwd. `verify-artifact-meta` checks only byte-derived and header-derived sidecar fields against an emitted artifact; it does not re-lower source or make provenance metadata semantic truth. Linking and execution remain downstream tooling concerns rather than compiler truth.

## Data Model

### Sets

- `CtxSet`: subset of `{boot, thread, irq, nmi}`
- `EffSet`: subset of `{alloc, block, preempt_off, ioport, mmio, dma_map, yield}`
- `CapSet`: set of capability atoms

### Capability Atoms (MVP)

- `Cap::PhysMap`
- `Cap::PageTableWrite`
- `Cap::IrqRoute`
- `Cap::IoPort(range)`
- `Cap::Mmio(base,len)`
- `Cap::DmaMap(dev_id)`

### Linear Capability Kinds (MVP)

- frame ownership handles
- DMA buffer ownership handles
- temporary map handles (`Map`/`Unmap`)

### Function Facts

Each function in KRIR carries:

- `ctx_ok: CtxSet`
- `eff_used: EffSet`
- `caps_req: CapSet`
- region attrs: `@noyield`, `lock_budget(N)`, optional `@leaf`, optional `@hotpath`

### Defaults (MVP)

If a function has no explicit annotations for these facts:

- `ctx_ok = {boot, thread}` (never defaults to `irq` or `nmi`)
- `eff_used = {}`
- `caps_req = {}`

### Context Policy Builtins

Compiler defines builtin `eff_allowed(ctx)` for each context:

- `eff_allowed(boot)`
- `eff_allowed(thread)`
- `eff_allowed(irq)`
- `eff_allowed(nmi)`

MVP minimum policy:

- `eff_allowed(irq)` excludes `alloc`, `block`, and `yield`
- `eff_allowed(nmi)` allows only `{ioport, preempt_off}` in KR0.1

### Extern/Unknown Symbol Rule (MVP)

- Any called-but-undefined symbol must have an `extern fn` declaration in module scope.
- Extern declarations participate in the same fact model (`ctx_ok`, `eff_used`, `caps_req`).
- Extern declarations must explicitly declare `@ctx(...)` and `@eff(...)`.
- Extern `@caps(...)` is optional and defaults to `{}`.
- Missing extern declaration for a called symbol is compile error.

### Capability Availability Model (KR0)

- Module declares `module_caps: CapSet`.
- `caps_avail(f) = module_caps` for all functions in KR0.
- Call and function checks use module-level capability availability.

## Core IR Ops

- `Call(callee, args)`
- `Acquire(lock_id, lock_class)`
- `Release(lock_id, lock_class)`
- `MmioRead(addr, width, order)`
- `MmioWrite(addr, value, width, order)`
- `Map(kind, args)`
- `Unmap(kind, handle)`
- `Fence(domain, kind)`
- `YieldPoint`

MMIO ordering:

- read order: `Relaxed | Acquire`
- write order: `Release | SeqCst`

Fence domains:

- `mmio`
- `cpu`

## Mandatory Passes (MVP)

1. `ctx-check`
- Enforce `ctx_ok(caller) subset_of ctx_ok(callee)`
- Forbid `YieldPoint` in `{irq, nmi}`

2. `effect-check`
- Enforce `eff_used(callee) subset_of eff_allowed(ctx)` for all possible caller contexts
- Forbid `YieldPoint` inside `@noyield`

3. `cap-check`
- Enforce `caps_req(callee) subset_of caps_avail(caller)` where `caps_avail(caller) = module_caps` in KR0
- Enforce linear caps are moved/consumed correctly on `Map`/`Unmap`/`DmaMap`

4. `mmio-verify`
- Validate MMIO width/access-mode legality
- Validate required fence patterns
- MVP policy: missing required fence is compile error

5. `lockgraph`
- Extract lock edges from `Acquire`/`Release` sequences per object
- Emit `lockgraph.json` per object
- Final link step merges lock graphs and rejects cycles
- Compute and report `max_lock_depth`
- Reject `YieldPoint` while any lock is held
- Reject calls to yielding callees while any lock is held
- Use interprocedural lock summaries in KR0.1; recursion is rejected in KR0.1

## Budget and Span Metrics (MVP)

### `lock_budget(N)`

- Budget unit is call-count
- Cost is +1 for every call to a function not marked `@leaf`
- For any path from `Acquire` to matching `Release`, cost must be `<= N`

### `no_yield_spans`

- Span unit uses the same call-count metric
- Report an upper bound on call-count between `YieldPoint`s in `thread` context
- `YieldPoint` inside `@noyield` is compile error
- Region with no reachable `YieldPoint` in thread context is reported as `unbounded`

## Call-Edge Rules (Summary)

For each `caller -> callee`:

- `ctx_ok(caller) subset_of ctx_ok(callee)`
- for all `c in ctx_ok(caller)`: `eff_used(callee) subset_of eff_allowed(c)`
- `caps_avail(caller) superset_of caps_req(callee)` (KR0: `caps_avail(caller) = module_caps`)

Any violation is a compile error.

## Artifact Outputs (MVP)

- `kernriftc --emit krir <file.kr>`
- `kernriftc --surface stable --emit=krbo -o <output.krbo> --meta-out <output.json> <file.kr>`
- `kernriftc --surface stable --emit=elfobj -o <output.o> --meta-out <output.json> <file.kr>`
- `kernriftc --surface stable --emit=krbo -o <output.krbo> <file.kr>`
- `kernriftc --surface stable --emit=elfobj -o <output.o> <file.kr>`
- `kernriftc --emit=krbo -o <output.krbo> --meta-out <output.json> <file.kr>`
- `kernriftc --emit=elfobj -o <output.o> --meta-out <output.json> <file.kr>`
- `kernriftc --emit=krbo -o <output.krbo> <file.kr>`
- `kernriftc --emit=elfobj -o <output.o> <file.kr>`
- `kernriftc --emit lockgraph <file.kr>`
- `kernriftc --emit caps <file.kr>`
- `kernriftc --emit contracts <file.kr>`
- `kernriftc check --policy <policy.toml> <file.kr>`
- `kernriftc check --contracts-out <contracts.json> <file.kr>`
- `kernriftc check --policy <policy.toml> --contracts-out <contracts.json> <file.kr>`
- `kernriftc check --policy <policy.toml> --contracts-out <contracts.json> --hash-out <contracts.sha256> <file.kr>`
- `kernriftc check --policy <policy.toml> --contracts-out <contracts.json> --hash-out <contracts.sha256> --sign-ed25519 <secret.hex> --sig-out <contracts.sig> <file.kr>`
- `kernriftc --report max_lock_depth,no_yield_spans <file.kr>`
- `kernriftc policy --policy <policy.toml> --contracts <contracts.json>`
- `kernriftc verify --contracts <contracts.json> --hash <contracts.sha256>`
- `kernriftc verify --contracts <contracts.json> --hash <contracts.sha256> --sig <contracts.sig> --pubkey <pubkey.hex>`

Contracts schema:

- `docs/schemas/kernrift_contracts_v1.schema.json`

## Lowering Invariants

- calling convention semantics
- section placement semantics
- symbol linkage and visibility semantics
- MMIO ordering semantics
- lock-class identity references

## Executable KRIR Boundary

Executable KRIR is the future backend-facing contract. In KR0.x it is intentionally tiny:

- linear function bodies only,
- direct calls only,
- unit result only,
- explicit block/terminator form,
- semantic facts remain attached but separate from executable ops.

Executable KRIR is not yet used by codegen in this branch. This branch only makes the contract explicit so later backend work has a deterministic, compiler-owned boundary.

## Backend Target Boundary

Future backend/codegen work must lower:

- canonical executable semantics
- to executable KRIR
- against an explicit backend target contract

The backend target contract is not executable KRIR and is not semantic authority. It records machine-facing constraints such as register sets, ABI, stack alignment, symbol naming, and section naming for a chosen target. In KR0.x the first defined contract is `x86_64-sysv`, but this branch still does not emit machine code.

## Target-Specific Assembly Boundary

Target-specific assembly is downstream of executable KRIR and downstream of the backend target contract:

- canonical executable semantics
- executable KRIR
- backend target contract
- target-specific assembly model

For KR0.x the first target-specific assembly model is intentionally tiny:

- `.text` section only,
- source symbol labels,
- ordered direct `call` instructions,
- terminal `ret`,
- no prologue/epilogue,
- no stack-slot allocation,
- no linker integration.

## Target-Specific Object Boundary

Target-specific object emission is downstream of:

- canonical executable semantics
- executable KRIR
- backend target contract

For KR0.x the first object-emission subset is intentionally tiny:

- ELF64 relocatable object only,
- one `.text` section,
- internal direct-call lowering only,
- no relocation sections,
- no stack-frame lowering,
- no executable generation.
