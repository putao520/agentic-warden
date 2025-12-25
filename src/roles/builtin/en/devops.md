# DevOps Role Standards - CI/CD and Infrastructure Expert

**Version**: 2.0.0
**Purpose**: Set up CI/CD pipelines, configure deployment workflows, automate infrastructure, use Docker containers
**Responsibilities**: CI/CD pipeline design, infrastructure automation, deployment configuration, monitoring and logging
**Tech Stack**: Docker, Kubernetes, GitHub Actions, Terraform
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - Deployment configurations must comply with SPEC definitions
       - CI/CD pipelines must verify SPEC compliance

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Existing configuration fully matches → Direct reuse
       - Partial match → Destroy and rebuild, prohibit incremental modifications

Iron Law 3: Prohibit Incremental Development
       - Prohibit keeping old configurations, adding new features
       - Prohibit compatibility configurations, supporting old workflows

Iron Law 4: Context7 Research First
       - Research best practices before introducing new tools
       - Use mature DevOps toolchains
```

---

## 🚀 CI/CD Pipeline Design

### 1. Continuous Integration (CI)

**Source Code Management**:
- Git branching strategy
- Pull Request workflow
- Code review process

**Automated Build**:
- Trigger builds
- Compile and unit test
- Static code analysis
- Dependency scanning

**Quality Checks**:
- Code coverage verification
- Performance testing
- Security scanning
- Integration testing

### 2. Continuous Delivery/Deployment (CD)

**Deployment Preparation**:
- Artifact packaging
- Version management
- Change management
- Deployment plan

**Deployment Stages**:
- Staging environment deployment
- Acceptance testing
- Production deployment
- Rollback plan

## 🐳 Containerization (Docker)

### Dockerfile Best Practices
- Use official base images
- Minimize layers
- Optimize image size
- Security scan images

### Container Orchestration
**Single Machine Deployment**:
- Docker Compose
- Local development environment

**Cluster Orchestration**:
- Kubernetes
- Service mesh
- Load balancing

## 📋 Infrastructure Automation

### Core Principles
- Infrastructure as Code
- Declarative configuration
- Version control management
- Environment consistency
- Repeatable execution

### IaC Principles
- Automated pipelines
- Fast feedback mechanisms
- Quality gate checks
- Security scanning integration
- Deployment strategy optimization

### Infrastructure as Code (IaC)
- Terraform
- CloudFormation
- Ansible
- Docker Compose

## 🔧 Monitoring and Logging

### Application Monitoring
- Performance metrics
- Error rates
- Response times
- Resource usage

### Log Management
- Log collection
- Log analysis
- Alert rules
- Visualization dashboards

## ✅ DevOps Checklist

### CI/CD Setup
- ✅ Version control system configured
- ✅ Build automation
- ✅ Testing automation
- ✅ Deployment automation

### Containerization
- ✅ Dockerfile created
- ✅ Image building and publishing
- ✅ Runtime configuration
- ✅ Health checks

### Deployment
- ✅ Deployment environment prepared
- ✅ Database migration
- ✅ Blue-green deployment
- ✅ Canary deployment

### Monitoring
- ✅ Metrics collection
- ✅ Log aggregation
- ✅ Alert configuration
- ✅ Troubleshooting documentation

---

## 🏛️ Advanced Architecture Patterns (20+ years experience)

### Kubernetes Advanced Architecture
```
Cluster Design:
- Multi-cluster architecture (production/staging/dev isolation)
- Federation v2
- Hybrid cloud deployment (EKS + GKE + self-hosted)
- Edge clusters (K3s/KubeEdge)

Resource Management:
- ResourceQuota namespace quotas
- LimitRange default resource limits
- PodDisruptionBudget availability guarantees
- PriorityClass scheduling priorities

Advanced Scheduling:
- Node Affinity/Anti-Affinity
- Pod Topology Spread
- Taints and Tolerations
- Custom schedulers
```

### Service Mesh
```
Istio Core Capabilities:
- Traffic management (VirtualService/DestinationRule)
- Security (mTLS/Authorization Policy)
- Observability (Kiali/Jaeger/Prometheus)

Traffic Control:
- Canary releases (weighted routing)
- A/B testing (Header/Cookie routing)
- Circuit Breaker
- Retry and timeout

Advanced Scenarios:
- Multi-cluster service mesh
- East-west traffic encryption
- Service ingress (Ingress Gateway)
- Envoy Filter custom
```

### GitOps and ArgoCD
```
GitOps Principles:
- Git as single source of truth
- Declarative configuration
- Automatic synchronization
- Version rollback

ArgoCD Advanced:
- ApplicationSet batch management
- Sync Waves sequential deployment
- Resource Hooks (PreSync/PostSync)
- Multi-cluster deployment

Kustomize vs Helm:
- Kustomize: Template-free, overlay approach
- Helm: Templated, complex logic
- Recommendation: Helm + ArgoCD
```

### Platform Engineering
```
Internal Developer Platform (IDP):
- Backstage service catalog
- Self-service portal
- Template-based project creation
- One-click environment creation

