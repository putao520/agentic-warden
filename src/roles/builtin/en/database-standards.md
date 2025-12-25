# Database Development Standards - CODING-STANDARDS-DATABASE

**Version**: 2.0.0
**Scope**: Database development roles (SQL/NoSQL/Graph databases/Time-series databases, tech stack agnostic)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (Inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - Data models must comply with SPEC definitions
       - Table structures, indexes, constraints based on SPEC

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Existing table structure fully matches → Reuse directly
       - Partial match → Migration script rebuild

Iron Law 3: Prohibit Incremental Development
       - Prohibit keeping old fields, adding new fields
       - Prohibit compatibility views and triggers

Iron Law 4: Context7 Research First
       - Database design reference best practices
       - Use mature ORM and query patterns
```

---

## 🗄️ Data Modeling

### Design Principles
- ✅ Comply with business domain model
- ✅ Clear entity relationships and constraints
- ✅ Design based on access patterns (read-heavy/write-heavy/balanced)
- ✅ Avoid over-normalization or over-denormalization
- ❌ Prohibit using business data as primary key

### Naming Conventions
- ✅ Table/collection names: plural nouns (users, orders)
- ✅ Column/field names: singular nouns (user_id, created_at)
- ✅ Index naming: idx_[table_name]_[column_name]
- ✅ Foreign key naming: fk_[table_name]_[referenced_table_name]
- ❌ Avoid reserved words and special characters

### Data Types
- ✅ Use smallest data type that meets requirements
- ✅ String length with explicit limits
- ✅ Store time in UTC
- ✅ Use fixed-point numbers or integers for amounts (avoid floating-point)
- ❌ Prohibit abusing TEXT/BLOB types

---

## 🔐 Data Integrity

### Constraint Settings
- ✅ Primary key constraint: Each table must have primary key
- ✅ Not null constraint: Required fields clearly marked
- ✅ Unique constraint: Business uniqueness guaranteed by indexes
- ✅ Foreign key constraint: Relationships clearly defined
- ✅ Check constraint: Business rules validated at database layer

### Default Values and Computed Fields
- ✅ Set reasonable defaults (created_at defaults to current time)
- ✅ Status fields have clear initial values
- ✅ Computed fields consider storage vs real-time computation tradeoff
- ❌ Avoid NULL ambiguity (use defaults or Optional types)

---

## 📊 Query Optimization

### Query Design
- ✅ Clear query intent, avoid SELECT *
- ✅ Use parameterized queries (prevent SQL injection)
- ✅ Break down complex queries into multi-step execution
- ✅ Avoid N+1 query problems
- ✅ Use EXPLAIN to analyze execution plan
- ❌ Prohibit function operations on columns in WHERE clause

### Index Strategy
- ✅ Build indexes on high-frequency query fields
- ✅ Composite indexes follow leftmost prefix principle
- ✅ Covering indexes optimize query performance
- ✅ Regularly monitor index usage
- ✅ Delete unused indexes
- ❌ Avoid over-indexing (affects write performance)

### Pagination and Limits
- ✅ Large datasets must be paginated
- ✅ Use cursor-based pagination not OFFSET (for large offsets)
- ✅ Limit single query return rows (< 10000 rows)
- ✅ Aggregation queries consider time range limits

---

## ⚡ Transaction Management

### Transaction Principles
- ✅ Clear transaction boundaries (ACID requirements)
- ✅ Keep transactions as short as possible (reduce lock hold time)
- ✅ Avoid external IO operations in transactions
- ✅ Use appropriate isolation level
- ✅ Explicit commit or rollback

### Concurrency Control
- ✅ Understand concurrency issues (dirty read, non-repeatable read, phantom read)
- ✅ Use optimistic locking or pessimistic locking
- ✅ Avoid deadlocks (access resources in same order)
- ✅ Set transaction timeout
- ❌ Avoid holding locks for long periods

---

## 🔄 Data Migration

### Migration Standards
- ✅ All schema changes via migration scripts
- ✅ Migration scripts must be repeatable (idempotent)
- ✅ Backward compatible change strategy
- ✅ Large table changes executed in batches
- ✅ Backup data before migration
- ❌ Prohibit manually modifying production database schema

### Version Control
- ✅ Migration files named by timestamp or version number
- ✅ Record migration history
- ✅ Provide rollback scripts
- ✅ Verify migrations in test environment
- ❌ Prohibit modifying already executed migration scripts

---

## 🛡️ Data Security

### Access Control
- ✅ Least privilege principle
- ✅ Application accounts only have necessary permissions (prohibit root connection)
- ✅ Encrypt sensitive data storage
- ✅ Regularly audit database access logs
- ❌ Prohibit hardcoding database credentials in code

### SQL Injection Protection
- ✅ 100% use parameterized queries/prepared statements
- ✅ Validate and sanitize user input
- ✅ Limit database error information exposure
- ❌ Prohibit string concatenated SQL

### Data Masking
- ✅ Mask sensitive fields (phone, email, ID card)
- ✅ Don't log sensitive data
- ✅ Use masked data in test environment
- ❌ Prohibit plaintext password storage

---

## 📈 Performance and Monitoring

### Performance Optimization
- ✅ Monitor slow query logs
- ✅ Regularly analyze table statistics
- ✅ Reasonable connection pool usage
- ✅ Cache hot data
- ✅ Read-write separation (read-heavy scenarios)
- ✅ Database sharding (ultra-large scale data)

### Capacity Planning
- ✅ Monitor data growth trends
- ✅ Regularly clean historical data
- ✅ Archive cold data
- ✅ Set table size alerts
- ✅ Reserve storage space

---

## 💾 Backup and Recovery

### Backup Strategy
- ✅ Regular full backups
- ✅ Incremental backups (high-frequency change scenarios)
- ✅ Verify backup recoverability
- ✅ Offsite backup storage
- ✅ Record backup time points

### Disaster Recovery
- ✅ Define Recovery Time Objective (RTO)
- ✅ Define Recovery Point Objective (RPO)
- ✅ Regularly drill recovery process
- ✅ Master-slave replication/cluster high availability
- ✅ Monitor replication lag

---

## 🧪 Database Testing

### Test Scope
- ✅ Migration script testing
- ✅ Query performance testing
- ✅ Concurrent stress testing
- ✅ Data integrity testing
- ✅ Backup recovery testing

### Test Data
- ✅ Use separate test database
- ✅ Simulate production data volume
- ✅ Test boundary conditions (null values, extreme values)
- ❌ Prohibit testing on production database

---

## 📋 Database Development Checklist

- [ ] Data model matches business domain
- [ ] Primary keys, indexes, constraints complete
- [ ] Queries use parameterization (prevent SQL injection)
- [ ] Indexes cover high-frequency queries
- [ ] Transaction boundaries clear and short
- [ ] Migration scripts idempotent and rollback-capable
- [ ] Sensitive data encryption and masking
- [ ] Slow query monitoring and optimization
- [ ] Backup strategy and recovery verification
- [ ] Migrations and queries have test coverage

---

---

## 🏛️ Advanced Architecture Patterns (20+ years experience)

### Distributed Database Architecture
```
Sharding Strategy:
- Horizontal sharding: By user ID/time range
- Vertical sharding: By business module
- Consistent hashing: Dynamic scaling
- Sharding key selection: High cardinality, even distribution, frequent queries

Read-Write Separation Architecture:
- Master-slave replication (async/semi-sync/sync)
- Read request load balancing
- Write-after-read consistency guarantee
- Automatic failover

Multi-Active Architecture:
- Multi-master replication (conflict resolution)
- Partition fault tolerance (CAP tradeoff)
- Nearest access (geographic distribution)
- Data sync latency monitoring
```

### NewSQL and Distributed Transactions
```
Distributed Transaction Patterns:
- 2PC (Two-Phase Commit): Strong consistency, poor performance
- TCC (Try-Confirm-Cancel): Eventual consistency
- Saga Pattern: Long transaction orchestration
- Local message table: Reliable message delivery

NewSQL Selection:
- TiDB: MySQL compatible, horizontal scaling
- CockroachDB: PostgreSQL compatible, strong consistency
- YugabyteDB: Multi-model support
- Applicable scenarios: OLTP + distributed
```

### Multi-Model Database Design
```
Relational (RDBMS):
- Applicable: Transaction processing, strong consistency requirements
- Representatives: PostgreSQL, MySQL

Document (Document):
- Applicable: Flexible schema, nested data
- Representatives: MongoDB, Couchbase

Time-Series (Time-Series):
- Applicable: Monitoring, IoT, financial quotes
- Representatives: TimescaleDB, InfluxDB

Graph (Graph):
- Applicable: Social networks, knowledge graphs
- Representatives: Neo4j, Amazon Neptune

Vector (Vector):
- Applicable: AI retrieval, similarity search
- Representatives: Pinecone, Milvus, pgvector
```

---

## 🔧 Essential Skills for Senior Developers

### Query Optimization Deep Techniques
```
Execution Plan Analysis:
- EXPLAIN ANALYZE actual execution statistics
- Identify Seq Scan vs Index Scan
- Identify Nested Loop vs Hash Join
- Evaluate Rows estimate accuracy

Index Advanced Strategy:
- Partial indexes (WHERE condition)
- Expression indexes (function indexes)
- Covering indexes (Include columns)
- Conditional indexes (filter indexes)

Query Rewrite Techniques:
- CTE recursive query optimization
- Window functions replace self-join
- EXISTS replaces IN (subquery)
- LATERAL JOIN advanced usage
```

### High Concurrency Scenario Optimization
```
Lock Optimization:
- Row-level locks vs table-level locks
- Optimistic locking (version number) vs pessimistic locking
- Avoid lock escalation
- Deadlock detection and prevention

Connection Pool Tuning:
- Pool size = (core_count * 2) + disk_count
- Connection lifecycle management
- Warm-up strategy
- Monitor idle connections

Batch Operation Optimization:
- Bulk INSERT
- COPY command (PostgreSQL)
- Process large transactions in batches
- Delayed index updates
```

### Data Archiving and Cold-Hot Separation
```
Tiered Storage Strategy:
- Hot data: SSD, high-frequency access
- Warm data: HDD, periodic access
- Cold data: Object storage, archive query

Archival Solutions:
- Time partitioning (by month/quarter)
- Automatic archive triggers
- Archive table compression
- Archive data queryable

Table Partitioning:
- Range partitioning (time)
- List partitioning (enumeration values)
- Hash partitioning (even distribution)
- Partition pruning
```

### High Availability and Disaster Recovery
```
Replication Topology:
- Cascading replication (reduce master load)
- Circular replication (multi-datacenter)
- Delayed replication (mistake recovery)

Failover:
- Automatic failover (Patroni/Orchestrator)
- VIP drift
- DNS switching
- Application layer routing

RPO/RTO Design:
- RPO=0: Synchronous replication (performance sacrifice)
- RPO<1min: Semi-synchronous replication
- RTO<30s: Automatic failover
```

---

## 🚨 Common Pitfalls for Senior Developers

### Design Traps
```
❌ Over-normalization:
- Split all data into independent tables
- Queries require multi-table JOIN
- Correct: Moderate denormalization based on access patterns

❌ Abuse JSON/JSONB fields:
- Store relational data as JSON
- Lose constraint and index advantages
- Correct: JSON for truly flexible data

❌ Ignore data growth:
- Design only considers current data volume
- Queries slow after table bloat
- Correct: Capacity planning, reserve partitions
```

### Performance Traps
```
❌ SELECT * inertia:
- Query all columns
- Cannot use covering indexes
- Correct: Explicitly specify needed columns

❌ ORM abuse:
- N+1 query problem
- Over-abstraction hides inefficient queries
- Correct: Monitor ORM-generated SQL

❌ Over-indexing:
- Build index on every column
- Write performance severely degraded
- Correct: Build indexes based on query patterns
```

### Operational Traps
```
❌ Large table DDL without evaluation:
- Direct ALTER TABLE large table
- Long table locks
- Correct: Online DDL tools (pt-osc/gh-ost)

❌ Backup not verified:
- Have backups but never recovery tested
- Discover backup corrupted when actually needed
- Correct: Regular recovery drills

❌ Ignore replication lag:
- Read from slave without considering lag
- Data inconsistency
- Correct: Monitor lag, critical reads go to master
```

---

## 📊 Performance Monitoring Metrics

| Metric | Target | Alert Threshold | Measurement Tool |
|--------|--------|-----------------|------------------|
| Query Response Time (P99) | < 100ms | > 500ms | APM/Slow query log |
| QPS | Scenario-based | > 80% capacity | Monitoring system |
| Connection Utilization | < 70% | > 90% | Connection pool monitoring |
| Cache Hit Rate | > 95% | < 80% | Database statistics |
| Replication Lag | < 1s | > 10s | Replication monitoring |
| Deadlock Frequency | 0 | > 1/hour | Database log |
| Disk Usage | < 70% | > 85% | System monitoring |
| IOPS | Storage-based | > 80% capacity | IO monitoring |
| Long Transactions | 0 | > 5 minutes | Transaction monitoring |
| Index Bloat | < 20% | > 50% | pg_stat_user_indexes |

---

## 📋 Database Development Checklist (Complete Version)

### Design Checks
- [ ] Data model matches business domain
- [ ] Partition/sharding strategy clear
- [ ] Primary keys, indexes, constraints complete
- [ ] Consider future data growth

### Query Checks
- [ ] All queries use parameterization
- [ ] Execution plan analyzed
- [ ] No N+1 query issues
- [ ] High-frequency queries have index coverage

### Transaction Checks
- [ ] Transaction boundaries clear and short
- [ ] Concurrency control strategy clear
- [ ] No long transactions

### Operational Checks
- [ ] Migration scripts idempotent and rollback-capable
- [ ] Backup strategy and recovery verified
- [ ] Monitoring and alerts configured
- [ ] High availability solution tested

---

**Database Development Principles Summary**:
Data Integrity, Query Optimization, Transaction ACID, Security Protection, Performance Monitoring, Backup Recovery, Migration Version Control, Least Privilege, Parameterized Queries, Capacity Planning
