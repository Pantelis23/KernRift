=== KernRift Benchmark Suite ===
Date: Sat Apr  4 10:28:50 AM UTC 2026
CPU: AMD Ryzen 9 7900X 12-Core Processor

--- fib ---

### fib

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 23ms |
| gcc -O2 | 37ms |
| rustc (debug) | 295ms |
| rustc -O2 | 90ms |

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
| krc: 448ms (runs: 448, 448, 477)
| gcc -O0: 385ms (runs: 386, 385, 384)
| gcc -O2: 79ms (runs: 79, 79, 80)
| rustc debug: 381ms (runs: 384, 381, 381)
| rustc -O2: 162ms (runs: 162, 162, 163)

--- sort ---

### sort

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 23ms |
| gcc -O2 | 28ms |
| rustc (debug) | 126ms |
| rustc -O2 | 95ms |

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
| krc: 348ms (runs: 348, 348, 348)
| gcc -O0: 150ms (runs: 149, 150, 150)
| gcc -O2: 272ms (runs: 271, 272, 272)
| rustc debug: 2631ms (runs: 2612, 2656, 2631)
| rustc -O2: 44ms (runs: 44, 44, 44)

--- sieve ---

### sieve

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 31ms |
| gcc -O2 | 29ms |
| rustc (debug) | 77ms |
| rustc -O2 | 89ms |

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
| krc: 8ms (runs: 8, 8, 8)
| gcc -O0: 3ms (runs: 4, 3, 3)
| gcc -O2: 2ms (runs: 2, 2, 2)
| rustc debug: 21ms (runs: 20, 22, 21)
| rustc -O2: 2ms (runs: 3, 2, 2)

--- matmul ---

### matmul

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 27ms |
| gcc -O2 | 35ms |
| rustc (debug) | 86ms |
| rustc -O2 | 93ms |

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
| krc: 35ms (runs: 35, 35, 36)
| gcc -O0: 16ms (runs: 16, 16, 16)
| gcc -O2: 4ms (runs: 4, 4, 4)
| rustc debug: 124ms (runs: 124, 124, 124)
| rustc -O2: 3ms (runs: 3, 3, 3)

=== Done ===
