---
name: nabu-memory
description: Proactively query multiple times throughout each session before starting any new step or subtask to find known gotchas, domain rules, and user preferences; distil and publish new decisions, gotchas, and conventions upon completion.
---

# Nabu durable memory

Use Nabu for durable facts, not transient task details.

## Querying Nabu:
Query Nabu **multiple times throughout every session**—never treat it as a one-time check at the beginning. Before starting **any new step or subtask** in a session (e.g. designing domain models, creating schema migrations, writing tests, executing builds, or debugging failures):

1. Call `nabu_tag_tree` (or `~/.cargo/bin/nabu tree`) to identify the narrowest relevant tag for the upcoming step.
2. Call `nabu_search` (or `~/.cargo/bin/nabu search "<query>"`) with that tag and step keywords. Search results are compact; call `nabu_get` only for useful IDs.

While working, publish facts that another future session would benefit from: decisions and their rationale, reproducible gotchas and fixes, tooling facts, codebase orientation, user conventions, and concise external-system digests.

At task end, distil durable findings into one or more artifacts. Do not publish a generic session recap.

## Rules:

- Query continuously: Always perform a Nabu search before transitioning to a new task step or subtask.
- If MCP transport fails, use the CLI at `~/.cargo/bin/nabu` (requires `BypassSandbox: true`).
- Register a missing tag first using `nabu_register_tag`; its parent must already be registered.
- Attach at least one active tag to every artifact.
- Never include credentials, tokens, cookies, API keys, private keys, or secrets.
- Artifacts are immutable. To correct or replace one, publish a new artifact with `supersedes`; never contradict old content without superseding it.
- Use `scope: local` for machine-specific paths/configuration and `scope: global` only for portable facts.
