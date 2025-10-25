---
name: wasm-validator
description: Diagnose WASM build failures, explain browser WebGPU constraints, help debug wasm-bindgen and web-sys integration issues, and guide getting your renderer working on the web again
---

# WASM Validator & Debugger

## Project Context

Your wgpu-renderer has a WebAssembly target that's currently broken (noted in README.md). You want to restore web support and understand WASM-specific rendering challenges.

**Current Status**: WASM build fails with getrandom dependency issues
**Goal**: Get the app running in browsers with full rendering capability

**WASM Stack**:
- Target: `wasm32-unknown-unknown`
- Build tool: `cargo xtask run-wasm --release`
- Binding layer: `wasm-bindgen`
- DOM/Canvas: `web-sys`
- Graphics: `wgpu` (WebGPU backend)
- Async runtime: `wasm-bindgen-futures`

## What This Skill Knows

### Your Current WASM Setup

**Build System** (`xtask/src/run_wasm.rs`):
- Compiles to `wasm32-unknown-unknown`
- Uses `wasm-bindgen` to generate JS bindings
- Serves compiled files with simple HTTP server
- Can build debug or release mode
- Supports `--no-serve` flag (just compile, don't serve)

**Dependencies with WASM Support**:
- `wgpu` - WebGPU backend works on WASM
- `wasm-bindgen` - JS/Rust interop
- `web-sys` - Web APIs (Canvas, Window, etc.)
- `wasm-bindgen-futures` - Async/await on WASM
- `web-time` - Timing on WASM (not std::time)
- `getrandom` - (needs "js" feature)

### Current Issue: getrandom Dependency

**Problem**:
```
error: the wasm*-unknown-unknown targets are not supported by default,
       you may need to enable the "js" feature
```

**Root Cause**:
- Some dependency uses `getrandom` for randomness
- `getrandom` requires explicit feature for WASM (can't use system randomness)
- Feature not enabled → compilation fails

**Solution Path**:
1. Find which crate depends on getrandom
2. Add `[target.'cfg(target_arch = "wasm32")'.dependencies]` in Cargo.toml
3. Enable `getrandom = { version = "0.2", features = ["js"] }`

### WASM-Specific Constraints You Need to Know

**Runtime Constraints**:
- Single-threaded (no parallelism)
- No native system APIs (must use web-sys)
- Limited memory (but typically 1GB available)
- Different async model (JavaScript event loop)

**Graphics Constraints**:
- WebGPU is newer, limited browser support
- Some wgpu features unavailable on WebGPU
- Performance bottlenecks differ from native
- No native debugging tools

**File I/O Constraints**:
- Can't load files from disk (security sandbox)
- Must embed assets or load from HTTP
- Texture loading needs special handling
- glTF models must be accessible via URL

**Precision Differences**:
- WebGPU may use different precision
- Floating-point math slightly different
- Shader compilation differs by browser

## When to Activate This Skill

Use this skill when:
- **Build fails**: "Why does my WASM build fail?"
- **Runtime errors**: "Why crashes in browser but works native?"
- **Feature issues**: "Can I use this in WebGPU?"
- **Performance problems**: "Why is this slow on WASM?"
- **Canvas/DOM integration**: "How do I access the canvas?"
- **Asset loading**: "How do I load textures in WASM?"
- **Debugging**: "How do I debug WASM in browser?"
- **Optimization**: "How do I reduce WASM binary size?"

## How This Skill Helps

### 1. **Diagnose Build Failures**
You ask: "Why does cargo xtask run-wasm fail?"
I help identify:
- Missing WASM feature flags
- Dependency version mismatches
- Unsupported syscalls
- Missing polyfills (for getrandom, etc.)

### 2. **Explain Browser Constraints**
You ask: "Why does this work on native but not WASM?"
I explain:
- What's different in WASM environment
- What web APIs are needed instead
- Browser compatibility concerns
- Performance implications

### 3. **Guide Feature Compatibility**
You ask: "Can I use this wgpu feature?"
I check:
- Is it supported in WebGPU?
- Does wgpu's WebGPU backend support it?
- Workarounds if unsupported
- Performance impact on web

### 4. **Debug Runtime Errors**
You ask: "I get this error in the browser console"
I help:
- Translate WASM error messages
- Suggest where problem likely is
- How to add debugging output
- Browser tools for diagnosis

### 5. **Optimize WASM Artifacts**
You ask: "My WASM binary is too large"
I suggest:
- Compile flags (release mode, opt-level)
- Dead code elimination
- LTO (Link Time Optimization)
- Size profiling tools

## Key Topics I Cover

### Build System
- Cargo targets (wasm32-unknown-unknown)
- Feature flags and their meanings
- Dependency resolution for WASM
- Cross-compilation quirks
- wasm-bindgen CLI options

### JavaScript Interop
- wasm-bindgen exports (functions, structs)
- Calling Rust from JavaScript
- Calling JavaScript from Rust
- Memory sharing (ArrayBuffer)
- Performance of Rust/JS calls

### Web APIs
- Canvas and WebGL/WebGPU
- Window, Document, DOM
- Event listeners
- RequestAnimationFrame
- Local storage

### WASM-Specific Rust
- cfg(target_arch = "wasm32") conditionals
- wasm-bindgen-futures for async
- web-time instead of std::time
- console_log! macro
- wasm-pack integration

### Debugging & Profiling
- Browser console (errors, logs)
- DevTools WASM debugging
- console.log from Rust
- Performance profiling
- Memory usage monitoring

### Performance
- WASM code size
- Runtime performance
- JavaScript boundary overhead
- Memory usage
- Compilation time

## Example Queries This Skill Answers

1. "Why does my WASM build fail with getrandom?"
2. "How do I load textures in the browser?"
3. "Why is my rendering slow on WASM?"
4. "Can I use this compute shader feature?"
5. "How do I debug WASM in the browser?"
6. "What's the difference between wasm32-unknown-unknown and wasm32-wasi?"
7. "How do I handle user input in WASM?"
8. "Why does this work native but not WASM?"
9. "How do I reduce my WASM binary size?"
10. "What browsers support WebGPU?"

## Current Build Error Context

**Full Error Message**:
```
error: the wasm*-unknown-unknown targets are not supported by default,
       you may need to enable the "js" feature.
       For more information see: https://docs.rs/getrandom/#webassembly-support
   --> /Users/fazilbabu/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/getrandom-0.2.15/src/lib.rs:342:9

error[E0433]: failed to resolve: use of unresolved module or unlinked crate `imp`
   --> /Users/fazilbabu/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/getrandom-0.2.15/src/lib.rs:398:9
```

**What This Means**:
- `getrandom` crate doesn't have WASM support enabled
- It's trying to find platform-specific RNG code (`imp` module)
- For WASM, must use JavaScript's crypto API

**How to Fix** (quick version):
1. Find what depends on getrandom (might be indirect)
2. Add to `Cargo.toml`:
   ```toml
   [target.'cfg(target_arch = "wasm32")'.dependencies]
   getrandom = { version = "0.2", features = ["js"] }
   ```
3. Or upgrade the crate that uses it (might have updated)

## Browser Support Status

**WebGPU Support** (as of Oct 2025):
- ✅ Chrome/Edge (experimental, behind flag)
- ✅ Firefox (experimental)
- ⚠️ Safari (limited, behind experimental flag)
- ❌ Mobile browsers (very limited support)

**Implications for Your Project**:
- Desktop browsers: WebGPU works fine
- Mobile: Fallback to WebGL (not supported in wgpu)
- Testing: Use latest Chrome/Firefox with flags enabled

## WASM Debugging Tips

**1. Browser Console Logging**:
```rust
// From Rust to console
console_log!("Debug message: {:?}", value);
```

**2. DevTools Debugging**:
- Open DevTools → Sources tab
- Find .wasm file
- Limited debugging (can't step through)

**3. Performance Profiling**:
- Use Performance tab in DevTools
- Check JavaScript boundary overhead
- Profile memory usage

**4. Asset Loading**:
- Put assets in web directory
- Load via fetch() or XMLHttpRequest
- Handle CORS properly

## How I Think About Your Project

WASM support is different from native rendering:
- Single-threaded changes optimization strategies
- Browser APIs replace system APIs
- Asset loading is fundamentally different
- Debugging requires different tools

I help you understand these differences and work within WASM constraints, not against them.

## References & Resources

- **WebGPU Spec**: https://gpuweb.github.io/gpuweb/
- **wasm-bindgen Book**: https://rustwasm.org/docs/wasm-bindgen/
- **wgpu WebGPU Backend**: https://github.com/gfx-rs/wgpu/tree/master/wgpu-core/src/backend/webgpu
- **getrandom WASM Support**: https://docs.rs/getrandom/#webassembly-support
- **WebAssembly Reference Manual**: https://webassembly.org/