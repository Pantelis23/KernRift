# Getting Started

## Install

```bash
# Linux / macOS (auto-detects x86_64 or ARM64)
bash install.sh

# Or download directly (x86_64)
curl -L -o krc https://github.com/Pantelis23/KernRift/releases/latest/download/krc-linux-x86_64
curl -L -o kr  https://github.com/Pantelis23/KernRift/releases/latest/download/kr
chmod +x krc kr
sudo mv krc kr /usr/local/bin/
```

## Your First Program

```kr
fn main() {
    uint64 msg = "Hello, World!\n"
    write(1, msg, 14)
    exit(0)
}
```

Save as `hello.kr` and compile:

```bash
krc hello.kr -o hello.krbo        # fat binary (x86_64 + ARM64, LZ4-compressed)
kr hello.krbo                     # run on any platform

krc --arch=x86_64 hello.kr -o hello   # native x86_64 ELF
./hello                               # run directly
```

## Language Basics

```kr
// Types
uint64 x = 42
uint32 y = 10
uint8 byte = 0xFF

// Control flow
if x > 10 {
    exit(1)
} else {
    exit(0)
}

while x > 0 {
    x -= 1
}

for i in 0..10 {
    // i goes from 0 to 9
}

// Functions
fn add(uint64 a, uint64 b) -> uint64 {
    return a + b
}

// Structs
struct Point {
    uint64 x
    uint64 y
}

// Enums
enum Color {
    Red = 0
    Green = 1
    Blue = 2
}

// Arrays
uint8[256] buffer
buffer[0] = 42

// Pointer operations (kernel memory access)
unsafe { *(addr as uint32) = value }
unsafe { *(addr as uint32) -> result }

// Static variables
static uint64 counter = 0
```

## Safety Analysis

```bash
krc check module.kr     # context, effect, lock, capability checks
krc lc module.kr        # living compiler patterns + fitness score
```

## Toolchain

| Tool | Purpose |
|------|---------|
| `krc` | Compiler — compiles `.kr` source to native ELF or `.krbo` fat binary |
| `kr` | Runner — executes `.krbo` fat binaries on any supported platform |
