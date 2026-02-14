# Windows Build Guide

## Overview

The `vprogs` project can be built on Windows, but there are some platform-specific considerations.

## Cairo Integration

The Cairo integration in `transaction-runtime` works on Windows without any special configuration. You can build and test it with:

```powershell
cd transaction-runtime\transaction-runtime
cargo build
cargo test
```

## Known Issues

### jemalloc on Windows

The `jemalloc` allocator (used by RocksDB) does not build natively on Windows because it requires Unix build tools (`sh`, `bash`). This affects the full workspace build.

**Workaround**: The `jemalloc` feature has been removed from the default RocksDB configuration. The project will use the system allocator on Windows, which is perfectly functional.

### Building the Full Workspace

To build the full workspace on Windows:

```powershell
# Build without jemalloc (default)
cargo build --workspace

# Or build specific crates
cargo build -p vprogs-transaction-runtime
```

## Testing

All Cairo integration tests pass on Windows:

```powershell
cd transaction-runtime\transaction-runtime
cargo test --test cairo_integration
```

## Dependencies

- Rust toolchain (stable or nightly)
- Visual Studio Build Tools (for native dependencies)
- No additional Unix tools required

## Performance Notes

- Windows builds use the system allocator instead of jemalloc
- Performance difference is minimal for most use cases
- For production deployments, consider using WSL2 or Linux for optimal performance

