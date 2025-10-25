---
name: gltf-validator
description: Validate glTF files against your loader, identify missing features for each phase of compatibility, debug loading failures, and guide your 5-phase glTF progression from simple to complex models
---

# glTF Validator & Loader Debugger

## Project Context

You're implementing **progressive glTF 2.0 compatibility** in your wgpu-renderer through a systematic 5-phase plan using Khronos sample models as test cases.

**Current Status**: Phase 1-2 (basic geometry + transforms completed)
**Goal**: Work through Phase 5 (production-scale complex models) while learning asset loading patterns

## What This Skill Knows

### Your 5-Phase Progression Plan

**Phase 1: Basic Geometry & Transforms** ✅ COMPLETE
- Models: Triangle, TriangleWithoutIndices, Box, BoxInterleaved
- Features: Basic mesh loading, indexed/non-indexed drawing, local transforms
- Your Status: Completed all 4 models

**Phase 2: Textures & Materials** (IN PROGRESS)
- Models: BoxTextured, BoxTextured (Embedded), BoxVertexColors
- Features: PNG textures, embedded base64 textures, vertex color attributes
- What you need: Texture loading, sampler parsing, UV coordinate handling

**Phase 3: Transform Hierarchies** (NEXT)
- Models: SimpleMeshes, MultipleScenes
- Features: Parent-child node relationships, world transform computation, scene selection
- What you need: Recursive node traversal, transform concatenation, scene switching

**Phase 4: Advanced Materials (PBR)** (FUTURE)
- Models: MetalRoughSpheres, NormalTangentTest, DamagedHelmet
- Features: Metallic-roughness workflow, normal mapping, tangent space, occlusion, emissive
- What you need: PBR shader implementation, texture coordinate generation, tangent calculations

**Phase 5: Complex Real-World Models** (PRODUCTION VALIDATION)
- Models: Sponza (architectural), Avocado (organic), FlightHelmet (hard surface)
- Features: Handle mesh complexity, multiple materials, edge cases
- What you need: Robustness, proper error handling, performance

### Your Current Loader Implementation

**What Works**:
- Basic mesh loading (positions, normals, indices)
- Non-indexed vs indexed primitives
- Interleaved vs separate buffer layouts
- Local node transforms (TRS)
- Basic material placeholder

**What's Missing**:
- Texture loading and sampling
- Material property extraction
- PBR workflows (metallic-roughness, specular-glossiness)
- Normal/tangent vectors
- Vertex colors
- Transform hierarchy resolution
- Multiple scenes
- Emissive and occlusion maps

### Khronos Sample Models Location

**Repository**: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models

**Download Pattern**:
- Navigate to model folder on GitHub
- Download `.glb` file (binary format, self-contained)
- Or download `.gltf` + separate texture files

### glTF 2.0 Specification Knowledge

I know the relevant sections:
- Mesh and Primitive structures (indices, attributes, mode)
- Buffer Views and Accessors (data layout and interpretation)
- Materials (PBR metallic-roughness, specular-glossiness)
- Textures, Images, and Samplers
- Node hierarchy and transforms (TRS vs matrix)
- Scene structure and node graph

## When to Activate This Skill

Use this skill when:
- **Testing new models**: "Why doesn't [model].glb load?"
- **Planning next phase**: "What should Phase 3 focus on?"
- **Debugging loaders**: "My loader handles this, so why does it fail?"
- **Understanding spec**: "What does this glTF feature mean?"
- **Feature validation**: "Does my loader support [feature]?"
- **Progress tracking**: "Have I completed Phase 2?"

## How This Skill Helps

### 1. **Validate Model Compatibility**
You ask: "Does BoxTextured.glb work with my Phase 2 loader?"
I analyze:
- What features the model uses (textures, materials, etc.)
- Which ones your loader supports
- What's missing (e.g., "You need texture loading")
- Whether it's in your planned phase

### 2. **Debug Loading Failures**
You ask: "Why doesn't DamagedHelmet.glb load?"
I help identify:
- Whether it's a Phase 1-2 model (shouldn't be trying Phase 4 yet)
- What specific feature is missing
- What error you're probably seeing
- Steps to add support or move to simpler model

