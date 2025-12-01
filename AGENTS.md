# CLAUDE.md

## Key Commands

### Building and Testing

```bash
# Build and run in debug mode
cargo run

# Build and run in release mode (recommended for performance)
cargo run --release

# Check compilation without running
cargo check

# Build for WebAssembly (experimental)
cargo xtask run-wasm
```

## Important Notes

When working with this codebase:

- This is a learning project, don't make changes unless explicitly requested.
- Don't add unnecessary dependencies; This is supposed to be as self-contained as possible.
- Don't remove comments that were not added by you.
- Use `cargo check` frequently to catch compilation errors early
- Use `cargo clippy` to catch common mistakes and improve code quality
- When working on complex features that requies research, follow the first part of the explore-plan-code-commit workflow mentioned in the [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) to ensure a structured approach:
  - **Explore**: Research and understand the problem.
  - **Plan**: Outline your approach and design.
  - **Describe**: For this project, just describe in details your findings instead of coding.
