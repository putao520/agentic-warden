# Common Programming Standards - CODING-STANDARDS-COMMON

**Version**: 2.0.0
**Scope**: All programming tasks (backend, frontend, systems, databases, etc.)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (Failure if Violated)

### Iron Law 1: SPEC is the Single Source of Truth (SSOT)

```
┌─────────────────────────────────────────────────────────────────────────┐
│  SPEC Authority Hierarchy (Absolutely Unbreakable)                      │
│                                                                         │
│  SPEC > Task Description > AI Understanding > User Verbal Requests      │
│                                                                         │
│  ❌ Forbidden: Start coding without reading SPEC                        │
│  ❌ Forbidden: Consider task description more accurate than SPEC       │
│  ❌ Forbidden: "I think X is better than Y" and deviate from SPEC      │
│  ❌ Forbidden: "SPEC is too complex, I'll simplify it"                  │
│  ❌ Forbidden: "SPEC doesn't say, but I think we should add"           │
│  ❌ Forbidden: Only implement part of SPEC requirements                │
│  ❌ Forbidden: Use tech stack not specified in SPEC                    │
│                                                                         │
│  ✅ Must: Read complete relevant SPEC documents before coding          │
│  ✅ Must: Understand specific requirements for each SPEC ID            │
│  ✅ Must: Code implementation 100% consistent with SPEC                │
│  ✅ Must: Report SPEC issues promptly rather than deciding yourself    │
│  ✅ Must: When code conflicts with SPEC, change code not SPEC          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 1: SPEC-Guided Deep Analysis (Reuse Decision)                   │
│  ─────────────────────────────────────────────────────────────────────  │
│  1. Comprehensive scan of existing modules:                            │
│     - Common modules: utilities, algorithms, data structures          │
│     - Infrastructure: config, logging, error handling, protocols       │
│     - Domain modules: business logic, data processing, compute        │
│                                                                         │
│  2. SPEC-based precise match evaluation:                               │
│     - Complete match: existing module fully meets SPEC requirements    │
│     - Partial match: existing module partially meets, needs changes    │
│     - No match: existing module cannot meet SPEC requirements          │
│                                                                         │
│  3. Reuse decision:                                                     │
│     ✅ Complete match → Reuse directly, no redevelopment needed        │
│     ❌ Partial/No match → Execute destroy-and-rebuild                  │
│                                                                         │
│  ⚠️ Key: Reuse based on SPEC functional completeness, not similarity   │
│  ⚠️ Key: Partial match equals no match, must destroy-and-rebuild       │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 2: SPEC-Driven Thorough Rewrite (Destroy-and-Rebuild)           │
│  ─────────────────────────────────────────────────────────────────────  │
│  Definition:                                                            │
│  - Not modification: not changing or extending existing code           │
│  - Not incremental: not gradually adding features or fixes            │
│  - Not refactoring: not adjusting existing code structure             │
│  - But thorough rewrite: delete all related code, redesign and reimplement│
│                                                                         │
│  Execution:                                                             │
│  1. Delete all old code that violates SPEC                             │
│  2. Design from scratch new implementation fully compliant with SPEC   │
│  3. Each SPEC ID must have clear, fully SPEC-compliant implementation  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Iron Law 3: Prohibit Incremental Development

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Absolutely Forbidden Incremental Development Behaviors                │
│                                                                         │
│  ❌ "Keep old implementation, add new features"                        │
│  ❌ "Compatibility code, support old interfaces"                       │
│  ❌ "Migration code, gradual transition"                               │
│  ❌ "Extend existing class, add new methods"                           │
│  ❌ "Modify existing function, add parameters"                         │
│  ❌ "For compatibility, keep old logic"                                │
│  ❌ "Do rough version first, complete later"                           │
│  ❌ "Supplement in future iterations"                                  │
│                                                                         │
│  Why must destroy-and-rebuild:                                         │
│  1. Avoid technical debt: incremental changes accumulate baggage      │
│  2. Ensure code quality: rewrite ensures compliance with latest standards│
│  3. Simplify thinking: no need to consider compatibility, focus on goal│
│  4. Improve development efficiency: faster and more reliable than complex│
│     incremental modifications                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

### Iron Law 4: Context7 Research First

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Must research mature libraries before new feature development          │
│                                                                         │
│  ✅ Must use:                                                          │
│     - Tech stack selection before new feature development              │
│     - Introducing new libraries or using library APIs                  │
│     - Reviewing best practices before code generation                 │
│     - Comparing multiple library choices                               │
│                                                                         │
│  ❌ Forbidden:                                                          │
│     - Implementing common features without research                   │
│     - Using outdated library versions or APIs                          │
│     - Writing library usage code from memory                           │
│     - Reinventing the wheel                                            │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 🎯 Core Design Principles

### SOLID Principles

**Single Responsibility Principle (SRP)**:
- ✅ One module/class/function负责一个功能
- ✅ One reason to change
- ❌ Prohibit "God classes" or "God functions"

**Open/Closed Principle (OCP)**:
- ✅ Open for extension, closed for modification
- ✅ Use interfaces, abstract classes, polymorphism for extension

**Liskov Substitution Principle (LSP)**:
- ✅ Subclasses can replace parent classes
- ✅ Subclasses don't change behavioral contracts of parents

**Interface Segregation Principle (ISP)**:
- ✅ Use multiple specialized interfaces rather than single general interface
- ❌ Avoid "fat interfaces"

**Dependency Inversion Principle (DIP)**:
- ✅ Depend on abstractions not concrete implementations
- ✅ Use Dependency Injection

### Other Core Principles

**DRY (Don't Repeat Yourself)**:
- ✅ Extract reusable code to functions/classes/modules
- ❌ Prohibit copy-paste code

**KISS (Keep It Simple, Stupid)**:
- ✅ Prefer simple, straightforward solutions
- ✅ Avoid over-engineering and unnecessary complexity

---

## 📝 Naming Conventions

### Variables and Functions
- **camelCase**: `userName`, `getUserById()`, `isValid`
- Use meaningful names (avoid `a`, `tmp`, `data`)
- Boolean values use `is`/`has`/`should` prefixes

### Classes and Components
- **PascalCase**: `UserService`, `DatabaseConnection`

### Constants
- **UPPER_SNAKE_CASE**: `MAX_RETRY_COUNT`, `API_BASE_URL`

### File Names
- **kebab-case**: `user-service.ts`, `database-config.js`

---

## 🏗️ Code Structure Standards

| Metric | Limit | Handling |
|-------|-------|----------|
| File size | ≤300 lines | Split into multiple modules |
| Function size | ≤50 lines | Split into multiple small functions |
| Nesting depth | ≤3 levels | Early return/extract functions |
| Cyclomatic complexity | ≤10 | Strategy pattern/lookup table |
| Parameter count | ≤5 parameters | Use object parameters |

---

## 🔒 Code Quality Requirements (Zero Tolerance)

### ❌ Strictly Forbidden

**Placeholders and incomplete code**:
- ❌ `TODO` / `FIXME` comments
- ❌ `stub` functions or empty implementations
- ❌ Commented-out code
- ❌ `console.log` debug statements (production code)

**Incomplete implementations**:
- ❌ Code lacking error handling
- ❌ Public interfaces lacking input validation
- ❌ Unreleased resources

### ✅ Mandatory Requirements

**Error handling**:
- ✅ All operations that can fail must have error handling
- ✅ Error messages clear and actionable
- ✅ Log errors (including context information)

**Input validation**:
- ✅ Validate all external input
- ✅ Type checking and boundary checking
- ✅ Reject invalid input with clear errors

**Resource management**:
- ✅ Close database connections, file handles, network connections promptly
- ✅ Use RAII, defer, with/using for automatic resource management

**Type safety**:
- ✅ Avoid `any` or unsafe type conversions
- ✅ Use generics for better type safety

---

## 🛡️ Security Requirements

### Input Validation
- ✅ Whitelist validation preferred over blacklist
- ✅ Length, format, type checking

### SQL Injection Protection
- ✅ Use parameterized queries or ORM
- ❌ Prohibit string concatenation for SQL

### XSS Protection
- ✅ Output encoding (HTML, JavaScript, URL)
- ✅ Set CSP (Content Security Policy)

### Authentication and Authorization
- ✅ Check permissions before executing operations
- ✅ Principle of least privilege

### Sensitive Data
- ✅ Encrypt passwords, keys, tokens for storage
- ❌ Don't log sensitive information

---

## ⚡ Performance Requirements

### Algorithm Complexity
- ✅ Avoid O(n²) and higher complexity (on large datasets)
- ✅ Use caching to reduce redundant computation

### Database Optimization
- ✅ Use indexes to accelerate queries
- ✅ Avoid N+1 query problems
- ✅ Paginate large dataset queries

### Asynchronous and Concurrent
- ✅ Use async for I/O operations
- ✅ Avoid blocking main thread
- ✅ Mind concurrent safety

---

## 🧪 Testing Requirements

### Unit Tests
- ✅ Test single functions/methods
- ✅ Fast execution (< 100ms)
- ✅ Independence (no external resource dependencies)
- ✅ Cover normal and exceptional paths

### Testing Principles
- ✅ AAA pattern (Arrange-Act-Assert)
- ✅ One test verifies one behavior
- ✅ Clear test names

### Boundary Testing
- ✅ Min value, max value
- ✅ Empty values, null, undefined
- ✅ Invalid input

---

## 🔍 Code Review Requirements

### Review Checklist

**SPEC consistency**:
- [ ] Code implementation 100% consistent with SPEC
- [ ] Each SPEC ID has corresponding implementation
- [ ] No unauthorized additions beyond SPEC

**Quality checks**:
- [ ] No TODO/FIXME/stub
- [ ] Complete error handling
- [ ] Complete input validation
- [ ] Resources properly released

**Architecture checks**:
- [ ] Follow SOLID principles
- [ ] No duplicate code
- [ ] Clear module boundaries

---

## ✅ Development Checklist

### Before Development
- [ ] Read complete relevant SPEC documents
- [ ] Confirm specific requirements for each SPEC ID
- [ ] Scan existing code, evaluate reuse possibilities
- [ ] Context7 research technical approach

### During Development
- [ ] Follow naming conventions
- [ ] Keep code simple (KISS)
- [ ] Avoid duplicate code (DRY)
- [ ] Implement all SPEC requirements (complete in one go)
- [ ] Complete error handling
- [ ] Input validation and security checks

### After Development
- [ ] Verify SPEC implementation completeness item by item
- [ ] Write unit tests
- [ ] Code review
- [ ] No TODO/FIXME/placeholders

---

**Core Philosophy**:
- SPEC is the single source of truth, code must 100% comply with SPEC
- Partial match equals no match, must destroy-and-rebuild
- Prohibit any form of incremental development
- Quality over speed, correctness over speed
