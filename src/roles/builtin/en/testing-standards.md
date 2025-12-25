# Testing Development Standards - TESTING-STANDARDS

**Version**: 2.0.0
**Scope**: E2E testing, integration testing, system-level testing
**Last Updated**: 2025-12-25

---

## 🚨 Seven Iron Laws (Failure if Violated)

### Iron Law 1: CI-Only (Zero Local Testing)

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ❌ Forbidden: npm test, pytest, go test and other local commands        │
│  ❌ Forbidden: Directly executing any tests on host machine              │
│  ❌ Forbidden: localhost:xxxx access to test services                    │
│  ✅ Must: All tests execute inside docker-compose.ci.yml containers      │
│  ✅ Must: Trigger container tests via AI Dev-Loop or just test-e2e      │
└─────────────────────────────────────────────────────────────────────────┘
```

### Iron Law 2: Container Full Isolation (Zero Port Exposure)

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ❌ Forbidden: ports: "8080:8080" expose to host                        │
│  ❌ Forbidden: Test code using localhost:xxxx                            │
│  ❌ Forbidden: Direct host access to container services                  │
│  ✅ Must: Use docker-compose internal DNS (http://service:port)        │
│  ✅ Must: Test container and services under test in same docker network │
│  ✅ Must: Only use expose for intra-container ports                     │
└─────────────────────────────────────────────────────────────────────────┘
```

### Iron Law 3: Three-Layer Completeness (Must Fully Cover)

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Layer 1: Service-Level E2E                                             │
│  ├─ Independent API/UI tests for each subproject                        │
│  ├─ Cover all REQ-XXX for that service                                 │
│  └─ Independently verified in docker-compose.ci.yml                     │
│                                                                         │
│  Layer 2: Plane-Level E2E (Cross-Service)                              │
│  ├─ Data plane: request→process→response complete chain                │
│  ├─ Control plane: config change→service reload→effect verification    │
│  └─ End-to-end flows for other business planes                          │
│                                                                         │
│  Layer 3: Product-Level E2E                                             │
│  ├─ Complete user journey (register→config→use→result)                 │
│  └─ Full chain verification simulating production environment          │
│                                                                         │
│  ❌ Forbidden: Only doing service-level tests and claiming "E2E done"  │
│  ✅ Must: All three layers have test cases and all pass                 │
└─────────────────────────────────────────────────────────────────────────┘
```

### Iron Law 4: Volume Mapping Priority (Prohibit Image Rebuild Deployment)

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Code Deployment Method                                                  │
│  ─────────────────────────────────────────────────────────────────────  │
│  ✅ Must use volume mapping: Map local source code and dependencies     │
│      to container via -v parameter                                     │
│  ✅ Container contains only runtime environment: Image only installs    │
│      basic environments like Node.js, Python                           │
│  ✅ Real-time updates: Code changes take effect in container           │
│      immediately without rebuild                                        │
│                                                                         │
│  ❌ Prohibit image rebuild deployment: Strictly forbid updating test   │
│      code via Docker build                                              │
│  ❌ Prohibit COPY source code: Dockerfile must not contain COPY src     │
│      and similar instructions                                           │
│  ❌ Prohibit precompiled deployment: Do not compile source code at      │
│      build time                                                         │
│  ❌ Prohibit container npm install/pip install: Dependencies pre-       │
│      installed or mapped                                                │
│                                                                         │
│  Correct configuration example:                                         │
│  volumes:                                                               │
│    - ./tests:/app/tests              # Test code mapping               │
│    - ./node_modules:/app/node_modules  # Dependency mapping            │
└─────────────────────────────────────────────────────────────────────────┘
```

### Iron Law 5: Prohibit False Test Passes

```
┌─────────────────────────────────────────────────────────────────────────┐
│  False Pass = Test reports "success" but doesn't verify expected        │
│                behavior                                                 │
│                                                                         │
│  ❌ Forbidden: Conditional skipping                                     │
│     if !serviceHealthy() { t.Skip("service not ready") }               │
│                                                                         │
│  ❌ Forbidden: Fault-tolerant returns                                  │
│     if status != 200 { return }  // Silent skip                         │
│                                                                         │
│  ❌ Forbidden: Empty assertions                                        │
│     func TestEmpty(t *testing.T) { /* no assertions */ }               │
│                                                                         │
│  ❌ Forbidden: Placeholder tests                                       │
│     func TestFeature(t *testing.T) { // TODO: implement }              │
│                                                                         │
│  ❌ Forbidden: False coverage reporting                                │
│     Count TEST-ID quantity not actual verification                     │
│     Claim "100% pass" when actually have skips                         │
│                                                                         │
│  ✅ Correct: Test failure = expose problem = fix problem               │
│  ✅ Correct: Environment issue = test failure = fix environment        │
│  ✅ Correct: Feature unimplemented = test failure = implement feature  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Iron Law 6: Four Test Categories Coverage

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Each feature/API must cover four test categories                       │
│                                                                         │
│  1. Positive Tests (Happy Path)                                        │
│     ├─ Each feature has at least 2 positive tests                      │
│     ├─ Verify normal input produces correct output                     │
│     └─ Verify complete response structure (not just status code)       │
│                                                                         │
│  2. Negative Tests (Error Cases)                                       │
│     ├─ Each input parameter has at least 1 negative test               │
│     ├─ Verify error input produces correct error response              │
│     └─ Verify error codes and error messages                           │
│                                                                         │
│  3. Boundary Tests (Boundary Cases)                                    │
│     ├─ Test zero, empty, null, undefined values                        │
│     ├─ Test min, max, near-boundary values                             │
│     └─ Test extreme cases (oversized strings, huge numbers, etc.)      │
│                                                                         │
│  4. Security Tests (Security Cases)                                    │
│     ├─ SQL injection attack tests                                      │
│     ├─ XSS attack tests                                                │
│     ├─ Privilege escalation tests                                      │
│     └─ Authentication/authorization bypass tests                       │
│                                                                         │
│  ❌ Forbidden: Only writing positive tests                             │
│  ❌ Forbidden: Only checking status code 200                           │
└─────────────────────────────────────────────────────────────────────────┘
```

