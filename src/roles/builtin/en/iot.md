# IoT Development Standards - CODING-STANDARDS-IOT

**Version**: 2.0.0
**Scope**: IoT development roles (Sensors/Actuators/Edge Computing/Gateways, Platform Agnostic)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Rules (Inherited from common.md)

> **Must follow the four core iron rules from common.md**

```
Iron Rule 1: SPEC is the Single Source of Truth (SSOT)
       - Device protocols must comply with SPEC definitions
       - Communication, data format, security based on SPEC

Iron Rule 2: Smart Reuse and Destroy-Rebuild
       - Existing driver fully matches → Direct reuse
       - Partial match → Delete and rewrite

Iron Rule 3: Prohibit Incremental Development
       - Prohibit adding new features to old firmware
       - Prohibit retaining compatibility protocols

Iron Rule 4: Context7 Research First
       - Use mature IoT platforms and SDKs
       - Prohibit implementing communication protocols yourself
```

---

## 📡 Device Communication

### Wireless Protocols
- ✅ WiFi/BLE/LoRa/Zigbee/NB-IoT
- ✅ Protocol Selection (Power/Range/Bandwidth Trade-off)
- ✅ Signal Strength Monitoring (RSSI)
- ✅ Auto-Reconnection Mechanism
- ✅ Network Switching (WiFi → Cellular)
- ❌ Avoid Prolonged Connection Failures

### Message Protocols
- ✅ MQTT/CoAP/HTTP/WebSocket
- ✅ QoS Level Selection (0/1/2)
- ✅ Message Serialization (JSON/Protobuf/CBOR)
- ✅ Topic Naming Conventions
- ✅ Retained Messages and Last Will
- ❌ Avoid Message Storms

### Data Transmission
- ✅ Batch Reporting (Reduce Transmission Frequency)
- ✅ Data Compression
- ✅ Resumable Transfer
- ✅ Retransmission and Deduplication
- ✅ Traffic Monitoring
- ❌ Avoid Frequent Small Data Transmissions

---

## 🔋 Power Management

### Low-Power Design
- ✅ Deep Sleep Mode
- ✅ Timed Wake-Up
- ✅ Interrupt Wake-Up (Sensor Events)
- ✅ Dynamic Voltage and Frequency Scaling (DVFS)
- ✅ Peripherals On-Demand Enable
- ❌ Avoid Polling (Use Interrupts)

### Battery Optimization
- ✅ Battery Level Monitoring
- ✅ Low Battery Alerts
- ✅ Graceful Shutdown
- ✅ Energy Harvesting (Solar/Vibration)
- ❌ Avoid Battery Over-Discharge

### Transmission Optimization
- ✅ Reduce Connection Establishment Frequency
- ✅ Keep-Alive Connections
- ✅ Local Data Caching (Offline Upload)
- ✅ Change-Triggered Reporting (Not Scheduled)
- ❌ Avoid Meaningless Heartbeats

---

## 📊 Sensors and Data Acquisition

### Sensor Management
- ✅ Sensor Calibration
- ✅ Sampling Frequency Configuration
- ✅ Data Filtering (Moving Average/Kalman)
- ✅ Outlier Detection and Rejection
- ✅ Multi-Sensor Fusion
- ❌ Avoid Using Raw Data Directly

### Data Quality
- ✅ Data Validity Verification
- ✅ Range Checking
- ✅ Timestamp Recording
- ✅ Data Integrity (CRC)
- ❌ Avoid Transmitting Erroneous Data

### Local Processing
- ✅ Edge Computing (Reduce Cloud Pressure)
- ✅ Local Aggregation and Statistics
- ✅ Threshold Alerting
- ✅ Data Preprocessing
- ❌ Avoid Sending All Data to Cloud

---

## 🎛️ Actuator Control

### Control Logic
- ✅ Status Feedback
- ✅ Control Command Confirmation
- ✅ Safety Interlock (Prevent Misoperation)
- ✅ Manual Priority (Physical Button)
- ❌ Avoid Unconfirmed Blind Control

### Remote Control
- ✅ Command Verification (Signature/Token)
- ✅ Permission Check
- ✅ Operation Logging
- ✅ Timeout Protection
- ❌ Prohibit Unauthorized Control

---

## 🔒 Security

### Device Authentication
- ✅ Device Unique Identifier (Device ID)
- ✅ Certificate Authentication (X.509)
- ✅ Symmetric/Asymmetric Keys
- ✅ Key Rotation
- ❌ Prohibit Hardcoded Keys

### Communication Encryption
- ✅ TLS/DTLS Encrypted Transmission
- ✅ End-to-End Encryption
- ✅ Anti-Replay Attack (Nonce/Timestamp)
- ✅ Signature Verification
- ❌ Prohibit Plaintext Transmission of Sensitive Data

### Firmware Security
- ✅ Secure Boot
- ✅ Firmware Signature Verification
- ✅ Debug Interface Disabled (Production)
- ✅ Encrypted Firmware Storage
- ❌ Avoid Firmware Reverse Engineering

