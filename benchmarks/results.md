=== KernRift Benchmark Suite ===
Date: Sun Apr 19 02:33:20 AM UTC 2026
CPU: AMD Ryzen 9 7900X 12-Core Processor

--- fib ---

### fib

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 21ms |
| gcc -O2 | 34ms |
| rustc (debug) | 58ms |
| rustc -O2 | 63ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 339 B |
| gcc -O0 | 15800 B |
| gcc -O2 | 15800 B |
| rustc debug | 3889248 B |
| rustc -O2 | 3887792 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 407ms (runs: 407, 407, 407)
| gcc -O0: 378ms (runs: 377, 378, 378)
| gcc -O2: 77ms (runs: 77, 78, 77)
| rustc debug: 378ms (runs: 378, 379, 378)
| rustc -O2: 161ms (runs: 161, 161, 161)

--- sort ---

### sort

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 22ms |
| gcc -O2 | 25ms |
| rustc (debug) | 68ms |
| rustc -O2 | 85ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 596 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3905344 B |
| rustc -O2 | 3888048 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 152ms (runs: 154, 152, 151)
| gcc -O0: 149ms (runs: 148, 150, 149)
| gcc -O2: 268ms (runs: 268, 267, 270)
| rustc debug: 2607ms (runs: 2609, 2607, 2607)
| rustc -O2: 44ms (runs: 44, 44, 43)

--- sieve ---

### sieve

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 22ms |
| gcc -O2 | 26ms |
| rustc (debug) | 67ms |
| rustc -O2 | 78ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 531 B |
| gcc -O0 | 16008 B |
| gcc -O2 | 16008 B |
| rustc debug | 3901200 B |
| rustc -O2 | 3888144 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 3ms (runs: 3, 3, 3)
| gcc -O0: 3ms (runs: 3, 3, 6)
| gcc -O2: 2ms (runs: 2, 2, 2)
| rustc debug: 20ms (runs: 20, 20, 20)
| rustc -O2: 2ms (runs: 2, 2, 2)

--- matmul ---

### matmul

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 2ms |
| gcc -O0 | 23ms |
| gcc -O2 | 28ms |
| rustc (debug) | 67ms |
| rustc -O2 | 81ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1513 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3900272 B |
| rustc -O2 | 3888488 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 33ms (runs: 32, 33, 33)
| gcc -O0: 15ms (runs: 15, 15, 15)
| gcc -O2: 4ms (runs: 4, 4, 4)
| rustc debug: 122ms (runs: 122, 122, 122)
| rustc -O2: 3ms (runs: 3, 3, 3)

=== Done ===
