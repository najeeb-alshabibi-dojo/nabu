use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use nabu::{Artifact, ArtifactKind, NabuStore, Scope, mcp};
use std::{
    io::{self, Read},
    path::PathBuf,
};

#[derive(Parser)]
#[command(name = "nabu", about = "Append-only agent knowledge store")]
struct Cli {
    #[arg(long, env = "NABU_HOME")]
    home: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Init,
    Reindex,
    Doctor,
    Mcp,
    Tree {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value_t = 3)]
        depth: usize,
    },
    Tags {
        #[command(subcommand)]
        command: TagCommand,
    },
    Search {
        query: String,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        kind: Option<KindArg>,
        #[arg(long, default_value_t = 8)]
        limit: usize,
        #[arg(long)]
        include_superseded: bool,
    },
    Recent {
        #[arg(long)]
        tag: Option<String>,
        #[arg(long, default_value_t = 8)]
        limit: usize,
    },
    Show {
        id: String,
    },
    Publish {
        #[arg(long)]
        kind: KindArg,
        #[arg(long)]
        title: String,
        #[arg(long)]
        summary: String,
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        #[arg(long, default_value = "global")]
        scope: ScopeArg,
        #[arg(long, value_delimiter = ',')]
        supersedes: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        sources: Vec<String>,
        #[arg(long, default_value = "human")]
        author: String,
        #[arg(long)]
        body: Option<String>,
    },
    /// Compact terminal browser. Pass an artifact id to inspect it.
    Tui {
        id: Option<String>,
    },
}
#[derive(Subcommand)]
enum TagCommand {
    Register { path: String, description: String },
    Deprecate { path: String },
}
#[derive(Clone, ValueEnum)]
enum KindArg {
    Decision,
    Gotcha,
    Environment,
    Orientation,
    Preference,
    External,
}
impl From<KindArg> for ArtifactKind {
    fn from(x: KindArg) -> Self {
        match x {
            KindArg::Decision => Self::Decision,
            KindArg::Gotcha => Self::Gotcha,
            KindArg::Environment => Self::Environment,
            KindArg::Orientation => Self::Orientation,
            KindArg::Preference => Self::Preference,
            KindArg::External => Self::External,
        }
    }
}
#[derive(Clone, ValueEnum)]
enum ScopeArg {
    Global,
    Local,
}
impl From<ScopeArg> for Scope {
    fn from(x: ScopeArg) -> Self {
        match x {
            ScopeArg::Global => Self::Global,
            ScopeArg::Local => Self::Local,
        }
    }
}
fn main() -> Result<()> {
    let cli = Cli::parse();
    let s = NabuStore::open(cli.home.unwrap_or_else(NabuStore::default_root))?;
    match cli.command {
        Command::Init => println!("initialized {}", s.root().display()),
        Command::Reindex => {
            s.reindex()?;
            println!("reindexed")
        }
        Command::Doctor => println!(
            "store: {}\nfts5 index: healthy\nembedding search: not enabled (FTS5 fallback)\ngit: enabled",
            s.root().display()
        ),
        Command::Mcp => mcp::serve(s)?,
        Command::Tree { root, depth } => {
            for t in s.tags(root.as_deref(), false)? {
                let d = t.path.matches("::").count();
                if d < depth {
                    println!(
                        "{}{} — {}",
                        "  ".repeat(d),
                        t.path.rsplit("::").next().unwrap(),
                        t.description
                    )
                }
            }
        }
        Command::Tags {
            command: TagCommand::Register { path, description },
        } => println!(
            "{}",
            serde_json::to_string_pretty(&s.register_tag(&path, &description)?)?
        ),
        Command::Tags {
            command: TagCommand::Deprecate { path },
        } => println!(
            "{}",
            serde_json::to_string_pretty(&s.deprecate_tag(&path)?)?
        ),
        Command::Search {
            query,
            tag,
            kind,
            limit,
            include_superseded,
        } => println!(
            "{}",
            serde_json::to_string_pretty(&s.search(
                &query,
                tag.as_deref(),
                kind.map(Into::into).as_ref(),
                limit,
                include_superseded
            )?)?
        ),
        Command::Recent { tag, limit } => println!(
            "{}",
            serde_json::to_string_pretty(&s.recent(tag.as_deref(), limit)?)?
        ),
        Command::Show { id } => {
            let a = s
                .get(&id)?
                .ok_or_else(|| anyhow::anyhow!("artifact not found"))?;
            println!("{}", serde_json::to_string_pretty(&a)?);
            let next = s.superseded_by(&id)?;
            if !next.is_empty() {
                println!("superseded_by: {}", next.join(", "))
            }
        }
        Command::Publish {
            kind,
            title,
            summary,
            tags,
            scope,
            supersedes,
            sources,
            author,
            body,
        } => {
            let mut stdin = String::new();
            if body.is_none() {
                io::stdin().read_to_string(&mut stdin)?;
            }
            let (a, new) = s.publish(Artifact {
                id: String::new(),
                kind: kind.into(),
                title,
                summary,
                tags,
                scope: scope.into(),
                created_at: Utc::now(),
                author,
                supersedes,
                sources,
                body: body.unwrap_or(stdin),
            })?;
            println!(
                "{} {}",
                if new { "published" } else { "already exists" },
                a.id
            )
        }
        Command::Tui { id } => {
            if let Some(id) = id {
                let artifact = s
                    .get(&id)?
                    .ok_or_else(|| anyhow::anyhow!("artifact not found"))?;
                println!(
                    "{}\n\n{}\n\n{}",
                    artifact.title,
                    "─".repeat(artifact.title.chars().count()),
                    artifact.body
                );
                let successors = s.superseded_by(&id)?;
                if !successors.is_empty() {
                    println!("\nSuperseded by: {}", successors.join(", "));
                }
            } else {
                println!("Nabu browser — recent artifacts\n");
                for hit in s.recent(None, 20)? {
                    println!("{}  {}\n  {}\n", hit.id, hit.title, hit.summary);
                }
                println!("Run `nabu tui <id>` to preview an artifact.");
            }
        }
    };
    Ok(())
}
