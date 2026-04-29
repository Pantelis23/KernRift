=== KernRift Benchmark Suite ===
Date: Wed Apr 29 04:45:41 AM UTC 2026
CPU: AMD Ryzen 9 7900X 12-Core Processor

--- fib ---

### fib

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 2ms |
| gcc -O0 | 73ms |
| gcc -O2 | 47ms |
| rustc (debug) | 377ms |
| rustc -O2 | 84ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 320 B |
| gcc -O0 | 15800 B |
| gcc -O2 | 15800 B |
| rustc debug | 3889248 B |
| rustc -O2 | 3887792 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 420ms (runs: 415, 420, 437)
| gcc -O0: 380ms (runs: 381, 380, 378)
| gcc -O2: 80ms (runs: 78, 81, 80)
| rustc debug: 381ms (runs: 390, 381, 378)
| rustc -O2: 162ms (runs: 162, 162, 161)

--- sort ---

### sort

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 31ms |
| gcc -O2 | 30ms |
| rustc (debug) | 116ms |
| rustc -O2 | 89ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 672 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3905344 B |
| rustc -O2 | 3888048 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 150ms (runs: 152, 150, 150)
| gcc -O0: 148ms (runs: 148, 153, 148)
| gcc -O2: 269ms (runs: 270, 269, 268)
| rustc debug: 2634ms (runs: 2614, 2635, 2634)
| rustc -O2: 45ms (runs: 44, 45, 46)

--- sieve ---

### sieve

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 2ms |
| gcc -O0 | 26ms |
| gcc -O2 | 30ms |
| rustc (debug) | 73ms |
| rustc -O2 | 86ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 632 B |
| gcc -O0 | 16008 B |
| gcc -O2 | 16008 B |
| rustc debug | 3901200 B |
| rustc -O2 | 3888144 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 3ms (runs: 3, 3, 3)
| gcc -O0: 3ms (runs: 3, 3, 4)
| gcc -O2: 2ms (runs: 2, 2, 2)
| rustc debug: 21ms (runs: 21, 21, 20)
| rustc -O2: 2ms (runs: 2, 2, 2)

--- matmul ---

### matmul

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 2ms |
| gcc -O0 | 31ms |
| gcc -O2 | 31ms |
| rustc (debug) | 69ms |
| rustc -O2 | 80ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1392 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3900272 B |
| rustc -O2 | 3888488 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 30ms (runs: 30, 30, 29)
| gcc -O0: 15ms (runs: 15, 15, 15)
| gcc -O2: 4ms (runs: 4, 4, 4)
| rustc debug: 126ms (runs: 124, 126, 127)
| rustc -O2: 3ms (runs: 3, 3, 3)

=== Done ===
