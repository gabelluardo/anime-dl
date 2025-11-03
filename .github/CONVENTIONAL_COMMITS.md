# Conventional Commits Guide

This project follows the [Conventional Commits](https://www.conventionalcommits.org/) specification.

## Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

## Types

- **feat**: A new feature
- **fix**: A bug fix
- **docs**: Documentation only changes
- **style**: Changes that do not affect the meaning of the code (white-space, formatting, etc)
- **refactor**: A code change that neither fixes a bug nor adds a feature
- **perf**: A code change that improves performance
- **test**: Adding missing tests or correcting existing tests
- **build**: Changes that affect the build system or external dependencies
- **ci**: Changes to our CI configuration files and scripts
- **chore**: Other changes that don't modify src or test files
- **revert**: Reverts a previous commit

## Examples

```
feat: add Italian subtitle support
fix: correct episode number parsing
docs: update installation instructions
chore: upgrade dependencies
```

## Pull Request Titles

Pull request titles must follow the same format. The PR title will be used as the commit message when squash merging.

## Validation

A GitHub Action automatically validates PR titles to ensure they follow the conventional commits format.
