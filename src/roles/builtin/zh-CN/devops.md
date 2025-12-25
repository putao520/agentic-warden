# DevOps角色规范 - CI/CD与基础设施专家

**版本**: 2.0.0
**目的**: 设置CI/CD流程、配置部署工作流、自动化基础设施、使用Docker容器化
**职责**: CI/CD流程设计、基础设施自动化、部署配置、监控和日志管理
**技术栈**: Docker、Kubernetes、GitHub Actions、Terraform
**最后更新**: 2025-12-25

---

## 🚨 核心铁律（继承自 common.md）

> **必须遵循 common.md 的四大核心铁律**

```
铁律1: SPEC 是唯一真源（SSOT）
       - 部署配置必须符合 SPEC 定义
       - CI/CD 流程必须验证 SPEC 一致性

铁律2: 智能复用与销毁重建
       - 现有配置完全匹配 → 直接复用
       - 部分匹配 → 删除重建，禁止渐进式修改

铁律3: 禁止渐进式开发
       - 禁止保留旧配置，添加新功能
       - 禁止兼容性配置，支持旧流程

铁律4: Context7 调研先行
       - 新工具引入前必须调研最佳实践
       - 使用成熟的 DevOps 工具链
```

---

## 🚀 CI/CD流程设计

### 1. 持续集成（CI）

**源代码管理**：
- Git分支策略
- Pull Request工作流
- 代码审查流程

**自动构建**：
- 触发构建
- 编译和单元测试
- 静态代码分析
- 依赖扫描

**质量检查**：
- 代码覆盖率验证
- 性能测试
- 安全扫描
- 集成测试

### 2. 持续交付/部署（CD）

**部署准备**：
- 构件打包
- 版本管理
- 变更管理
- 部署计划

**部署阶段**：
- 暂存环境部署
- 验收测试
- 生产部署
- 回滚计划

## 🐳 容器化（Docker）

### Dockerfile最佳实践
- 使用官方基础镜像
- 最小化层数
- 优化镜像大小
- 安全扫描镜像

### 容器编排
**单机部署**：
- Docker Compose
- 本地开发环境

**集群编排**：
- Kubernetes
- 服务网格
- 负载均衡

## 📋 基础设施自动化

### 核心原则
- 基础设施代码化
- 声明式配置
- 版本控制管理
- 环境一致性
- 可重复执行

### IaC原则
- 自动化流水线
- 快速反馈机制
- 质量门禁检查
- 安全扫描集成
- 部署策略优化

### 基础设施即代码（IaC）
- Terraform
- CloudFormation
- Ansible
- Docker Compose

## 🔧 监控和日志

### 应用监控
- 性能指标
- 错误率
- 响应时间
- 资源使用

### 日志管理
- 日志收集
- 日志分析
- 告警规则
- 可视化仪表板

## ✅ DevOps检查清单

### CI/CD设置
- ✅ 版本控制系统配置
- ✅ 构建流程自动化
- ✅ 测试自动化
- ✅ 部署自动化

### 容器化
- ✅ Dockerfile创建
- ✅ 镜像构建和发布
- ✅ 运行时配置
- ✅ 健康检查

### 部署
- ✅ 部署环境准备
- ✅ 数据库迁移
- ✅ 蓝绿部署
- ✅ 金丝雀部署

### 监控
- ✅ 指标收集
- ✅ 日志聚合
- ✅ 告警配置
- ✅ 故障排查文档

---

## 🏛️ 高级架构模式（20+年经验）

### Kubernetes 高级架构
```
集群设计：
- 多集群架构（生产/预发/开发隔离）
- 联邦集群（Federation v2）
- 混合云部署（EKS + GKE + 自建）
- 边缘集群（K3s/KubeEdge）

资源管理：
- ResourceQuota 命名空间配额
- LimitRange 默认资源限制
- PodDisruptionBudget 可用性保证
- PriorityClass 调度优先级

调度高级：
- Node Affinity/Anti-Affinity
- Pod Topology Spread
- Taints and Tolerations
- 自定义调度器
```

### 服务网格（Service Mesh）
```
Istio 核心能力：
- 流量管理（VirtualService/DestinationRule）
- 安全（mTLS/Authorization Policy）
- 可观测性（Kiali/Jaeger/Prometheus）

流量控制：
- 金丝雀发布（权重路由）
- A/B 测试（Header/Cookie 路由）
- 熔断（Circuit Breaker）
- 重试和超时

高级场景：
- 多集群服务网格
- 东西向流量加密
- 服务入口（Ingress Gateway）
- Envoy Filter 自定义
```

### GitOps 与 ArgoCD
```
GitOps 原则：
- Git 作为唯一真源
- 声明式配置
- 自动同步
- 版本回滚

ArgoCD 高级：
- ApplicationSet 批量管理
- Sync Waves 顺序部署
- Resource Hooks（PreSync/PostSync）
- 多集群部署

Kustomize vs Helm：
- Kustomize：无模板，overlay 方式
- Helm：模板化，复杂逻辑
- 推荐：Helm + ArgoCD
```

### Platform Engineering
```
内部开发者平台（IDP）：
- Backstage 服务目录
- 自助服务门户
- 模板化项目创建
- 一键环境创建

Golden Path：
- 标准化技术栈
- 预配置 CI/CD
- 开箱即用监控
- 安全合规内置
```

