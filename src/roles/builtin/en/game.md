# Game Development Standards - CODING-STANDARDS-GAME

**Version**: 2.0.0
**Scope**: Game development roles (2D/3D/mobile/PC/console games, engine-agnostic)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - Game mechanics must comply with SPEC definitions
       - Values, rules, interactions based on SPEC

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Existing system fully matches → Direct reuse
       - Partial match → Destroy and rebuild

Iron Law 3: Prohibit Incremental Development
       - Prohibit adding new features to old systems
       - Prohibit keeping compatibility code

Iron Law 4: Context7 Research First
       - Use mature game frameworks and plugins
       - Prohibit implementing physics/rendering engines yourself
```

---

## 🎮 Game Loop and Timing

### Main Loop Design
- ✅ Fixed timestep for game logic updates
- ✅ Variable timestep for rendering
- ✅ Pass Delta Time to all needed systems
- ✅ Frame-rate independent game logic
- ❌ Avoid binding game logic to frame rate

### Time Management
- ✅ Use game time not real time
- ✅ Support time scaling (slow motion/fast forward)
- ✅ Stop game logic updates when paused
- ✅ Unified time system for countdowns and timers
- ❌ Avoid hardcoded delay values

### Performance Targets
- ✅ Clear target frame rate (60fps/30fps)
- ✅ Per-frame time budget (16.6ms@60fps)
- ✅ Monitor frame time variance
- ✅ Graceful degradation on frame drops
- ❌ Avoid single-frame timeouts (stuttering)

---

## 🏗️ Architecture Patterns

### Component-Based Design
- ✅ Entity-Component-System (ECS) or component pattern
- ✅ Single responsibility for components
- ✅ Loose coupling between components
- ✅ Communicate via messages/events
- ❌ Avoid direct references between components

### State Management
- ✅ Use state machines for game states
- ✅ Clear state transition conditions
- ✅ Hierarchical state machines (nested states)
- ✅ Clear state enter/exit logic
- ❌ Avoid complex conditional judgments

### Scene Management
- ✅ Asynchronous scene loading/unloading
- ✅ Show loading screen during scene transitions
- ✅ Resource preloading
- ✅ Memory management (unload unused resources)
- ❌ Avoid stuttering during scene switches

---

## 🎨 Resource Management

### Resource Loading
- ✅ Asynchronous resource loading
- ✅ Resource pooling (object pools, audio pools)
- ✅ Lazy load non-critical resources
- ✅ Level-of-detail (LOD) loading
- ✅ Resource reference counting
- ❌ Avoid synchronous blocking loads

### Resource Optimization
- ✅ Texture compression and mipmaps
- ✅ Audio compression
- ✅ Model optimization (face count, bone count)
- ✅ Resource packaging and compression
- ✅ Mobile resource resolution adaptation
- ❌ Avoid uncompressed raw resources

### Memory Management
- ✅ Timely release of unused resources
- ✅ Memory pools to reduce allocation overhead
- ✅ Monitor memory usage
- ✅ Memory leak detection
- ❌ Avoid frequent allocation/deallocation

---

## 🕹️ Input Handling

### Input System
- ✅ Support multiple input devices (keyboard/mouse/gamepad/touch)
- ✅ Configurable input mapping
- ✅ Input buffering (prevent lost input)
- ✅ Combo keys and gesture recognition
- ✅ Input priority management (UI prioritized over game)
- ❌ Avoid hardcoded input keys

### Responsiveness
- ✅ Immediate feedback (key response <100ms)
- ✅ Input prediction (network games)
- ✅ Debouncing and accidental touch prevention
- ❌ Avoid input latency

---

## ⚙️ Physics and Collision

### Physics System
- ✅ Use fixed timestep for physics updates
- ✅ Separate physics and rendering
- ✅ Collision layers and collision matrix
- ✅ Separate triggers and colliders
- ❌ Avoid global collision detection every frame

### Collision Optimization
- ✅ Spatial partitioning (quadtree/octree/grid)
- ✅ Sleeping mechanism (stationary objects not calculated)
- ✅ Simplified colliders (use simple shapes)
- ✅ Layered detection (broad phase + narrow phase)
- ❌ Avoid complex polygon collisions

---

## 🎯 Game Logic

### Data-Driven Design
- ✅ Data-driven game configuration (JSON/YAML/tables)
- ✅ Separate level data from code
- ✅ Item/prop/skill data tables
- ✅ Hot-reload configuration data
- ❌ Avoid hardcoded game values

### Balance
- ✅ Parameterized balance formulas
- ✅ Balance testing and tuning
- ✅ Difficulty curve configuration
- ✅ Controlled randomness (seeds)
- ❌ Avoid magic numbers

### AI System
- ✅ Behavior trees or state machines
- ✅ Frame-rate limited AI decisions (not every frame)
- ✅ AI debugging visualization
- ✅ Graded AI complexity
- ❌ Avoid overly complex AI (affects performance)

---

## 🌐 Multiplayer Games

### Network Synchronization
- ✅ Client prediction and server validation
- ✅ Interpolation and extrapolation
- ✅ Lag compensation
- ✅ Snapshot synchronization
- ❌ Avoid trusting client data

### Anti-Cheat
- ✅ Server authority
- ✅ Server validation of critical logic
- ✅ Speed hacking detection
- ✅ Data encryption
- ❌ Prohibit direct client modification of critical data

---

## 💾 Save and Serialization

### Save System
- ✅ Auto-save and manual save
- ✅ Multiple save slots
- ✅ Save version control
- ✅ Backward compatibility with old saves
- ✅ Data validation (prevent corruption)
- ❌ Avoid plaintext storage (prevent tampering)

### Serialization
- ✅ Use mature serialization libraries
- ✅ Incremental serialization (save only changes)
- ✅ Compress save data
- ✅ Cross-platform compatibility
- ❌ Avoid serializing complex object graphs

---

## 🎵 Audio

### Audio Management
- ✅ Separate sound effects and music
- ✅ Grouped volume control (master/SFX/music/voice)
- ✅ Audio priority (limit concurrent playback)
- ✅ 3D audio positioning
- ❌ Avoid audio leaks (timely release)

### Optimization
- ✅ Streaming audio playback (long music)
- ✅ Compressed audio formats
- ✅ Sound effect preloading
- ❌ Avoid uncompressed WAV files

---

## 📊 Performance Optimization

### Rendering Optimization
- ✅ Batching and instancing
- ✅ Occlusion culling and frustum culling
- ✅ Level of Detail (LOD)
- ✅ Object pools (reduce instantiation)
- ✅ Draw Call optimization
- ❌ Avoid overdraw

### CPU Optimization
- ✅ Spread expensive operations across frames
- ✅ Multi-threading (physics/AI/loading)
- ✅ Cache calculation results
- ✅ Avoid lookups and traversals
- ❌ Avoid GC pressure (reduce allocations)

### Profiling
- ✅ Regular performance analysis
- ✅ Identify performance bottlenecks
- ✅ Target platform testing
- ✅ Memory and frame rate monitoring
- ❌ Don't prematurely optimize

---

## 🧪 Testing

### Test Coverage
- ✅ Unit testing (game logic)
- ✅ Integration testing (system interaction)
- ✅ Performance testing (frame rate/memory)
- ✅ Balance testing (values/difficulty)
- ✅ Compatibility testing (multi-platform/devices)

### Debugging Tools
- ✅ Console commands (for debugging)
- ✅ Visual debugging (colliders/paths)
- ✅ Cheat codes (fast testing)
- ✅ Logging and screenshots
- ❌ Remove debug code in release

---

## 📋 Game Development Checklist

- [ ] Frame-rate independent game logic
- [ ] Asynchronous resource loading and pooling
- [ ] Input system supports multiple devices
- [ ] Physics collision optimization (spatial partitioning)
- [ ] Data-driven game configuration
- [ ] Network synchronization and anti-cheat (multiplayer)
- [ ] Save system version compatibility
- [ ] Audio management and optimization
- [ ] Rendering and CPU optimization (target frame rate)
- [ ] Performance analysis and testing

---

**Summary of Game Development Principles**:
Frame-rate independence, component-based architecture, resource pooling, data-driven design, performance optimization, network synchronization, save compatibility, input responsiveness, physics optimization, debugging tools
