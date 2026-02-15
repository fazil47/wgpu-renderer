# WESL Guide

Based on wesl-lang.dev.

## What is WESL?
WESL (WebGPU Extended Shading Language) is a superset of WGSL that adds features for modularity and conditional compilation. It compiles down to standard WGSL.

## Key Features

### Imports
Split shaders into multiple files.
```wgsl
import package::utils::math;
```

### Conditional Compilation
Configure shaders at compile time.
```wgsl
@if(DEBUG) {
    // Debug code
}
```

### Shader Libraries
Share code via package managers.

## Basic Usage
```wgsl
import package::colors::chartreuse;
import random_wgsl::pcg_2u_3f;

fn random_color(uv: vec2u) -> vec3f {
  var color = pcg_2u_3f(uv);
  @if(DEBUG) color = chartreuse;
  return color;
}
```
