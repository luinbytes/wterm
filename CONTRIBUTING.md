# Contributing to Warp FOSS Clone

First off, thank you for considering contributing to Warp FOSS Clone! It's people like you that make this project great.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
- [Development Setup](#development-setup)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)

## Code of Conduct

This project and everyone participating in it is governed by basic principles of respect and inclusivity. By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## How Can I Contribute?

### Report Bugs

Before creating bug reports, please check the existing issues to avoid duplicates. When you create a bug report, include as many details as possible:

- **Use a clear and descriptive title**
- **Describe the exact steps to reproduce the problem**
- **Provide specific examples to demonstrate the steps**
- **Describe the behavior you observed and expected**
- **Include screenshots if helpful**
- **Include your environment details** (OS, Go version, etc.)

Use the [Bug Report Template](.github/ISSUE_TEMPLATE/bug_report.md).

### Suggest Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion:

- **Use a clear and descriptive title**
- **Provide a detailed description of the suggested enhancement**
- **Explain why this enhancement would be useful**
- **List some other applications where this exists (if applicable)**

Use the [Feature Request Template](.github/ISSUE_TEMPLATE/feature_request.md).

### Pull Requests

- Fill in the required template
- Do not include issue numbers in the PR title
- Include screenshots and animated GIFs in your pull request whenever possible
- Follow the [Go](#go) coding standards
- Document new code
- End all files with a newline

## Development Setup

See [README.md](README.md#development-setup) for complete setup instructions.

## Development Workflow

1. **Fork** the repo on GitHub
2. **Clone** your fork locally
   ```bash
   git clone https://github.com/your-username/wterm.git
   cd wterm
   ```
3. **Create a branch** for your work
   ```bash
   git checkout -b feature/my-feature
   ```
4. **Make your changes** and commit them
5. **Push** to your fork
   ```bash
   git push origin feature/my-feature
   ```
6. **Create a Pull Request** from your fork to the main repo

## Coding Standards

### Go

- Run `gofmt` before committing
- Run `go vet ./...` and fix all warnings
- Run `golangci-lint` and fix all issues (optional but recommended)
- Write tests for new functionality
- Document exported functions and types with godoc comments
- Follow [Effective Go](https://go.dev/doc/effective_go)

### Code Organization

- Keep modules focused and cohesive
- Use clear, descriptive names
- Avoid deeply nested code
- Handle errors appropriately (don't unwrap in production code)

## Commit Guidelines

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests liberally after the first line
- Consider starting the commit message with an applicable emoji:
  - 🎨 `:art:` when improving the format/structure of the code
  - 🐎 `:racehorse:` when improving performance
  - 🚱 `:non-potable_water:` when plugging memory leaks
  - 📝 `:memo:` when writing docs
  - 🐛 `:bug:` when fixing a bug
  - 🔥 `:fire:` when removing code or files
  - 💚 `:green_heart:` when fixing the CI build
  - ✅ `:white_check_mark:` when adding tests
  - 🔒 `:lock:` when dealing with security
  - ⬆️ `:arrow_up:` when upgrading dependencies
  - ⬇️ `:arrow_down:` when downgrading dependencies

## Pull Request Process

1. Ensure any install or build dependencies are removed before the end of the layer when doing a build.
2. Update the README.md with details of changes to the interface, this includes new environment variables, exposed ports, useful file locations and container parameters.
3. Increase the version numbers in any examples files and the README.md to the new version that this Pull Request would represent. The versioning scheme we use is [SemVer](http://semver.org/).
4. You may merge the Pull Request in once you have the sign-off of two other developers, or if you do not have permission to do that, you may request the second reviewer to merge it for you.

## Questions?

Feel free to open an issue with the "question" label, or reach out to the maintainers directly.

---

Thank you for contributing! 🎉