---

## 🔄 Device Management

### Device Provisioning
- ✅ SmartConfig/Bluetooth Provisioning/AP Mode
- ✅ Provisioning Timeout and Failure Handling
- ✅ Encrypted Network Information Storage
- ✅ Reset Function (Factory Reset)
- ❌ Avoid Provisioning Difficulties

### Device Registration
- ✅ Auto-Registration to Cloud Platform
- ✅ Device Metadata Reporting
- ✅ Registration Failure Retry
- ✅ Device Grouping
- ❌ Avoid Duplicate Registration

### OTA Updates
- ✅ Firmware Version Management
- ✅ Differential Updates (Reduce Traffic)
- ✅ Resumable Transfer
- ✅ Update Failure Rollback
- ✅ A/B Partitions (Dual System)
- ❌ Avoid Brick Devices Due to Updates

---

## 🌐 Cloud Integration

### Device Shadow
- ✅ Desired State and Reported State
- ✅ Offline Message Queue
- ✅ State Synchronization
- ✅ Version Control
- ❌ Avoid State Inconsistency

### Data Reporting
- ✅ Telemetry Data
- ✅ Property Data
- ✅ Event Data
- ✅ Time Series Storage
- ❌ Avoid Data Loss

### Rule Engine
- ✅ Data Linkage
- ✅ Alert Rules
- ✅ Scenario Automation
- ✅ Conditional Triggers
- ❌ Avoid Rule Conflicts

---

## 📈 Monitoring and Diagnostics

### Device Monitoring
- ✅ Online Status Monitoring
- ✅ Device Health Check
- ✅ Abnormal Behavior Detection
- ✅ Performance Metrics Reporting
- ❌ Do Not Ignore Offline Devices

### Logging and Debugging
- ✅ Remote Log Reporting
- ✅ Log Leveling (Error/Warn/Info)
- ✅ Local Log Caching
- ✅ Remote Debugging Interface
- ❌ Avoid Log Flooding (Power Drain)

### Fault Diagnosis
- ✅ Error Code Definition
- ✅ Fault Self-Diagnosis
- ✅ Fault Reporting
- ✅ Watchdog Reset Statistics
- ❌ Avoid Hiding Faults

---

## 🧪 Testing

### Functional Testing
- ✅ Sensor Data Accuracy
- ✅ Actuator Control Correctness
- ✅ Network Connection Stability
- ✅ OTA Update Testing
- ✅ Power Recovery Testing
- ❌ Do Not Skip Boundary Testing

### Environmental Testing
- ✅ Temperature/Humidity Environmental Testing
- ✅ Electromagnetic Compatibility (EMC) Testing
- ✅ Drop/Vibration Testing
- ✅ Waterproof/Dustproof Testing
- ❌ Avoid Testing Only in Laboratory

### Stress Testing
- ✅ Long-Duration Run Testing
- ✅ Network Abnormality Testing
- ✅ Massive Device Concurrent Testing
- ✅ Power Consumption Testing
- ❌ Do Not Ignore Extreme Cases

---

## 📋 IoT Development Checklist

- [ ] Wireless protocol selection and reconnection mechanism
- [ ] Low-power design (sleep/wake)
- [ ] Sensor data filtering and validation
- [ ] Device authentication and communication encryption
- [ ] OTA update security (signature/rollback)
- [ ] Device provisioning and registration flow
- [ ] Cloud integration (MQTT/device shadow)
- [ ] Monitoring and log reporting
- [ ] Environmental and stress testing
- [ ] Battery level monitoring and low battery handling

---

---

## 🏛️ Advanced IoT Architecture (20+ Years Experience)

### Edge Computing Architecture
```
Edge Layers:
- Device Edge: Local Sensor Processing
- Gateway Edge: Protocol Conversion, Data Aggregation
- Fog Computing: Regional-Level Processing
- Cloud: Global Analysis and Storage

Edge Intelligence:
- TinyML: Machine Learning on MCUs
- Model Quantization and Compression
- Incremental Learning
- Federated Learning

Edge Orchestration:
- KubeEdge: K8s Extension to Edge
- EdgeX Foundry: Edge Framework
- AWS Greengrass/Azure IoT Edge
- Containerized Edge Applications
```

### Large-Scale Device Management
```
Device Lifecycle:
- Device Provisioning
- Identity Registration
- Runtime Monitoring
- Firmware Updates
- Retirement Cleanup

Fleet Management:
- Device Grouping and Tagging
- Batch Operations
- Configuration Deployment
- State Synchronization

Zero-Touch Provisioning:
- Auto-Discovery
- Auto-Registration
- Auto-Configuration
- Secure Boot
```

### Digital Twin
```
Twin Model:
- Physical Entity Mapping
- Real-Time State Synchronization
- Historical Data Storage
- Predictive Analytics

Twin Platforms:
- Azure Digital Twins
- AWS IoT TwinMaker
- Self-Built Twin Framework

Application Scenarios:
- Device Simulation
- Predictive Maintenance
- Scenario Analysis
- Remote Diagnostics
```

