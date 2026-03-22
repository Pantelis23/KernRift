# PE + Mach-O Native Executable Emitter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `kernriftc hello.kr` produce a native executable on macOS (Mach-O) and Windows (PE/COFF), while keeping Linux ELF working unchanged.

**Architecture:** Add Mach-O and COFF relocatable object structs + emitters to `crates/krir/src/lib.rs` (platform-agnostic binary format code), add platform-specific startup stubs and linker invocations to `crates/kernriftc/src/lib.rs`, then replace the single Linux-only guard in `emit_x86_64_executable_bytes` with per-platform `#[cfg]` dispatch via a new `emit_native_executable` helper.

**Tech Stack:** Rust, hand-written Mach-O64 / COFF byte emission (no external crate), `cc`/`clang`/`cl`/`lld-link` invoked at runtime via `std::process::Command`.

---

## File Structure

| File | Change |
|------|--------|
| `crates/krir/src/lib.rs` | Add `X86_64MachORelocatableObject`, `X86_64MachOFunctionSymbol`, `X86_64MachORelocation`, `export_compiler_owned_object_to_x86_64_macho`, `emit_x86_64_macho_object_bytes`, `lower_executable_krir_to_x86_64_macho_object`; same pattern for COFF |
| `crates/kernriftc/src/lib.rs` | Add `hosted_startup_stub_asm_macos`, `link_x86_64_macos_executable` (cfg macos); `hosted_startup_stub_c_windows`, `link_x86_64_windows_executable` (cfg windows); refactor `emit_x86_64_executable_bytes` + new `emit_native_executable` |
| `crates/kernriftc/src/lib.rs` imports | Add conditional `use krir::{...}` for macos/windows new functions |
| `crates/krir/tests/` (new file) | `macho_coff_format.rs` — smoke tests for Mach-O magic bytes and COFF machine-type bytes |

---

## Background (read before implementing)

### How the pipeline works

`kernriftc hello.kr` → `parse_backend_emit_args("krboexe", ...)` → `emit_backend_artifact_file_with_surface_and_target(..., KrboExecutable, ...)` → `emit_x86_64_executable_bytes(&executable, &target)`.

Currently `emit_x86_64_executable_bytes` returns an error on non-Linux. The fix: detect the host OS at compile-time and dispatch to the correct path.

### Platform targets

| OS | `BackendTargetContract` constructor | `sections.text` | symbol prefix |
|----|-------------------------------------|-----------------|---------------|
| Linux | `BackendTargetContract::x86_64_sysv()` | `".text"` | `""` |
| macOS | `BackendTargetContract::x86_64_macho()` | `"__TEXT,__text"` | `"_"` |
| Windows | `BackendTargetContract::x86_64_win64()` | `".text"` | `""` |

### Mach-O64 relocatable object layout

```
 Offset | Content                                     | Size
--------+---------------------------------------------+-------
      0 | mach_header_64                              |   32
     32 | LC_SEGMENT_64 (cmd=0x19, cmdsize=152)       |   72
    104 | section_64 (__TEXT/__text)                  |   80
    184 | LC_SYMTAB (cmd=0x2, cmdsize=24)             |   24
    208 | text bytes (padded to 4-byte align)         | variable
  208+P | relocation entries (8 bytes each, if any)   | variable
  208+P+R | nlist_64 entries (16 bytes each)          | variable
  ...   | string table                                | variable
```

Key numeric constants:
- `MH_MAGIC_64` = `0xFEEDFACF`
- `CPU_TYPE_X86_64` = `0x01000007`
- `MH_OBJECT` = `0x00000001`
- `MH_SUBSECTIONS_VIA_SYMBOLS` = `0x00002000`
- `LC_SEGMENT_64` = `0x00000019`
- `LC_SYMTAB` = `0x00000002`
- Section flags `__text`: `0x80000400` (pure_instructions | some_instructions)
- `N_SECT | N_EXT` = `0x0F` (defined external symbol)
- `N_EXT | N_UNDF` = `0x01` (undefined external symbol)
- X86_64_RELOC_BRANCH r_info: `(sym_idx << 8) | 0xD2` (type=2, extern=1, pcrel=1, length=2)
- Symbol names get `_` prefix in the string table (e.g. `entry` → `_entry`)

### COFF layout

```
 Offset | Content                                    | Size
--------+--------------------------------------------+------
      0 | IMAGE_FILE_HEADER                          |   20
     20 | IMAGE_SECTION_HEADER for .text             |   40
     60 | text bytes (padded to 4-byte align)        | variable
  60+P  | IMAGE_RELOCATION entries (10 bytes each)   | variable
  60+P+R | symbol table (18 bytes each)              | variable
  ...   | string table (4-byte size + strings)       | variable
```

Key numeric constants:
- Machine: `0x8664` (AMD64)
- Section characteristics: `0x60500020` (CODE | ALIGN_16BYTES | MEM_READ | MEM_EXECUTE)
- `IMAGE_REL_AMD64_REL32` = `0x0004`
- Symbol StorageClass EXTERNAL = `0x02`, STATIC = `0x03`
- Symbol type function = `0x0020`
- `IMAGE_SYM_UNDEFINED` SectionNumber = `0`
- No `_` prefix on Windows x86-64 ABI

### Helper functions already in `crates/krir/src/lib.rs`

`push_u16_le`, `push_u32_le`, `push_u64_le`, `push_i64_le`, `append_with_alignment` — all available as private functions; new functions in the same file can use them.

### Fixup resolution logic (same as ELF exporter)

- `DefinedText` fixup → compute displacement directly and patch `text_bytes` in place
- `UndefinedExternal` fixup → leave `text_bytes` bytes at 0; emit a platform relocation entry

---

## Task 1: Mach-O object structs, export, and emit in `crates/krir/src/lib.rs`

**Files:**
- Modify: `crates/krir/src/lib.rs` — add after the ELF relocation structs (~line 6433)

- [ ] **Step 1: Write a failing test**

In `crates/krir/src/lib.rs` at the bottom of the `#[cfg(test)]` block, add:

