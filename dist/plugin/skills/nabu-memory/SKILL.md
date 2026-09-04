---
name: nabu-memory
description: Use Nabu's durable, append-only knowledge store while working.
---

# Nabu durable memory

Use Nabu for durable facts, not transient task details.

Before non-trivial work:

1. Call `nabu_tag_tree` to identify the narrowest relevant tag.
2. Call `nabu_search` with that tag. Search results are compact; call `nabu_get` only for useful IDs.

While working, publish facts that another future session would benefit from: decisions and their rationale, reproducible gotchas and fixes, tooling facts, codebase orientation, user conventions, and concise external-system digests.

At task end, distil durable findings into one or more artifacts. Do not publish a generic session recap.

Rules:

- Register a missing tag first using `nabu_register_tag`; its parent must already be registered.
- Attach at least one active tag to every artifact.
- Never include credentials, tokens, cookies, API keys, private keys, or secrets.
- Artifacts are immutable. To correct or replace one, publish a new artifact with `supersedes`; never contradict old content without superseding it.
- Use `scope: local` for machine-specific paths/configuration and `scope: global` only for portable facts.
