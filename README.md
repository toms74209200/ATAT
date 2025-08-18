# ATAT

CLI tool for synchronizing TODO.md with GitHub Issues

## Motivation

Managing tasks between local TODO.md files and GitHub Issues often leads to duplication and synchronization problems. ATAT solves this by providing a git-like workflow to keep both in sync automatically.

## Prerequisites

- Rust 1.86.0

## Usage

### Authentication

```bash
atat login
```

### Repository Setup

Add a repository to sync with:

```bash
atat remote add owner/repo
```

View current repository configuration:

```bash
atat remote
```

Remove a repository:

```bash
atat remote remove owner/repo
```

### Commands

Push TODO.md to GitHub Issues

```bash
atat push
``` 

Pull GitHub Issues to TODO.md

```bash
atat pull
```

### TODO.md Format

ATAT works with standard markdown checkbox format:

```markdown
- [ ] Implement new feature
- [x] Fix bug in authentication
- [ ] Update documentation
```

After synchronization, Issue numbers will be automatically added:

```markdown
- [ ] Implement new feature #123
- [x] Fix bug in authentication #124
- [ ] Update documentation #125
```

## License

[MIT License](LICENSE)

## Author

[toms74209200](<https://github.com/toms74209200>)
