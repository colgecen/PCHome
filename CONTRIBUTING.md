# Contributing to PChome

Thank you for your interest in contributing to PChome! This document provides guidelines and information for contributors.

## Code of Conduct

By participating in this project, you agree to uphold our Code of Conduct. Please report unacceptable behavior to the project maintainers.

## How to Contribute

### Reporting Bugs

- Search existing issues to avoid duplicates
- Use the bug report template when creating a new issue
- Include steps to reproduce, expected behavior, and actual behavior
- Provide environment details (OS, Rust version, Go version, etc.)

### Suggesting Features

- Search existing issues and discussions first
- Use the feature request template
- Clearly describe the problem and proposed solution
- Consider whether the feature aligns with project goals

### Pull Requests

1. Fork the repository and create a new branch from `main`
2. Follow the conventional commits specification for commit messages
3. Ensure all CI checks pass
4. Update documentation as needed
5. Add tests for new functionality
6. Submit a PR using the provided template

## Development Setup

See [README.md](README.md) for setup instructions.

## Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` A new feature
- `fix:` A bug fix
- `docs:` Documentation changes
- `style:` Code style changes (formatting, missing semicolons)
- `refactor:` Code refactoring without feature or bug fix changes
- `perf:` Performance improvements
- `test:` Adding or updating tests
- `build:` Changes to build system or dependencies
- `ci:` CI configuration changes
- `chore:` Other changes that don't modify src or test files
- `revert:` Reverting previous commits

## Branching Strategy

- `main` is the primary production branch
- `develop` is the integration branch
- Feature branches: `feature/<short-description>`
- Bug fix branches: `fix/<issue-number>-<short-description>`
- Release branches: `release/<version>`

## License

By contributing, you agree that your contributions will be licensed under the project's license.
