=== KernRift Benchmark Suite ===
Date: Wed Apr  1 02:34:18 PM UTC 2026
CPU: AMD Ryzen 9 7900X 12-Core Processor

--- fib ---

### fib

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 0ms |
| gcc -O0 | 48ms |
| gcc -O2 | 41ms |
| rustc (debug) | 64ms |
| rustc -O2 | 66ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 396 B |
| gcc -O0 | 15800 B |
| gcc -O2 | 15800 B |
| rustc debug | 3889248 B |
| rustc -O2 | 3887792 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 476ms (runs: 477, 476, 446)
| gcc -O0: 382ms (runs: 382, 382, 382)
| gcc -O2: 79ms (runs: 78, 79, 79)
| rustc debug: 383ms (runs: 383, 383, 383)
| rustc -O2: 163ms (runs: 163, 163, 163)

--- sort ---

### sort

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 29ms |
| gcc -O2 | 28ms |
| rustc (debug) | 71ms |
| rustc -O2 | 83ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1040 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3905344 B |
| rustc -O2 | 3888048 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 347ms (runs: 347, 347, 346)
| gcc -O0: 149ms (runs: 149, 149, 148)
| gcc -O2: 271ms (runs: 271, 271, 271)
| rustc debug: 2604ms (runs: 2611, 2597, 2604)
| rustc -O2: 44ms (runs: 44, 44, 44)

--- sieve ---

### sieve

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 23ms |
| gcc -O2 | 27ms |
| rustc (debug) | 68ms |
| rustc -O2 | 85ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1110 B |
| gcc -O0 | 16008 B |
| gcc -O2 | 16008 B |
| rustc debug | 3901200 B |
| rustc -O2 | 3888144 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 7ms (runs: 7, 8, 7)
| gcc -O0: 3ms (runs: 3, 4, 3)
| gcc -O2: 2ms (runs: 2, 2, 2)
| rustc debug: 20ms (runs: 20, 20, 20)
| rustc -O2: 2ms (runs: 2, 2, 2)

--- matmul ---

### matmul

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 22ms |
| gcc -O2 | 28ms |
| rustc (debug) | 67ms |
| rustc -O2 | 79ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1858 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3900272 B |
| rustc -O2 | 3888488 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 35ms (runs: 35, 35, 35)
| gcc -O0: 16ms (runs: 16, 16, 15)
| gcc -O2: 4ms (runs: 4, 4, 4)
| rustc debug: 123ms (runs: 123, 123, 123)
| rustc -O2: 3ms (runs: 3, 3, 3)

=== Done ===
