# Nabu

Nabu is an append-only local knowledge store for coding agents. It keeps durable
facts as Markdown files, organizes them with hierarchical tags, and provides
full-text search through a generated SQLite index. Each store is also a Git
repository, so changes to its knowledge can be reviewed and versioned.

## Features

- Store decisions, gotchas, environment facts, project orientation, preferences,
  and external references.
- Search artifact titles, summaries, and bodies using SQLite FTS5.
- Add hierarchical tags such as `system::java`.
- Keep knowledge global or scoped to the current machine.
- Preserve history through append-only artifacts and explicit supersession.
- Connect coding agents over the Model Context Protocol (MCP).

## Build and install

Nabu requires Rust with support for the 2024 edition.

```sh
cargo build --release
```

The executable is available at `target/release/nabu`. To run the test suite:

```sh
cargo test
```

## Getting started

Initialize the default store and check its health:

```sh
nabu init
nabu doctor
```

By default, the store is created at `~/.agent-memory`. Set `NABU_HOME` to use a
different location:

```sh
NABU_HOME="$HOME/my-agent-memory" nabu init
```

Register a root tag before registering tags beneath it:

```sh
nabu tags register project "Project-specific knowledge"
nabu tags register project::nabu "Nabu project facts"
nabu tree
```

Publish an artifact by piping its body to `nabu publish`:

```sh
printf '%s\n' "Nabu stores artifacts as Markdown and indexes them with SQLite FTS5." |
  nabu publish \
    --kind orientation \
    --title "Nabu storage format" \
    --summary "Artifacts are Markdown files with a generated SQLite search index." \
    --tags project::nabu
```

Search and retrieve knowledge:

```sh
nabu search "SQLite index" --tag project
nabu recent --tag project::nabu
nabu show <artifact-id>
```

Artifact IDs can be supplied in full or as an unambiguous prefix. To update a
fact, publish a new artifact that supersedes its old ID; artifacts are not
edited in place. Use `--scope local` with `publish` for knowledge specific to
this machine; the default scope is `global`.

Run `nabu --help` or `nabu <command> --help` for all options.

## Install the agent skill and instructions

The distributable agent assets are in `dist/plugin`: the `nabu-memory` skill
and the `AGENTS.md` instructions. Install both for the agent client you use;
installing the MCP server alone does not install these assets.

### Antigravity

For Antigravity CLI, install the plugin from the repository root:

```sh
agy plugin install ./dist/plugin
agy plugin list
```

For Antigravity IDE or app, install the skill globally:

```sh
mkdir -p "$HOME/.gemini/config/skills"
cp -R dist/plugin/skills/nabu-memory "$HOME/.gemini/config/skills/"
```

To apply the instructions globally, merge `dist/plugin/rules/AGENTS.md` into
`~/.gemini/config/AGENTS.md`. Do not replace an existing instruction file;
combine its contents with the Nabu instructions. Alternatively, put the
instructions in the workspace's `AGENTS.md` to limit them to that project.
Review the active skill in Antigravity's **Customizations** settings.

See the [Antigravity skills guide](https://antigravity.google/docs/skills).

### GitHub Copilot CLI

Install the skill from the repository root:

```sh
copilot skill add ./dist/plugin/skills/nabu-memory
copilot skill list
```

Install the instructions as a personal instruction file:

```sh
mkdir -p "$HOME/.copilot/instructions"
cp dist/plugin/rules/AGENTS.md \
  "$HOME/.copilot/instructions/nabu.instructions.md"
```

Alternatively, put the instructions in the workspace's `AGENTS.md` to apply
them only to that project. If Copilot CLI is already running, reload skills
with `/skills reload`. See GitHub's guides to
[adding agent skills](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills)
and [adding MCP servers and customizations](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers).

## MCP server

Build Nabu first with `cargo build --release`. Configure the MCP client to run
the resulting executable at `target/release/nabu` with the `mcp` argument. Use
an absolute path to the executable in client configurations.

### Antigravity

In Antigravity IDE, open the agent panel's **…** menu, choose **MCP Servers**,
then **Manage MCP Servers** and **View raw config**. Add the following entry to
the `mcpServers` object in `mcp_config.json`:

```json
{
  "nabu": {
    "command": "/absolute/path/to/nabu",
    "args": ["mcp"],
    "env": {
      "NABU_HOME": "/absolute/path/to/agent-memory"
    }
  }
}
```

For Antigravity app settings, manage the server under **Settings >
Customizations > Installed MCP Servers**. The global configuration file is
`~/.gemini/config/mcp_config.json`; Antigravity CLI also supports a workspace
configuration at `.agents/mcp_config.json`. See the
[Antigravity MCP guide](https://antigravity.google/docs/mcp) for details.

### GitHub Copilot CLI

Add Nabu as a local stdio server from your terminal:

```sh
copilot mcp add \
  nabu \
  --env NABU_HOME=/absolute/path/to/agent-memory \
  -- /absolute/path/to/nabu mcp
```

Alternatively, start Copilot CLI, enter `/mcp add`, choose **Local** or
**STDIO**, and set the command to `/absolute/path/to/nabu mcp`. Add
`NABU_HOME=/absolute/path/to/agent-memory` as an environment variable if you
want a non-default store location. Check the connection with `copilot mcp list`
or `/mcp` inside an interactive session. User-level configuration is stored in
`~/.copilot/mcp-config.json`.

See GitHub's guide to
[adding MCP servers for Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers).

Nabu's MCP server provides tools for browsing tags, searching and retrieving
artifacts, publishing knowledge, registering tags, listing recent artifacts,
and following supersession links.

## Store layout

The store contains Markdown artifacts under `artifacts/`, tag definitions under
`tags/`, and supersession records under `edges/`. Nabu maintains a generated
SQLite full-text index in `.index/` and initializes Git in the store directory.
The index and lock files are ignored by Git; Markdown records are the durable
source of truth.

## License

Nabu is licensed under the MIT License. See [LICENSE](LICENSE).
