# Deployment Development Standards

**Version**: 2.0.0
**Last Updated**: 2025-12-25

## Role Positioning
**Deployment Configuration and Release Management**
- Primary focus: deployment scripts, configuration management, release pipelines
- Tech stack: Docker, Kubernetes, CI/CD, configuration management tools
- Use cases: application deployment, environment configuration, release management, containerization

---

## 🚨 Core Iron Laws (inherited from common.md)

> **Must follow the four core iron laws from common.md**

```
Iron Law 1: SPEC is the Single Source of Truth (SSOT)
       - Deployment configuration must be fully consistent with SPEC
       - Environment variables, port mappings, etc. must comply with SPEC definitions

Iron Law 2: Intelligent Reuse and Destroy-and-Rebuild
       - Existing config fully meets requirements → Reuse
       - Partially meets → Delete and rewrite

Iron Law 3: Prohibit Incremental Development
       - Prohibit keeping old Dockerfile and adding new layers
       - Prohibit compatibility configurations supporting multiple versions

Iron Law 4: Context7 Research First
       - Must research containerization best practices
       - Use officially recommended deployment patterns
```

---

## Coding Standards

### Deployment Principles
- Environment consistency guarantee
- Externalized configuration management
- Automated deployment process
- Rolling update strategy
- Rapid rollback mechanism

### Configuration Management
- Environment variable management
- Secure key storage
- Configuration version control
- Configuration validation mechanism
- Secure default configuration

## Tech Stack Guidance

### Containerized Deployment
- **Docker**: Multi-stage builds, image optimization, secure configuration
- **Kubernetes**: Deployment configuration, Service definition, ConfigMap/Secret
- **Docker Compose**: Local environment, development configuration, test environment

### Configuration Management Tools
- **Helm**: Kubernetes package management, template configuration, version management
- **Kustomize**: Kubernetes configuration management, environment customization, patch mechanism
- **Ansible**: Configuration automation, playbook authoring, modular design

### Environment Management
- **Environment Isolation**: Dev/test/staging/production environment separation
- **Configuration Management**: Environment variables, configuration files, key management
- **Dependency Management**: Service dependencies, data source dependencies, external API dependencies

## Quality Standards

### Deployment Quality
- Deployment success rate > 99%
- Deployment time < 10 minutes
- Rollback time < 5 minutes
- Configuration consistency
- Environment isolation

### Security Requirements
- Image security scanning
- Secure key storage
- Network access control
- Privilege minimization
- Audit log recording

## Delivery Standards

### Implementation Requirements
- ✅ Dockerfile best practices
- ✅ Kubernetes configuration files
- ✅ Automated deployment scripts
- ✅ Environment configuration management
- ✅ Monitoring and logging configuration

### Documentation Requirements
- ✅ Deployment operations manual
- ✅ Environment configuration documentation
- ✅ Incident handling procedures
- ✅ Security configuration guide
- ✅ Monitoring and alerting configuration

## Deployment Checklist

### Configuration Checks
- ✅ Complete environment variables
- ✅ Secure key configuration
- ✅ Reasonable resource limits
- ✅ Health check configuration
- ✅ Network policy settings

### Deployment Verification
- ✅ Service starts normally
- ✅ Health checks pass
- ✅ API interfaces available
- ✅ Database connections normal
- ✅ Log output normal

### Rollback Preparation
- ✅ Rollback scripts ready
- ✅ Data backups complete
- ✅ Version tags clear
- ✅ Rollback tests pass
- ✅ Emergency contacts identified
