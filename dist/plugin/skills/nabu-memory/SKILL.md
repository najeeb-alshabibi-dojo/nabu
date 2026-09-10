---
name: nabu-memory
description:
  Proactively query and publish durable project, machine, configuration,
  tooling, and user-preference facts. Use scoped tags to find the most relevant
  artifacts and record discoveries that prevent future errors or streamline
  development.
---

# Nabu durable memory

Use Nabu for durable facts, not transient task details. Its purpose is to make
the next agent more effective: record anything surprising, error-prone, or
repeatedly useful about the project, the user's workflow, or the machine on
which work happens.

## Querying and navigating tags

Query Nabu **multiple times throughout every session**—never treat it as a
one-time check at the beginning. Before starting **any new step or subtask**
(for example, exploring a repository area, changing configuration, designing a
model, running a build, or debugging a failure):

1. Call `nabu_tag_tree` (or the configured Nabu CLI's `tree` command) and
   identify the narrowest existing tag that matches the work. Examples:
   `system::java` for a local JDK issue, `editor::neovim::lsp` for editor
   behavior, or `avalanche::zebra-inv-app` for repository-specific facts.
2. Search that tag with concrete task keywords:
   `nabu_search(query="<tool or symptom> <task>", tags="<narrow-tag>")`. Results
   are compact; call `nabu_get` only for useful IDs.
3. If the narrow search is empty or insufficient, expand deliberately: search
   its parent tag, then a related cross-cutting tag such as `system`, `tooling`,
   or `personal`. Do not rely on untagged searches when a relevant tag exists.
4. For work that spans concerns, perform separate targeted searches rather than
   one vague search—for example, search both a project tag and `system::java`
   before investigating a build failure.

Tags are hierarchical scopes for discovery. Prefer the most specific active tag,
add multiple tags only when an artifact genuinely applies to each area, and
register a missing child tag under its closest active parent before publishing.
Use artifact `scope: local` for facts tied to this machine or user environment;
use `scope: global` only for portable facts that remain true elsewhere.

## What to preserve

Publish a concise artifact as soon as you discover a durable fact that a future
agent could use to avoid an error, shorten investigation, or work in the user's
preferred way. In particular, preserve:

- **Machine and configuration facts:** installed tool locations and versions,
  runtime or SDK selection, shell/editor/IDE behavior, package-manager setup,
  container or service configuration, filesystem conventions, platform quirks,
  permissions, network or sandbox constraints, and reproducible setup fixes.
- **Unexpected behavior and workarounds:** an error's trigger, its reliable
  diagnosis, the smallest effective fix, and any command or configuration
  required to reproduce it.
- **Workflow accelerators:** trusted commands, target-specific test or build
  entry points, repository orientation, integration behavior, and conventions
  that make future work faster or less error-prone.
- **User preferences:** coding, testing, communication, tooling, and workflow
  preferences that should consistently guide later sessions.
- **Project decisions and external facts:** decisions with rationale, durable
  domain rules, and concise digests of authoritative external systems.

Do not wait until the end when a discovery is clearly reusable. At task end,
distil any remaining durable findings into focused artifacts; do not publish a
generic session recap or transient progress notes.

## Rules

- Query continuously: always perform a Nabu tag-tree lookup and targeted search
  before transitioning to a new task step or subtask.
- If MCP transport fails, use the Nabu CLI when it is available in the current
  harness. Follow the harness's documented permission, executable-discovery, and
  configuration conventions; do not assume an installation path or bypass mode.
- Register a missing tag first using `nabu_register_tag`; its parent must
  already be registered.
- Attach at least one active, specific tag to every artifact. Use the narrowest
  tag that makes the artifact discoverable, plus a cross-cutting tag only when
  useful.
- Never include credentials, tokens, cookies, API keys, private keys, or
  secrets.
- Artifacts are immutable. To correct or replace one, publish a new artifact
  with `supersedes`; never contradict old content without superseding it.
- Use `scope: local` for machine-specific paths, configuration, environment
  behavior, or user-specific preferences. Use `scope: global` only for portable
  facts.
