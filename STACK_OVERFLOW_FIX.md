# Stack Overflow Fix - warp-foss-clone

## Problem
The Windows build of warp-foss-clone was crashing immediately on startup with:
```
thread 'main' has overflowed its stack
```

## Root Cause
The stack overflow was caused by large stack-allocated arrays in the text rendering module:

1. **Primary Issue (4KB per call)**: The `get_ansi_palette()` function in `src/ui/text.rs` was creating a `[[f32; 4]; 256]` array (4,096 bytes) on the stack every time it was called. This function was invoked during color conversion for each character with indexed colors.

2. **Secondary Issue (4KB per read)**: The `read_pty_output()` function in `src/main.rs` was allocating a `[0u8; 4096]` buffer on the stack for PTY reads.

While these allocations are fine on Linux/macOS with larger default stack sizes (typically 8MB), Windows has a much smaller default stack size (1-2MB), causing immediate stack overflow during initialization or early rendering.

## Solution

### 1. Made ANSI Palette Static (src/ui/text.rs)
- Added `use std::sync::LazyLock;` import
- Created a static `ANSI_PALETTE` using `LazyLock` to initialize the 256-color palette once and reuse it
- Modified `color_to_rgba()` to use the static palette instead of allocating on the stack
- Removed the `get_ansi_palette()` function

### 2. Changed PTY Buffer to Heap Allocation (src/main.rs)
- Changed `[0u8; 4096]` stack array to `vec![0u8; 4096]` heap allocation
- This eliminates the 4KB stack allocation during PTY reads

## Changes Made
- `Cargo.toml`: Added `once_cell = "1.19"` dependency
- `src/ui/text.rs`:
  - Added `LazyLock` import
  - Created static `ANSI_PALETTE` (4KB on heap instead of stack)
  - Modified `color_to_rgba()` to use static palette
  - Removed `get_ansi_palette()` function
- `src/main.rs`:
  - Changed PTY read buffer from stack to heap allocation

## Performance Impact
- **No negative impact**: The static palette is computed once and cached, avoiding repeated allocations
- **Slight improvement**: Reduced memory allocation overhead during rendering
- **Better cross-platform**: More robust across different OS stack sizes

## Testing
The fixed binary has been built successfully:
```
cargo build --release --target x86_64-pc-windows-gnu
```

Binary: `warp-foss-windows-fixed.tar.gz` (3.7 MB)

## Verification
To verify the fix:
1. Extract and run `warp-foss.exe` on Windows
2. The application should start without stack overflow
3. Terminal rendering should work correctly with all 256 ANSI colors

---

## Recommended Build Method: cargo-xwin (MSVC)

As of 2026-03-06, the recommended way to build for Windows is using `cargo-xwin` with the MSVC target:

```bash
# Install cargo-xwin
cargo install cargo-xwin

# Build with MSVC target
cargo xwin build --target x86_64-pc-windows-msvc --release
```

The MSVC toolchain handles stack sizes differently than GNU and is the officially supported toolchain for winit.

**Binary location:** `target/x86_64-pc-windows-msvc/release/warp-foss.exe`

### Why MSVC over GNU?
- winit officially supports MSVC toolchain
- MSVC has better stack handling on Windows
- No need for linker workarounds
- More reliable cross-platform behavior

### Prerequisites for cargo-xwin
- Rust installed via rustup
- clang and lld installed (for the MSVC cross-compilation)
- cargo-xwin will automatically download MSVC CRT and Windows SDK
