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

### Project Structure

- `app/` - Main application crate
- `ecs/` - Custom ECS crate
- `maths/` - Custom maths crate
- `xtask/` - Build automation scripts
- `assets/` - 3D models and textures
- `static/` - Web deployment files

## Important Notes

When working with this codebase:

- Don't add unnecessary dependencies; This is a learning project that's supposed
  to be as self-contained as possible.
- Make sure project structure in `CLAUDE.md` stays up-to-date
- Don't add frivolous comments
- Use `cargo check` frequently to catch compilation errors early
- Use `cargo clippy` to catch common mistakes and improve code quality
- WGPU resources often need explicit label names for debugging
- See `CLAUDE-DECISIONS.md` for architectural decision history and rationale
- Check `./claude-diagrams` for diagrams that explain the architecture and
  design, but only if explicitly requested.
- When working on complex features that requies research, follow the
  explore-plan-code-commit workflow mentioned in the
  [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices)
  to ensure a structured approach:
  - **Explore**: Research and understand the problem. While doing this keep a
    record of learning resources in `CLAUDE-RESOURCES.md` for me to read and
    review later.
  - **Plan**: Outline your approach and design
  - **Code**: Implement the solution and keep a record of decisions made in
    `CLAUDE-DECISIONS.md`
  - **Commit**: Commit changes with clear messages
