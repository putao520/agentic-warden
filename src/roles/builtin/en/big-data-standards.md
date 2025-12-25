# Big Data Development Standards - CODING-STANDARDS-BIG-DATA

**Version**: 2.0.0
**Scope**: Big data development roles (Batch processing/Stream processing/Data lake/Data warehouse, tech stack agnostic)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (Inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - Data pipeline design must comply with SPEC definitions
       - Schema, data flow, processing logic based on SPEC

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Existing pipeline fully matches → Reuse directly
       - Partial match → Delete and rebuild

Iron Law 3: Prohibit Incremental Development
       - Prohibit adding new features to old pipelines
       - Prohibit retaining compatibility schemas

Iron Law 4: Context7 Research First
       - Use mature big data frameworks
       - Prohibit self-implementing ETL tools
```

---

## 🏗️ Data Pipeline Design

### Pipeline Principles
- ✅ Single responsibility: Each job does only one thing
- ✅ Idempotency: Repeated execution produces same result
- ✅ Restartability: Support recovery from failure point
- ✅ Clear data lineage
- ❌ Avoid strong coupling between pipelines

### Data Flow Design
- ✅ Clear input, processing, output boundaries
- ✅ Use checkpoint mechanism
- ✅ Design data backfill strategy
- ✅ Distinguish incremental and full processing
- ✅ Handle late data
- ❌ Avoid circular dependencies

---

## 📊 Batch Processing Development

### Job Design
- ✅ Data partitioning (by time/region/business)
- ✅ Reasonable batch size settings
- ✅ Parallelism matches resources
- ✅ Failed tasks retryable
- ✅ Intermediate result persistence
- ❌ Avoid single point bottlenecks

### Scheduling Management
- ✅ Clear job dependencies (DAG)
- ✅ Set reasonable timeouts
- ✅ Configure alerts and monitoring
- ✅ Distinguish normal failures from abnormal failures
- ✅ Record job execution history
- ❌ Avoid hardcoded schedule times

### Data Quality
- ✅ Input data validation (schema, range, completeness)
- ✅ Data quality checks during processing
- ✅ Output data consistency validation
- ✅ Bad data isolation (bad data partitions)
- ✅ Data quality metrics monitoring

---

## ⚡ Stream Processing Development

### Stream Processing Principles
- ✅ Process unbounded data streams
- ✅ Event time vs processing time
- ✅ Windowing mechanisms (tumbling/sliding/session windows)
- ✅ Watermark handling late data
- ✅ State management and checkpoints
- ❌ Avoid unbounded state growth

### Real-Time Guarantees
- ✅ Clear latency requirements (second/minute level)
- ✅ Backpressure mechanism
- ✅ Flow control and rate limiting
- ✅ Monitor processing latency
- ❌ Avoid blocking operations

### Consistency Guarantees
- ✅ At Least Once vs Exactly Once
- ✅ Transactional output
- ✅ Deduplication mechanism
- ✅ Order guarantees (within partition)
- ✅ Idempotency design

---

## 🗂️ Data Partitioning and Storage

### Partitioning Strategy
- ✅ Partition by time (year/month/day/hour)
- ✅ Partition by business dimension (region/category)
- ✅ Avoid data skew
- ✅ Partition pruning optimization
- ✅ Reasonable partition count control (< 10000)
- ❌ Avoid small file problem

### File Formats
- ✅ Use columnar storage (Parquet, ORC)
- ✅ Enable compression (Snappy, ZSTD)
- ✅ Schema evolution compatibility
- ✅ Reasonable file size (128MB-1GB)
- ❌ Avoid plain text formats (production environment)

### Data Lifecycle
- ✅ Define data retention policies
- ✅ Cold-hot data tiered storage
- ✅ Automatic historical data archiving
- ✅ Expired data cleanup
- ✅ Cost optimization storage

---

## 🔄 Schema Management

### Schema Design
- ✅ Backward compatible schema evolution
- ✅ Use schema registry
- ✅ Versioned schema management
- ✅ Clear field types and constraints
- ❌ Avoid breaking changes

### Data Types
- ✅ Use appropriate data types (reduce storage and compute cost)
- ✅ Reasonable use of nested structures (avoid too deep)
- ✅ Timestamps uniformly use UTC
- ✅ String fields with length limits
- ❌ Avoid dynamic types (affects performance)

---

## ⚙️ Resource Management

### Resource Configuration
- ✅ Configure memory based on data volume
- ✅ Reasonable parallelism settings
- ✅ CPU and IO balance
- ✅ Resource isolation (tasks don't affect each other)
- ✅ Elastic scaling
- ❌ Avoid over-provisioning resources

### Performance Optimization
- ✅ Reduce data shuffle
- ✅ Use broadcast variables (small table joins)
- ✅ Local aggregation reduces network transfer
- ✅ Cache reused datasets
- ✅ Predicate pushdown
- ✅ Column pruning

### Cost Optimization
- ✅ Use spot instances (non-critical jobs)
- ✅ On-demand cluster start/stop
- ✅ Monitor resource utilization
- ✅ Optimize data storage costs
- ❌ Avoid idle resource waste

---

## 🛡️ Data Security

### Access Control
- ✅ Least privilege principle
- ✅ Data classification management
- ✅ Sensitive data encrypted storage
- ✅ Audit logging
- ❌ Prohibit plaintext sensitive data storage

### Data Masking
- ✅ Production data masked for development testing
- ✅ Sensitive fields hashed or encrypted
- ✅ PII data (personal identity information) protection
- ✅ Data export permission control

---

## 📈 Monitoring and Observability

### Monitoring Metrics
- ✅ Job execution duration
- ✅ Data processing volume
- ✅ Resource utilization (CPU/memory/disk/network)
- ✅ Error rate and retry count
- ✅ Data latency (stream processing)
- ✅ Data quality metrics

### Alert Mechanisms
- ✅ Job failure alerts
- ✅ Data latency threshold alerts
- ✅ Data quality anomaly alerts
- ✅ Resource usage anomaly alerts
- ✅ SLA violation alerts

### Logging and Tracing
- ✅ Structured logging
- ✅ Log critical operations
- ✅ Distributed tracing (Trace ID)
- ✅ Data lineage tracking
- ❌ Avoid log flooding (over-logging)

---

## 🧪 Testing

### Testing Strategy
- ✅ Unit tests (data transformation logic)
- ✅ Integration tests (end-to-end pipeline)
- ✅ Data quality tests
- ✅ Performance tests (large data volume)
- ✅ Boundary tests (empty data, bad data)

### Test Data
- ✅ Use production data samples
- ✅ Synthetic data generation
- ✅ Test environment data isolation
- ✅ Simulate data skew scenarios
- ❌ Prohibit testing in production environment

---

## 📋 Big Data Development Checklist

- [ ] Pipeline idempotency and restartability
- [ ] Data partitioning reasonable (avoid small files and data skew)
- [ ] Schema version management and compatibility
- [ ] Resource configuration reasonable (memory, parallelism)
- [ ] Data quality validation
- [ ] Monitoring and alerting configured
- [ ] Sensitive data encryption and masking
- [ ] Logging and tracing complete
- [ ] Failure retry and fault tolerance
- [ ] Cost optimization (storage, compute)

---

---

## 🏛️ Advanced Data Architecture (20+ years experience)

### Modern Data Architecture Paradigms
```
Data Mesh:
- Domain ownership: Data owned by domain teams
- Data as product: Data published as products
- Self-service platform: Unified infrastructure
- Federated governance: Decentralized governance
- Applicable: Large organizations, multi-domain

Data Lakehouse:
- Combine data lake and data warehouse advantages
- Delta Lake/Iceberg/Hudi table formats
- ACID transaction support
- Schema evolution and time travel
- Unified batch-stream processing

Lambda vs Kappa Architecture:
- Lambda: Batch + stream dual paths
- Kappa: Stream only, unified architecture
- Selection consideration: Complexity vs consistency
```

### Stream-Batch Unified Architecture
```
Unified Processing Engine:
- Apache Flink: Stream-batch unified
- Apache Beam: Cross-engine abstraction
- Spark Structured Streaming: Micro-batch + stream

Real-Time Data Warehouse:
- ODS (Operational Data Store): Real-time data lake ingestion
- DWD (Data Warehouse Detail): Real-time cleaning
- DWS (Data Warehouse Summary): Real-time aggregation
- ADS (Application Data Store): Real-time serving

Real-Time Features:
- Incremental Processing
- Materialized Views
- Change Data Capture (CDC)
```

### Data Governance Architecture
```
Metadata Management:
- Apache Atlas: Lineage tracking
- DataHub: Metadata platform
- Amundsen: Data discovery

Data Quality Frameworks:
- Great Expectations: Data validation
- Deequ: Spark data quality
- Data Contracts

Data Catalog:
- Automated discovery
- Business glossary
- Sensitive data classification
- Data asset search
```

---

## 🔧 Essential Skills for Senior Big Data Experts

### Spark Deep Optimization
```
Memory Management:
- Executor Memory = Heap + Off-Heap
- spark.memory.fraction tuning
- Serialization (Kryo vs Java)
- Broadcast variable size control

Shuffle Optimization:
- spark.sql.shuffle.partitions tuning
- AQE (Adaptive Query Execution)
- Coalesce vs Repartition
- Skew handling (Salting)

Execution Plan Optimization:
- Predicate pushdown verification
- Join strategy selection (Broadcast/Sort-Merge/Shuffle-Hash)
- CBO (Cost-Based Optimizer)
- Catalyst optimizer understanding
```

### Flink Deep Optimization
```
State Management:
- State backend selection (Memory/RocksDB)
- Incremental checkpoints
- State TTL
- State size control

Backpressure Handling:
- Identify backpressure source
- Buffer tuning
- Parallelism adjustment
- Async IO

Exactly-Once Semantics:
- Two-phase commit (2PC)
- Idempotent writes
- Transactional Sink
- Changelog Stream
```

### Performance Tuning Methodology
```
Problem Diagnosis:
1. Confirm bottleneck (CPU/Memory/IO/Network)
2. Analyze execution plan
3. Identify data skew
4. Check resource configuration

Tuning Strategy:
- Data level: Partitioning, compression, format
- Operator level: Parallelism, memory, shuffle
- Cluster level: Resource allocation, queue configuration
- Code level: Avoid UDF, use vectorization
```

### Cost Optimization Practices
```
Storage Cost:
- Cold-hot tiering (S3 Glacier)
- Columnar compression (ZSTD/LZ4)
- Data lifecycle management
- Small file merging

Compute Cost:
- Spot/Preemptible instances
- Elastic scaling
- Resource utilization optimization
- Job scheduling optimization

FinOps Practices:
- Cost attribution (by team/project)
- Budget alerts
- Resource usage reports
- Continuous optimization loop
```

---

## 🚨 Common Pitfalls for Senior Big Data Experts

### Architecture Traps
```
❌ Excessive real-time:
- Use stream processing for all scenarios
- Increase complexity and cost
- Correct: Choose based on latency requirements

❌ Ignore data quality:
- Only focus on pipeline functionality
- Dirty data pollutes downstream
- Correct: Data quality check gatekeeping

❌ Schema野蛮 growth:
- Arbitrary field addition
- No version management
- Correct: Schema Registry, compatibility checks
```

### Performance Traps
```
❌ Data skew not handled:
- Uneven join key distribution
- Single task drags overall
- Correct: Salting, Broadcast, AQE

❌ Small file proliferation:
- High-frequency writes generate many small files
- High metadata pressure, slow queries
- Correct: Merge jobs, Compaction

❌ Over-partitioning:
- Too many partitions
- Queries actually slower
- Correct: Reasonable partition granularity, < 10000
```

### Operational Traps
```
❌ Unreliable checkpoints:
- Checkpoint failures not alerted
- Cannot recover on failure
- Correct: Checkpoint monitoring, backup verification

❌ Resource over-provisioning:
- Every job configured with large memory
- Serious resource waste
- Correct: On-demand configuration, dynamic resource allocation

❌ Ignore backfill scenarios:
- Only consider incremental processing
- Historical data cannot be processed
- Correct: Design backfill strategy
```

---

## 📊 Performance Monitoring Metrics

| Metric | Target | Alert Threshold | Measurement Tool |
|--------|--------|-----------------|------------------|
| Job Success Rate | > 99% | < 95% | Scheduling system |
| Data Latency (Stream) | < 1 minute | > 5 minutes | Monitoring system |
| Processing Throughput | SLA-based | < 80% expected | Metrics |
| Checkpoint Success Rate | 100% | < 99% | Flink Dashboard |
| Data Quality Pass Rate | > 99.9% | < 99% | Quality platform |
| Resource Utilization | 60-80% | < 30% or > 90% | Cluster monitoring |
| Shuffle Data Volume | Job-based | Abnormal growth | Spark UI |
| GC Time Percentage | < 5% | > 10% | JVM monitoring |
| Small File Count | < 1000/partition | > 5000 | Storage monitoring |
| Data Skew Ratio | < 2x | > 10x | Execution plan |

---

## 📋 Big Data Development Checklist (Complete Version)

### Pipeline Design
- [ ] Idempotency and restartability
- [ ] Backfill strategy design
- [ ] Data lineage recording
- [ ] Late data handling

### Performance Optimization
- [ ] Partitioning strategy reasonable
- [ ] No data skew
- [ ] No small file issues
- [ ] Shuffle optimization

### Data Quality
- [ ] Input validation
- [ ] In-process checks
- [ ] Output validation
- [ ] Bad data isolation

### Operational Assurance
- [ ] Monitoring and alerting
- [ ] Logging and tracing
- [ ] Reliable checkpoints
- [ ] Cost optimization

---

**Big Data Development Principles Summary**:
Idempotency, Restartability, Data Quality, Partition Optimization, Resource Management, Monitoring and Alerting, Schema Evolution, Stream-Batch Unification, Cost Optimization, Data Security
