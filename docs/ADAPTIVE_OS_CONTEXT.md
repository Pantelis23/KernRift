# Adaptive OS Context for KernRift

## Source Snapshot

This mapping is based on local project files in `C:\Users\pante\Desktop\Projects\Work\Adaptive_OS` inspected on 2026-02-13.

## Observed Constraints and Gaps

1. Scheduling and execution mode
- Cooperative-only preemption in critical paths (`STATUS.md:456`, `STATUS.md:981`)
- Ring 3 separation still missing in parts of the system (`STATUS.md:419`, `STATUS.md:457`)

2. Driver and interrupt handling
- Polling-based interrupt transfer behavior in USB path (`STATUS.md:889`)
- Driver stack still evolving (storage/network gaps) (`STATUS.md:345`, `STATUS.md:346`)

3. Memory and allocator behavior
- Frame allocator linear scan called out as inefficient (`STATUS.md:177`)
- Additional memory subsystem limitations noted in status docs (`STATUS.md:980`)

4. Safety policy intent already exists
- Existing policy declarations include `no_blocking_in_irq` and lock hold limits (`README.md:241`, `README.md:242`)
- Architecture docs already reference preemption latency and lock constraints (`Enhanced_Architecture_v2.md:69`, `Enhanced_Architecture_v2.md:73`)

## KernRift Responses

1. Context safety by type/effect system
- Enforce valid call edges across canonical KR0 facts such as `@ctx(irq)`, `@ctx(thread)`, `@eff(block)`, and `@eff(preempt_off)`
- Reject blocking or heap allocation where effects disallow it

2. Lock correctness
- Require lock-class declarations
- Build compile-time lock-order graph and reject cycles

3. MMIO and interrupt correctness
- Structured MMIO declarations plus typed `mmio_read<T>(addr)` / `mmio_write<T>(addr, value)` operations
- Enforce ordering/fence semantics through explicit primitives checked in KRIR

4. Capability boundaries
- Privileged operations require capabilities in function signatures
- Effect and capability mismatch is compile-time error, not runtime convention

5. Code-shape + performance
- Mark explicit hot paths and preserve layout/inline constraints through codegen
- Validate stack bounds for IRQ/exception entry paths
- Add `yieldpoint()` and `@noyield` to model cooperative-preemption constraints explicitly
- Add `lock_budget(N)` and emit worst-case lock/yield spans as build reports

## Cooperative Preemption and Lock Limits

To match current AOS behavior and constraints:

- `@noyield` marks regions that must not call scheduler yield paths
- `yieldpoint()` is explicit and compiler-checked
- `lock_budget(N)` uses call-count metric:
- unit is 1 per call to a non-`@leaf` function
- path from `Acquire` to matching `Release` must have call-count `<= N`

MVP checks:

- No yield inside `@noyield` region
- No `yieldpoint()` while holding spinlock
- No `yieldpoint()` in IRQ context
- Emit report fields for `max_lock_depth` and `no_yield_spans`

## Acceptance Signals for AOS Integration

- Compile-time rejection of at least one known invalid IRQ/lock usage pattern
- Driver hot path output inspected for expected volatile/fence instruction patterns
- Scheduler/interrupt path stack bound report emitted during build
- C ABI integration validated with one mixed C + KernRift kernel module

Concrete command targets for CI/dev:

- `kernriftc --report max_lock_depth,no_yield_spans <file.kr>`
- `kernriftc --emit lockgraph <file.kr>`
- `kernriftc --emit caps <file.kr>`
