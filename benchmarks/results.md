=== KernRift Benchmark Suite ===
Date: Mon Apr  6 12:55:36 AM UTC 2026
CPU: AMD Ryzen 9 7900X 12-Core Processor

--- fib ---

### fib

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 24ms |
| gcc -O2 | 36ms |
| rustc (debug) | 63ms |
| rustc -O2 | 67ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 393 B |
| gcc -O0 | 15800 B |
| gcc -O2 | 15800 B |
| rustc debug | 3889248 B |
| rustc -O2 | 3887792 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 476ms (runs: 447, 476, 476)
| gcc -O0: 382ms (runs: 382, 383, 382)
| gcc -O2: 78ms (runs: 78, 78, 78)
| rustc debug: 383ms (runs: 383, 383, 383)
| rustc -O2: 163ms (runs: 163, 163, 163)

--- sort ---

### sort

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 22ms |
| gcc -O2 | 26ms |
| rustc (debug) | 72ms |
| rustc -O2 | 87ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1031 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3905344 B |
| rustc -O2 | 3888048 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 349ms (runs: 349, 355, 349)
| gcc -O0: 150ms (runs: 155, 150, 150)
| gcc -O2: 268ms (runs: 269, 268, 268)
| rustc debug: 2620ms (runs: 2620, 2620, 2621)
| rustc -O2: 44ms (runs: 44, 44, 44)

--- sieve ---

### sieve

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 23ms |
| gcc -O2 | 28ms |
| rustc (debug) | 71ms |
| rustc -O2 | 88ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1101 B |
| gcc -O0 | 16008 B |
| gcc -O2 | 16008 B |
| rustc debug | 3901200 B |
| rustc -O2 | 3888144 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 8ms (runs: 9, 8, 8)
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
| gcc -O0 | 24ms |
| gcc -O2 | 30ms |
| rustc (debug) | 69ms |
| rustc -O2 | 88ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1840 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3900272 B |
| rustc -O2 | 3888488 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 35ms (runs: 35, 35, 35)
| gcc -O0: 15ms (runs: 15, 16, 15)
| gcc -O2: 4ms (runs: 4, 4, 4)
| rustc debug: 123ms (runs: 123, 123, 128)
| rustc -O2: 3ms (runs: 3, 4, 3)

=== Done ===