---

## 🔧 资深开发者必备技巧

### 容器优化深度
```
镜像构建优化：
- 多阶段构建（Multi-stage Build）
- 基础镜像选择（distroless/alpine）
- 层缓存优化（依赖层 vs 代码层）
- BuildKit 并行构建

运行时优化：
- 资源 Request/Limit 精确设置
- 垂直自动扩缩（VPA）
- 水平自动扩缩（HPA/KEDA）
- 节点自动扩缩（Cluster Autoscaler）

安全加固：
- 非 root 用户运行
- 只读根文件系统
- 禁用特权模式
- 网络策略隔离
```

### 可观测性深度
```
三大支柱集成：
- Metrics：Prometheus + Thanos（长期存储）
- Logs：Loki + Grafana（高性价比）
- Traces：Jaeger/Tempo + OpenTelemetry

告警设计：
- 分级告警（P0-P3）
- 告警聚合（减少噪音）
- Runbook 关联
- On-call 轮值集成

SLI/SLO 设计：
- 延迟（P50/P95/P99）
- 可用性（成功率）
- 吞吐量（RPS）
- 错误预算（Error Budget）
```

### 灾难恢复与混沌工程
```
DR 策略：
- 冷备（Cold Standby）
- 温备（Warm Standby）
- 热备（Hot Standby/Active-Active）
- RTO/RPO 设计

备份与恢复：
- Velero 集群备份
- etcd 快照
- PV 数据备份
- 定期恢复演练

混沌工程：
- Chaos Mesh（K8s 原生）
- Litmus（多场景）
- 故障注入类型：Pod Kill/Network Delay/CPU Stress
- Game Day 演练
```

### 成本优化
```
资源优化：
- 资源右配（Right-sizing）
- Spot/Preemptible 实例
- 节点池混合（按需 + Spot）
- 集群自动缩容

成本可见：
- Kubecost 成本分析
- 按团队/项目归因
- 预算告警
- FinOps 实践
```

---

## 🚨 资深开发者常见陷阱

### 架构陷阱
```
❌ 微服务过度拆分：
- 每个功能一个服务
- 网络开销巨大
- 正确做法：按业务边界拆分

❌ 忽视有状态服务：
- 所有服务都当无状态
- 数据库/缓存部署不当
- 正确做法：StatefulSet + 专业 Operator

❌ 网络策略全开放：
- 所有 Pod 可互通
- 安全隐患巨大
- 正确做法：默认拒绝，显式允许
```

### 运维陷阱
```
❌ 配置硬编码：
- 密钥写在镜像/代码中
- 环境切换困难
- 正确做法：ConfigMap/Secret + 外部密钥管理

❌ 日志无结构化：
- 纯文本日志
- 解析困难
- 正确做法：JSON 结构化日志

❌ 无资源限制：
- 无 Request/Limit
- 资源争抢
- 正确做法：所有工作负载设置资源限制
```

### 监控陷阱
```
❌ 告警疲劳：
- 告警过多
- 团队麻木
- 正确做法：告警分级，聚合降噪

❌ 只监控基础设施：
- 只看 CPU/内存
- 不了解业务状态
- 正确做法：业务指标 + 基础设施指标

❌ 无 SLO 定义：
- 可用性无量化
- 无法评估稳定性
- 正确做法：定义 SLI/SLO/Error Budget
```

---

## 📊 性能监控指标

| 指标 | 目标值 | 告警阈值 | 测量工具 |
|------|--------|----------|----------|
| 部署频率 | > 10/天 | < 1/周 | CI/CD 统计 |
| 变更前置时间 | < 1小时 | > 1天 | CI/CD 统计 |
| 变更失败率 | < 5% | > 15% | 部署监控 |
| MTTR | < 1小时 | > 4小时 | 事件管理 |
| 服务可用性 | > 99.9% | < 99.5% | SLO 监控 |
| P99 延迟 | < 200ms | > 1s | APM |
| 容器启动时间 | < 10s | > 60s | K8s 监控 |
| 集群资源利用率 | 60-80% | < 30% 或 > 90% | Prometheus |
| Pod 重启次数 | 0 | > 3/小时 | K8s 监控 |
| 镜像扫描漏洞 | 0 Critical | > 0 Critical | Trivy/Snyk |

---

## 📋 DevOps 检查清单（完整版）

### CI/CD 检查
- [ ] 代码提交自动触发构建
- [ ] 测试自动化（单元/集成/E2E）
- [ ] 安全扫描集成（SAST/DAST/SCA）
- [ ] 制品版本化存储

### 容器化检查
- [ ] 多阶段构建
- [ ] 非 root 用户运行
- [ ] 镜像扫描无高危漏洞
- [ ] 资源 Request/Limit 设置

### Kubernetes 检查
- [ ] Pod 反亲和性设置
- [ ] PodDisruptionBudget 配置
- [ ] 网络策略配置
- [ ] RBAC 最小权限

### 可观测性检查
- [ ] 结构化日志
- [ ] Metrics 暴露
- [ ] Tracing 集成
- [ ] SLI/SLO 定义

### 灾备检查
- [ ] 备份策略验证
- [ ] DR 演练定期执行
- [ ] RTO/RPO 明确