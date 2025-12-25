# Embedded Development Standards - CODING-STANDARDS-EMBEDDED

**Version**: 2.0.0
**Scope**: Embedded development roles (MCU/SoC/RTOS/Bare-metal, platform agnostic)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (Inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - Hardware interfaces must comply with SPEC definitions
       - Timing, protocols, resource constraints based on SPEC

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Existing driver fully matches → Reuse directly
       - Partial match → Delete and rewrite

Iron Law 3: Prohibit Incremental Development
       - Prohibit adding new features to old drivers
       - Prohibit retaining compatibility code

Iron Law 4: Context7 Research First
       - Use official HAL/SDK
       - Prohibit self-implementing communication protocol stacks
```

---

## 💾 Resource Management

### Memory Management
- ✅ Static allocation priority (stack/global)
- ✅ Avoid dynamic allocation (malloc/free)
- ✅ Stack size reasonably configured
- ✅ Memory alignment
- ✅ Monitor memory usage (stack/heap)
- ❌ Prohibit infinite recursion (stack overflow)

### Code Optimization
- ✅ Code size optimization (-Os)
- ✅ Use constants and macros (save RAM)
- ✅ Only link needed libraries
- ✅ Remove unused code
- ✅ Inline critical functions
- ❌ Avoid over-optimization (readability)

### Data Storage
- ✅ Distinguish RAM/ROM/Flash storage
- ✅ Constants in ROM (const/PROGMEM)
- ✅ Flash write count limits
- ✅ EEPROM wear leveling
- ❌ Avoid frequent Flash writes

---

## ⚡ Real-Time

### Interrupt Handling
- ✅ Interrupt service routines as short as possible
- ✅ Deferred processing (bottom-half mechanism)
- ✅ Prohibit blocking in ISR
- ✅ Prohibit memory allocation in ISR
- ✅ Interrupt priority reasonably configured
- ❌ Avoid deep interrupt nesting

### Task Scheduling
- ✅ Real-time tasks with high priority
- ✅ Clear task periods
- ✅ Worst-case execution time (WCET) analysis
- ✅ Avoid priority inversion
- ✅ Use preemptive scheduling (RTOS)
- ❌ Avoid task starvation

### Timing Guarantees
- ✅ Hard real-time task deadline guarantee
- ✅ Soft real-time task best effort
- ✅ Watchdog timer
- ✅ Timeout detection
- ❌ Avoid indeterminate delays

---

## 🔌 Hardware Interaction

### Register Operations
- ✅ Use hardware abstraction layer (HAL)
- ✅ Clear bit operations (BIT_SET/BIT_CLEAR)
- ✅ Register access volatile modified
- ✅ Read-modify-write atomicity
- ❌ Avoid direct hardcoded addresses

### GPIO and Peripherals
- ✅ GPIO initialization configuration
- ✅ Interrupt pin debouncing
- ✅ Peripheral clock enable
- ✅ DMA improve efficiency
- ✅ Peripheral multiplex conflict detection
- ❌ Avoid floating pins

### Communication Protocols
- ✅ UART/SPI/I2C correct configuration
- ✅ Timeout and error handling
- ✅ Buffer overflow check
- ✅ CRC/checksum verification
- ❌ Avoid polling waits (use interrupt/DMA)

---

## 🔋 Power Optimization

### Low-Power Modes
- ✅ Enter sleep mode when idle
- ✅ Deep sleep wake mechanism
- ✅ Peripheral clock gating
- ✅ Reduce clock frequency
- ❌ Avoid busy-wait

### Power Monitoring
- ✅ Measure power consumption
- ✅ Optimize wake frequency
- ✅ Batch process tasks
- ✅ Sensor on-demand sampling
- ❌ Avoid unnecessary wakes

---

## 🛡️ Safety and Reliability

### Error Handling
- ✅ Assert checks
- ✅ Error code returns
- ✅ Watchdog reset
- ✅ Hardware fault detection
- ✅ Power failure protection
- ❌ Don't ignore errors

### Data Integrity
- ✅ CRC/checksum verification
- ✅ Important data redundant storage
- ✅ Data backup and recovery
- ✅ Flash partition protection
- ❌ Avoid data corruption

### Secure Boot
- ✅ Bootloader firmware verification
- ✅ Signature verification
- ✅ Rollback protection
- ✅ Secure key storage
- ❌ Prohibit debug interface exposure (production)

---

## 🔄 Concurrency Control

### Critical Section Protection
- ✅ Disable interrupts protecting critical sections
- ✅ Mutex
- ✅ Semaphore
- ✅ Critical sections as short as possible
- ❌ Avoid deadlocks

### Data Sharing
- ✅ volatile modified shared variables
- ✅ Atomic operations
- ✅ Lock-free data structures (Ring Buffer)
- ✅ Message queues
- ❌ Avoid race conditions

---

## 🚀 Firmware Update

### OTA Update
- ✅ Dual partition (A/B partition)
- ✅ Pre-update verification
- ✅ Update failure rollback
- ✅ Power failure protection
- ✅ Incremental update (reduce data volume)
- ❌ Avoid brick risk

### Version Management
- ✅ Firmware version number
- ✅ Compatibility check
- ✅ Downgrade protection
- ✅ Version log
- ❌ Avoid version confusion

---

## 🧪 Testing and Debugging

### Unit Testing
- ✅ Business logic unit tests
- ✅ Mock hardware
- ✅ Boundary condition tests
- ✅ Stress tests
- ❌ Don't skip tests

### Hardware Testing
- ✅ Test on target hardware
- ✅ Long-term stability tests
- ✅ Temperature/voltage variation tests
- ✅ EMC tests
- ❌ Avoid testing only in ideal environment

### Debugging Tools
- ✅ JTAG/SWD debugging
- ✅ Log output (UART/ITM)
- ✅ Assertions and error codes
- ✅ Memory dumps
- ❌ Avoid printf debugging (high resource consumption)

---

## 📋 Embedded Development Checklist

- [ ] Memory usage optimization (static allocation priority)
- [ ] Interrupt service routines short and efficient
- [ ] Real-time task scheduling reasonable
- [ ] Hardware register access safe
- [ ] Power optimization (sleep/clock gating)
- [ ] Error handling and watchdog
- [ ] Critical section protection (prevent race)
- [ ] Firmware update secure (verification/rollback)
- [ ] Target hardware thoroughly tested
- [ ] Code size and performance optimized

---

---

## 🏛️ Advanced Embedded Architecture (20+ years experience)

### RTOS Advanced Architecture
```
Task Design Patterns:
- Producer-consumer: Sensor acquisition → Data processing
- State machine pattern: Complex control logic
- Event-driven: Interrupt → Event → Handle
- Priority inheritance: Solve priority inversion

Memory Architecture:
- Static allocation (compile-time determined)
- Memory pools (fixed-size blocks)
- Fragmentation-free design
- Stack protection (MPU)

Timing Analysis:
- WCET (Worst-Case Execution Time)
- Response time analysis
- Schedulability verification
- Time partitioning (ARINC 653)
```

### Safety-Critical Systems
```
Functional Safety Standards:
- ISO 26262 (Automotive)
- IEC 61508 (General)
- DO-178C (Aviation)
- IEC 62443 (Industrial)

ASIL/SIL Level Design:
- Redundant design (dual-core/triple-modular)
- Fault detection and response
- Watchdog hierarchy
- Safety monitors

Verification and Testing:
- Unit test coverage 100%
- MC/DC coverage
- Static analysis (MISRA)
- Formal verification
```

### Hardware Abstraction Layer Design
```
HAL Architecture:
- Driver layering: Hardware → HAL → Middleware → Application
- Platform abstraction: Easy porting
- BSP separation: Board support package
- Device tree/configuration files

Driver Design:
- Blocking vs non-blocking
- Polling vs interrupt vs DMA
- Buffer management
- Power state management
```

---

## 🔧 Essential Skills for Senior Embedded Experts

### Debugging Deep Techniques
```
Hardware Debugging:
- JTAG/SWD breakpoints and tracing
- ITM/ETM tracing
- Logic analyzers
- Oscilloscope protocol decoding

Memory Debugging:
- Memory dump analysis
- Stack backtrace
- Memory protection unit (MPU)
- Stack usage analysis

Timing Debugging:
- Pin toggle measurement
- Oscilloscope timing analysis
- Real-time tracing (SystemView)
- Latency jitter analysis
```

### Power Optimization Deep
```
Measurement Methods:
- Current probe measurement
- Power analyzer
- Different mode power characterization
- Battery life modeling

Optimization Strategies:
- Dynamic voltage frequency scaling (DVFS)
- Peripheral clock gating
- Sleep mode selection
- Wake source optimization

Ultra-Low Power Design:
- Sub-threshold circuits
- Energy harvesting
- Event-driven wake
- Power budget management
```

### Real-Time Performance Tuning
```
Interrupt Optimization:
- Interrupt latency measurement
- Interrupt priority planning
- Interrupt tail chaining
- Vector interrupt controller

Task Optimization:
- Context switch overhead
- Task time slicing
- Ready queue optimization
- Scheduler tick optimization

DMA Advanced Usage:
- Double buffering/ping-pong buffering
- Chained DMA
- Circular mode
- Transfer completion callback
```

### Secure Boot and Firmware Protection
```
Secure Boot Chain:
- ROM Bootloader → Primary bootloader → Secondary bootloader → Application
- Signature verification (RSA/ECDSA)
- Hash chain verification
- Rollback protection

Firmware Protection:
- Code encryption
- Read protection (RDP)
- Debug port disable
- Tamper detection

Key Management:
- Secure storage (OTP/Fuse)
- Key derivation
- Key update mechanism
- Hardware security module (HSM)
```

---

## 🚨 Common Pitfalls for Senior Embedded Experts

### Architecture Traps
```
❌ Ignore WCET analysis:
- Assume tasks always complete
- Actually experience deadline violations
- Correct: Analyze and test worst-case scenarios

❌ Overuse dynamic allocation:
- Heap fragmentation
- Allocation failures
- Correct: Static allocation + memory pools

❌ Interrupt handling too long:
- Affects real-time performance
- Priority inversion
- Correct: Quick return, deferred processing
```

### Debugging Traps
```
❌ Rely on printf debugging:
- Changes timing
- High resource consumption
- Correct: Use ITM/RTT

❌ Ignore optimization level impact:
- Debug and release behavior different
- Variables optimized away
- Correct: Test at target configuration

❌ Don't test extreme conditions:
- Only test at room temperature
- Ignore voltage fluctuations
- Correct: Environmental limit testing
```

### Security Traps
```
❌ Plaintext key storage:
- Firmware reverse engineering leak
- Correct: Secure storage + encryption

❌ Ignore debug interfaces:
- Production devices debuggable
- Correct: Disable or protect debug ports

❌ Firmware update without verification:
- Accept malicious firmware
- Correct: Signature verification + rollback protection
```

---

## 📊 Performance Monitoring Metrics

| Metric | Target | Alert Threshold | Measurement Method |
|--------|--------|-----------------|-------------------|
| Interrupt Latency | < 10μs | > 50μs | Oscilloscope/ITM |
| Task Response Time | < WCET | > 90% WCET | Tracing tools |
| Stack Usage | < 70% | > 90% | Stack watermark |
| CPU Utilization | < 70% | > 90% | RTOS statistics |
| Power (Active) | Design-based | > Budget | Current probe |
| Power (Sleep) | < 10μA | > 100μA | Current probe |
| Boot Time | < 1s | > 5s | Oscilloscope |
| Watchdog Triggers | 0 | > 0 | Log |
| Hard Faults | 0 | > 0 | Error log |
| Flash Write Count | < 10% life | > 50% life | Wear counter |

---

## 📋 Embedded Development Checklist (Complete Version)

### Resource Management
- [ ] Static memory allocation
- [ ] Stack size reasonably configured
- [ ] No memory leaks
- [ ] Flash/RAM usage monitoring

### Real-Time
- [ ] WCET analysis complete
- [ ] Interrupt response time met
- [ ] No priority inversion
- [ ] Watchdog normal

### Security
- [ ] Secure boot chain complete
- [ ] Firmware signature verification
- [ ] Debug port disabled
- [ ] Key secure storage

### Power
- [ ] Sleep mode correct
- [ ] Power budget met
- [ ] Wake mechanism reliable

---

**Embedded Development Principles Summary**:
Resource Constraints, Real-Time, Hardware Interaction, Power Optimization, Safety and Reliability, Concurrency Control, Firmware Update, Thorough Testing, Debugging Tools, Code Optimization
