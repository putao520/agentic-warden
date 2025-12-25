# Graphics Programming Standards - CODING-STANDARDS-GRAPHICS

**Version**: 2.0.0
**Scope**: Graphics programming roles (2D/3D rendering/GPU programming/Shaders, API agnostic)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (Inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - Rendering pipeline must comply with SPEC definitions
       - Lighting, materials, post-processing based on SPEC

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Existing shader fully matches → Reuse directly
       - Partial match → Delete and rewrite

Iron Law 3: Prohibit Incremental Development
       - Prohibit adding new features to old shaders
       - Prohibit retaining compatibility code paths

Iron Law 4: Context7 Research First
       - Use mature rendering frameworks
       - Prohibit self-implementing complex algorithms like PBR/GI
```

---

## 🎨 Rendering Pipeline

### Pipeline Organization
- ✅ Separate rendering stages (geometry/lighting/post-processing)
- ✅ State sorting (reduce state switches)
- ✅ Batch sorting (material/texture/shader)
- ✅ Transparent objects rendered last
- ❌ Avoid frequent rendering state switches

### Coordinate Systems
- ✅ Clear coordinate system (left-handed/right-handed)
- ✅ Clear transformation matrix order
- ✅ Reasonable projection matrix parameters
- ✅ Correct frustum settings
- ❌ Avoid matrix multiplication order errors

### Depth and Culling
- ✅ Enable depth testing
- ✅ Backface culling
- ✅ Frustum culling
- ✅ Occlusion culling
- ✅ Early depth testing (Early-Z)
- ❌ Avoid overdraw

---

## 🖌️ Shader Development

### Shader Design
- ✅ Vertex shader: Transformation and lighting calculation
- ✅ Fragment shader: Texture sampling and color calculation
- ✅ Compute shader: General GPU computation
- ✅ Geometry shader: Dynamic geometry generation (use sparingly)
- ❌ Avoid complex calculations in fragment shader

### Shader Optimization
- ✅ Reduce branches (if statements)
- ✅ Vectorized operations (vec4, mat4)
- ✅ Precompute constants
- ✅ Texture sampling optimization (Mipmaps)
- ✅ Precision declaration (lowp/mediump/highp)
- ❌ Avoid loops (or limit iteration count)

### Shader Compilation
- ✅ Compile-time error checking
- ✅ Shader variant management
- ✅ Uber Shader vs specialized shaders
- ✅ Shader precompilation
- ❌ Avoid runtime compilation errors

---

## 🗂️ Resource Management

### Buffer Management
- ✅ Vertex buffer objects (VBO)
- ✅ Index buffer objects (IBO)
- ✅ Uniform buffer objects (UBO)
- ✅ Instance buffers
- ✅ Buffer reuse (object pooling)
- ❌ Avoid frequent creation and destruction

### Texture Management
- ✅ Texture compression (DXT/ETC/ASTC)
- ✅ Mipmaps generation
- ✅ Texture filtering (bilinear/trilinear/anisotropic)
- ✅ Texture atlases
- ✅ Stream large texture loading
- ❌ Avoid uncompressed textures

### Memory Management
- ✅ Monitor video memory usage
- ✅ Timely release unused resources
- ✅ LOD (Level of Detail)
- ✅ Asynchronous resource loading
- ❌ Avoid video memory leaks

---

## ⚡ Performance Optimization

### Batching
- ✅ Instanced rendering
- ✅ Batch merging
- ✅ Reduce Draw Calls
- ✅ Indirect drawing
- ❌ Avoid rebuilding buffers every frame

### GPU Optimization
- ✅ Asynchronous computing
- ✅ GPU profiling
- ✅ Pixel fill rate optimization
- ✅ Bandwidth optimization
- ✅ Register pressure management
- ❌ Avoid GPU stalls

### CPU-GPU Synchronization
- ✅ Double/triple buffering
- ✅ Asynchronous resource upload
- ✅ Reduce CPU-GPU synchronization points
- ✅ Command buffer pre-recording
- ❌ Avoid pipeline stalls

---

## 💡 Lighting and Materials

### Lighting Models
- ✅ PBR (Physically Based Rendering)
- ✅ Deferred rendering vs forward rendering
- ✅ Shadow mapping
- ✅ Ambient occlusion (AO)
- ✅ Global illumination (GI) approximation
- ❌ Avoid too many light sources (performance)

### Material Systems
- ✅ Material property parameterization
- ✅ Material instancing
- ✅ Material LOD
- ✅ Material batching
- ❌ Avoid duplicate materials

### Shadow Optimization
- ✅ Cascaded shadow maps (CSM)
- ✅ Reasonable shadow resolution
- ✅ Soft shadows (PCF)
- ✅ Shadow distance limits
- ❌ Avoid full-scene shadows

---

## 🎞️ Post-Processing

### Post-Processing Pipeline
- ✅ HDR rendering
- ✅ Tone mapping
- ✅ Gamma correction
- ✅ Anti-aliasing (FXAA/TAA/MSAA)
- ✅ Bloom
- ❌ Avoid excessive post-processing (performance)

### Framebuffers
- ✅ Off-screen rendering
- ✅ Multiple render targets (MRT)
- ✅ Framebuffer reuse
- ✅ Depth/stencil buffers
- ❌ Avoid unnecessary framebuffers

---

## 🖥️ Multi-Platform Compatibility

### API Abstraction
- ✅ Abstract rendering interface
- ✅ Platform-specific optimizations
- ✅ Cross-platform shader compilation
- ✅ Resource format compatibility
- ❌ Avoid hardcoded API calls

### Performance Tiering
- ✅ Adjust quality based on hardware
- ✅ Auto-detect GPU capabilities
- ✅ Configurable rendering options
- ✅ Mobile-specific optimizations
- ❌ Avoid one-size-fits-all configuration

---

## 🧪 Debugging and Profiling

### Debugging Tools
- ✅ Graphics debuggers (RenderDoc/NSight)
- ✅ Wireframe mode
- ✅ Normal/UV visualization
- ✅ Depth/shadow buffer visualization
- ✅ Shader hot reload
- ❌ Don't rely on printf debugging

### Performance Analysis
- ✅ GPU profiler
- ✅ Draw call statistics
- ✅ Frame time analysis
- ✅ Memory usage monitoring
- ✅ Bottleneck identification (CPU/GPU)
- ❌ Avoid premature optimization

---

## 📋 Graphics Programming Checklist

- [ ] Rendering state sorting and batch optimization
- [ ] Depth testing and backface culling
- [ ] Shader optimization (reduce branches and loops)
- [ ] Texture compression and mipmaps
- [ ] Instancing and batching (reduce Draw Calls)
- [ ] Lighting and shadow optimization
- [ ] Post-processing reasonable configuration
- [ ] Resource management (video memory monitoring)
- [ ] Multi-platform compatibility
- [ ] Performance analysis and debugging

---

---

## 🏛️ Advanced Graphics Architecture (20+ years experience)

### Modern Rendering Pipeline
```
Deferred Rendering:
- G-Buffer structure design
- Deferred lighting
- Decouple lighting and geometry
- Applicable: Many light sources

Forward+ (Forward Plus):
- Light clustering
- Light culling
- Transparent object support
- Applicable: Mobile-friendly

Hybrid Rendering:
- Deferred + forward transparency
- Visibility buffer
- Virtual textures

Ray Tracing:
- DXR/Vulkan RT
- BVH acceleration structures
- Hybrid rendering (ray tracing + rasterization)
- Denoising algorithms
```

### GPU-Driven Rendering
```
Indirect Drawing:
- Indirect Draw
- Multi-Draw Indirect
- GPU culling

Programmable Rendering Pipeline:
- Mesh Shader
- Task Shader
- Meshlet geometry

GPU Computing:
- Compute shader general computing
- GPU particle system simulation
- Physics simulation
- GPU-based occlusion culling
```

### Large World Rendering
```
LOD Systems:
- Discrete LOD
- Continuous LOD (CLOD)
- Geometric LOD + texture LOD

Streaming:
- Terrain streaming
- Virtual textures
- Memory management
- Background loading

Spatial Data Structures:
- Quadtree/Octree
- BVH
- Spatial hashing
- Hierarchical culling
```

---

## 🔧 Essential Skills for Senior Graphics Experts

### Shader Deep Optimization
```
ALU Optimization:
- MAD (multiply-add fusion)
- Vector operations
- Reduce scalar operations
- Constant folding

Memory Access Optimization:
- Texture cache locality
- Coalesced memory access
- Reduce bandwidth
- Tile-based architecture optimization (mobile)

Branch Optimization:
- Avoid divergent branches
- Use step/lerp instead of if
- Precompute LUT
- Uber shader variant management
```

### Performance Analysis Deep
```
GPU Profilers:
- RenderDoc
- NVIDIA NSight
- PIX for Windows
- Xcode GPU Tools

Analysis Methods:
- Find bottleneck: Vertex/pixel/bandwidth
- Frame breakdown analysis
- Hotspot identification
- A/B comparison testing

Common Bottlenecks:
- Overdraw
- State switches
- Texture bandwidth
- Vertex processing
```

### Modern API Techniques
```
Vulkan/DX12 Advantages:
- Multi-threaded command recording
- Explicit resource management
- Pipeline state objects
- Descriptor sets/tables

Synchronization and Barriers:
- Resource transition barriers
- Queue families
- Semaphores
- Fences

Memory Management:
- Memory type selection
- Sub-allocation
- Memory aliasing
- Resource heaps
```

### Advanced Lighting Techniques
```
Global Illumination:
- Ray tracing GI
- Voxel GI (VXGI)
- Screen space GI (SSGI)
- Irradiance probes

Shadow Techniques:
- Cascaded shadow maps (CSM)
- Percentage-closer soft shadows (PCSS)
- Shadow volumes
- Ray traced shadows

Reflections:
- Screen space reflections (SSR)
- Ray traced reflections
- Reflection probes
- Planar reflections
```

---

## 🚨 Common Pitfalls for Senior Graphics Experts

### Architecture Traps
```
❌ Premature optimization:
- Optimize without profiling
- Optimize non-bottlenecks
- Correct: Profile first

❌ Don't consider worst case:
- Only test simple scenes
- Ignore extreme cases
- Correct: Stress testing

❌ API abstraction too thick:
- Over-encapsulation
- Performance loss
- Correct: Thin abstraction, direct access
```

### Performance Traps
```
❌ Too many Draw Calls:
- No batching
- Frequent state switches
- Correct: Instancing, batching

❌ Excessive post-processing:
- All effects enabled
- Full resolution processing
- Correct: Downsampling, on-demand enable

❌ Ignore mobile:
- Directly use PC optimizations on mobile
- Ignore tile-based
- Correct: Platform-specific optimizations
```

### Compatibility Traps
```
❌ Hardcoded extensions:
- Assume extension exists
- No fallback path
- Correct: Capability detection

❌ Ignore driver differences:
- Only test on one GPU vendor
- Ignore driver bugs
- Correct: Multi-vendor testing

❌ Floating-point precision issues:
- Large coordinate precision loss
- World space calculations
- Correct: Camera-relative rendering
```

---

## 📊 Performance Monitoring Metrics

| Metric | Target | Alert Threshold | Measurement Tool |
|--------|--------|-----------------|------------------|
| Frame Rate | 60 FPS | < 30 FPS | GPU Profiler |
| Frame Time | < 16.7ms | > 33ms | GPU Profiler |
| Draw Calls | < 2000 | > 5000 | Engine stats |
| Triangle Count | Scene-based | > Budget | Engine stats |
| Overdraw | < 2x | > 4x | GPU Profiler |
| Video Memory Usage | < 80% | > 95% | GPU monitoring |
| Texture Memory | Budget-based | > Budget | Engine stats |
| Vertex Processing Time | < 5ms | > 10ms | GPU Profiler |
| Pixel Processing Time | < 10ms | > 16ms | GPU Profiler |
| State Switches | < 1000 | > 3000 | Engine stats |

---

## 📋 Graphics Programming Checklist (Complete Version)

### Rendering Pipeline
- [ ] Rendering architecture choice reasonable
- [ ] State sorting optimization
- [ ] Culling strategy complete
- [ ] LOD system working properly

### Performance Optimization
- [ ] Draw Calls within budget
- [ ] Batching/instancing
- [ ] Texture compression
- [ ] Shader optimization

### Visual Quality
- [ ] Lighting model correct
- [ ] Shadow quality met
- [ ] Post-processing reasonable configuration
- [ ] Anti-aliasing effective

### Cross-Platform
- [ ] Multi-GPU vendor testing
- [ ] Mobile optimization
- [ ] Capability detection and fallback

---

**Graphics Programming Principles Summary**:
Pipeline Optimization, State Sorting, Batching, Shader Optimization, Texture Management, Lighting and Shadows, Post-Processing, Resource Management, Multi-Platform, Performance Analysis
