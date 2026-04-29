=== KernRift Benchmark Suite ===
Date: Wed Apr 29 03:05:59 AM UTC 2026
CPU: AMD Ryzen 9 7900X 12-Core Processor

--- fib ---

### fib

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 1ms |
| gcc -O0 | 23ms |
| gcc -O2 | 37ms |
| rustc (debug) | 62ms |
| rustc -O2 | 68ms |

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
| krc: 427ms (runs: 415, 427, 436)
| gcc -O0: 383ms (runs: 381, 383, 383)
| gcc -O2: 78ms (runs: 82, 78, 78)
| rustc debug: 383ms (runs: 379, 383, 386)
| rustc -O2: 162ms (runs: 162, 162, 162)

--- sort ---

### sort

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 2ms |
| gcc -O0 | 23ms |
| gcc -O2 | 28ms |
| rustc (debug) | 74ms |
| rustc -O2 | 86ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 552 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3905344 B |
| rustc -O2 | 3888048 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 108ms (runs: 107, 114, 108)
| gcc -O0: 150ms (runs: 150, 151, 149)
| gcc -O2: 270ms (runs: 269, 271, 270)
| rustc debug: 2629ms (runs: 2633, 2629, 2615)
| rustc -O2: 45ms (runs: 45, 44, 45)

--- sieve ---

### sieve

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 2ms |
| gcc -O0 | 24ms |
| gcc -O2 | 29ms |
| rustc (debug) | 71ms |
| rustc -O2 | 84ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 496 B |
| gcc -O0 | 16008 B |
| gcc -O2 | 16008 B |
| rustc debug | 3901200 B |
| rustc -O2 | 3888144 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 3ms (runs: 3, 3, 3)
| gcc -O0: 4ms (runs: 4, 4, 4)
| gcc -O2: 2ms (runs: 2, 2, 2)
| rustc debug: 21ms (runs: 21, 21, 22)
| rustc -O2: 2ms (runs: 2, 2, 2)

--- matmul ---

### matmul

**Compile time:**
| Compiler | Time |
|----------|------|
| krc (self-hosted) | 2ms |
| gcc -O0 | 25ms |
| gcc -O2 | 32ms |
| rustc (debug) | 73ms |
| rustc -O2 | 85ms |

**Binary size:**
| Binary | Size |
|--------|------|
| krc | 1320 B |
| gcc -O0 | 15960 B |
| gcc -O2 | 15960 B |
| rustc debug | 3900272 B |
| rustc -O2 | 3888488 B |

**Runtime (median of 3):**
| Binary | Time |
|--------|------|
| krc: 33ms (runs: 34, 33, 33)
| gcc -O0: 16ms (runs: 16, 16, 16)
| gcc -O2: 4ms (runs: 4, 4, 4)
| rustc debug: 126ms (runs: 124, 127, 126)
| rustc -O2: 3ms (runs: 4, 3, 3)

=== Done ===
