# Global Agent Instructions

## Testing Strategy: Fine-Grained & Targeted
- **Default to targeted test execution**: Run only the specific test class or method directly relevant to the changes (e.g. `mvn test -pl <module> -Dtest=TestClass#testMethod`, or specific test file in JavaScript/TypeScript).
- **Avoid running full module or monorepo test suites** for scoped changes. Reserve broad or entire-suite test runs only for wide-reaching, cross-cutting, or high-risk architectural changes.

## Knowledge Base (Nabu) Lifecycle
- **Query proactively and repeatedly throughout each session**:
  - Do NOT query Nabu only once at session start. Query it **multiple times per session**, before starting **any new step or subtask** (e.g. before domain modeling, before schema/migration changes, before writing tests, before running builds, and before debugging investigations).
  - Search using `nabu_search` (or `~/.cargo/bin/nabu search "<query>"`) with relevant tags or keywords to check for known gotchas, domain rules, architectural patterns, and user preferences before executing that step.
  - Call `nabu_tag_tree` (or `~/.cargo/bin/nabu tree`) whenever exploring a new area of the codebase to discover applicable tags.
- **Publish durable knowledge**: Proactively distil and publish (`nabu_publish` or `~/.cargo/bin/nabu publish`) non-obvious domain gotchas, architectural decisions with rationale, and user conventions/preferences.