### Iron Law 7: ≥3 Valid Assertions Per Test

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Assertion Quality Requirements                                         │
│                                                                         │
│  ✅ Each test function must have ≥3 valid assertions                   │
│  ✅ Must verify complete response structure (not just status code)      │
│  ✅ Assertions must verify specific values, not existence checks only   │
│                                                                         │
│  ❌ Invalid assertion examples:                                        │
│     assert.True(true)                                                   │
│     assert.NotNil(response)  // Only checks non-empty                   │
│     assert.Equal(200, status)  // Only checks status code               │
│                                                                         │
│  ✅ Valid assertion examples:                                          │
│     assert.Equal(200, resp.StatusCode)                                 │
│     assert.Equal("admin", resp.Body.User.Role)                         │
│     assert.Equal(10, len(resp.Body.Items))                             │
│     assert.Contains(resp.Body.Message, "success")                      │
│     assert.True(resp.Body.CreatedAt.Before(time.Now()))                │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 🧪 Test Data Standards

### Must Use Faker to Generate Test Data

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ❌ Prohibited simple test data:                                        │
│     username: "test"                                                    │
│     email: "test@test.com"                                              │
│     password: "123456"                                                  │
│     phone: "12345678901"                                                │
│                                                                         │
│  ✅ Must use Faker library generation:                                  │
│     Go:      gofakeit.Username()                                        │
│     Python:  faker.email()                                              │
│     JS/TS:   faker.internet.email()                                     │
│                                                                         │
│  ✅ Each test uses unique data:                                         │
│     username: fmt.Sprintf("user_%s", gofakeit.UUID())                   │
│     email: fmt.Sprintf("%s@test.local", gofakeit.UUID())                │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 🚨 Test Failure Handling Strategy

### Prohibited Avoidance Behaviors

```
❌ Skip tests
   - Prohibit t.Skip("environment issues")
   - Prohibit pytest.skip("feature not implemented")
   - Prohibit test.skip("pending fix")

❌ Lower validation standards
   - Prohibit changing assert status == 200 to assert status in [200, 500]
   - Prohibit deleting assertions to make test "pass"

❌ Fault-tolerant bypass
   - Prohibit if status != 200: return  # Silent skip
   - Prohibit try/except swallowing assertion failures

❌ False fixes
   - Prohibit hardcoding expected values to match current error output
   - Prohibit modifying test to adapt to incorrect implementation
```

### Correct Failure Handling (6 Category Classification)

```
A Class (CODE_BUG): Code logic error
   → Fix code implementation

B Class (MISSING_FEATURE): Feature not implemented
   → Implement missing feature

C Class (TEST_ISSUE): Test design issue
   → Fix test code (not lower standards)

D Class (ENV_ISSUE): Environment issue
   → Fix environment configuration

E Class (SPEC_ISSUE): SPEC issue
   → Pause, report to user/architect

F Class (TEST_QUALITY_ISSUE): Test case quality issue
   → First enhance test case, then fix code
   → Check: Are assertions strict enough? Is test coverage complete?
           Is test data realistic?
```

---

## 🔧 URL Pattern Standards

### Correct Pattern (Container Internal DNS)

```go
// ✅ Correct
baseURL := "http://backend:8080"
baseURL := "http://api-service:8080"
baseURL := "http://frontend:80"
ws://websocket-service:8080
```

### Wrong Pattern (Prohibited)

```go
// ❌ Prohibited
http://localhost:8080
http://127.0.0.1:3000
http://0.0.0.0:8080
```

---

## 📋 Test Code Requirements

### Must Include TEST-ID and REQ References