---

## 🔧 Essential Skills for Senior IoT Experts

### Communication Protocol Deep Dive
```
MQTT Advanced:
- QoS Selection Strategy
- Session Persistence
- Shared Subscriptions (Load Balancing)
- MQTT 5.0 New Features (Reason Codes, Properties)

CoAP Advanced:
- Observe Mode
- Block Transfer
- Resource Discovery
- DTLS Security

Low-Power Wide Area Networks:
- LoRaWAN: Long Range, Low Power
- NB-IoT/LTE-M: Cellular IoT
- Sigfox: Ultra-Low Power
- Selection Considerations: Range/Power/Bandwidth/Cost
```

### Security Deep Dive
```
Device Identity:
- PKI Certificate System
- Device Certificate Lifecycle
- Certificate Rotation
- Hardware Security Module (HSM/TPM)

Secure Boot:
- Root of Trust
- Secure Boot Chain
- Firmware Signing
- Secure Upgrade

End-to-End Security:
- Device → Gateway → Cloud Encryption
- Key Derivation
- Session Keys
- Perfect Forward Secrecy
```

### Power Optimization Deep Dive
```
Protocol Layer Optimization:
- Message Aggregation
- Compression (CBOR vs JSON)
- Connection Reuse
- Session Resumption

Hardware Layer Optimization:
- RF Tuning
- Antenna Design
- Power Management IC
- Energy Harvesting

Battery Modeling:
- Battery Characteristic Curves
- Temperature Effects
- Aging Models
- Remaining Life Prediction
```

### Reliability Design
```
Network Fault Tolerance:
- Store and Forward
- Message Queues
- Retransmission Strategy
- Offline Mode

Device Fault Tolerance:
- Watchdog Hierarchy
- Fault Detection
- Auto-Recovery
- Backup Path

Data Reliability:
- Data Validation
- Retransmission Confirmation
- Idempotent Processing
- Deduplication Mechanism
```

---

## 🚨 Common Pitfalls for Senior IoT Experts

### Architecture Pitfalls
```
❌ Send All Data to Cloud:
- Bandwidth Waste
- High Latency
- High Cost
- Correct approach: Edge preprocessing

❌ Ignore Network Instability:
- Assume network always available
- No offline support
- Correct approach: Store and forward, offline mode

❌ Security as Afterthought:
- Implement features first, add security later
- Security as add-on feature
- Correct approach: Security by design
```

### Protocol Pitfalls
```
❌ Wrong QoS Selection:
- Critical messages use QoS 0
- All messages use QoS 2
- Correct approach: Select based on business needs

❌ Heartbeats Too Frequent:
- Power waste
- Bandwidth waste
- Correct approach: Adjust period based on business

❌ Messages Too Large:
- Single message too large
- No fragmentation mechanism
- Correct approach: Message splitting, block transfer
```

### Operations Pitfalls
```
❌ OTA Without Rollback:
- Update failure bricks device
- Correct approach: A/B partition, auto rollback

❌ No Device Health Monitoring:
- Unknown device offline
- Correct approach: Heartbeat monitoring, offline alerts

❌ Too Many/Few Logs:
- Power waste or difficult troubleshooting
- Correct approach: Tiered logging, on-demand reporting
```

---

## 📊 Performance Monitoring Metrics

| Metric | Target | Alert Threshold | Measurement Method |
|--------|--------|-----------------|-------------------|
| Device Online Rate | > 99% | < 95% | Platform Statistics |
| Message Latency (P99) | < 1s | > 5s | Platform Statistics |
| Message Delivery Rate | > 99.9% | < 99% | Platform Statistics |
| Power Consumption (Active) | Design Based | > Budget | Current Measurement |
| Battery Life | Design Based | < 80% Expected | Battery Monitoring |
| OTA Success Rate | > 99% | < 95% | Platform Statistics |
| Provisioning Success Rate | > 95% | < 80% | Log Statistics |
| Device Restart Count | 0/day | > 3/day | Log Statistics |
| Data Quality Score | > 99% | < 95% | Data Validation |
| Security Alerts | 0 | > 0 | Security Monitoring |

---

## 📋 IoT Development Checklist (Complete)

### Communication Reliability
- [ ] Reasonable protocol selection
- [ ] Offline store and forward
- [ ] Message acknowledgment mechanism
- [ ] Automatic network reconnection

### Security Complete
- [ ] Device identity authentication
- [ ] Communication encryption
- [ ] Secure boot
- [ ] Firmware signature

### Power Optimization
- [ ] Reasonable sleep mode
- [ ] Communication optimization
- [ ] Battery monitoring
- [ ] Power budget met

### Operations Ready
- [ ] OTA update mechanism
- [ ] Device monitoring
- [ ] Log reporting
- [ ] Fault diagnosis

---

**IoT Development Principles Summary**:
Low Power, Reliable Communication, Edge Computing, Security Authentication, OTA Updates, Device Management, Cloud Integration, Data Quality, Monitoring and Diagnostics, Environmental Adaptation
