# WPGU-Renderer

A rasterizer and raytracer renderer written in Rust using the
[wgpu](https://github.com/gfx-rs/wgpu) library.

![Demo](demo.png)

To run:

```zsh
cargo run --release
```

To run integration tests:

```zsh
cargo test --release
```

On macOS, tests run headless using the native Metal backend. In Linux containers
without a GPU, install [llvmpipe](https://docs.mesa3d.org/drivers/llvmpipe.html)
(Mesa's software Vulkan renderer) to provide a backend:

```zsh
# Debian/Ubuntu
apt-get install mesa-vulkan-drivers

# Alpine
apk add mesa-vulkan-gallium

# Fedora
dnf install mesa-vulkan-drivers
```

To run wasm:

```zsh
cargo xtask run-wasm --release
```