```rust
#[test]
fn macho_object_bytes_start_with_magic() {
    use std::collections::BTreeMap;
    // Minimal: one defined function, no external calls
    let object = X86_64MachORelocatableObject {
        text_bytes: vec![0xC3], // single `ret`
        function_symbols: vec![X86_64MachOFunctionSymbol {
            name: "entry".to_string(),
            offset: 0,
            size: 1,
        }],
        undefined_function_symbols: vec![],
        relocations: vec![],
    };
    let bytes = emit_x86_64_macho_object_bytes(&object);
    assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE], "must start with MH_MAGIC_64");
}
```

Run: `cargo test -p krir macho_object_bytes_start_with_magic 2>&1 | head -20`

Expected: FAIL with "unresolved" or "not found" for `X86_64MachORelocatableObject`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p krir macho_object_bytes_start_with_magic 2>&1 | head -20`

Expected: compile error — the types don't exist yet.

- [ ] **Step 3: Add structs and export function**

Locate the end of `X86_64ElfRelocation` struct (around line 6433 in `lib.rs`). After that line, add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64MachORelocatableObject {
    pub text_bytes: Vec<u8>,
    pub function_symbols: Vec<X86_64MachOFunctionSymbol>,
    pub undefined_function_symbols: Vec<String>,
    pub relocations: Vec<X86_64MachORelocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64MachOFunctionSymbol {
    pub name: String,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64MachORelocation {
    pub offset: u32,
    pub target_symbol: String,
}

pub fn validate_compiler_owned_object_for_x86_64_macho_export(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<(), String> {
    target.validate()?;
    if target.target_id != BackendTargetId::X86_64MachO {
        return Err("x86_64 Mach-O export requires x86_64-macho target contract".to_string());
    }
    object.validate()?;
    if object.header.target_id != target.target_id {
        return Err("x86_64 Mach-O export: object target_id must match target contract".to_string());
    }
    if object.code.name != target.sections.text {
        return Err("x86_64 Mach-O export: object code section must match target text section".to_string());
    }
    Ok(())
}

pub fn export_compiler_owned_object_to_x86_64_macho(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<X86_64MachORelocatableObject, String> {
    validate_compiler_owned_object_for_x86_64_macho_export(object, target)?;

    let symbol_offsets = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|s| (s.name.as_str(), s.offset))
        .collect::<BTreeMap<_, _>>();
    let symbol_defs = object
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.definition))
        .collect::<BTreeMap<_, _>>();

    let mut text_bytes = object.code.bytes.clone();
    let mut relocations = Vec::new();

    for fixup in &object.fixups {
        let Some(target_def) = symbol_defs.get(fixup.target_symbol.as_str()) else {
            return Err(format!(
                "x86_64 Mach-O export: missing target symbol '{}' for fixup",
                fixup.target_symbol
            ));
        };
        match (fixup.kind, target_def) {
            (
                CompilerOwnedFixupKind::X86_64CallRel32,
                CompilerOwnedObjectSymbolDefinition::DefinedText,
            ) => {
                let target_offset = *symbol_offsets.get(fixup.target_symbol.as_str()).unwrap();
                let next_ip = fixup.patch_offset + u64::from(fixup.width_bytes);
                let displacement = (target_offset as i64) - (next_ip as i64);
                let rel32 = i32::try_from(displacement).map_err(|_| {
                    format!(
                        "x86_64 Mach-O export: call displacement to '{}' does not fit rel32",
                        fixup.target_symbol
                    )
                })?;
                let patch = usize::try_from(fixup.patch_offset).expect("patch offset fits usize");
                text_bytes[patch..patch + 4].copy_from_slice(&rel32.to_le_bytes());
            }
            (
                CompilerOwnedFixupKind::X86_64CallRel32,
                CompilerOwnedObjectSymbolDefinition::UndefinedExternal,
            ) => {
                relocations.push(X86_64MachORelocation {
                    offset: u32::try_from(fixup.patch_offset)
                        .expect("patch offset fits u32"),
                    target_symbol: fixup.target_symbol.clone(),
                });
            }
        }
    }

    let function_symbols = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|s| X86_64MachOFunctionSymbol {
            name: s.name.clone(),
            offset: s.offset,
            size: s.size,
        })
        .collect::<Vec<_>>();
    let undefined_function_symbols = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::UndefinedExternal)
        .map(|s| s.name.clone())
        .collect::<Vec<_>>();

    Ok(X86_64MachORelocatableObject {
        text_bytes,
        function_symbols,
        undefined_function_symbols,
        relocations,
    })
}

pub fn lower_executable_krir_to_x86_64_macho_object(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<X86_64MachORelocatableObject, String> {
    validate_x86_64_object_linear_subset(module, target)?;
    let object = lower_executable_krir_to_compiler_owned_object(module, target)?;
    export_compiler_owned_object_to_x86_64_macho(&object, target)
}
```

- [ ] **Step 4: Add `emit_x86_64_macho_object_bytes`**

After `lower_executable_krir_to_x86_64_macho_object` (or after `emit_x86_64_object_bytes`), add:

