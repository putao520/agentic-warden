# 测试开发规范 - TESTING-STANDARDS

**版本**: 2.0.0
**适用范围**: E2E测试、集成测试、系统级测试
**最后更新**: 2025-12-25

---

## 🚨 七大铁律（违反即失败）

### 铁律1: CI-Only（零本地测试）

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ❌ 禁止：npm test, pytest, go test 等本地命令                          │
│  ❌ 禁止：在主机直接执行任何测试                                         │
│  ❌ 禁止：localhost:xxxx 访问测试服务                                   │
│  ✅ 必须：所有测试在 docker-compose.ci.yml 容器内执行                   │
│  ✅ 必须：通过 AI Dev-Loop 或 just test-e2e 触发容器内测试              │
└─────────────────────────────────────────────────────────────────────────┘
```

### 铁律2: 容器全隔离（零端口暴露）

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ❌ 禁止：ports: "8080:8080" 暴露到主机                                  │
│  ❌ 禁止：测试代码使用 localhost:xxxx                                   │
│  ❌ 禁止：从主机直接访问容器服务                                         │
│  ✅ 必须：使用 docker-compose 内网 DNS（http://service:port）           │
│  ✅ 必须：测试容器与被测服务在同一docker网络                             │
│  ✅ 必须：仅使用 expose 暴露容器内端口                                   │
└─────────────────────────────────────────────────────────────────────────┘
```

