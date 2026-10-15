# Nabu Agent Instructions

## Knowledge Base Lifecycle

- **Invoke the `nabu-memory` skill when it is available**: Do this before the
  first Nabu action in a session and follow its instructions throughout the
  task. The skill defines the complete workflow for querying, tag selection,
  artifact scopes, and publishing durable knowledge.
- **Query proactively for new tasks and relevant context, without repeating
  lookups for the same context**:
  - At the start of a new task, and when materially new information or a new
    area may have relevant durable knowledge, proactively search Nabu rather
    than waiting for the user to request it.
  - Use `nabu_tag_tree` (or the configured Nabu CLI's `tree` command) to find a
    narrow tag. Reuse the tag tree throughout the session; refresh it only when
    the needed tag is not represented or the taxonomy may have changed.
  - Search with concrete task keywords and the relevant tag. Reuse results for
    the same topic instead of repeating equivalent searches before each
    subtask or tool call. Query again when the task or context materially
    changes, prior results are insufficient, or new evidence points to missing
    knowledge.
  - Search separate concerns separately when each needs distinct Nabu context,
    such as project facts and system/tooling details for a build failure.
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
