# Git Hooks

This directory contains project-specific Git hooks for quality checks.

## Installation

After cloning the repository, run:

```bash
git config core.hooksPath .githooks
```

This enables the project's hooks instead of the global Git hooks.

## Available Hooks

### pre-commit
- Ensures `README.md` is updated when bumping the version in `Cargo.toml`
- Helps maintain documentation accuracy for each release

### commit-msg (optional)
- Warns when version updates may need README review

## Disabling Hooks (Not Recommended)

If you need to bypass hooks temporarily:

```bash
git commit --no-verify
git push --no-verify
```