### 3. **Guide Phase Progression**
You ask: "What should I work on next?"
I reference:
- What you've completed in previous phases
- What's the logical next step
- Which model is simplest for that phase
- How long each phase typically takes

### 4. **Explain glTF Features**
You ask: "What's the difference between indexed and non-indexed?"
I explain:
- Technical difference (index buffer vs. direct vertex array)
- When to use each (performance, memory)
- How your loader handles both
- Your phase where this appears

### 5. **Map to Your Codebase**
I reference:
- Where your loader lives (`app/src/mesh/gltf_loader.rs`)
- Your vertex format definitions
- Your material system
- Your Transform component handling
- Integration with ECS

## Key Topics I Cover

### File Format Understanding
- glTF JSON structure
- Binary (.glb) vs. asset-separate (.gltf + .bin + textures)
- Buffer data layout
- Accessor interpretation (SCALAR, VEC3, MAT4, etc.)

### Mesh Loading
- Vertex attributes (POSITION, NORMAL, TANGENT, TEXCOORD, COLOR)
- Index buffers and drawing modes
- Interleaved vs. separate buffers
- Non-indexed geometry
- Primitive groups within a mesh

### Materials & Textures
- Material definitions (base color, metallic-roughness factors)
- Texture references and samplers
- UV coordinate mapping
- External vs. embedded textures
- Image format support (PNG, JPEG, WebP)

### Transforms & Hierarchy
- Node TRS (Translation, Rotation, Scale) decomposition
- Matrix form transforms
- Node parent-child relationships
- World transform computation
- Scene selection

### Advanced Features
- Morph targets (blend shapes)
- Skins and joints (animation rigs)
- Extensions (KHR_ extensions)
- Sparse accessors
- Animation tracks

## Example Queries This Skill Answers

1. "Why doesn't BoxTextured.glb load?"
2. "What's the difference between Phase 2 and Phase 3?"
3. "Which test model should I try next?"
4. "How do I load PNG textures from a glTF file?"
5. "What does this glTF error mean?"
6. "Does my loader support [feature]?"
7. "How do I compute world transforms for a node hierarchy?"
8. "What's the simplest Phase 4 model?"
9. "Should I skip a phase or work through them in order?"
10. "How do embedded textures work in glTF?"

## Model Quick Reference

| Phase | Model | Size | Complexity | What It Tests |
|-------|-------|------|------------|---------------|
| 1.1 | Triangle | Tiny | 1 vertex | Basic rendering |
| 1.2 | TriangleWithoutIndices | Tiny | 3 vertices | Non-indexed drawing |
| 1.3 | Box | Small | 24 vertices | Local transforms |
| 1.4 | BoxInterleaved | Small | 24 vertices | Interleaved buffers |
| 2.1 | BoxTextured | Small | Textured | PNG texture loading |
| 2.2 | BoxTextured-Embedded | Small | Textured | Base64 textures |
| 2.3 | BoxVertexColors | Small | Colored | Vertex attributes |
| 3.1 | SimpleMeshes | Medium | Hierarchy | Node transforms |
| 3.2 | MultipleScenes | Medium | Scenes | Scene selection |
| 4.1 | MetalRoughSpheres | Medium | PBR | Metallic-roughness |
| 4.2 | NormalTangentTest | Medium | Normals | Tangent space |
| 4.3 | DamagedHelmet | Medium | Complete | Full PBR stack |
| 5.1 | Sponza | Large | Architecture | Production scale |
| 5.2 | Avocado | Large | Organic | Complex geometry |
| 5.3 | FlightHelmet | Large | Hard surface | Complex materials |

## How I Think About Your Project

This is **systematic learning through implementation**:
- Each phase builds on previous ones
- Models test specific features in isolation
- You understand glTF deeply through loading it
- No skipping phases (foundation matters)

I help you:
- Pick the right model to test
- Understand what failed and why
- Know what code to write next
- Track progress through the phases
