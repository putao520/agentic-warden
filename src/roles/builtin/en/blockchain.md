# Blockchain Development Standards - CODING-STANDARDS-BLOCKCHAIN

**Version**: 2.0.0
**Scope**: Blockchain development roles (Smart Contracts/DApps/Chain Development, Platform Agnostic)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Rules (Inherited from common.md)

> **Must follow the four core iron rules from common.md**

```
Iron Rule 1: SPEC is the Single Source of Truth (SSOT)
       - Smart contracts must comply with SPEC definitions
       - Interfaces, events, permissions based on SPEC

Iron Rule 2: Smart Reuse and Destroy-Rebuild
       - Existing contract fully matches → Direct reuse
       - Partial match → Deploy new contract

Iron Rule 3: Prohibit Incremental Development
       - Prohibit adding new features to old contracts
       - Prohibit retaining compatibility functions

Iron Rule 4: Context7 Research First
       - Use mature security libraries (OpenZeppelin)
       - Prohibit implementing encryption/authentication yourself
```

---

## 🔒 Smart Contract Security

### Security Principles
- ✅ Secure by Default
- ✅ Principle of Least Privilege
- ✅ Defensive Programming
- ✅ Code Auditing (Third-party audit)
- ❌ Never assume input is trustworthy

### Common Vulnerability Protection
- ✅ Reentrancy Guard
- ✅ Integer Overflow/Underflow Check (SafeMath)
- ✅ Front-Running Protection
- ✅ Timestamp Dependency Risks
- ✅ Careful Use of Self-Destruct Functions
- ❌ Prohibit using tx.origin for authorization

### Access Control
- ✅ Use Modifiers for Permission Control
- ✅ Role-Based Access Control (RBAC)
- ✅ Multi-Signature Mechanism (Critical Operations)
- ✅ Timelock
- ❌ Avoid Single Point of Failure (Single Admin)

---

## ⛽ Gas Optimization

### Storage Optimization
- ✅ Reduce storage operations (most expensive)
- ✅ Variable Packing (Struct Packing)
- ✅ Use Events Instead of Storage
- ✅ Short-Circuit Evaluation
- ✅ Delete Unused Storage to Free Gas
- ❌ Avoid Dynamic Arrays (Unlimited Growth)

### Code Optimization
- ✅ Use Constants and Immutable Variables
- ✅ Cache State Variables to Memory
- ✅ Loop Optimization (Reduce Iterations)
- ✅ Clear Function Visibility (external vs public)
- ✅ Use Libraries to Share Code
- ❌ Avoid Unnecessary Calculations

### Call Optimization
- ✅ Batch Operations (Reduce Transaction Count)
- ✅ Use Delegate Call to Reuse Logic
- ✅ Estimate Gas Limits
- ❌ Avoid Infinite Loops

---

## 🏗️ Contract Design

### Architecture Patterns
- ✅ Single Responsibility Contracts
- ✅ Proxy Pattern (Upgradeable Contracts)
- ✅ Factory Pattern (Batch Deployment)
- ✅ Registry Pattern (Contract Discovery)
- ❌ Avoid Monolithic Contracts (Too Large)

### Upgradeability
- ✅ Use Proxy Contracts (Proxy Pattern)
- ✅ Separate Storage and Logic
- ✅ Initialize Functions Instead of Constructors
- ✅ Secure Upgrade Mechanism (Multi-sig/Timelock)
- ❌ Avoid Breaking Storage Layout

### Modularity
- ✅ Clear Interface Definitions
- ✅ Library Contract Reuse
- ✅ Loose Coupling Between Contracts
- ✅ Event-Driven Communication
- ❌ Avoid Circular Dependencies

---

## 💰 Tokens and Assets

### ERC Standard Compliance
- ✅ Strictly Follow ERC-20/721/1155 Standards
- ✅ Complete Implementation of Required Interfaces
- ✅ Correct Event Triggering
- ✅ Return Value and Exception Handling
- ❌ Do Not Modify Standard Interfaces

### Asset Security
- ✅ Check Balance Before Transfer
- ✅ Approval Mechanism
- ✅ Prevent Accidental Destruction
- ✅ Support Emergency Pause (Pausable)
- ❌ Prohibit Automatic Minting Vulnerabilities

### Precision Handling
- ✅ Use Fixed-Point Numbers (Avoid Floating-Point)
- ✅ Explicit Precision Units
- ✅ Prevent Precision Loss
- ✅ Division at the End
- ❌ Avoid Division by Zero Errors

---

## 📡 Events and Logs

### Event Design
- ✅ Trigger Events on Key State Changes
- ✅ Event Parameters Indexed (Searchable)
- ✅ Clear Event Naming (Verb + Noun)
- ✅ Events Contain Sufficient Context
- ❌ Do Not Overuse indexed (Max 3)

### Log Optimization
- ✅ Use Events Instead of Storage (Save Gas)
- ✅ Off-Chain Event Indexing
- ✅ Event Versioning (Upgrade Compatible)
- ❌ Avoid Logging Sensitive Data to Events

---

## 🔐 Cryptography

