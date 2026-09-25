# Security Policy

AIW / agentic-warden handles AI CLI processes, provider routing, local state, semantic memory, hooks, MCP integration, and other host-level capabilities. Security reports should be handled privately when disclosure could put users at risk.

## Supported versions

During the current pre-1.0 phase, security fixes are made against the latest tagged release. Older releases are not guaranteed to receive backports; please reproduce on the latest release when possible.

## Reporting a vulnerability

Please do **not** open a public GitHub issue for a vulnerability that could enable code execution, credential/token exposure, privilege or capability bypass, unsafe process control, path/file access outside the intended scope, or other security-sensitive behavior.

Use GitHub Private Vulnerability Reporting / Security Advisories for this repository when available:

https://github.com/putao520/agentic-warden/security/advisories/new

Include the affected version or commit, platform, reproduction steps or a minimal PoC, impact, and any required preconditions.

Ordinary bugs, feature requests, build failures, and performance issues should continue to use public GitHub Issues.

## Coordinated disclosure

Please allow time for triage and a fix before publishing exploit details for an unpatched issue. We will keep the report updated as the issue is assessed and addressed.
