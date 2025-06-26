## Important Files to Reference

When working on this project, always reference these key files for context and requirements:

### Technical Requirements and Standards
- `/spec/requirements.md` - Primary technical requirements, architecture design, and implementation guidelines
  - **When to reference**: Before starting new feature design, when deciding implementation approach, when checking coding standards
  - **Key information**: Architecture constraints, dependency restrictions, testing requirements
- `/spec/spec.md` - Detailed specifications and use cases
  - **When to reference**: Before detailed implementation, when understanding use cases, when designing interfaces
  - **Key information**: User scenarios, input/output formats, expected behaviors

### Project Status

- `/TODO.md` - Current development status, issues, and progress tracking
  - **When to reference**: 
    - Before starting work (checking current status)
    - During work (recording progress)
    - When completing work (marking completed items)
    - When discovering issues (recording problems)
  - **What to check/update**: Current work context, error and bug tracking, implementation status

example:
```markdown
- [ ] Implement feature C
- [ ] Fix bug in module D
- [ ] Update documentation
```

## Development Process and Guidelines

### General Development Workflow
1. Always start with the latest main branch
2. Review and agree on implementation strategy before starting work:
   - Read `/spec/requirements.md`
   - Read `/spec/spec.md`
   - Read relevant `.feature` files
   - Consider test size implications (small/medium/large) based on implementation needs
   - Document the proposed implementation strategy
   - Get team agreement on the implementation approach through:
     - Share the documented strategy with team members
     - Discuss potential trade-offs and alternatives
     - Address concerns and incorporate feedback
     - Obtain explicit approval before proceeding
   - **Important Note on Using edit_file Tool**:
     - All edit_file tool executions performed without prior agreement will be rejected
     - Rejected edit_file operations are permanently discarded and cannot be recovered
     - The edit_file tool can only be used with proper agreement or upon specific request

3. Create a feature branch for each task/bugfix
4. Follow TDD approach: test → implementation → refactoring
5. Update `TODO.md` at task level
6. Run make all before committing
7. Create a Pull Request after ensuring all checks pass

## Project Tracking

The `TODO.md` file must be updated at the task level:

- Task start: Record new task in 現在の作業 section
- During task: Update when encountering errors, issues, or blockers
- Task completion: Move to 完了した機能 section with summary
