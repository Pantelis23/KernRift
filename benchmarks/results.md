=== KernRift Benchmark Suite ===
Date: Thu Apr  2 05:28:13 PM UTC 2026
CPU: AMD Ryzen 9 7900X 12-Core Processor

--- fib ---

### fib

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 36ms |
| gcc -O2 | 35ms |
| rustc (debug) | 65ms |
| rustc -O2 | 71ms |

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
| krc: 447ms (runs: 447, 449, 447)
| gcc -O0: 386ms (runs: 382, 386, 401)
| gcc -O2: 79ms (runs: 79, 79, 79)
| rustc debug: 385ms (runs: 385, 385, 384)
| rustc -O2: 164ms (runs: 164, 164, 164)

--- sort ---

### sort

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 31ms |
| gcc -O2 | 31ms |
| rustc (debug) | 96ms |
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
| krc: 352ms (runs: 351, 375, 352)
| gcc -O0: 150ms (runs: 150, 151, 150)
| gcc -O2: 272ms (runs: 272, 274, 272)
| rustc debug: 2637ms (runs: 2622, 2641, 2637)
| rustc -O2: 44ms (runs: 44, 44, 44)

--- sieve ---

### sieve

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 32ms |
| gcc -O2 | 29ms |
| rustc (debug) | 79ms |
| rustc -O2 | 92ms |

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
| gcc -O0: 4ms (runs: 4, 4, 4)
| gcc -O2: 2ms (runs: 2, 2, 2)
| rustc debug: 21ms (runs: 22, 21, 20)
| rustc -O2: 2ms (runs: 2, 2, 2)

--- matmul ---

### matmul

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 24ms |
| gcc -O2 | 31ms |
| rustc (debug) | 70ms |
| rustc -O2 | 81ms |

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
| gcc -O0: 15ms (runs: 16, 15, 15)
| gcc -O2: 4ms (runs: 4, 4, 4)
| rustc debug: 124ms (runs: 124, 124, 124)
| rustc -O2: 3ms (runs: 3, 3, 3)

=== Done ===
