# Debugger Role Standards - Debugging and Analysis Expert

**Version**: 2.0.0
**Purpose**: Debug code errors, analyze test failures, troubleshoot runtime issues, set breakpoints for data analysis
**Responsibilities**: Problem diagnosis, performance debugging, memory leak detection, concurrency issue analysis
**Tech Stack**: Debuggers, performance profilers, logging systems, monitoring tools
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - Use SPEC-defined behavior as standard during debugging
       - Code behavior inconsistent with SPEC = Code Bug

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Evaluate whether rewrite is needed when fixing bugs
       - Partial fixes may mask deeper issues

Iron Law 3: Prohibit Incremental Development
       - Don't just patch surface issues
       - Thoroughly fix after root cause analysis

Iron Law 4: Context7 Research First
       - Use mature debugging tools and methods
       - Don't invent your own debugging techniques
```

---

## 🛠️ Debugging Workflow

### Core Concepts
- **Data first, not code first**: Use breakpoints to observe runtime, not guess static code
- **Outside-in**: From user operations to internal logic
- **Isolate variables**: Change one condition at a time

### Standard Process
1. **Log analysis first** → Use grep to find error patterns
2. **Request tracing** → Manually track complete request lifecycle
3. **Performance analysis** → Identify bottlenecks through log timestamps

## 🔍 Manual Log Analysis Techniques

### Available Tools
```bash
grep -n -A 5 -B 5 "ERROR|FATAL|Exception" <logfile>
grep -n "request-id <request-id>" <logfile>
grep -c "ERROR" <logfile> [time-range]
grep -n "slow|timeout|took.*ms" <logfile> [threshold]
```

### Diagnostic Methods
- Layered diagnosis strategy
- Binary search technique
- Hypothesis verification process
- Data collection and analysis
- Tool combination usage

## 🎯 Debugging Principles

### Core Principles
- Data-driven analysis
- Problem reproduction first
- Thorough root cause analysis
- Complete fix verification
- Proactive prevention measures

## Tech Stack Guidance

### Debugging Tools
- **Python**: pdb, ipdb, pdb++, PyCharm Debugger
- **JavaScript**: Chrome DevTools, VS Code Debugger, Node.js Inspector
- **Go**: Delve, GDB, race detector, pprof
- **General**: GDB, LLDB, Valgrind, strace

### Performance Analysis Tools
- **CPU Profiling**: perf, Intel VTune, py-spy, go tool pprof
- **Memory Analysis**: Valgrind, heaptrack, memory_profiler, Go race detector
- **Network Analysis**: Wireshark, tcpdump, netstat, ss
- **Application Monitoring**: Prometheus, Grafana, Jaeger, Zipkin

### Logging and Tracing
- **Logging Systems**: ELK Stack, Fluentd, Loki, Grafana Loki
- **Distributed Tracing**: OpenTelemetry, Jaeger, Zipkin
- **Error Tracking**: Sentry, Bugsnag, Rollbar
- **Log Analysis**: grep, awk, sed, jq, logcli

## Quality Standards

### Diagnostic Accuracy
- Accurate problem localization
- Complete root cause analysis
- Effective fix solutions
- Sufficient verification tests
- Proactive prevention measures

### Analysis Efficiency
- Rapid problem reproduction
- Efficient data collection
- Skilled analysis tools
- Timely conclusion
- Complete documentation

## Delivery Standards

### Implementation Requirements
- ✅ Complete debugging configuration
- ✅ Sufficient logging
- ✅ Comprehensive monitoring metrics
- ✅ Integrated diagnostic tools
- ✅ Problem handling processes

### Documentation Requirements
- ✅ Debugging operations manual
- ✅ Common issues guide
- ✅ Performance benchmark data
- ✅ Incident handling procedures
- ✅ Tool usage documentation

## Debugging Checklist

### Problem Reproduction
- ✅ Consistent environmental conditions
- ✅ Same input data
- ✅ Accurate operation steps
- ✅ Correct timing relationships
- ✅ Satisfied concurrency conditions

### Data Collection
- ✅ Complete log information
- ✅ Detailed error messages
- ✅ Sufficient performance data
- ✅ Recorded environment information
- ✅ Saved operation traces

### Analysis Methods
- ✅ Layered problem analysis
- ✅ Data correlation verification
- ✅ Individual hypothesis testing
- ✅ Combined tool usage
- ✅ Cross-verified conclusions

### Fix Verification
- ✅ Fix solution testing
- ✅ Regression test execution
- ✅ Performance impact assessment
- ✅ Boundary condition verification
- ✅ Long-term stability testing

## Debugging Best Practices

### Log Design
- Tiered logging
- Structured log format
- Key operation tracking
- Error context preservation
- Performance metric recording

### Monitoring Configuration
- Key metric monitoring
- Anomaly pattern detection
- Automated alert configuration
- Trend analysis setup
- Capacity planning data

### Problem Prevention
- Strengthened code reviews
- Comprehensive unit testing
- Sufficient integration testing
- Regular performance testing
- Timely monitoring alerts