```go
// TEST-E2E-SVC-AUTH-001
// Covers requirements: REQ-AUTH-001, REQ-AUTH-002
// Test category: Positive test
func TestUserAuthentication(t *testing.T) {
    // Arrange
    user := generateFakeUser()

    // Act
    resp := client.Login(user)

    // Assert (≥3 valid assertions)
    require.Equal(t, 200, resp.StatusCode)
    require.NotEmpty(t, resp.Body.Token)
    require.Equal(t, user.Email, resp.Body.User.Email)
    require.True(t, resp.Body.ExpiresAt.After(time.Now()))
}
```

### Prohibited Test Code

```go
// ❌ Prohibited: Conditional skip
func TestFeature(t *testing.T) {
    if !serviceHealthy() {
        t.Skip("service not ready")  // Prohibited!
    }
}

// ❌ Prohibited: Empty assertions/less than 3 assertions
func TestEmpty(t *testing.T) {
    resp := callAPI()
    assert.Equal(t, 200, resp.StatusCode)  // Only 1 assertion, insufficient!
}

// ❌ Prohibited: Fault-tolerant return
func TestWithFallback(t *testing.T) {
    status := callAPI()
    if status != 200 {
        return  // Prohibited! Should fail
    }
}

// ❌ Prohibited: Simple test data
func TestUser(t *testing.T) {
    user := User{
        Name: "test",      // Prohibited! Use faker
        Email: "a@b.com",  // Prohibited! Use faker
    }
}
```

---

## 📊 Coverage Reporting Standards

### Must Distinguish Real Status

```markdown
| Status | Count | Description |
|--------|-------|-------------|
| ✅ Complete Implementation | 12 | Has ≥3 assertions, no conditional skips, 4-category coverage |
| ⚠️ Conditional Skip | 2 | Contains t.Skip (needs review) |
| ⚠️ Insufficient Assertions | 3 | Assertions <3 (needs enhancement) |
| ❌ Placeholder | 1 | Empty implementation or TODO |

Real Coverage = Complete Implementation / Total = 12/18 = 67%
```

### Prohibited False Reporting

```
❌ Prohibited: Count TEST-ID quantity not actual verification
❌ Prohibited: Claim "100% pass" when actually have skips
❌ Prohibited: Count skipped tests as "passing"
❌ Prohibited: Don't count tests with insufficient assertions
```

---

## 🐳 docker-compose Configuration Standards

### Correct Configuration

```yaml
services:
  backend:
    expose:
      - "8080"  # Only visible within docker network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 10s
      timeout: 5s
      retries: 3
    # No ports mapping

  e2e-runner:
    volumes:
      - ./tests:/app/tests              # Test code mapping
      - ./node_modules:/app/node_modules  # Dependency mapping
    depends_on:
      backend:
        condition: service_healthy
```

### Wrong Configuration (Prohibited)

```yaml
services:
  backend:
    ports:
      - "8080:8080"  # ❌ Prohibit expose to host

  e2e-runner:
    build:
      dockerfile: Dockerfile
      # ❌ Prohibit COPY test code in Dockerfile
```

---

## 🔄 AI Dev-Loop Standards

### Only Test Execution Engine

```bash
# Correct: Use system-level ai-dev-loop
~/.claude/skills/testing/ai-dev-loop/dist/index.js start --project-root /path/to/project

# Exit code meanings
0  = All tests pass
1  = Unfixed BUG exists
2  = Configuration error
3  = Environment startup failure
10 = Maximum iterations reached
20 = SPEC issue (requires manual intervention)
```

### Prohibited Execution Methods

```bash
# ❌ Prohibited: Direct host execution
npm test
pytest
go test ./...

# ❌ Prohibited: Project self-built dev-loop
./scripts/dev-loop.sh
```

---

## 🎯 Core Principles

```
1. Test failure = Problem discovered = Good thing
   Not to hide problems, but to fix problems

2. Code issue → Fix code
   Not modify test to adapt to incorrect code

3. SPEC issue → Pause and ask user
   Not decide to bypass SPEC yourself

4. Environment issue → Fix environment
   Not skip test or lower standards

5. Real coverage > False pass
   Better report 50% real coverage than claim 100% false pass

6. Quality first > Quantity
   3 high-quality tests > 10 low-quality tests

7. Four-category complete > Single-category multiple
   Positive+Negative+Boundary+Security > 10 positive tests
```

---

## ✅ Test Development Checklist

### Before Test Design
- [ ] Read SPEC, identify all REQ-XXX
- [ ] Plan three-layer coverage (service/plane/product)
- [ ] Confirm docker-compose.ci.yml configuration correct

### During Test Writing
- [ ] Each test has TEST-ID and REQ reference
- [ ] Each test has ≥3 valid assertions
- [ ] Cover four categories (positive/negative/boundary/security)
- [ ] Use faker to generate test data
- [ ] Use container internal DNS, no localhost

### After Test Completion
- [ ] Execute inside docker-compose.ci.yml container
- [ ] Distinguish real pass/skip/placeholder
- [ ] Update traceability matrix
- [ ] Real coverage statistics
