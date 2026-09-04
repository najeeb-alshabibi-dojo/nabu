# Global Agent Instructions

## Testing Strategy: Fine-Grained & Targeted
- **Default to targeted test execution**: Run only the specific test class or method directly relevant to the changes (e.g. `mvn test -pl <module> -Dtest=TestClass#testMethod`, or specific test file in JavaScript/TypeScript).
- **Avoid running full module or monorepo test suites** for scoped changes. Reserve broad or entire-suite test runs only for wide-reaching, cross-cutting, or high-risk architectural changes.

## Knowledge Base (Nabu) Lifecycle
- **Before non-trivial tasks or debugging**: Proactively query Nabu using `nabu_search` (and `nabu_tag_tree`) to check for known gotchas, architectural patterns, and user preferences before designing solutions.
- **Publish durable knowledge**: Proactively distil and publish (`nabu_publish`) non-obvious domain gotchas, architectural decisions with rationale, and user conventions/preferences.