### Signature Verification
- ✅ Use Standard Signature Algorithms (ECDSA)
- ✅ Anti-Replay Attack (Nonce/Timestamp)
- ✅ Hash Signed Messages
- ✅ Verify Signer Identity
- ❌ Prohibit Direct Use of Private Keys

### Hash Functions
- ✅ Use keccak256 for Hashing
- ✅ Concatenate Data Type Before Hashing (Collision Prevention)
- ✅ Merkle Tree Verification
- ❌ Avoid Using Weak Hash Algorithms

### Random Numbers
- ✅ Use Verifiable Random Functions (VRF)
- ✅ Off-Chain Randomness + On-Chain Verification
- ❌ Prohibit Using Block Hash as Random Source (Predictable)
- ❌ Prohibit Using block.timestamp as Random Source

---

## 🧪 Testing

### Test Coverage
- ✅ Unit Tests (Each Function)
- ✅ Integration Tests (Contract Interaction)
- ✅ Boundary Tests (Extremes/Overflow)
- ✅ Security Tests (Vulnerability Scanning)
- ✅ Gas Consumption Tests
- ✅ Upgrade Tests (Upgradeable Contracts)

### Testnet Deployment
- ✅ Thorough Testing on Testnet
- ✅ Simulate Real Scenarios
- ✅ Stress Testing
- ✅ Third-Party Integration Testing
- ❌ Do Not Deploy Untested Code to Mainnet

### Formal Verification
- ✅ Formal Verification of Critical Contracts
- ✅ Invariant Checking
- ✅ Property Testing
- ✅ Symbolic Execution

---

## 🌐 DApp Development

### Frontend Integration
- ✅ Use Web3 Libraries (ethers.js/web3.js)
- ✅ Wallet Connection (MetaMask/WalletConnect)
- ✅ Network Switching Prompts
- ✅ Transaction Status Tracking
- ✅ Error Handling and User Prompts
- ❌ Do Not Store Private Keys in Frontend

### User Experience
- ✅ Show Gas Estimates Before Transaction Confirmation
- ✅ Display Loading While Waiting for Transaction Confirmation
- ✅ Friendly Prompts on Transaction Failure
- ✅ Support Transaction Acceleration/Cancellation
- ❌ Avoid Unresponsive Waiting

---

## 🔧 Deployment and Operations

### Deployment Process
- ✅ Use Scripts for Automated Deployment
- ✅ Multi-Signature Deployment for Critical Contracts
- ✅ Verify Contract Source Code (Etherscan)
- ✅ Record Contract Addresses and Transaction Hashes
- ✅ Document Deployment Steps
- ❌ Avoid Manual Deployment (Error-Prone)

### Monitoring and Alerting
- ✅ Monitor Contract Status
- ✅ Alert on Abnormal Activity
- ✅ Balance Monitoring
- ✅ Gas Price Monitoring
- ✅ Event Listening

### Emergency Response
- ✅ Pause Mechanism (Circuit Breaker)
- ✅ Emergency Withdraw Function
- ✅ Upgrade/Fix Process
- ✅ Incident Response Plan
- ❌ Do Not Rely on Single Point of Control

---

## 📋 Blockchain Development Checklist

- [ ] Reentrancy and integer overflow protection
- [ ] Access control and permission management
- [ ] Gas optimization (storage, computation, calls)
- [ ] Complete event triggering
- [ ] Follow ERC standards (token contracts)
- [ ] Thorough testnet testing
- [ ] Security audit (third-party)
- [ ] Upgradeability and emergency pause
- [ ] Frontend integration and user experience
- [ ] Deployment scripts and monitoring

---

---

## 🏛️ Advanced Blockchain Architecture (20+ Years Experience)

### DeFi Protocol Architecture
```
Core Patterns:
- AMM (Automated Market Maker): Uniswap v2/v3
- Lending Protocol: Compound/Aave
- Stablecoin Mechanism: MakerDAO
- Yield Aggregation: Yearn

Architecture Points:
- Protocol Composability
- Flash Loan Attack Protection
- Oracle Dependency Management
- Governance Mechanism Design

Risk Management:
- Liquidation Mechanism
- Collateral Ratio Monitoring
- Price Slippage Protection
- Emergency Pause
```

### Layer 2 Development
```
Scaling Solutions:
- Rollup: Optimistic/ZK
- Sidechain: Polygon PoS
- State Channels: Lightning Network
- Validium

Development Points:
- L1/L2 Communication
- Data Availability
- Withdrawal Delay
- Cross-Chain Bridge Security

ZK Technology:
- SNARKs/STARKs
- Proof Generation and Verification
- Circuit Design
- Trusted Setup
```

### Cross-Chain Architecture
```
Cross-Chain Patterns:
- Atomic Swaps
- Hashed Timelock Contracts
- Relay Chain
- Bridge Protocol

Security Considerations:
- Bridge Contract Security
- Validator Selection
- Multi-Signature Threshold
- Fraud Proofs

Interoperability Protocols:
- IBC (Cosmos)
- XCM (Polkadot)
- LayerZero
- Wormhole
```

---

## 🔧 Essential Skills for Senior Blockchain Experts

