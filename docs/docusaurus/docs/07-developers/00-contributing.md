---
id: contributing
title: Contributing Guide
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Contributing to Sequent Voting Platform

Thank you for your interest in contributing to the Sequent Voting Platform! This guide will help you get started with contributing to our open-source project.

## Ways to Contribute

There are many ways to contribute to Sequent:

- **Report bugs** - Help us identify and fix issues
- **Suggest features** - Share ideas for improvements
- **Improve documentation** - Help make our docs clearer and more comprehensive
- **Submit code** - Fix bugs or implement new features
- **Review pull requests** - Help review contributions from other developers
- **Answer questions** - Help other users in our [Discord community](https://discord.gg/WfvSTmcdY8)

## Getting Started

### Prerequisites

Before contributing, make sure you have:

- A [GitHub account](https://github.com/signup)
- Git installed on your local machine
- Docker installed for running the development environment
- Basic knowledge of the technologies we use (see [System Architecture](../05-reference/03-software-architecture/intro.md))
docs/docusaurus/docs/0
### Development Environment Setup

The fastest way to start developing is using VS Code Dev Containers or GitHub Codespaces:

1. **Fork the repository** on GitHub
2. **Clone your fork**:
   ```bash
   git clone https://github.com/YOUR-USERNAME/step.git
   cd step
   ```
3. **Open in VS Code** with the Dev Containers extension installed
4. The environment will automatically set up all dependencies

For the devcontainer modes and the development loop, see [Fast feedback loops](./03-development-environment/fast-feedback.md); for build and test commands, see [Build, test and lint](./03-development-environment/build-and-test.md).

### Development Workflow

1. **Create a branch** for your work. Maintainers name branches after the
   tracking issue in [sequentech/meta](https://github.com/sequentech/meta/issues)
   and the target branch, e.g. `feat/meta-<issue>-<description>/main` or
   `fix/meta-<issue>/release/9.4`:
   ```bash
   git checkout -b feat/meta-1234-short-description/main
   ```

2. **Make your changes** following our coding standards

3. **Test your changes** thoroughly:
   - Run existing tests
   - Add new tests for new features
   - Verify the documentation builds correctly

4. **Commit your changes** with clear, descriptive messages:
   ```bash
   git add .
   git commit -m "feat: add new feature description"
   ```

5. **Push to your fork**:
   ```bash
   git push origin feat/meta-1234-short-description/main
   ```

6. **Open a Pull Request** on GitHub

## Coding Standards

### General Guidelines

- Write clear, readable code with meaningful variable names
- Add comments for complex logic
- Keep functions small and focused
- Follow the existing code style in each part of the project

- Write the failing test first, then the code that makes it pass (TDD)
- Remove dead code, commented-out code, stale or generated boilerplate
  comments, and TODOs without a linked issue before review

### Product Design Rules

- **No client-specific features**: design a general solution that serves the
  requesting client and every existing one.
- **Composable features**: any combination of features must work, or the
  constraint must be explicit through validation, warnings or documentation.
- **Policies are enums, not booleans**, in Rust and TypeScript: an on/off
  policy almost always grows more modes.
- **Backwards-compatible data**: new fields in stored or exported structures are
  optional or have defaults, so older election events still load and import.

### Language-Specific Standards

#### Rust
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Add documentation comments for public APIs
- Propagate `Option` and `Result` with `?` or explicit handling; the
  [assurance lint policy](./03-development-environment/production-assurance.md)
  forbids panicking shortcuts in production code
- Reuse the types in `sequent_core::types` instead of redefining them
- Use named constants and enums (with `Display`/`FromStr`) instead of repeated
  string literals

#### TypeScript/JavaScript
- Use TypeScript for new frontend code, without `any`
- Await every promise and handle its errors
- Follow the ESLint configuration
- Run `yarn prettify:fix` before committing
- Use functional components and hooks in React
- Prefix environment variables with their service (`HASURA_`, `B4_`, ...) and add
  new ones to the relevant Docker Compose files

#### Documentation
- Write in clear, concise English
- Include code examples where appropriate
- Update the documentation when changing functionality
- Check for broken links before submitting

## Commit Message Convention

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

- `feat:` - A new feature
- `fix:` - A bug fix
- `docs:` - Documentation only changes
- `style:` - Code style changes (formatting, missing semicolons, etc.)
- `refactor:` - Code changes that neither fix bugs nor add features
- `perf:` - Performance improvements
- `test:` - Adding or updating tests
- `chore:` - Changes to build process or auxiliary tools

Example:
```
feat: add ballot verification to voting portal

This commit adds the ability for voters to verify their ballot
through the voting portal interface.
```

## Contributor License Agreement (CLA)

Before we can accept your contribution, you must sign the **Sequent Contributor License Agreement (CLA)**. This is a one-time requirement that ensures:

- You have the legal right to contribute the code
- Sequent can use and distribute your contributions
- The project remains open source under the AGPL-3.0 license
- Contributors are properly credited and protected

### How It Works

1. When you submit your first pull request, the CLA Assistant bot will automatically comment on your PR
2. Review the [full CLA document](https://github.com/sequentech/step/blob/main/.github/cla/CLA.md)
3. Sign by commenting on your PR with: `I have read the CLA Document and I hereby sign the CLA`
4. The bot will record your signature and update your PR status
5. You only need to sign once - all future contributions are covered

The CLA signing process is quick and straightforward. If you have any questions about the CLA, please reach out on our [Discord community](https://discord.gg/WfvSTmcdY8) or comment on your pull request.

## Pull Request Process

1. **Ensure your PR**:
   - Has a clear title and description
   - Starts its description with `Parent issue: https://github.com/sequentech/meta/issues/<number>`
     (maintainers open the issue first when none exists)
   - Includes tests for new functionality, including rejected input and edge cases
   - Updates the documentation under `docs/docusaurus/` for new features
   - Adds new Keycloak permissions to `IPermissions` in
     `packages/admin-portal/src/types/keycloak.ts` and to the default tenant
     realm under `.devcontainer/keycloak/import/`
   - Checks frontend changes at different screen sizes
   - Passes all CI checks
   - **Has a signed CLA** (for first-time contributors)

2. **PR Review**:
   - At least one maintainer review is required
   - Address review comments promptly
   - Be open to feedback and discussion

3. **Release branches** (`release/X.Y`): the same fix is cherry-picked to every
   applicable release branch, and support PRs leave release notes alone; they
   are generated from the issue.

4. **After Approval**:
   - Maintainers will merge your PR
   - Your contribution will be included in the next release

## Testing

### Running Tests

[Build, test and lint](./03-development-environment/build-and-test.md) lists the
per-stack commands, and [Fast feedback loops](./03-development-environment/fast-feedback.md)
selects the focused test for a change:

```bash
scripts/dev/step-dev test --affected
```

### Writing Tests

- Write unit tests for new functions and modules
- Write integration tests for API endpoints
- Write end-to-end tests for critical user flows
- Aim for good test coverage, especially for critical code paths

## Documentation

When contributing documentation:

1. **Preview your changes**:
   ```bash
   cd docs/docusaurus
   npm start
   ```

2. **Check for broken links**:
   ```bash
   npm run build
   ```

3. **Follow the documentation structure** in the `docs/` folder

## License

By contributing to Sequent Voting Platform, you agree that your contributions will be licensed under the AGPL-3.0-only license. Make sure all new files include the appropriate SPDX license header:

```
<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
```

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive environment for all contributors, regardless of:
- Age, body size, disability, ethnicity, gender identity and expression
- Level of experience, education, socio-economic status
- Nationality, personal appearance, race, religion, or sexual identity and orientation

### Our Standards

**Positive behavior includes**:
- Being respectful and considerate
- Accepting constructive criticism gracefully
- Focusing on what's best for the community
- Showing empathy towards other community members

**Unacceptable behavior includes**:
- Harassment, trolling, or insulting comments
- Personal or political attacks
- Publishing others' private information without permission
- Any conduct inappropriate in a professional setting

### Enforcement

Instances of unacceptable behavior may be reported to the project maintainers. All complaints will be reviewed and investigated, resulting in a response deemed necessary and appropriate.

## Getting Help

If you need help or have questions:

- **Discord**: Join our [Discord community](https://discord.gg/WfvSTmcdY8)
- **GitHub Issues**: Check existing issues or create a new one
- **Documentation**: Browse our [comprehensive documentation](https://docs.sequentech.io/docusaurus/main/)

## Recognition

Contributors who make significant contributions will be:
- Listed in our project contributors
- Acknowledged in release notes
- Invited to participate in project discussions and decisions

Thank you for contributing to Sequent Voting Platform! Your efforts help make online voting more secure, transparent, and accessible for everyone.
