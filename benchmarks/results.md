=== KernRift Benchmark Suite ===
Date: Sat Apr 18 03:34:54 PM UTC 2026
CPU: AMD Ryzen 9 7900X 12-Core Processor

--- fib ---

### fib

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 67ms |
| gcc -O2 | 41ms |
| rustc (debug) | 282ms |
| rustc -O2 | 83ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 383 B |
| gcc -O0 | 15800 B |
| gcc -O2 | 15800 B |
| rustc debug | 3889248 B |
| rustc -O2 | 3887792 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 473ms (runs: 473, 473, 473)
| gcc -O0: 380ms (runs: 380, 381, 380)
| gcc -O2: 79ms (runs: 78, 79, 79)
| rustc debug: 381ms (runs: 381, 381, 381)
| rustc -O2: 162ms (runs: 162, 163, 162)

--- sort ---

### sort

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 8ms |
| gcc -O0 | 22ms |
| gcc -O2 | 28ms |
| rustc (debug) | 109ms |
| rustc -O2 | 86ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 772 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3905344 B |
| rustc -O2 | 3888048 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 171ms (runs: 171, 171, 171)
| gcc -O0: 149ms (runs: 149, 149, 149)
| gcc -O2: 267ms (runs: 268, 267, 267)
| rustc debug: 2607ms (runs: 2608, 2607, 2607)
| rustc -O2: 43ms (runs: 43, 43, 44)

--- sieve ---

### sieve

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 9ms |
| gcc -O0 | 23ms |
| gcc -O2 | 27ms |
| rustc (debug) | 69ms |
| rustc -O2 | 82ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 719 B |
| gcc -O0 | 16008 B |
| gcc -O2 | 16008 B |
| rustc debug | 3901200 B |
| rustc -O2 | 3888144 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 4ms (runs: 4, 4, 4)
| gcc -O0: 3ms (runs: 4, 3, 3)
| gcc -O2: 2ms (runs: 2, 2, 1)
| rustc debug: 20ms (runs: 20, 20, 21)
| rustc -O2: 2ms (runs: 2, 2, 2)

--- matmul ---

### matmul

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 9ms |
| gcc -O0 | 22ms |
| gcc -O2 | 28ms |
| rustc (debug) | 64ms |
| rustc -O2 | 79ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1968 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3900272 B |
| rustc -O2 | 3888488 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 34ms (runs: 34, 34, 34)
| gcc -O0: 15ms (runs: 16, 15, 15)
| gcc -O2: 4ms (runs: 4, 4, 4)
| rustc debug: 123ms (runs: 123, 123, 122)
| rustc -O2: 3ms (runs: 3, 3, 3)

=== Done ===
