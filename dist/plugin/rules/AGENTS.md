# Nabu Agent Instructions

## Knowledge Base Lifecycle

- **Invoke the `nabu-memory` skill when it is available**: Do this before the
  first Nabu action in a session and follow its instructions throughout the
  task. The skill defines the complete workflow for querying, tag selection,
  artifact scopes, and publishing durable knowledge.
- **Query proactively and repeatedly throughout each session**:
  - Do NOT query Nabu only once at session start. Query it **multiple times per
    session**, before starting **any new step or subtask** (e.g. before domain
    modeling, before schema/migration changes, before writing tests, before
    running builds, and before debugging investigations).
  - Call `nabu_tag_tree` (or the configured Nabu CLI's `tree` command) to find
    the narrowest relevant tag, then search it with concrete task keywords. If
    needed, expand to its parent or a related cross-cutting tag instead of
    relying on an untagged search.
  - Search separate concerns separately: for example, search both the project
    tag and a `system` or `tooling` tag before investigating a build or
    environment failure.
- **Publish durable knowledge**: Proactively publish through `nabu_publish` or
  the configured Nabu CLI non-obvious domain gotchas, architectural decisions
  with rationale, user conventions/preferences, and reusable machine or
  configuration facts. Capture tool/runtime setup, platform quirks, editor or
  shell behavior, permissions, sandbox/network constraints, and reproducible
  workarounds when they reduce future errors or streamline work. Use
  `scope: local` for machine- or user-specific facts and `scope: global` only
  for portable knowledge.
- **Handle transport failures portably**: If the MCP transport is unavailable,
  use the Nabu CLI only when it is available in the current harness, following
  that harness's documented permission, executable-discovery, and configuration
  conventions. Do not assume an installation path or bypass mode.
