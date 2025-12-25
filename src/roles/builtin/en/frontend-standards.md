# Frontend Development Standards - CODING-STANDARDS-FRONTEND

**Version**: 2.0.0
**Scope**: Frontend development roles (Web/Mobile APP/Desktop applications, tech stack agnostic)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Laws (Inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - UI implementation must comply with SPEC definitions
       - Interactions, layouts, styles based on SPEC

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Existing component fully matches → Reuse directly
       - Partial match → Destroy and rebuild, no incremental modifications

Iron Law 3: Prohibit Incremental Development
       - Prohibit adding new features to old components
       - Prohibit retaining compatibility code

Iron Law 4: Context7 Research First
       - Use mature UI libraries and components
       - Prohibit self-implementing common UI components
```

---

## 🏗️ Component Design

### Component Responsibilities
- ✅ Single component file < 300 lines
- ✅ Component responsible for only one function or UI fragment
- ✅ Separate container components from presentational components
- ❌ Prohibit "do-it-all components" containing multiple unrelated functions

### Component Hierarchy
- ✅ Atomic components: buttons, inputs, icons (indivisible)
- ✅ Molecular components: search box = input + button
- ✅ Organism components: header = logo + navigation + search
- ✅ Nesting depth < 5 levels

### Props/Interface Design
- ✅ Single component Props < 10 items
- ✅ Required and optional parameters clearly marked
- ✅ Boolean values use is/has/should prefix
- ✅ Event callbacks use on prefix
- ✅ Use type definitions (TypeScript/Flow/PropTypes)
- ❌ Prohibit Props type of any

---

## 📊 State Management

### State Principles
- ✅ Each data has only one authoritative source (Single Source of Truth)
- ✅ Store only necessary state, don't store what can be calculated
- ✅ Lift shared state to common parent component
- ✅ Use immutable updates (don't directly modify state)
- ❌ Prohibit maintaining same data in multiple places

### Data Flow
- ✅ Data flows from parent to child components
- ✅ Events flow from child to parent components
- ✅ State changes trigger UI updates
- ❌ Avoid two-way binding complexity (unless framework mandatory)

---

## 🎨 HTML/CSS Standards

### HTML Semantics
- ✅ Use semantic tags (header, nav, main, article, footer)
- ✅ Form fields must have labels
- ✅ Images must have alt attributes
- ✅ Pass W3C validation
- ❌ Avoid overusing div and span

### CSS Naming
- ✅ Use consistent naming method (BEM, CSS Modules, CSS-in-JS)
- ✅ Style scope isolation, avoid global pollution
- ✅ Semantic class names, express purpose not style
- ❌ Prohibit inline styles (unless dynamically calculated)

### Responsive Design
- ✅ Mobile-first design
- ✅ Use relative units (rem, em, %, vh/vw)
- ✅ Use media queries to adapt to different screens
- ✅ Test common device sizes (phone, tablet, desktop)
- ✅ Touch target ≥ 44x44px

---

## ⚡ Performance Optimization

### Rendering Optimization
- ✅ Avoid unnecessary re-renders (use caching mechanisms)
- ✅ List rendering must have unique keys
- ✅ Long lists (>100 items) use virtualization
- ✅ Large datasets paginate loading
- ❌ Prohibit defining components in render functions

### Code Splitting
- ✅ Route-level code splitting
- ✅ Large component lazy loading
- ✅ Third-party libraries on-demand import
- ✅ Initial load size < 200KB (after gzip)

### Resource Optimization
- ✅ Image lazy loading
- ✅ Use modern image formats (WebP, AVIF)
- ✅ Responsive images (srcset)
- ✅ Compress and optimize resources
- ✅ Critical resource preloading (preload)

---

## ♿ Accessibility

### WCAG Compliance
- ✅ Keyboard accessible (Tab navigation)
- ✅ Screen reader friendly (ARIA labels)
- ✅ Color contrast ≥ 4.5:1 (normal text)
- ✅ Focus visible (focus state)
- ✅ Clear form error messages

### Common Requirements
- ✅ Interactive elements have focus state
- ✅ Buttons and links have clear text
- ✅ Dynamic content updates notify screen readers
- ❌ Prohibit distinguishing state only by color

---

## 🔒 Frontend Security

### XSS Protection
- ✅ Use framework's auto-escaping
- ❌ Prohibit using dangerous HTML injection APIs (like dangerouslySetInnerHTML)
- ✅ User input must be validated and sanitized
- ✅ Set CSP (Content Security Policy)

### CSRF Protection
- ✅ Use CSRF Token
- ✅ SameSite Cookie
- ✅ Verify request origin

### Sensitive Data
- ❌ Prohibit storing sensitive information on frontend (passwords, full ID cards)
- ✅ Store tokens in HttpOnly Cookie or secure storage
- ✅ HTTPS transmission
- ✅ Secondary confirmation for sensitive operations

---

## 🧪 Frontend Testing

### Test Coverage
- ✅ Component rendering tests
- ✅ User interaction tests
- ✅ State change tests
- ✅ Edge cases and error handling
- ❌ Avoid testing implementation details

### Test Scope
- ✅ Key business components must have tests
- ✅ Utility functions must have unit tests
- ✅ Cross-browser testing
- ✅ Mobile testing

---

## 📋 Frontend Development Checklist

- [ ] Single component responsibility (< 300 lines)
- [ ] Props type definitions complete
- [ ] Clear state management (single data source)
- [ ] Semantic HTML tags
- [ ] CSS style isolation
- [ ] Responsive design
- [ ] Performance optimization (lazy loading, virtualization)
- [ ] Accessibility (keyboard, ARIA, contrast)
- [ ] XSS/CSRF protection
- [ ] Component and utility function tests

---

---

## 🏛️ Advanced Architecture Patterns (20+ years experience)

### Micro-Frontend Architecture
```
✅ Applicable scenarios:
- Large applications with multi-team collaboration
- Modules requiring independent deployment
- Heterogeneous tech stacks (React/Vue/Angular coexistence)

Architecture patterns:
- Module Federation (Webpack 5)
- Single-SPA orchestration
- qiankun sandbox isolation
- Web Components boundaries

Communication mechanisms:
- CustomEvent cross-app communication
- Shared state management (Redux/Zustand Store Slice)
- PostMessage secure channels
```

### Advanced State Management Patterns
```
Atomic State (Jotai/Recoil):
- Bottom-up state atoms
- Derived state auto-calculation
- Precise subscription, minimal re-render

Server State (TanStack Query/SWR):
- Request caching and deduplication
- Optimistic updates
- Background refresh
- Offline support

State Machines (XState):
- Complex business flow modeling
- Explicit state transitions
- Visual debugging
```

### Rendering Architecture Choices
```
CSR (Client-Side Rendering):
- Applicable: Interactive-intensive applications (backend management)
- Drawbacks: Slow first screen, poor SEO

SSR (Server-Side Rendering):
- Applicable: Content websites, SEO requirements
- Technologies: Next.js/Nuxt.js
- Note: Hydration cost

SSG (Static Site Generation):
- Applicable: Blogs, documentation sites
- Advantages: Best performance

ISR (Incremental Static Regeneration):
- Applicable: E-commerce product pages
- Combines SSG and SSR advantages

Streaming SSR:
- React 18 Suspense
- Progressive rendering
```

---

## 🔧 Essential Skills for Senior Developers

### Build Optimization Deep Techniques
```
Bundle Analysis:
- webpack-bundle-analyzer
- source-map-explorer
- Dependency size visualization

Tree Shaking Optimization:
- Ensure sideEffects: false
- Avoid re-export
- Use ESM format libraries

Code Splitting Strategy:
- Route-level splitting (basic)
- Component-level splitting (advanced)
- Data prefetch splitting (expert)

Long-term Caching:
- contenthash filenames
- Extract stable dependencies (vendor chunk)
- Runtime separation (runtime chunk)
```

### Runtime Performance Deep Optimization
```
React Optimization:
- React.memo + useMemo + useCallback trio
- State sinking, avoid lifting
- Context splitting, avoid overall re-render
- Use useTransition to delay non-urgent updates

Vue Optimization:
- v-once static content
- v-memo conditional caching
- Functional components
- KeepAlive component caching

General Optimization:
- requestIdleCallback idle scheduling
- IntersectionObserver lazy loading
- ResizeObserver layout monitoring
- Virtual scrolling (react-window/vue-virtual-scroller)
```

### Debugging and Performance Analysis
```
DevTools Advanced Usage:
- Performance Tab flame graph analysis
- Memory Tab memory leak detection
- Coverage Tab code coverage
- Layers Tab composite layer analysis

React DevTools:
- Profiler component render analysis
- Highlight Updates re-render visualization
- Components tree state inspection

Performance Metrics Monitoring:
- Core Web Vitals (LCP/FID/CLS)
- TTFB/FCP/TTI
- Lighthouse CI integration
```

### Complex Form Handling
```
Form Library Selection:
- React Hook Form (performance priority)
- Formik (comprehensive features)
- VeeValidate (Vue ecosystem)

Advanced Patterns:
- Dynamic forms (JSON Schema driven)
- Form wizards (multi-step)
- Form linkage (conditional fields)
- Async validation (debounce)

Performance Points:
- Uncontrolled components (reduce re-render)
- Field-level validation (local update)
- Form state isolation
```

---

## 🚨 Common Pitfalls for Senior Developers

### Architecture Traps
```
❌ Over-abstraction:
- Create overly general components for "reuse"
- More config items than code
- Correct: Start specific, then abstract, Rule of Three

❌ State globalization:
- Put all state in global Store
- Causes severe component coupling
- Correct: State proximity principle, local over global

❌ Micro-frontend abuse:
- Force micro-frontends on small projects
- Increase complexity without actual benefits
- Correct: Evaluate team size and project complexity
```

### Performance Traps
```
❌ useMemo/useCallback abuse:
- Add caching everywhere
- Actually increases memory overhead
- Correct: Profile before optimizing, don't optimize blindly

❌ Excessive component splitting:
- One component per DOM element
- Props drilling hell
- Correct: Reasonable granularity, components with clear responsibilities

❌ Unlimited image loading:
- No concurrent request limit
- Network congestion
- Correct: Request queue, priority scheduling
```

### Testing Traps
```
❌ Testing implementation details:
- Check component internal state
- Check private method calls
- Correct: Test behavior and output

❌ Snapshot testing abuse:
- Snapshot complex components
- Update snapshots every time
- Correct: Snapshots only for simple static components

❌ Insufficient E2E coverage:
- Only unit tests
- No integration scenario coverage
- Correct: Pyramid strategy, critical path E2E
```

---

## 📊 Performance Monitoring Metrics

| Metric | Target | Alert Threshold | Measurement Method |
|--------|--------|-----------------|-------------------|
| LCP | < 2.5s | > 4s | Lighthouse/RUM |
| FID | < 100ms | > 300ms | Lighthouse/RUM |
| CLS | < 0.1 | > 0.25 | Lighthouse/RUM |
| TTI | < 3.8s | > 7.3s | Lighthouse |
| FCP | < 1.8s | > 3s | Lighthouse |
| Bundle Size (gzip) | < 200KB | > 500KB | Bundle Analyzer |
| First Screen Render | < 1.5s | > 3s | Performance API |
| Memory Usage | < 100MB | > 300MB | Memory Tab |
| Component Re-render | < 3/interaction | > 10 | React Profiler |

---

## 📋 Frontend Development Checklist (Complete Version)

### Basic Checks
- [ ] Single component responsibility (< 300 lines)
- [ ] Props type definitions complete
- [ ] Clear state management (single data source)
- [ ] Semantic HTML tags
- [ ] CSS style isolation
- [ ] Responsive design

### Performance Checks
- [ ] Core Web Vitals met
- [ ] Bundle Size < 200KB (gzip)
- [ ] Route-level code splitting
- [ ] Image lazy loading and modern formats
- [ ] Long list virtualization
- [ ] No memory leaks

### Security Checks
- [ ] XSS/CSRF protection
- [ ] Sensitive data not stored on frontend
- [ ] CSP policy configured
- [ ] HTTPS enforced

### Testing Checks
- [ ] Component unit tests
- [ ] Integration test coverage
- [ ] E2E critical path tests
- [ ] Cross-browser testing

---

**Frontend Development Principles Summary**:
Component-based, Single Responsibility, State Minimization, Semantic HTML, Style Isolation, Responsive Design, Performance First, Accessibility, Security Protection, Test Coverage
