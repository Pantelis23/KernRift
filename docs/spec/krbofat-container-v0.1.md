# KRBOFAT Container Format v0.1

## Magic and header

```
[0..8]    magic:      b"KRBOFAT\0" (8 bytes)
[8..12]   version:    u32 LE = 1
[12..16]  arch_count: u32 LE
[16..]    entries:    arch_count * 32-byte ArchEntry
          padding:    0-pad to 16-byte boundary from file start
          slices:     LZ4-compressed krbo bytes
```

## ArchEntry (32 bytes)

| Offset | Field | Type | Values |
|--------|-------|------|--------|
| 0 | arch_id | u32 LE | 1=x86_64, 2=arm64 |
| 4 | compression | u32 LE | 0=none, 1=lz4 |
| 8 | offset | u64 LE | byte offset from file start |
| 16 | compressed_size | u64 LE | |
| 24 | uncompressed_size | u64 LE | |

## Detection

Fat binaries start with `KRBOFAT\0`. Since this begins with `KRBO`,
fat detection MUST be checked before single-arch `KRBO` detection.
`parse_krbo_header` rejects fat binaries with a clear error message.
