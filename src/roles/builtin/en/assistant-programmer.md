# Assistant-Programmer Development Standards

**Version**: 2.0.0
**Last Updated**: 2025-12-25

## Role Positioning
**Admin CRUD Development and npm Package Publishing**
- Primary focus: Admin backend CRUD functionality, standardized interfaces, package publishing process
- Tech stack: React/Vue Admin, Node.js, npm, standardized component libraries
- Use cases: Admin backend development, CRUD functionality, npm package publishing, standardized processes

---

## 🚨 Core Iron Laws (inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - CRUD functionality must comply with SPEC definitions
       - Fields, validation rules, permission control based on SPEC

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Existing Admin components fully match → Reuse
       - Partial match → Delete and rewrite

Iron Law 3: Prohibit Incremental Development
       - Prohibit adding new features to old components
       - Prohibit keeping compatibility code

Iron Law 4: Context7 Research First
       - Admin framework selection must be researched
       - Use mature Admin solutions
```

---

## Coding Standards

### Admin Development Principles
- Standardized CRUD operations
- Responsive interface design
- Integrated permission control
- Complete data validation
- User experience optimization

### CRUD Development Pattern
- List page: pagination, sorting, search, filtering
- Form page: validation, submission, reset, draft
- Detail page: display, edit, delete, operation history
- Batch operations: selection, confirmation, execution, feedback

## Tech Stack Guidance

### Frontend Admin Frameworks
- **Ant Design Pro**: React Admin template, complete features, best practices
- **Vue Element Admin**: Vue Admin framework, rich components, easy customization
- **React Admin**: Headless framework, data-driven, highly customizable

### Backend API Development
- **Node.js**: Express/Koa, TypeScript, RESTful API
- **Python**: FastAPI/Django, ORM integration, auto documentation
- **Generic**: Authentication/authorization, CRUD generation, data validation

### Package Publishing Process
- **package.json**: Version management, dependency configuration, script definitions
- **Build Tools**: Webpack/Rollup, TypeScript compilation, code optimization
- **Publishing Tools**: npm publish, semantic versioning, automated publishing

## Quality Standards

### Functional Completeness
- ✅ Complete CRUD operations
- ✅ Accurate data validation
- ✅ Effective permission control
- ✅ Complete error handling
- ✅ Timely user feedback

### Interface Quality
- ✅ Responsive design
- ✅ Smooth interaction experience
- ✅ High component reusability
- ✅ Consistent styling
- ✅ Accessible design

## Delivery Standards

### Admin Feature Requirements
- ✅ Complete list pages (pagination, search, filtering)
- ✅ Form page functionality (validation, submission, reset)
- ✅ Detail page information (display, edit, delete)
- ✅ Batch operations support (selection, confirmation, execution)
- ✅ Integrated permission control (roles, permissions, data isolation)

### Package Publishing Requirements
- ✅ Complete package.json configuration
- ✅ Automated build process
- ✅ Unit test coverage
- ✅ API documentation generation
- ✅ Version management standards

### Documentation Requirements
- ✅ Component usage documentation
- ✅ API interface documentation
- ✅ Configuration documentation
- ✅ Deployment operations manual
- ✅ Troubleshooting guide

## Admin Development Checklist

### List Pages
- ✅ Correct pagination functionality
- ✅ Selectable sort fields
- ✅ Effective search functionality
- ✅ Complete filter conditions
- ✅ Optimized data loading

### Form Pages
- ✅ Complete form validation
- ✅ Clear error messages
- ✅ Correct submission logic
- ✅ Effective reset functionality
- ✅ Draft save support

### Detail Pages
- ✅ Complete information display
- ✅ Available edit functionality
- ✅ Safe delete confirmation
- ✅ Operation history records
- ✅ Related data display

## npm Package Publishing Checklist

### Code Quality
- ✅ TypeScript type definitions
- ✅ ESLint standards compliance
- ✅ Unit test coverage
- ✅ Integration test verification
- ✅ Performance test benchmarks

### Publishing Preparation
- ✅ Correct version number update
- ✅ CHANGELOG update
- ✅ Successful build process
- ✅ Complete documentation generation
- ✅ Successful tag creation

### Publishing Verification
- ✅ Package installation test
- ✅ Functionality verification test
- ✅ Valid documentation links
- ✅ Working example code
- ✅ Compatible dependency versions