### 铁律3: 三层完整性（必须全覆盖）

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Layer 1: 服务级E2E                                                     │
│  ├─ 每个子项目独立的API/UI测试                                          │
│  ├─ 覆盖该服务的所有REQ-XXX                                             │
│  └─ 在docker-compose.ci.yml中独立验证                                   │
│                                                                         │
│  Layer 2: 平面级E2E（跨服务）                                            │
│  ├─ 数据平面：请求→处理→响应的完整链路                                  │
│  ├─ 控制平面：配置变更→服务重载→生效验证                                │
│  └─ 其他业务平面的端到端流程                                            │
│                                                                         │
│  Layer 3: 产品级E2E                                                     │
│  ├─ 完整用户旅程（注册→配置→使用→结果）                                │
│  └─ 生产环境模拟的全链路验证                                            │
│                                                                         │
│  ❌ 禁止：只做服务级测试就声称"E2E完成"                                 │
│  ✅ 必须：三层都有测试用例且全部通过                                     │
└─────────────────────────────────────────────────────────────────────────┘
```

### 铁律4: 卷映射优先（禁止重建镜像部署）

```
┌─────────────────────────────────────────────────────────────────────────┐
│  代码部署方式                                                            │
│  ─────────────────────────────────────────────────────────────────────  │
│  ✅ 必须使用卷映射：通过 -v 参数将本地源代码和依赖映射到容器              │
│  ✅ 容器只包含运行时环境：镜像只安装 Node.js、Python 等基础环境          │
│  ✅ 实时更新：代码修改后立即在容器内生效，无需重建                        │
│                                                                         │
│  ❌ 禁止重建镜像部署代码：严禁通过 Docker build 更新测试代码             │
│  ❌ 禁止 COPY 源代码：Dockerfile 不得包含 COPY src 等指令               │
│  ❌ 禁止预编译部署：不得在构建时编译源代码                               │
│  ❌ 禁止容器内 npm install/pip install：依赖预先安装或映射               │
│                                                                         │
│  正确配置示例：                                                          │
│  volumes:                                                               │
│    - ./tests:/app/tests              # 测试代码映射                     │
│    - ./node_modules:/app/node_modules  # 依赖映射                       │
└─────────────────────────────────────────────────────────────────────────┘
```

### 铁律5: 禁止虚假测试通过

```
┌─────────────────────────────────────────────────────────────────────────┐
│  虚假通过 = 测试报告"成功"，但实际未验证预期行为                         │
│                                                                         │
│  ❌ 禁止：条件式跳过                                                     │
│     if !serviceHealthy() { t.Skip("service not ready") }               │
│                                                                         │
│  ❌ 禁止：容错返回                                                       │
│     if status != 200 { return }  // 静默跳过                            │
│                                                                         │
│  ❌ 禁止：空断言                                                         │
│     func TestEmpty(t *testing.T) { /* 无断言 */ }                       │
│                                                                         │
│  ❌ 禁止：占位符测试                                                     │
│     func TestFeature(t *testing.T) { // TODO: implement }              │
│                                                                         │
│  ❌ 禁止：虚假覆盖率汇报                                                 │
│     统计 TEST-ID 数量而非实际验证                                       │
│     声称"100%通过"而实际有跳过                                          │
│                                                                         │
│  ✅ 正确：测试失败 = 暴露问题 = 修复问题                                 │
│  ✅ 正确：环境问题 = 测试失败 = 修复环境                                 │
│  ✅ 正确：功能未实现 = 测试失败 = 实现功能                               │
└─────────────────────────────────────────────────────────────────────────┘
```

### 铁律6: 测试四类别覆盖

```
┌─────────────────────────────────────────────────────────────────────────┐
│  每个功能/API 必须覆盖四类测试                                           │
│                                                                         │
│  1. 正向测试（Happy Path）                                              │
│     ├─ 每个功能至少 2 个正向测试                                        │
│     ├─ 验证正常输入产生正确输出                                         │
│     └─ 验证完整响应结构（不只是状态码）                                  │
│                                                                         │
│  2. 负向测试（Error Cases）                                             │
│     ├─ 每个输入参数至少 1 个负向测试                                    │
│     ├─ 验证错误输入产生正确错误响应                                     │
│     └─ 验证错误码和错误消息                                             │
│                                                                         │
│  3. 边界测试（Boundary Cases）                                          │
│     ├─ 测试零值、空值、null、undefined                                  │
│     ├─ 测试最小值、最大值、边界附近值                                   │
│     └─ 测试极端情况（超长字符串、超大数字等）                           │
│                                                                         │
│  4. 安全测试（Security Cases）                                          │
│     ├─ SQL 注入攻击测试                                                 │
│     ├─ XSS 攻击测试                                                     │
│     ├─ 越权访问测试                                                     │
│     └─ 认证/授权绕过测试                                                │
│                                                                         │
│  ❌ 禁止：只写正向测试                                                   │
│  ❌ 禁止：只检查状态码 200                                               │
└─────────────────────────────────────────────────────────────────────────┘
```

### 铁律7: 每测试 ≥3 个有效断言

```
┌─────────────────────────────────────────────────────────────────────────┐
│  断言质量要求                                                            │
│                                                                         │
│  ✅ 每个测试函数必须有 ≥3 个有效断言                                    │
│  ✅ 必须验证完整响应结构（不只是状态码）                                 │
│  ✅ 断言必须验证具体值，不是仅存在性检查                                 │
│                                                                         │
│  ❌ 无效断言示例：                                                       │
│     assert.True(true)                                                   │
│     assert.NotNil(response)  // 只检查非空                              │
│     assert.Equal(200, status)  // 只检查状态码                          │
│                                                                         │
│  ✅ 有效断言示例：                                                       │
│     assert.Equal(200, resp.StatusCode)                                  │
│     assert.Equal("admin", resp.Body.User.Role)                          │
│     assert.Equal(10, len(resp.Body.Items))                              │
│     assert.Contains(resp.Body.Message, "success")                       │
│     assert.True(resp.Body.CreatedAt.Before(time.Now()))                 │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 🧪 测试数据规范

### 必须使用 Faker 生成测试数据

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ❌ 禁止简单测试数据：                                                   │
│     username: "test"                                                    │
│     email: "test@test.com"                                              │
│     password: "123456"                                                  │
│     phone: "12345678901"                                                │
│                                                                         │
│  ✅ 必须使用 Faker 库生成：                                              │
│     Go:      gofakeit.Username()                                        │
│     Python:  faker.email()                                              │
│     JS/TS:   faker.internet.email()                                     │
│                                                                         │
│  ✅ 每次测试使用唯一数据：                                               │
│     username: fmt.Sprintf("user_%s", gofakeit.UUID())                   │
│     email: fmt.Sprintf("%s@test.local", gofakeit.UUID())                │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 🚨 测试失败处理策略

### 禁止的回避行为

```
❌ 跳过测试
   - 禁止 t.Skip("环境问题")
   - 禁止 pytest.skip("功能未实现")
   - 禁止 test.skip("待修复")

❌ 降低验证标准
   - 禁止 把 assert status == 200 改成 assert status in [200, 500]
   - 禁止 删除断言让测试"通过"

❌ 容错绕过
   - 禁止 if status != 200: return  # 静默跳过
   - 禁止 try/except 吞掉断言失败

❌ 虚假修复
   - 禁止 硬编码预期值匹配当前错误输出
   - 禁止 修改测试来适应错误的实现
```

### 正确的失败处理（6类分类）

```
A类(CODE_BUG): 代码逻辑错误
   → 修复代码实现

B类(MISSING_FEATURE): 功能未实现
   → 实现缺失功能

C类(TEST_ISSUE): 测试设计问题
   → 修复测试代码（不是降低标准）

D类(ENV_ISSUE): 环境问题
   → 修复环境配置

E类(SPEC_ISSUE): SPEC问题
   → 暂停，报告给用户/architect

F类(TEST_QUALITY_ISSUE): 测试用例质量问题
   → 先增强测试用例，再修复代码
   → 检查：断言是否足够严格？测试覆盖是否完整？测试数据是否真实？
```

---

## 🔧 URL模式规范

### 正确模式（容器内DNS）

```go
// ✅ 正确
baseURL := "http://backend:8080"
baseURL := "http://api-service:8080"
baseURL := "http://frontend:80"
ws://websocket-service:8080
```

### 错误模式（禁止）

```go
// ❌ 禁止
http://localhost:8080
http://127.0.0.1:3000
http://0.0.0.0:8080
```

---

## 📋 测试代码要求

### 必须包含 TEST-ID 和 REQ 引用

```go
// TEST-E2E-SVC-AUTH-001
// 覆盖需求: REQ-AUTH-001, REQ-AUTH-002
// 测试类别: 正向测试
func TestUserAuthentication(t *testing.T) {
    // Arrange
    user := generateFakeUser()

    // Act
    resp := client.Login(user)

    // Assert (≥3 个有效断言)
    require.Equal(t, 200, resp.StatusCode)
    require.NotEmpty(t, resp.Body.Token)
    require.Equal(t, user.Email, resp.Body.User.Email)
    require.True(t, resp.Body.ExpiresAt.After(time.Now()))
}
```

### 禁止的测试代码

```go
// ❌ 禁止：条件跳过
func TestFeature(t *testing.T) {
    if !serviceHealthy() {
        t.Skip("service not ready")  // 禁止！
    }
}

// ❌ 禁止：空断言/不足3个断言
func TestEmpty(t *testing.T) {
    resp := callAPI()
    assert.Equal(t, 200, resp.StatusCode)  // 只有1个断言，不足！
}

// ❌ 禁止：容错返回
func TestWithFallback(t *testing.T) {
    status := callAPI()
    if status != 200 {
        return  // 禁止！应该失败
    }
}

// ❌ 禁止：简单测试数据
func TestUser(t *testing.T) {
    user := User{
        Name: "test",      // 禁止！使用 faker
        Email: "a@b.com",  // 禁止！使用 faker
    }
}
```

---

## 📊 覆盖率汇报规范

### 必须区分真实状态

```markdown
| 状态 | 数量 | 说明 |
|------|------|------|
| ✅ 完整实现 | 12 | 有 ≥3 断言、无条件跳过、四类别覆盖 |
| ⚠️ 条件跳过 | 2 | 包含 t.Skip（需审查） |
| ⚠️ 断言不足 | 3 | 断言 <3 个（需增强） |
| ❌ 占位符 | 1 | 空实现或 TODO |

真实覆盖率 = 完整实现 / 总数 = 12/18 = 67%
```

### 禁止的虚假汇报

```
❌ 禁止：统计 TEST-ID 数量而非实际验证
❌ 禁止：声称"100%通过"而实际有跳过
❌ 禁止：把跳过的测试算作"通过"
❌ 禁止：不统计断言不足的测试
```

---

## 🐳 docker-compose 配置规范

### 正确配置

```yaml
services:
  backend:
    expose:
      - "8080"  # 仅在docker网络内可见
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 10s
      timeout: 5s
      retries: 3
    # 无 ports 映射

  e2e-runner:
    volumes:
      - ./tests:/app/tests              # 测试代码映射
      - ./node_modules:/app/node_modules  # 依赖映射
    depends_on:
      backend:
        condition: service_healthy
```

### 错误配置（禁止）

```yaml
services:
  backend:
    ports:
      - "8080:8080"  # ❌ 禁止暴露到主机

  e2e-runner:
    build:
      dockerfile: Dockerfile
      # ❌ 禁止在 Dockerfile 中 COPY 测试代码
```

---

## 🔄 AI Dev-Loop 规范

### 唯一测试执行引擎

```bash
# 正确：使用系统级 ai-dev-loop
~/.claude/skills/testing/ai-dev-loop/dist/index.js start --project-root /path/to/project

# 退出码含义
0  = 所有测试通过
1  = 存在未修复的 BUG
2  = 配置错误
3  = 环境启动失败
10 = 达到最大迭代次数
20 = SPEC 问题（需人工介入）
```

### 禁止的执行方式

```bash
# ❌ 禁止：主机直接执行
npm test
pytest
go test ./...

# ❌ 禁止：项目自建 dev-loop
./scripts/dev-loop.sh
```

---

## 🎯 核心原则

```
1. 测试失败 = 发现问题 = 好事
   不是要隐藏问题，而是要修复问题

2. 代码问题 → 修复代码
   不是修改测试来适应错误的代码

3. SPEC问题 → 暂停问用户
   不是自己决定绕过SPEC

4. 环境问题 → 修复环境
   不是跳过测试或降低标准

5. 真实覆盖 > 虚假通过
   宁可报告50%真实覆盖，不可声称100%虚假通过

6. 质量优先 > 数量
   3个高质量测试 > 10个低质量测试

7. 四类别完整 > 单类别多个
   正向+负向+边界+安全 > 10个正向测试
```

---

## ✅ 测试开发检查清单

### 测试设计前
- [ ] 阅读 SPEC，识别所有 REQ-XXX
- [ ] 规划三层覆盖（服务级/平面级/产品级）
- [ ] 确认 docker-compose.ci.yml 配置正确

### 测试编写中
- [ ] 每个测试有 TEST-ID 和 REQ 引用
- [ ] 每个测试 ≥3 个有效断言
- [ ] 覆盖四类别（正向/负向/边界/安全）
- [ ] 使用 faker 生成测试数据
- [ ] 使用容器内 DNS，无 localhost

### 测试完成后
- [ ] 在 docker-compose.ci.yml 容器内执行
- [ ] 区分真实通过/跳过/占位符
- [ ] 更新追溯矩阵
- [ ] 真实覆盖率统计