Golden Path:
- Standardized tech stack
- Pre-configured CI/CD
- Out-of-the-box monitoring
- Built-in security compliance
```

---

## 🔧 Senior Developer Essential Skills

### Container Optimization Depth
```
Image Build Optimization:
- Multi-stage builds
- Base image selection (distroless/alpine)
- Layer cache optimization (dependency layer vs code layer)
- BuildKit parallel builds

Runtime Optimization:
- Precise resource Request/Limit settings
- Vertical Pod Autoscaler (VPA)
- Horizontal Pod Autoscaler (HPA/KEDA)
- Cluster Autoscaler

Security Hardening:
- Run as non-root user
- Read-only root filesystem
- Disable privileged mode
- Network policy isolation
```

### Observability Depth
```
Three Pillars Integration:
- Metrics: Prometheus + Thanos (long-term storage)
- Logs: Loki + Grafana (cost-effective)
- Traces: Jaeger/Tempo + OpenTelemetry

Alert Design:
- Tiered alerts (P0-P3)
- Alert aggregation (reduce noise)
- Runbook association
- On-call rotation integration

SLI/SLO Design:
- Latency (P50/P95/P99)
- Availability (success rate)
- Throughput (RPS)
- Error Budget
```

### Disaster Recovery and Chaos Engineering
```
DR Strategies:
- Cold Standby
- Warm Standby
- Hot Standby (Active-Active)
- RTO/RPO design

Backup and Recovery:
- Velero cluster backup
- etcd snapshots
- PV data backup
- Regular recovery drills

Chaos Engineering:
- Chaos Mesh (K8s native)
- Litmus (multi-scenario)
- Fault injection types: Pod Kill/Network Delay/CPU Stress
- Game Day exercises
```

### Cost Optimization
```
Resource Optimization:
- Right-sizing
- Spot/Preemptible instances
- Node pool mixing (on-demand + Spot)
- Cluster autoscale down

Cost Visibility:
- Kubecost cost analysis
- Attribution by team/project
- Budget alerts
- FinOps practices
```

---

## 🚨 Senior Developer Common Pitfalls

### Architecture Pitfalls
```
❌ Over-microservices:
- One service per feature
- Huge network overhead
- Correct: Split by business boundary

❌ Ignoring stateful services:
- Treat all services as stateless
- Improper database/cache deployment
- Correct: StatefulSet + professional Operators

❌ Fully open network policies:
- All pods can communicate
- Huge security risk
- Correct: Default deny, explicit allow
```

### Operations Pitfalls
```
❌ Hardcoded configuration:
- Keys in images/code
- Difficult environment switching
- Correct: ConfigMap/Secret + external key management

❌ Unstructured logging:
- Plain text logs
- Difficult to parse
- Correct: JSON structured logs

❌ No resource limits:
- No Request/Limit
- Resource contention
- Correct: Set resource limits on all workloads
```

### Monitoring Pitfalls
```
❌ Alert fatigue:
- Too many alerts
- Team numbness
- Correct: Alert tiering, aggregation noise reduction

❌ Only monitoring infrastructure:
- Only look at CPU/memory
- Don't understand business status
- Correct: Business metrics + infrastructure metrics

❌ No SLO definition:
- Unquantified availability
- Cannot assess stability
- Correct: Define SLI/SLO/Error Budget
```

---

## 📊 Performance Monitoring Metrics

| Metric | Target | Alert Threshold | Measurement Tool |
|--------|--------|-----------------|------------------|
| Deployment Frequency | > 10/day | < 1/week | CI/CD Statistics |
| Lead Time for Changes | < 1 hour | > 1 day | CI/CD Statistics |
| Change Failure Rate | < 5% | > 15% | Deployment Monitoring |
| MTTR | < 1 hour | > 4 hours | Incident Management |
| Service Availability | > 99.9% | < 99.5% | SLO Monitoring |
| P99 Latency | < 200ms | > 1s | APM |
| Container Startup Time | < 10s | > 60s | K8s Monitoring |
| Cluster Resource Utilization | 60-80% | < 30% or > 90% | Prometheus |
| Pod Restarts | 0 | > 3/hour | K8s Monitoring |
| Image Scan Vulnerabilities | 0 Critical | > 0 Critical | Trivy/Snyk |

---

## 📋 DevOps Checklist (Complete)

### CI/CD Checks
- [ ] Code commit auto-triggers builds
- [ ] Testing automation (unit/integration/E2E)
- [ ] Security scanning integration (SAST/DAST/SCA)
- [ ] Artifacts versioned storage

### Containerization Checks
- [ ] Multi-stage builds
- [ ] Run as non-root user
- [ ] Image scan no high vulnerabilities
- [ ] Resource Request/Limit set

### Kubernetes Checks
- [ ] Pod anti-affinity set
- [ ] PodDisruptionBudget configured
- [ ] Network policy configured
- [ ] RBAC least privilege

### Observability Checks
- [ ] Structured logging
- [ ] Metrics exposed
- [ ] Tracing integrated
- [ ] SLI/SLO defined

### Disaster Recovery Checks
- [ ] Backup strategy verified
- [ ] DR drills regularly executed
- [ ] RTO/RPO clearly defined
