=== KernRift Benchmark Suite ===
Date: Thu Apr  2 07:38:46 AM UTC 2026
CPU: AMD Ryzen 9 7900X 12-Core Processor

--- fib ---

### fib

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 0ms |
| gcc -O0 | 29ms |
| gcc -O2 | 36ms |
| rustc (debug) | 61ms |
| rustc -O2 | 73ms |

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
| krc: 482ms (runs: 464, 485, 482)
| gcc -O0: 388ms (runs: 388, 388, 385)
| gcc -O2: 79ms (runs: 79, 79, 80)
| rustc debug: 385ms (runs: 386, 385, 385)
| rustc -O2: 163ms (runs: 163, 163, 163)

--- sort ---

### sort

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 23ms |
| gcc -O2 | 26ms |
| rustc (debug) | 70ms |
| rustc -O2 | 82ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1019 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3905344 B |
| rustc -O2 | 3888048 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 352ms (runs: 352, 350, 352)
| gcc -O0: 151ms (runs: 151, 153, 150)
| gcc -O2: 273ms (runs: 273, 272, 275)
| rustc debug: 2633ms (runs: 2620, 2639, 2633)
| rustc -O2: 44ms (runs: 44, 45, 44)

--- sieve ---

### sieve

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 30ms |
| gcc -O2 | 29ms |
| rustc (debug) | 71ms |
| rustc -O2 | 80ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1089 B |
| gcc -O0 | 16008 B |
| gcc -O2 | 16008 B |
| rustc debug | 3901200 B |
| rustc -O2 | 3888144 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 8ms (runs: 8, 8, 8)
| gcc -O0: 3ms (runs: 3, 3, 3)
| gcc -O2: 2ms (runs: 2, 2, 2)
| rustc debug: 20ms (runs: 21, 20, 20)
| rustc -O2: 2ms (runs: 2, 2, 2)

--- matmul ---

### matmul

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 23ms |
| gcc -O2 | 33ms |
| rustc (debug) | 71ms |
| rustc -O2 | 83ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1828 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3900272 B |
| rustc -O2 | 3888488 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 35ms (runs: 36, 35, 35)
| gcc -O0: 16ms (runs: 15, 16, 16)
| gcc -O2: 4ms (runs: 4, 4, 4)
| rustc debug: 125ms (runs: 127, 124, 125)
| rustc -O2: 3ms (runs: 3, 3, 3)

=== Done ===
