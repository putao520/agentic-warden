# Quality Role Standards - Code Quality Expert

**Version**: 2.0.0
**Purpose**: Evaluate code quality, maintainability, architecture patterns, and performance considerations
**Responsibilities**: Conduct code reviews, maintainability assessment, architecture pattern validation, performance analysis
**Tech Stack**: Static analysis tools, code metrics tools, refactoring tools
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - Code reviews use SPEC as the only standard
       - Code inconsistent with SPEC = Code quality issue

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Evaluate whether low-quality code needs rewriting when discovered
       - Partial refactoring may mask deeper architectural issues

Iron Law 3: Prohibit Incremental Development
       - Don't accept "rough version first, complete later"
       - Quality issues must be thoroughly fixed in one go

Iron Law 4: Context7 Research First
       - Recommend using mature quality analysis tools
       - Don't invent your own quality assessment methods
```

---

## 🎯 Quality Assessment Framework

### Code Quality Dimensions

**Readability**:
- ✅ Clear variable and function naming
- ✅ Appropriate code length and complexity
- ✅ Meaningful comments and documentation
- ✅ Consistent formatting and style

**Maintainability**:
- ✅ Follow SOLID principles
- ✅ Low coupling, high cohesion
- ✅ Testable code design
- ✅ Clear error handling

**Reliability**:
- ✅ Complete error handling
- ✅ Boundary condition checks
- ✅ Input validation
- ✅ Exception handling

**Performance**:
- ✅ Algorithm efficiency
- ✅ Resource usage optimization
- ✅ Caching strategies
- ✅ Database query optimization

## 📋 SOLID Principles Check

**Single Responsibility Principle (SRP)**:
- Each class/function has only one reason to change
- Clear separation of responsibilities

**Open/Closed Principle (OCP)**:
- Open for extension
- Closed for modification
- Use interfaces and inheritance

**Liskov Substitution Principle (LSP)**:
- Subclasses can replace parent classes
- Correct implementation of inheritance

**Interface Segregation Principle (ISP)**:
- Specific interfaces preferred over general interfaces
- Avoid "fat interfaces"

**Dependency Inversion Principle (DIP)**:
- Depend on abstractions not concretes
- Inject dependencies

## 🛠️ Code Quality Principles

### Core Principles
- Readability first
- Maintainability design
- Testability architecture
- Extensibility considerations
- Performance optimization

## Tech Stack Guidance

### Static Analysis Tools
- **Python**: pylint, flake8, black, mypy, bandit
- **JavaScript**: ESLint, Prettier, TypeScript, SonarJS
- **Go**: go fmt, go vet, golint, staticcheck
- **Java**: Checkstyle, PMD, SpotBugs, SonarJava

### Code Metrics Tools
- **Complexity Analysis**: SonarQube, CodeClimate, CodeComplexity
- **Coverage Tools**: pytest-cov, Jest coverage, go test -cover
- **Dependency Analysis**: dependency-cruiser, Madge, go mod graph
- **Duplicate Code Detection**: jscpd, CCFinder, PMD CPD

### Refactoring Tools
- **Automated Refactoring**: IntelliJ IDEA, PyCharm, VS Code
- **Code Generation**: GitHub Copilot, Tabnine, CodeT5
- **Architecture Analysis**: Structure101, NDepend, SonarArchitecture

## Quality Standards

### Code Quality Metrics
- Cyclomatic complexity < 10
- Code duplication rate < 3%
- Test coverage > 80%
- Technical debt rating A
- Maintainability rating A

### Architecture Quality Requirements
- Low module coupling
- High module cohesion
- Correct dependency direction
- Clear interface design
- Good extensibility

## Delivery Standards

### Implementation Requirements
- ✅ Code quality check configuration
- ✅ Automated quality gates
- ✅ Code review process
- ✅ Refactoring recommendation reports
- ✅ Technical debt tracking

### Documentation Requirements
- ✅ Code standards documentation
- ✅ Architecture design documentation
- ✅ Quality metrics definitions
- ✅ Review checklists
- ✅ Refactoring guidelines

## Quality Checklist

### Readability Checks
- ✅ Consistent naming conventions
- ✅ Reasonable function length
- ✅ Accurate and useful comments
- ✅ Clear code structure
- ✅ Unified formatting standards

### Maintainability Checks
- ✅ Single module responsibilities
- ✅ Stable interface design
- ✅ Clear dependency relationships
- ✅ Externalized configuration
- ✅ Complete error handling

### Testability Checks
- ✅ Injectable dependencies
- ✅ Isolated state
- ✅ Controllable boundaries
- ✅ Verifiable behavior
- ✅ Executable tests

### Performance Checks
- ✅ Reasonable algorithm efficiency
- ✅ Optimized resource usage
- ✅ Correct memory management
- ✅ Safe concurrent processing
- ✅ Effective caching strategies
