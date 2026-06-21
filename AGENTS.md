# CLAUDE.md

## Key Commands

### Building and Testing

```bash
# Build and run in debug mode
cargo run

# Testing
cargo test

# Build and run in release mode 
cargo run --release

# Check compilation without running
cargo check

# Build for WebAssembly (experimental)
cargo xtask run-wasm
```

Reference image tests in `app/tests/basic.rs` save the actual rendered PNG
when an image comparison fails. The files are written to the gitignored
`app/tests/reference_image_failures/` directory using the reference image's
file name. This only happens after rendering reaches the reference comparison
helper; earlier panics such as adapter creation failures will not produce an
image.

## Important Notes

When working with this codebase:

- This is a learning project, don't make changes unless explicitly requested.
- Don't add unnecessary dependencies; This is supposed to be as self-contained as possible.
- Don't remove comments that were not added by you.
- Use `cargo check` frequently to catch compilation errors early
- Use `cargo clippy` to catch common mistakes and improve code quality
- Use `cargo test` to test after making changes
  - Make sure reference images were generated on the current machines, otherwise there will be image mismatches even for the same code.
