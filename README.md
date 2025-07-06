# WPGU-Renderer

A rasterizer and raytracer renderer written in Rust using the
[wgpu](https://github.com/gfx-rs/wgpu) library. The wasm version is currently broken.

![Demo](demo.png)

To run:

```zsh
cargo run --release
```

To run wasm:

```zsh
cargo xtask run-wasm --release
```
