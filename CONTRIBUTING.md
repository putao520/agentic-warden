# Contributing

Thanks for contributing to AIW / agentic-warden.

## Before opening a change

- Use GitHub Issues for bugs, feature proposals, and design discussion when the change is non-trivial.
- Keep pull requests focused on one problem. Avoid unrelated refactors or dependency churn in the same PR.
- For behavior or performance changes, include project-specific evidence rather than relying only on upstream claims or microbenchmarks.
- Security-sensitive findings should follow [SECURITY.md](./SECURITY.md) instead of being disclosed in a public issue.

## Pull requests

A useful PR should explain:

1. the problem being solved;
2. why the proposed change is appropriate for this project;
3. how it was tested;
4. relevant platform or compatibility impact.

Please keep existing behavior stable unless the PR explicitly documents an intentional change. Cross-platform changes should consider Linux, macOS, and Windows where applicable.

## Testing

Run the relevant Rust formatting, build, lint, and test checks for the area you changed. If a platform-specific problem is being fixed, include the platform and verification command/result in the PR description.

## Maintainer decisions

A proposal may be declined when its maintenance cost, portability impact, security surface, or project-specific evidence does not justify the change. Off-by-default features still need to demonstrate that their support cost is worthwhile.