### Smart Contract Security Deep Dive
```
Audit Methods:
- Manual Code Review
- Automated Tools (Slither/Mythril)
- Formal Verification (Certora)
- Fuzzing (Echidna)

Attack Vector Analysis:
- Reentrancy Attack Variants
- Flash Loan Attacks
- Oracle Manipulation
- Governance Attacks
- MEV Attacks

Defense Patterns:
- CEI (Checks-Effects-Interactions)
- Reentrancy Locks
- TWAP (Time-Weighted Average Price)
- Timelock Governance
```

### Gas Optimization Deep Dive
```
Storage Optimization:
- Variable Packing (256-bit Slots)
- Mapping vs Arrays
- Immutable Variables
- Transient Storage (EIP-1153)

Computation Optimization:
- Bitwise Operations Instead of Division
- Short-Circuit Evaluation
- Cache Storage Reads
- Inline Assembly (Yul)

Pattern Optimization:
- Merkle Airdrops
- Batch Operations
- Lazy Minting
- Off-Chain Signature Verification
```

### Upgrade Pattern Deep Dive
```
Proxy Patterns:
- Transparent Proxy
- UUPS Proxy
- Beacon Proxy
- Diamond Pattern (EIP-2535)

Storage Layout:
- Storage Slot Management
- Storage Collision Detection
- Upgrade Compatibility Check
- Initializer Pattern

Upgrade Security:
- Multi-Sig Governance
- Timelock Delay
- Upgrade Testing
- Rollback Plan
```

### MEV and Transaction Ordering
```
MEV Types:
- Front-running
- Sandwich Attacks
- Arbitrage
- Liquidation

Protection Strategies:
- Private Pools (Flashbots)
- Commit-Reveal Scheme
- Timelock
- Slippage Protection

MEV Exploitation (Compliant):
- Arbitrage Bots
- Liquidation Bots
- Searcher Strategies
```

---

## 🚨 Common Pitfalls for Senior Blockchain Experts

### Security Pitfalls
```
❌ Assuming External Calls are Safe:
- Trusting external contracts
- No reentrancy protection
- Correct approach: CEI pattern, reentrancy locks

❌ Oracle Single Point of Dependency:
- Single price source
- No delay/average handling
- Correct approach: Multi-source aggregation, TWAP

❌ Ignoring Permission Management:
- Overly centralized permissions
- No timelock
- Correct approach: Multi-sig, timelock, progressive decentralization
```

### Economic Pitfalls
```
❌ Economic Model Flaws:
- Unsustainable token economics
- Misaligned incentives
- Correct approach: Game theory analysis

❌ Insufficient Liquidation Mechanism:
- Liquidation delays
- Price drops too fast
- Correct approach: Multi-tier liquidation, insurance fund

❌ Ignoring Flash Loan Risks:
- Vulnerable to costless attacks
- Correct approach: Flash loan protection, price validation
```

### Operations Pitfalls
```
❌ No Post-Deployment Monitoring:
- Abnormal activity undetected
- Correct approach: Event monitoring, alerting

❌ No Emergency Response Plan:
- Panic during attacks
- Correct approach: Plan rehearsals, emergency pause

❌ Poor Private Key Management:
- Large amounts in hot wallets
- Correct approach: Cold/hot separation, multi-sig
```

---

## 📊 Performance Monitoring Metrics

| Metric | Target | Alert Threshold | Measurement Method |
|--------|--------|-----------------|-------------------|
| Gas Consumption | < Budget | > 150% Budget | Transaction Statistics |
| Transaction Success Rate | > 99% | < 95% | On-Chain Analysis |
| Contract TVL | Business Based | Abnormal Change | On-Chain Query |
| Governance Participation Rate | > 30% | < 10% | Governance Statistics |
| Audit Coverage | 100% | < 100% | Audit Reports |
| Security Incidents | 0 | > 0 | Monitoring System |
| Liquidation Health | > 150% | < 120% | Protocol Statistics |
| Oracle Latency | < 1 minute | > 10 minutes | Monitoring System |
| Governance Proposal Pass Time | Design Based | Abnormal | Governance Statistics |
| Smart Contract Upgrade Count | As Needed | Frequent Abnormal | Version History |

---

## 📋 Blockchain Development Checklist (Complete)

### Security Check
- [ ] Reentrancy attack protection
- [ ] Integer overflow check
- [ ] Complete access control
- [ ] Oracle security

### Economic Design
- [ ] Sustainable token economics
- [ ] Aligned incentive mechanisms
- [ ] Sound liquidation mechanism
- [ ] Flash loan protection

### Code Quality
- [ ] Third-party audit
- [ ] Formal verification
- [ ] Complete test coverage
- [ ] Gas optimization

### Operations Readiness
- [ ] Monitoring and alerting
- [ ] Emergency pause mechanism
- [ ] Upgrade process
- [ ] Incident response plan

---

**Blockchain Development Principles Summary**:
Security First, Gas Optimization, Code Auditing, Thorough Testing, Upgradeability, Complete Events, Access Control, Defensive Programming, User Experience, Monitoring and Alerting