```rust
pub fn emit_x86_64_macho_object_bytes(object: &X86_64MachORelocatableObject) -> Vec<u8> {
    // String table: leading null + "_name\0" for each symbol
    let mut strtab = vec![0u8];
    let mut name_offsets: BTreeMap<String, u32> = BTreeMap::new();
    for sym in &object.function_symbols {
        let offset = strtab.len() as u32;
        let prefixed = format!("_{}", sym.name);
        strtab.extend_from_slice(prefixed.as_bytes());
        strtab.push(0);
        name_offsets.insert(sym.name.clone(), offset);
    }
    for sym in &object.undefined_function_symbols {
        let offset = strtab.len() as u32;
        let prefixed = format!("_{}", sym);
        strtab.extend_from_slice(prefixed.as_bytes());
        strtab.push(0);
        name_offsets.insert(sym.clone(), offset);
    }

    // nlist_64 entries (16 bytes each)
    let mut symtab: Vec<u8> = Vec::new();
    for sym in &object.function_symbols {
        let strx = *name_offsets.get(&sym.name).expect("symbol name in strtab");
        push_u32_le(&mut symtab, strx);
        symtab.push(0x0F); // N_SECT | N_EXT
        symtab.push(1);    // section ordinal 1 = __text
        push_u16_le(&mut symtab, 0);
        push_u64_le(&mut symtab, sym.offset);
    }
    for sym in &object.undefined_function_symbols {
        let strx = *name_offsets.get(sym.as_str()).expect("undef symbol in strtab");
        push_u32_le(&mut symtab, strx);
        symtab.push(0x01); // N_EXT | N_UNDF
        symtab.push(0);    // NO_SECT
        push_u16_le(&mut symtab, 0);
        push_u64_le(&mut symtab, 0);
    }
    let nsyms = (object.function_symbols.len() + object.undefined_function_symbols.len()) as u32;

    // Symbol index map for relocations (undefined symbols follow defined)
    let mut sym_index: BTreeMap<String, u32> = BTreeMap::new();
    for (i, sym) in object.function_symbols.iter().enumerate() {
        sym_index.insert(sym.name.clone(), i as u32);
    }
    for (i, sym) in object.undefined_function_symbols.iter().enumerate() {
        sym_index.insert(sym.clone(), (object.function_symbols.len() + i) as u32);
    }

    // Relocation entries (8 bytes each): r_address (i32) + r_info (u32)
    let mut reloc_bytes: Vec<u8> = Vec::new();
    for reloc in &object.relocations {
        push_u32_le(&mut reloc_bytes, reloc.offset);
        let idx = *sym_index.get(&reloc.target_symbol).expect("reloc target in sym_index");
        let r_info = (idx << 8) | 0xD2; // X86_64_RELOC_BRANCH, extern, pcrel, length=2
        push_u32_le(&mut reloc_bytes, r_info);
    }
    let nreloc = object.relocations.len() as u32;

    // Layout: header(32) + LC_SEGMENT_64+section(152) + LC_SYMTAB(24) + text + relocs + syms + strtab
    let text_offset: u32 = 208;
    let text_len = object.text_bytes.len() as u32;
    let text_padded = (text_len + 3) & !3u32;
    let reloc_offset: u32 = if nreloc == 0 { 0 } else { text_offset + text_padded };
    let sym_offset: u32 = text_offset + text_padded + reloc_bytes.len() as u32;
    let str_offset: u32 = sym_offset + symtab.len() as u32;
    let sizeofcmds: u32 = 152 + 24; // LC_SEGMENT_64+section + LC_SYMTAB

    let mut out: Vec<u8> = Vec::new();

    // mach_header_64
    push_u32_le(&mut out, 0xFEEDFACF); // MH_MAGIC_64
    push_u32_le(&mut out, 0x01000007); // CPU_TYPE_X86_64
    push_u32_le(&mut out, 0x00000003); // CPU_SUBTYPE_X86_64_ALL
    push_u32_le(&mut out, 0x00000001); // MH_OBJECT
    push_u32_le(&mut out, 2);           // ncmds
    push_u32_le(&mut out, sizeofcmds);
    push_u32_le(&mut out, 0x00002000); // MH_SUBSECTIONS_VIA_SYMBOLS
    push_u32_le(&mut out, 0);           // reserved

    // LC_SEGMENT_64
    push_u32_le(&mut out, 0x00000019); // LC_SEGMENT_64
    push_u32_le(&mut out, 152);        // cmdsize = 72 + 80
    out.extend_from_slice(b"__TEXT\0\0\0\0\0\0\0\0\0\0");
    push_u64_le(&mut out, 0);                      // vmaddr
    push_u64_le(&mut out, text_len as u64);        // vmsize
    push_u64_le(&mut out, text_offset as u64);     // fileoff
    push_u64_le(&mut out, text_len as u64);        // filesize
    push_u32_le(&mut out, 7);  // maxprot  (rwx)
    push_u32_le(&mut out, 5);  // initprot (rx)
    push_u32_le(&mut out, 1);  // nsects
    push_u32_le(&mut out, 0);  // flags

    // section_64 for __text
    out.extend_from_slice(b"__text\0\0\0\0\0\0\0\0\0\0");
    out.extend_from_slice(b"__TEXT\0\0\0\0\0\0\0\0\0\0");
    push_u64_le(&mut out, 0);                  // addr
    push_u64_le(&mut out, text_len as u64);    // size
    push_u32_le(&mut out, text_offset);        // offset
    push_u32_le(&mut out, 4);                  // align (log2 → 16)
    push_u32_le(&mut out, reloc_offset);       // reloff (0 if no relocs)
    push_u32_le(&mut out, nreloc);             // nreloc
    push_u32_le(&mut out, 0x80000400);         // flags
    push_u32_le(&mut out, 0);                  // reserved1
    push_u32_le(&mut out, 0);                  // reserved2
    push_u32_le(&mut out, 0);                  // reserved3

    // LC_SYMTAB
    push_u32_le(&mut out, 0x00000002); // LC_SYMTAB
    push_u32_le(&mut out, 24);          // cmdsize
    push_u32_le(&mut out, sym_offset);
    push_u32_le(&mut out, nsyms);
    push_u32_le(&mut out, str_offset);
    push_u32_le(&mut out, strtab.len() as u32);

    // text (padded to 4)
    out.extend_from_slice(&object.text_bytes);
    while out.len() < (text_offset + text_padded) as usize {
        out.push(0);
    }
    // relocations
    out.extend_from_slice(&reloc_bytes);
    // symbol table
    out.extend_from_slice(&symtab);
    // string table
    out.extend_from_slice(&strtab);

    out
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p krir macho_object_bytes_start_with_magic 2>&1`

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/krir/src/lib.rs
git commit -m "feat(krir): add Mach-O64 relocatable object emitter"
```

---

## Task 2: macOS startup stub and linker in `crates/kernriftc/src/lib.rs`

**Files:**
- Modify: `crates/kernriftc/src/lib.rs`

- [ ] **Step 1: Write a failing test**

In `crates/kernriftc/src/lib.rs` at the bottom of the `#[cfg(test)]` block (or if the test block is in a separate file, look for the existing tests), add:

```rust
#[cfg(target_os = "macos")]
#[test]
fn macos_startup_stub_is_non_empty_asm() {
    let stub = hosted_startup_stub_asm_macos();
    assert!(stub.contains("_start"), "must define _start");
    assert!(stub.contains("0x20000C5"), "must use macOS mmap syscall");
    assert!(stub.contains("_entry"), "must call _entry");
}
```

Run: `cargo test -p kernriftc macos_startup_stub_is_non_empty_asm 2>&1`

Expected: FAIL with "not found" (function doesn't exist yet).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kernriftc macos_startup_stub_is_non_empty_asm 2>&1 | head -10`

- [ ] **Step 3: Add macOS startup stub and linker**

Locate `hosted_startup_stub_asm()` in `crates/kernriftc/src/lib.rs` (around line 658). After that function and its closing `}`, add the macOS functions:

```rust
#[cfg(target_os = "macos")]
fn hosted_startup_stub_asm_macos() -> &'static str {
    // macOS BSD syscalls use 0x2000000 prefix.
    // mmap: 0x20000C5, MAP_PRIVATE|MAP_FIXED|MAP_ANON = 0x02|0x10|0x1000 = 0x1012
    // write: 0x2000004, exit: 0x2000001
    concat!(
        ".text\n",
        ".globl _start\n",
        "_start:\n",
        // mmap(0x10000000, 0x1000, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_FIXED|MAP_ANON, -1, 0)
        "    mov $0x20000C5, %rax\n",
        "    mov $0x10000000, %rdi\n",
        "    mov $0x1000, %rsi\n",
        "    mov $3, %rdx\n",
        "    mov $0x1012, %r10\n",
        "    mov $-1, %r8\n",
        "    xor %r9d, %r9d\n",
        "    syscall\n",
        // call _entry
        "    call _entry\n",
        // scan buf[0..4096] for null terminator
        "    mov $0x10000000, %rdi\n",
        "    xor %rdx, %rdx\n",
        ".Lscan:\n",
        "    cmpb $0, (%rdi,%rdx)\n",
        "    je .Lwrite\n",
        "    inc %rdx\n",
        "    cmp $0x1000, %rdx\n",
        "    jl .Lscan\n",
        // write(1, 0x10000000, len)
        ".Lwrite:\n",
        "    test %rdx, %rdx\n",
        "    jz .Lexit\n",
        "    mov $0x2000004, %rax\n",
        "    mov $1, %rdi\n",
        "    mov $0x10000000, %rsi\n",
        "    syscall\n",
        // exit(0)
        ".Lexit:\n",
        "    mov $0x2000001, %rax\n",
        "    xor %edi, %edi\n",
        "    syscall\n",
    )
}

#[cfg(target_os = "macos")]
fn link_x86_64_macos_executable(object_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let cc = find_host_tool(&["cc", "clang"])
        .ok_or_else(|| "final executable emit requires a host compiler (cc or clang)".to_string())?;

    let temp_dir = unique_temp_dir("machoexe");
    fs::create_dir_all(&temp_dir).map_err(|err| {
        format!("failed to create temporary link directory '{}': {}", temp_dir.display(), err)
    })?;
    let cleanup = TempArtifactDir { path: temp_dir.clone() };

    let input_object = temp_dir.join("input.o");
    let startup_source = temp_dir.join("startup.s");
    let startup_object = temp_dir.join("startup.o");
    let output_path = temp_dir.join("output");

    fs::write(&input_object, object_bytes).map_err(|err| {
        format!("failed to write temporary object '{}': {}", input_object.display(), err)
    })?;
    fs::write(&startup_source, hosted_startup_stub_asm_macos()).map_err(|err| {
        format!("failed to write startup stub '{}': {}", startup_source.display(), err)
    })?;

    let asm_out = Command::new(&cc)
        .arg("-c")
        .arg(&startup_source)
        .arg("-o")
        .arg(&startup_object)
        .output()
        .map_err(|err| format!("failed to run assembler '{}': {}", cc, err))?;
    if !asm_out.status.success() {
        return Err(format!(
            "failed to assemble startup stub with '{}'\nstdout:\n{}\nstderr:\n{}",
            cc,
            String::from_utf8_lossy(&asm_out.stdout),
            String::from_utf8_lossy(&asm_out.stderr)
        ));
    }

    let link_out = Command::new(&cc)
        .arg("-nostdlib")
        .arg("-Wl,-e,_start")
        .arg("-o")
        .arg(&output_path)
        .arg(&startup_object)
        .arg(&input_object)
        .output()
        .map_err(|err| format!("failed to run linker '{}': {}", cc, err))?;
    if !link_out.status.success() {
        return Err(format!(
            "failed to link with '{}'\nstdout:\n{}\nstderr:\n{}",
            cc,
            String::from_utf8_lossy(&link_out.stdout),
            String::from_utf8_lossy(&link_out.stderr)
        ));
    }

    let bytes = fs::read(&output_path).map_err(|err| {
        format!("failed to read linked executable '{}': {}", output_path.display(), err)
    })?;
    drop(cleanup);
    Ok(bytes)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kernriftc macos_startup_stub_is_non_empty_asm 2>&1`

Expected: PASS (or IGNORED on non-macOS)

- [ ] **Step 5: Commit**

```bash
git add crates/kernriftc/src/lib.rs
git commit -m "feat(kernriftc): add macOS startup stub and Mach-O linker driver"
```

---

## Task 3: COFF object structs, export, and emit in `crates/krir/src/lib.rs`

**Files:**
- Modify: `crates/krir/src/lib.rs`

- [ ] **Step 1: Write a failing test**

In the `#[cfg(test)]` block of `crates/krir/src/lib.rs`, add:

```rust
#[test]
fn coff_object_bytes_start_with_amd64_machine() {
    let object = X86_64CoffRelocatableObject {
        text_bytes: vec![0xC3],
        function_symbols: vec![X86_64CoffFunctionSymbol {
            name: "entry".to_string(),
            offset: 0,
            size: 1,
        }],
        undefined_function_symbols: vec![],
        relocations: vec![],
    };
    let bytes = emit_x86_64_coff_bytes(&object);
    // First 2 bytes = Machine field = 0x8664 (AMD64) in little-endian
    assert_eq!(&bytes[0..2], &[0x64, 0x86], "must start with AMD64 machine type");
}
```

Run: `cargo test -p krir coff_object_bytes_start_with_amd64_machine 2>&1 | head -10`

Expected: FAIL — types not found.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p krir coff_object_bytes_start_with_amd64_machine 2>&1 | head -10`

- [ ] **Step 3: Add COFF structs and export function**

After the Mach-O structs (added in Task 1), add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64CoffRelocatableObject {
    pub text_bytes: Vec<u8>,
    pub function_symbols: Vec<X86_64CoffFunctionSymbol>,
    pub undefined_function_symbols: Vec<String>,
    pub relocations: Vec<X86_64CoffRelocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64CoffFunctionSymbol {
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct X86_64CoffRelocation {
    pub section_offset: u32,
    pub target_symbol: String,
}

pub fn validate_compiler_owned_object_for_x86_64_coff_export(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<(), String> {
    target.validate()?;
    if target.target_id != BackendTargetId::X86_64Win64 {
        return Err("x86_64 COFF export requires x86_64-win64 target contract".to_string());
    }
    object.validate()?;
    if object.header.target_id != target.target_id {
        return Err("x86_64 COFF export: object target_id must match target contract".to_string());
    }
    if object.code.name != target.sections.text {
        return Err("x86_64 COFF export: object code section must match target text section".to_string());
    }
    Ok(())
}

pub fn export_compiler_owned_object_to_x86_64_coff(
    object: &CompilerOwnedObject,
    target: &BackendTargetContract,
) -> Result<X86_64CoffRelocatableObject, String> {
    validate_compiler_owned_object_for_x86_64_coff_export(object, target)?;

    let symbol_offsets = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|s| (s.name.as_str(), s.offset))
        .collect::<BTreeMap<_, _>>();
    let symbol_defs = object
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.definition))
        .collect::<BTreeMap<_, _>>();

    let mut text_bytes = object.code.bytes.clone();
    let mut relocations = Vec::new();

    for fixup in &object.fixups {
        let Some(target_def) = symbol_defs.get(fixup.target_symbol.as_str()) else {
            return Err(format!(
                "x86_64 COFF export: missing target symbol '{}' for fixup",
                fixup.target_symbol
            ));
        };
        match (fixup.kind, target_def) {
            (
                CompilerOwnedFixupKind::X86_64CallRel32,
                CompilerOwnedObjectSymbolDefinition::DefinedText,
            ) => {
                let target_offset = *symbol_offsets.get(fixup.target_symbol.as_str()).unwrap();
                let next_ip = fixup.patch_offset + u64::from(fixup.width_bytes);
                let displacement = (target_offset as i64) - (next_ip as i64);
                let rel32 = i32::try_from(displacement).map_err(|_| {
                    format!(
                        "x86_64 COFF export: call displacement to '{}' does not fit rel32",
                        fixup.target_symbol
                    )
                })?;
                let patch = usize::try_from(fixup.patch_offset).expect("patch offset fits usize");
                text_bytes[patch..patch + 4].copy_from_slice(&rel32.to_le_bytes());
            }
            (
                CompilerOwnedFixupKind::X86_64CallRel32,
                CompilerOwnedObjectSymbolDefinition::UndefinedExternal,
            ) => {
                relocations.push(X86_64CoffRelocation {
                    section_offset: u32::try_from(fixup.patch_offset)
                        .expect("patch offset fits u32"),
                    target_symbol: fixup.target_symbol.clone(),
                });
            }
        }
    }

    let function_symbols = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::DefinedText)
        .map(|s| X86_64CoffFunctionSymbol {
            name: s.name.clone(),
            offset: u32::try_from(s.offset).expect("symbol offset fits u32"),
            size: u32::try_from(s.size).expect("symbol size fits u32"),
        })
        .collect::<Vec<_>>();
    let undefined_function_symbols = object
        .symbols
        .iter()
        .filter(|s| s.definition == CompilerOwnedObjectSymbolDefinition::UndefinedExternal)
        .map(|s| s.name.clone())
        .collect::<Vec<_>>();

    Ok(X86_64CoffRelocatableObject {
        text_bytes,
        function_symbols,
        undefined_function_symbols,
        relocations,
    })
}

pub fn lower_executable_krir_to_x86_64_coff_object(
    module: &ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<X86_64CoffRelocatableObject, String> {
    validate_x86_64_object_linear_subset(module, target)?;
    let object = lower_executable_krir_to_compiler_owned_object(module, target)?;
    export_compiler_owned_object_to_x86_64_coff(&object, target)
}
```

- [ ] **Step 4: Add `emit_x86_64_coff_bytes`**

After `lower_executable_krir_to_x86_64_coff_object`, add:

```rust
pub fn emit_x86_64_coff_bytes(object: &X86_64CoffRelocatableObject) -> Vec<u8> {
    // Symbol names > 8 chars go in the string table.
    // String table layout: 4-byte total size + null-terminated strings.
    let mut strtab_strings: Vec<u8> = Vec::new(); // strings only (no 4-byte size prefix yet)
    let mut name_strtab_offsets: BTreeMap<String, u32> = BTreeMap::new();

    let coff_name_bytes = |name: &str, strtab: &mut Vec<u8>, offsets: &mut BTreeMap<String, u32>| -> [u8; 8] {
        if name.len() <= 8 {
            let mut buf = [0u8; 8];
            buf[..name.len()].copy_from_slice(name.as_bytes());
            buf
        } else {
            // 4-byte zero + 4-byte offset into string table (past the 4-byte size field)
            let str_offset = 4 + strtab.len() as u32; // 4 for the size field itself
            if !offsets.contains_key(name) {
                offsets.insert(name.to_string(), str_offset);
                strtab.extend_from_slice(name.as_bytes());
                strtab.push(0);
            }
            let off = *offsets.get(name).unwrap();
            let mut buf = [0u8; 8];
            buf[4..8].copy_from_slice(&off.to_le_bytes());
            buf
        }
    };

    // Symbol table: [section symbol] + [defined fns] + [undefined fns]
    // section symbol index = 0
    // defined fn indices = 1..=N
    // undefined fn indices = N+1..
    let num_syms = 1 + object.function_symbols.len() + object.undefined_function_symbols.len();

    let mut sym_index: BTreeMap<String, u32> = BTreeMap::new();
    for (i, sym) in object.function_symbols.iter().enumerate() {
        sym_index.insert(sym.name.clone(), (1 + i) as u32);
    }
    for (i, sym) in object.undefined_function_symbols.iter().enumerate() {
        sym_index.insert(sym.clone(), (1 + object.function_symbols.len() + i) as u32);
    }

    // COFF relocation entries (10 bytes each)
    let mut relocs: Vec<u8> = Vec::new();
    for reloc in &object.relocations {
        push_u32_le(&mut relocs, reloc.section_offset);
        let idx = *sym_index.get(&reloc.target_symbol).expect("reloc target in sym_index");
        push_u32_le(&mut relocs, idx);
        push_u16_le(&mut relocs, 0x0004); // IMAGE_REL_AMD64_REL32
    }
    let nrelocs = object.relocations.len() as u16;

    // File layout offsets
    let text_raw_offset: u32 = 20 + 40; // COFF header + 1 section header = 60
    let text_len = object.text_bytes.len() as u32;
    let text_padded = (text_len + 3) & !3u32;
    let reloc_ptr: u32 = text_raw_offset + text_padded;
    let sym_table_ptr: u32 = reloc_ptr + relocs.len() as u32;

    let mut out: Vec<u8> = Vec::new();

    // IMAGE_FILE_HEADER (20 bytes)
    push_u16_le(&mut out, 0x8664); // Machine = AMD64
    push_u16_le(&mut out, 1);      // NumberOfSections
    push_u32_le(&mut out, 0);      // TimeDateStamp
    push_u32_le(&mut out, sym_table_ptr); // PointerToSymbolTable
    push_u32_le(&mut out, num_syms as u32); // NumberOfSymbols
    push_u16_le(&mut out, 0);      // SizeOfOptionalHeader
    push_u16_le(&mut out, 0);      // Characteristics

    // IMAGE_SECTION_HEADER for .text (40 bytes)
    out.extend_from_slice(b".text\0\0\0"); // Name (8 bytes)
    push_u32_le(&mut out, 0);              // VirtualSize (0 for .obj)
    push_u32_le(&mut out, 0);              // VirtualAddress
    push_u32_le(&mut out, text_padded);    // SizeOfRawData
    push_u32_le(&mut out, text_raw_offset); // PointerToRawData
    push_u32_le(&mut out, if nrelocs == 0 { 0 } else { reloc_ptr }); // PointerToRelocations
    push_u32_le(&mut out, 0);              // PointerToLinenumbers
    push_u16_le(&mut out, nrelocs);        // NumberOfRelocations
    push_u16_le(&mut out, 0);              // NumberOfLinenumbers
    push_u32_le(&mut out, 0x60500020);     // Characteristics (CODE|ALIGN_16|MEM_READ|MEM_EXEC)

    // Text bytes (padded to 4)
    out.extend_from_slice(&object.text_bytes);
    while out.len() < (text_raw_offset + text_padded) as usize {
        out.push(0);
    }

    // Relocation entries
    out.extend_from_slice(&relocs);

    // Symbol table (18 bytes per entry)
    // Helper: push one IMAGE_SYMBOL
    let mut push_sym = |out: &mut Vec<u8>, name_buf: [u8; 8], value: u32, section: i16, ty: u16, class: u8| {
        out.extend_from_slice(&name_buf);
        push_u32_le(out, value);
        out.extend_from_slice(&section.to_le_bytes());
        push_u16_le(out, ty);
        out.push(class);
        out.push(0); // NumberOfAuxSymbols
    };

    // Section symbol (.text)
    let section_name_buf = {
        let mut b = [0u8; 8];
        b[..5].copy_from_slice(b".text");
        b
    };
    push_sym(&mut out, section_name_buf, 0, 1, 0, 0x03); // StorageClass STATIC

    // Defined function symbols
    for sym in &object.function_symbols {
        let name_buf = coff_name_bytes(&sym.name, &mut strtab_strings, &mut name_strtab_offsets);
        push_sym(&mut out, name_buf, sym.offset, 1, 0x0020, 0x02); // EXTERNAL, function type
    }

    // Undefined function symbols
    for sym in &object.undefined_function_symbols {
        let name_buf = coff_name_bytes(sym, &mut strtab_strings, &mut name_strtab_offsets);
        push_sym(&mut out, name_buf, 0, 0, 0x0020, 0x02); // IMAGE_SYM_UNDEFINED, EXTERNAL
    }

    // String table: 4-byte total size (including itself) + strings
    let strtab_total_size = (4 + strtab_strings.len()) as u32;
    push_u32_le(&mut out, strtab_total_size);
    out.extend_from_slice(&strtab_strings);

    out
}
```

**Important:** The `coff_name_bytes` closure captures mutable references. In Rust, you cannot call a closure that captures `&mut strtab_strings` and `&mut name_strtab_offsets` inside another closure that also mutably borrows them. Refactor `coff_name_bytes` into a free function or inline the logic:

```rust
fn coff_encode_name(
    name: &str,
    strtab: &mut Vec<u8>,
    offsets: &mut BTreeMap<String, u32>,
) -> [u8; 8] {
    if name.len() <= 8 {
        let mut buf = [0u8; 8];
        buf[..name.len()].copy_from_slice(name.as_bytes());
        buf
    } else {
        if !offsets.contains_key(name) {
            let str_offset = 4 + strtab.len() as u32;
            offsets.insert(name.to_string(), str_offset);
            strtab.extend_from_slice(name.as_bytes());
            strtab.push(0);
        }
        let off = *offsets.get(name).unwrap();
        let mut buf = [0u8; 8];
        buf[4..8].copy_from_slice(&off.to_le_bytes());
        buf
    }
}
```

Replace the closure with calls to this free function: `coff_encode_name(&sym.name, &mut strtab_strings, &mut name_strtab_offsets)`.

The `push_sym` closure also has a borrow problem (captures `&mut out`). Replace it with an inline push or a free `fn push_coff_sym(out: &mut Vec<u8>, ...)`.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p krir coff_object_bytes_start_with_amd64_machine 2>&1`

Expected: PASS

- [ ] **Step 6: Verify both krir tests pass together**

Run: `cargo test -p krir 2>&1 | tail -5`

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/krir/src/lib.rs
git commit -m "feat(krir): add COFF/PE relocatable object emitter"
```

---

## Task 4: Windows startup stub and linker in `crates/kernriftc/src/lib.rs`

**Files:**
- Modify: `crates/kernriftc/src/lib.rs`

- [ ] **Step 1: Write a failing test**

```rust
#[cfg(target_os = "windows")]
#[test]
fn windows_startup_stub_mentions_virtualalloc() {
    let stub = hosted_startup_stub_c_windows();
    assert!(stub.contains("VirtualAlloc"), "stub must call VirtualAlloc");
    assert!(stub.contains("kernrift_start"), "stub must define kernrift_start");
}
```

Run: `cargo test -p kernriftc windows_startup_stub_mentions_virtualalloc 2>&1 | head -10`

Expected: FAIL (function not found).

- [ ] **Step 2: Run test to verify it fails**

Run the command above.

- [ ] **Step 3: Add Windows startup stub and linker**

After `link_x86_64_macos_executable` (from Task 2), add:

```rust
#[cfg(target_os = "windows")]
fn hosted_startup_stub_c_windows() -> &'static str {
    concat!(
        "#define WIN32_LEAN_AND_MEAN\n",
        "#include <windows.h>\n",
        "extern void entry(void);\n",
        "void kernrift_start(void) {\n",
        "    VirtualAlloc((LPVOID)0x10000000, 0x1000,\n",
        "                 MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);\n",
        "    entry();\n",
        "    const char *buf = (const char *)0x10000000;\n",
        "    DWORD len = 0;\n",
        "    while (len < 0x1000 && buf[len] != '\\0') { len++; }\n",
        "    if (len > 0) {\n",
        "        HANDLE h = GetStdHandle(STD_OUTPUT_HANDLE);\n",
        "        DWORD written;\n",
        "        WriteFile(h, buf, len, &written, NULL);\n",
        "    }\n",
        "    ExitProcess(0);\n",
        "}\n",
    )
}

#[cfg(target_os = "windows")]
fn link_x86_64_windows_executable(object_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let cc = find_host_tool(&["clang", "cl"])
        .ok_or_else(|| "final executable emit requires a host C compiler (clang.exe or cl.exe)".to_string())?;
    let linker = find_host_tool(&["lld-link", "link"])
        .ok_or_else(|| "final executable emit requires a host linker (lld-link.exe or link.exe)".to_string())?;

    let temp_dir = unique_temp_dir("winexe");
    fs::create_dir_all(&temp_dir).map_err(|err| {
        format!("failed to create temporary link directory '{}': {}", temp_dir.display(), err)
    })?;
    let cleanup = TempArtifactDir { path: temp_dir.clone() };

    let input_object = temp_dir.join("input.obj");
    let startup_source = temp_dir.join("startup.c");
    let startup_object = temp_dir.join("startup.obj");
    let output_path = temp_dir.join("output.exe");

    fs::write(&input_object, object_bytes).map_err(|err| {
        format!("failed to write temporary object '{}': {}", input_object.display(), err)
    })?;
    fs::write(&startup_source, hosted_startup_stub_c_windows()).map_err(|err| {
        format!("failed to write startup stub '{}': {}", startup_source.display(), err)
    })?;

    // Compile the C startup stub
    let cc_args: Vec<std::ffi::OsString> = if cc == "cl" {
        vec![
            "/c".into(),
            startup_source.as_os_str().into(),
            format!("/Fo{}", startup_object.display()).into(),
        ]
    } else {
        vec![
            "-c".into(),
            startup_source.as_os_str().into(),
            "-o".into(),
            startup_object.as_os_str().into(),
        ]
    };
    let cc_out = Command::new(&cc)
        .args(&cc_args)
        .output()
        .map_err(|err| format!("failed to run compiler '{}': {}", cc, err))?;
    if !cc_out.status.success() {
        return Err(format!(
            "failed to compile startup stub with '{}'\nstdout:\n{}\nstderr:\n{}",
            cc,
            String::from_utf8_lossy(&cc_out.stdout),
            String::from_utf8_lossy(&cc_out.stderr)
        ));
    }

    // Link
    let link_out = Command::new(&linker)
        .arg("/entry:kernrift_start")
        .arg("/subsystem:console")
        .arg(format!("/out:{}", output_path.display()))
        .arg(&startup_object)
        .arg(&input_object)
        .output()
        .map_err(|err| format!("failed to run linker '{}': {}", linker, err))?;
    if !link_out.status.success() {
        return Err(format!(
            "failed to link with '{}'\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&link_out.stdout),
            String::from_utf8_lossy(&link_out.stderr)
        ));
    }

    let bytes = fs::read(&output_path).map_err(|err| {
        format!("failed to read linked executable '{}': {}", output_path.display(), err)
    })?;
    drop(cleanup);
    Ok(bytes)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kernriftc windows_startup_stub_mentions_virtualalloc 2>&1`

Expected: PASS (or IGNORED on non-Windows)

- [ ] **Step 5: Commit**

```bash
git add crates/kernriftc/src/lib.rs
git commit -m "feat(kernriftc): add Windows startup stub and PE/COFF linker driver"
```

---

## Task 5: Platform dispatch — refactor `emit_x86_64_executable_bytes`

**Files:**
- Modify: `crates/kernriftc/src/lib.rs`

**Goal:** Replace the single `if !cfg!(target_os = "linux") { return Err(...) }` guard with per-platform `#[cfg]`-dispatched `emit_native_executable` functions.

- [ ] **Step 1: Write a test that will show the change works on Linux**

```rust
#[cfg(target_os = "linux")]
#[test]
fn emit_native_executable_linux_requires_entry() {
    use krir::{ExecutableKrirModule, lower_current_krir_to_executable_krir};
    // A module without an 'entry' function
    let src = "@ctx(thread) fn not_entry() {}";
    let krir = kernriftc::compile_source(src).unwrap();
    let exec = lower_current_krir_to_executable_krir(&krir).unwrap();
    let result = emit_x86_64_executable_bytes(&exec, &BackendTargetContract::x86_64_sysv());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("entry"));
}
```

Run: `cargo test -p kernriftc emit_native_executable_linux_requires_entry 2>&1`

Expected: PASS (the existing code already returns an error for missing entry).

- [ ] **Step 2: Refactor `emit_x86_64_executable_bytes` and add `emit_native_executable`**

Replace the entire `emit_x86_64_executable_bytes` function body with:

```rust
fn emit_x86_64_executable_bytes(
    executable: &krir::ExecutableKrirModule,
    _target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    if !executable.extern_declarations.is_empty() {
        let unresolved = executable
            .extern_declarations
            .iter()
            .map(|decl| decl.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "final executable emit currently requires no extern declarations; unresolved externs: {}",
            unresolved
        ));
    }

    if !executable
        .functions
        .iter()
        .any(|function| function.name == "entry")
    {
        return Err(
            "final executable emit currently requires a defined 'entry' function".to_string(),
        );
    }

    emit_native_executable(executable)
}
```

Then add the four platform-dispatched helpers **outside** (after) `emit_x86_64_executable_bytes`:

```rust
#[cfg(target_os = "linux")]
fn emit_native_executable(executable: &krir::ExecutableKrirModule) -> Result<Vec<u8>, String> {
    let target = BackendTargetContract::x86_64_sysv();
    let object = lower_executable_krir_to_x86_64_object(executable, &target)?;
    let object_bytes = emit_x86_64_object_bytes(&object);
    link_x86_64_linux_executable(&object_bytes)
}

#[cfg(target_os = "macos")]
fn emit_native_executable(executable: &krir::ExecutableKrirModule) -> Result<Vec<u8>, String> {
    use krir::{lower_executable_krir_to_x86_64_macho_object, emit_x86_64_macho_object_bytes};
    let target = BackendTargetContract::x86_64_macho();
    let object = lower_executable_krir_to_x86_64_macho_object(executable, &target)?;
    let object_bytes = emit_x86_64_macho_object_bytes(&object);
    link_x86_64_macos_executable(&object_bytes)
}

#[cfg(target_os = "windows")]
fn emit_native_executable(executable: &krir::ExecutableKrirModule) -> Result<Vec<u8>, String> {
    use krir::{lower_executable_krir_to_x86_64_coff_object, emit_x86_64_coff_bytes};
    let target = BackendTargetContract::x86_64_win64();
    let object = lower_executable_krir_to_x86_64_coff_object(executable, &target)?;
    let coff_bytes = emit_x86_64_coff_bytes(&object);
    link_x86_64_windows_executable(&coff_bytes)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn emit_native_executable(_: &krir::ExecutableKrirModule) -> Result<Vec<u8>, String> {
    Err(
        "compiling to a native executable requires Linux, macOS, or Windows.\n\
         Analysis commands (kernriftc check, kernriftc policy, etc.) work on all platforms."
            .to_string(),
    )
}
```

- [ ] **Step 3: Run all kernriftc tests**

Run: `cargo test -p kernriftc 2>&1 | tail -10`

Expected: all tests pass.

- [ ] **Step 4: Run the full test suite**

Run: `cargo test --workspace 2>&1 | tail -20`

Expected: all tests pass.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy --workspace 2>&1 | grep "^error" | head -20`

Expected: no errors.

- [ ] **Step 6: Run rustfmt check**

Run: `cargo fmt --all -- --check 2>&1`

Expected: no output (clean).

- [ ] **Step 7: Commit**

```bash
git add crates/kernriftc/src/lib.rs
git commit -m "feat(kernriftc): dispatch to native executable emitter per platform (macOS/Windows/Linux)"
```

---

## Task 6: End-to-end smoke test on the current host

**Note:** This task only runs on the current build machine (Linux in the dev environment). macOS and Windows paths are covered by the unit tests in Tasks 1–4 that verify the startup stub and object format bytes. Full e2e testing on macOS/Windows requires a CI environment with those OSes.

- [ ] **Step 1: Compile `hello.kr` and verify it runs**

Run: `kernriftc hello.kr && ./hello.krbo`

Expected output: `Hello, World!`

If `kernriftc` isn't in PATH, use: `cargo run -p kernriftc --bin kernriftc -- hello.kr && ./hello.krbo`

- [ ] **Step 2: Run CI validation script**

Run: `bash tools/validation/local_safe.sh 2>&1 | tail -20`

Expected: all checks pass.

- [ ] **Step 3: Commit (if any fixup needed from smoke test)**

```bash
git add -p  # stage only relevant changes
git commit -m "fix: address issues found during e2e smoke test"
```

---

## Known Pitfalls

1. **Rust closures capturing `&mut`**: `push_sym` and `coff_encode_name` can't be closures if they call each other or are called after another mutable borrow of the same vec. Promote them to module-level `fn` helpers.

2. **Mach-O `sizeofcmds` in header**: Must be 152 + 24 = 176, not 208 (208 is the text offset which includes the 32-byte header).

3. **COFF section `SizeOfRawData`**: Must be the padded size (`text_padded`), not `text_len`. The linker uses this to read the right number of bytes.

4. **Mach-O `reloc_offset = 0` when no relocations**: The section_64 `reloff` field must be 0 when `nreloc == 0` (some tools reject non-zero `reloff` with `nreloc == 0`).

5. **macOS linker requires Xcode CLT**: `cc -nostdlib -Wl,-e,_start` links with the system linker. Users need Xcode Command Line Tools (`xcode-select --install`). The error message from `find_host_tool` returning `None` is already helpful.

6. **Windows lld-link kernel32**: `lld-link` and `link.exe` auto-find `kernel32.lib` when the Windows SDK is in the LIBPATH environment variable (set by `vcvarsall.bat` or Visual Studio Developer Command Prompt). If the user uses a plain terminal, they may need to run from the Developer Prompt or set LIBPATH manually. Add to the linker error message: "run from a Visual Studio Developer Command Prompt or set LIBPATH to the Windows SDK Lib directory".
