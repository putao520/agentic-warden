# Multimedia Development Standards - CODING-STANDARDS-MULTIMEDIA

**Version**: 2.0.0
**Scope**: Audio/video development roles (audio processing/video codec/streaming, technology-agnostic)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - Codec configurations must comply with SPEC definitions
       - Format, bitrate, resolution based on SPEC

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Existing pipeline fully matches → Direct reuse
       - Partial match → Destroy and rebuild

Iron Law 3: Prohibit Incremental Development
       - Prohibit adding new features to old pipelines
       - Prohibit keeping compatible formats

Iron Law 4: Context7 Research First
       - Use mature codec libraries (FFmpeg)
       - Prohibit implementing codec algorithms yourself
```

---

## 🎵 Audio Processing

### Sampling and Formats
- ✅ Reasonable sample rates (44.1kHz/48kHz)
- ✅ Clear bit depth (16bit/24bit/32bit float)
- ✅ Channel count configuration (mono/stereo/multi-channel)
- ✅ Format conversion quality control
- ❌ Avoid sample rate mismatch (distortion)

### Audio Buffering
- ✅ Reasonable buffer size (latency vs stability)
- ✅ Ring Buffer
- ✅ Buffer underflow/overflow handling
- ✅ Double buffering/triple buffering
- ❌ Avoid audio stuttering (Buffer Underrun)

### Audio Effects
- ✅ Mixing algorithms
- ✅ Equalizer (EQ)
- ✅ Compressor
- ✅ Reverb
- ✅ 3D audio positioning
- ❌ Avoid clipping distortion

---

## 🎥 Video Processing

### Codec
- ✅ Encoder selection (H.264/H.265/VP9/AV1)
- ✅ Bitrate control (CBR/VBR)
- ✅ Keyframe interval (GOP)
- ✅ Encoding presets (speed vs quality)
- ✅ Hardware acceleration (GPU/dedicated chips)
- ❌ Avoid decoding failures

### Frame Processing
- ✅ Standard frame rates (24/25/30/60fps)
- ✅ Resolution scaling (interpolation algorithms)
- ✅ Color space conversion (RGB/YUV)
- ✅ Deinterlacing
- ❌ Avoid frame drops

### Video Filters
- ✅ Color correction
- ✅ Sharpening/blur
- ✅ Cropping/rotation
- ✅ Watermarks/subtitles
- ❌ Avoid overprocessing (quality loss)

---

## ⏱️ Synchronization and Timing

### Audio-Video Sync
- ✅ PTS (Presentation Timestamp)
- ✅ DTS (Decoding Timestamp)
- ✅ Unified audio-video time base
- ✅ Drift compensation
- ✅ Lip Sync
- ❌ Avoid audio-video desynchronization

### Clock Management
- ✅ Use system clock or media clock
- ✅ Clock synchronization (NTP)
- ✅ Monotonically increasing timestamps
- ✅ Playback speed control
- ❌ Avoid timestamp jumps

---

## 📡 Streaming

### Streaming Protocols
- ✅ HLS/DASH/RTMP/WebRTC
- ✅ Adaptive Bitrate (ABR)
- ✅ Reasonable chunk size (2-10 seconds)
- ✅ Preloading strategy
- ❌ Avoid stream interruptions

### Network Transport
- ✅ Buffering strategy (reduce stuttering)
- ✅ Retransmission mechanism
- ✅ Congestion control
- ✅ Bandwidth estimation
- ✅ Packet loss recovery (FEC)
- ❌ Avoid buffer overflow

### Low Latency
- ✅ Real-time transport (RTP/RTSP)
- ✅ Reduce buffering time
- ✅ I-frame optimization
- ✅ Network optimization (UDP)
- ❌ Avoid latency accumulation

---

## 🗂️ Containers and Formats

### Container Formats
- ✅ MP4/MKV/AVI/WebM
- ✅ Multi-track support (audio/video/subtitles)
- ✅ Metadata
- ✅ Indexing (Seeking)
- ❌ Avoid container corruption

### Format Conversion
- ✅ Lossless conversion
- ✅ Transcoding quality control
- ✅ Batch processing
- ✅ Progress monitoring
- ❌ Avoid format incompatibility

---

## ⚡ Performance Optimization

### Codec Optimization
- ✅ Hardware acceleration (NVENC/QSV/AMF)
- ✅ Multi-threaded parallelization
- ✅ SIMD optimization
- ✅ Zero-Copy
- ❌ Avoid CPU overload

### Memory Management
- ✅ Buffer pooling
- ✅ Timely release of decoded frames
- ✅ Memory alignment
- ✅ Monitor memory usage
- ❌ Avoid memory leaks

### Real-time Processing
- ✅ Frame processing time budget
- ✅ Priority scheduling
- ✅ Frame skipping (drop frame strategy)
- ✅ Latency monitoring
- ❌ Avoid processing latency accumulation

---

## 🔒 DRM and Copyright

### Content Protection
- ✅ DRM integration (Widevine/FairPlay/PlayReady)
- ✅ Encrypted transmission (HTTPS)
- ✅ License verification
- ✅ Screen recording protection (HDCP)
- ❌ Prohibit plaintext transmission of protected content

### Watermarks
- ✅ Digital watermark embedding
- ✅ Invisible watermarks
- ✅ Tamper resistance
- ❌ Avoid affecting video quality

---

## 🎛️ Player Development

### Playback Control
- ✅ Play/pause/stop
- ✅ Progress seeking
- ✅ Playback speed adjustment
- ✅ Volume control
- ✅ Fullscreen toggle
- ❌ Avoid unresponsive operations

### State Management
- ✅ Player state machine
- ✅ Error handling and retry
- ✅ Buffer status display
- ✅ Network disconnect recovery
- ❌ Avoid state confusion

### User Experience
- ✅ Fast startup (first frame <1s)
- ✅ Smooth playback (no stuttering)
- ✅ Smart preloading
- ✅ Friendly error messages
- ❌ Avoid black screen/freezing

---

## 🧪 Testing

### Functional Testing
- ✅ Codec correctness
- ✅ Multi-format compatibility
- ✅ Audio-video synchronization
- ✅ Seeking accuracy
- ✅ Boundary conditions (empty files/corrupted files)
- ❌ Don't skip boundary testing

### Performance Testing
- ✅ Codec performance
- ✅ Memory usage
- ✅ CPU/GPU utilization
- ✅ Latency testing
- ✅ Stress testing (long playback)
- ❌ Avoid performance regression

### Compatibility Testing
- ✅ Multi-device testing
- ✅ Multi-platform testing
- ✅ Multi-browser testing (web players)
- ✅ Network environment testing
- ❌ Don't only test in ideal environments

---

## 📋 Audio-Video Development Checklist

- [ ] Correct audio sample rate and format
- [ ] Reasonable video codec configuration
- [ ] Audio-video synchronization (PTS/DTS)
- [ ] Streaming buffering and adaptive bitrate
- [ ] Hardware acceleration and performance optimization
- [ ] DRM and copyright protection (if needed)
- [ ] Player state management and error handling
- [ ] Multi-format compatibility testing
- [ ] Performance and stress testing
- [ ] User experience optimization (startup speed/smoothness)

---

**Summary of Audio-Video Development Principles**:
Format standards, codec optimization, audio-video synchronization, streaming transport, performance optimization, DRM protection, player experience, compatibility testing, real-time processing, buffering strategies
