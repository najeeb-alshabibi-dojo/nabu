use anyhow::{Context, Result, bail};
use blake3::Hasher;
use chrono::{DateTime, Utc};
use fs2::FileExt;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
};

const INDEX_DIR: &str = ".index";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Decision,
    Gotcha,
    Environment,
    Orientation,
    Preference,
    External,
}
impl ArtifactKind {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "decision" => Ok(Self::Decision),
            "gotcha" => Ok(Self::Gotcha),
            "environment" => Ok(Self::Environment),
            "orientation" => Ok(Self::Orientation),
            "preference" => Ok(Self::Preference),
            "external" => Ok(Self::External),
            _ => bail!("invalid artifact kind `{value}`"),
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Gotcha => "gotcha",
            Self::Environment => "environment",
            Self::Orientation => "orientation",
            Self::Preference => "preference",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Global,
    Local,
}
impl Scope {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "global" => Ok(Self::Global),
            "local" => Ok(Self::Local),
            _ => bail!("scope must be global or local"),
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Local => "local",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub kind: ArtifactKind,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub scope: Scope,
    pub created_at: DateTime<Utc>,
    pub author: String,
    #[serde(default)]
    pub supersedes: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub path: String,
    pub description: String,
    pub status: String,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub score: f64,
    pub scope: Scope,
}

pub struct NabuStore {
    root: PathBuf,
    machine_id: String,
}
impl NabuStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        let lock = Self::lock_file(&root)?;
        lock.lock_exclusive()?;
        fs::create_dir_all(root.join("artifacts/global"))?;
        fs::create_dir_all(root.join("artifacts/local"))?;
        fs::create_dir_all(root.join("tags"))?;
        fs::create_dir_all(root.join("edges"))?;
        fs::create_dir_all(root.join(INDEX_DIR))?;
        let ignore = root.join(".gitignore");
        if !ignore.exists() {
            fs::write(&ignore, ".index/\n.lock\n.store.lock\n")?;
        } else {
            let contents = fs::read_to_string(&ignore)?;
            if !contents.lines().any(|line| line == ".store.lock") {
                fs::write(
                    &ignore,
                    format!(
                        "{}{}.store.lock\n",
                        contents,
                        if contents.is_empty() || contents.ends_with('\n') {
                            ""
                        } else {
                            "\n"
                        }
                    ),
                )?;
            }
        }
        let config = root.join("nabu.toml");
        if !config.exists() {
            let id = hostname();
            fs::write(&config, format!("machine_id = {:?}\n", id))?;
        }
        let machine_id = read_machine_id(&config)?;
        let store = Self { root, machine_id };
        store.init_git()?;
        store.reindex_unlocked()?;
        lock.unlock()?;
        Ok(store)
    }
    pub fn default_root() -> PathBuf {
        std::env::var_os("NABU_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs_home().join(".agent-memory"))
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    fn lock_file(root: &Path) -> Result<File> {
        Ok(File::options()
            .create(true)
            .read(true)
            .write(true)
            .open(root.join(".store.lock"))?)
    }
    fn shared_lock(&self) -> Result<File> {
        let lock = Self::lock_file(&self.root)?;
        lock.lock_shared()?;
        Ok(lock)
    }
    fn exclusive_lock(&self) -> Result<File> {
        let lock = Self::lock_file(&self.root)?;
        lock.lock_exclusive()?;
        Ok(lock)
    }
    fn db(&self) -> Result<Connection> {
        let db = Connection::open(self.root.join(INDEX_DIR).join("nabu.db"))?;
        db.execute_batch("PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS artifacts(id TEXT PRIMARY KEY, kind TEXT NOT NULL, title TEXT NOT NULL, summary TEXT NOT NULL, tags_json TEXT NOT NULL, scope TEXT NOT NULL, machine_id TEXT, created_at TEXT NOT NULL, author TEXT NOT NULL, path TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS supersessions(old_id TEXT NOT NULL, new_id TEXT NOT NULL, PRIMARY KEY(old_id,new_id));
            CREATE VIRTUAL TABLE IF NOT EXISTS artifact_fts USING fts5(id UNINDEXED,title,summary,body);")?;
        Ok(db)
    }
    pub fn register_tag(&self, path: &str, description: &str) -> Result<Tag> {
        let lock = self.exclusive_lock()?;
        validate_tag(path)?;
        if description.trim().is_empty() {
            bail!("a tag description is required");
        }
        let dest = self.tag_path(path);
        if dest.exists() {
            bail!("tag `{path}` is already registered");
        }
        if let Some(parent) = path.rsplit_once("::").map(|(p, _)| p) {
            if !self.tag_path(parent).exists() {
                bail!("parent tag `{parent}` is not registered");
            }
        }
        fs::create_dir_all(dest.parent().context("tag has no parent directory")?)?;
        let tag = Tag {
            path: path.into(),
            description: description.trim().into(),
            status: "active".into(),
            registered_at: Utc::now(),
        };
        fs::write(&dest, tag_markdown(&tag))?;
        self.git_commit(&format!("nabu: register tag {path}"))?;
        lock.unlock()?;
        Ok(tag)
    }
    pub fn deprecate_tag(&self, path: &str) -> Result<Tag> {
        let lock = self.exclusive_lock()?;
        let file = self.tag_path(path);
        let mut tag = parse_tag(
            &fs::read_to_string(&file)
                .with_context(|| format!("tag `{path}` is not registered"))?,
        )?;
        tag.status = "deprecated".into();
        fs::write(file, tag_markdown(&tag))?;
        self.git_commit(&format!("nabu: deprecate tag {path}"))?;
        lock.unlock()?;
        Ok(tag)
    }
    pub fn tags(&self, root: Option<&str>, include_deprecated: bool) -> Result<Vec<Tag>> {
        let lock = self.shared_lock()?;
        let mut out = Vec::new();
        self.walk_tags(&self.root.join("tags"), &mut out)?;
        out.retain(|t| {
            root.is_none_or(|r| t.path == r || t.path.starts_with(&format!("{r}::")))
                && (include_deprecated || t.status == "active")
        });
        out.sort_by(|a, b| a.path.cmp(&b.path));
        lock.unlock()?;
        Ok(out)
    }
    pub fn publish(&self, mut artifact: Artifact) -> Result<(Artifact, bool)> {
        let lock = self.exclusive_lock()?;
        if artifact.title.trim().is_empty() || artifact.summary.trim().is_empty() {
            bail!("title and summary are required");
        }
        if artifact.summary.chars().count() > 160 {
            bail!("summary must be 160 characters or less");
        }
        if artifact.tags.is_empty() {
            bail!("at least one tag is required");
        }
        if looks_like_secret(&artifact.title)
            || looks_like_secret(&artifact.summary)
            || looks_like_secret(&artifact.body)
        {
            bail!("write rejected: content appears to contain a secret");
        }
        artifact.tags.sort();
        artifact.tags.dedup();
        for tag in &artifact.tags {
            let tag_record = parse_tag(
                &fs::read_to_string(self.tag_path(tag))
                    .with_context(|| format!("tag `{tag}` is not registered"))?,
            )?;
            if tag_record.status != "active" {
                bail!("tag `{tag}` is deprecated");
            }
        }
        for id in &artifact.supersedes {
            if self.get_unlocked(id)?.is_none() {
                bail!("superseded artifact `{id}` does not exist");
            }
        }
        artifact.id = content_id(&artifact);
        artifact.created_at = Utc::now();
        if artifact.author.trim().is_empty() {
            artifact.author = "unknown".into();
        }
        let dest = self.artifact_path(&artifact);
        if dest.exists() {
            return Ok((parse_artifact(&fs::read_to_string(dest)?)?, false));
        }
        fs::create_dir_all(dest.parent().unwrap())?;
        fs::write(&dest, artifact_markdown(&artifact))?;
        if !artifact.supersedes.is_empty() {
            let edges = self.root.join("edges/supersessions.jsonl");
            let mut data = String::new();
            for old_id in &artifact.supersedes {
                data.push_str(
                    &serde_json::json!({"old_id":old_id,"new_id":artifact.id}).to_string(),
                );
                data.push('\n');
            }
            use std::io::Write;
            File::options()
                .create(true)
                .append(true)
                .open(edges)?
                .write_all(data.as_bytes())?;
        }
        self.reindex_unlocked()?;
        self.git_commit(&format!("nabu: publish {} {}", artifact.id, artifact.title))?;
        lock.unlock()?;
        Ok((artifact, true))
    }
    pub fn get(&self, id: &str) -> Result<Option<Artifact>> {
        let lock = self.shared_lock()?;
        let artifact = self.get_unlocked(id)?;
        lock.unlock()?;
        Ok(artifact)
    }
    fn get_unlocked(&self, id: &str) -> Result<Option<Artifact>> {
        let db = self.db()?;
        let path: Option<String> = db
            .query_row("SELECT path FROM artifacts WHERE id=?1", [id], |r| r.get(0))
            .ok();
        match path {
            Some(p) => Ok(Some(parse_artifact(&fs::read_to_string(p)?)?)),
            None => Ok(None),
        }
    }
    pub fn superseded_by(&self, id: &str) -> Result<Vec<String>> {
        let lock = self.shared_lock()?;
        let superseded_by = self.superseded_by_unlocked(id)?;
        lock.unlock()?;
        Ok(superseded_by)
    }
    fn superseded_by_unlocked(&self, id: &str) -> Result<Vec<String>> {
        let db = self.db()?;
        let mut s =
            db.prepare("SELECT new_id FROM supersessions WHERE old_id=? ORDER BY new_id")?;
        Ok(s.query_map([id], |r| r.get(0))?
            .collect::<Result<Vec<String>, _>>()?)
    }
    pub fn search(
        &self,
        query: &str,
        tag: Option<&str>,
        kind: Option<&ArtifactKind>,
        limit: usize,
        include_superseded: bool,
    ) -> Result<Vec<SearchHit>> {
        let lock = self.shared_lock()?;
        let db = self.db()?;
        let mut sql = String::from(
            "SELECT a.id,a.title,a.summary,a.tags_json,a.scope,bm25(artifact_fts) FROM artifact_fts JOIN artifacts a USING(id) WHERE artifact_fts MATCH ?",
        );
        if !include_superseded {
            sql.push_str(" AND NOT EXISTS (SELECT 1 FROM supersessions s WHERE s.old_id=a.id)");
        }
        if tag.is_some() {
            sql.push_str(" AND a.id IN (SELECT id FROM artifacts WHERE tags_json LIKE ?)");
        }
        if kind.is_some() {
            sql.push_str(" AND a.kind=?");
        }
        sql.push_str(" ORDER BY bm25(artifact_fts) LIMIT ?");
        let fts = fts_query(query);
        let mut stmt = db.prepare(&sql)?;
        let mut args: Vec<&dyn rusqlite::ToSql> = vec![&fts];
        let tagpat = tag.map(|v| format!("%\"{}%", v));
        if let Some(v) = &tagpat {
            args.push(v);
        }
        let ks = kind.map(|k| k.as_str().to_owned());
        if let Some(v) = &ks {
            args.push(v);
        }
        let lim = limit as i64;
        args.push(&lim);
        let rows = stmt.query_map(args.as_slice(), |r| {
            Ok(SearchHit {
                id: r.get(0)?,
                title: r.get(1)?,
                summary: r.get(2)?,
                tags: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or_default(),
                scope: Scope::parse(&r.get::<_, String>(4)?).unwrap(),
                score: -r.get::<_, f64>(5)?,
            })
        })?;
        let hits = rows.collect::<Result<Vec<_>, _>>()?;
        lock.unlock()?;
        Ok(hits)
    }
    pub fn recent(&self, tag: Option<&str>, limit: usize) -> Result<Vec<SearchHit>> {
        let lock = self.shared_lock()?;
        let db = self.db()?;
        let mut sql = String::from(
            "SELECT a.id,a.title,a.summary,a.tags_json,a.scope
             FROM artifacts a
             WHERE NOT EXISTS (SELECT 1 FROM supersessions s WHERE s.old_id=a.id)",
        );
        if tag.is_some() {
            sql.push_str(" AND a.tags_json LIKE ?");
        }
        sql.push_str(" ORDER BY a.created_at DESC LIMIT ?");
        let mut stmt = db.prepare(&sql)?;
        let limit = limit as i64;
        let mut args: Vec<&dyn rusqlite::ToSql> = Vec::new();
        let tag_pattern = tag.map(|value| format!("%\"{}%", value));
        if let Some(pattern) = &tag_pattern {
            args.push(pattern);
        }
        args.push(&limit);
        let rows = stmt.query_map(args.as_slice(), |r| {
            Ok(SearchHit {
                id: r.get(0)?,
                title: r.get(1)?,
                summary: r.get(2)?,
                tags: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or_default(),
                scope: Scope::parse(&r.get::<_, String>(4)?).unwrap(),
                score: 0.0,
            })
        })?;
        let hits = rows.collect::<Result<Vec<_>, _>>()?;
        lock.unlock()?;
        Ok(hits)
    }
    pub fn reindex(&self) -> Result<()> {
        let lock = self.exclusive_lock()?;
        self.reindex_unlocked()?;
        lock.unlock()?;
        Ok(())
    }
    fn reindex_unlocked(&self) -> Result<()> {
        let db_path = self.root.join(INDEX_DIR).join("nabu.db");
        if db_path.exists() {
            fs::remove_file(&db_path)?;
        }
        let db = self.db()?;
        let mut artifacts = Vec::new();
        self.walk_artifacts(&self.root.join("artifacts"), &mut artifacts)?;
        let tx = db.unchecked_transaction()?;
        for (artifact, path) in artifacts {
            tx.execute(
                "INSERT INTO artifacts VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    artifact.id,
                    artifact.kind.as_str(),
                    artifact.title,
                    artifact.summary,
                    serde_json::to_string(&artifact.tags)?,
                    artifact.scope.as_str(),
                    if artifact.scope == Scope::Local {
                        Some(&self.machine_id)
                    } else {
                        None
                    },
                    artifact.created_at.to_rfc3339(),
                    artifact.author,
                    path.to_string_lossy()
                ],
            )?;
            tx.execute(
                "INSERT INTO artifact_fts VALUES(?1,?2,?3,?4)",
                params![artifact.id, artifact.title, artifact.summary, artifact.body],
            )?;
            for old in artifact.supersedes {
                tx.execute(
                    "INSERT OR IGNORE INTO supersessions VALUES(?1,?2)",
                    params![old, artifact.id],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    fn artifact_path(&self, a: &Artifact) -> PathBuf {
        let shard = &a.id[3..5];
        match a.scope {
            Scope::Global => self
                .root
                .join("artifacts/global")
                .join(shard)
                .join(format!("{}.md", a.id)),
            Scope::Local => self
                .root
                .join("artifacts/local")
                .join(&self.machine_id)
                .join(shard)
                .join(format!("{}.md", a.id)),
        }
    }
    fn tag_path(&self, path: &str) -> PathBuf {
        self.root
            .join("tags")
            .join(path.replace("::", "/"))
            .join("tag.md")
    }
    fn walk_tags(&self, dir: &Path, out: &mut Vec<Tag>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        for e in fs::read_dir(dir)? {
            let e = e?;
            if e.file_type()?.is_dir() {
                self.walk_tags(&e.path(), out)?
            } else if e.file_name() == "tag.md" {
                out.push(parse_tag(&fs::read_to_string(e.path())?)?)
            }
        }
        Ok(())
    }
    fn walk_artifacts(&self, dir: &Path, out: &mut Vec<(Artifact, PathBuf)>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        for e in fs::read_dir(dir)? {
            let e = e?;
            if e.file_type()?.is_dir() {
                self.walk_artifacts(&e.path(), out)?
            } else if e.path().extension().is_some_and(|x| x == "md") {
                let p = e.path();
                out.push((parse_artifact(&fs::read_to_string(&p)?)?, p));
            }
        }
        Ok(())
    }
    fn init_git(&self) -> Result<()> {
        if !self.root.join(".git").exists() {
            git2::Repository::init(&self.root)?;
        }
        Ok(())
    }
    fn git_commit(&self, message: &str) -> Result<()> {
        let lock = File::create(self.root.join(".lock"))?;
        lock.lock_exclusive()?;
        let result = (|| {
            let repo = git2::Repository::open(&self.root)?;
            let mut index = repo.index()?;
            index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
            index.write()?;
            let tree = repo.find_tree(index.write_tree()?)?;
            let sig = git2::Signature::now("nabu", "nabu@localhost")?;
            let parent = repo
                .head()
                .ok()
                .and_then(|h| h.target())
                .and_then(|oid| repo.find_commit(oid).ok());
            if parent.as_ref().is_some_and(|p| p.tree_id() == tree.id()) {
                return Ok(());
            }
            match parent {
                Some(p) => {
                    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&p])?;
                }
                None => {
                    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?;
                }
            }
            Ok(())
        })();
        lock.unlock()?;
        result
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .unwrap_or_else(|_| "local".into())
        .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-")
}
fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
fn read_machine_id(p: &Path) -> Result<String> {
    let s = fs::read_to_string(p)?;
    Ok(s.lines()
        .find_map(|l| l.strip_prefix("machine_id = "))
        .map(|v| v.trim_matches('"').to_owned())
        .unwrap_or_else(hostname))
}
fn validate_tag(s: &str) -> Result<()> {
    if s.is_empty()
        || s.split("::").any(|p| {
            p.is_empty()
                || !p
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        })
    {
        bail!("tag paths use lowercase segments joined by `::`")
    }
    Ok(())
}
fn content_id(a: &Artifact) -> String {
    let mut h = Hasher::new();
    h.update(a.kind.as_str().as_bytes());
    h.update(&[0]);
    h.update(a.title.as_bytes());
    h.update(&[0]);
    h.update(a.body.as_bytes());
    h.update(&[0]);
    h.update(a.tags.join("\0").as_bytes());
    h.update(&[0]);
    h.update(a.scope.as_str().as_bytes());
    format!("nb_{}", &h.finalize().to_hex()[..16])
}
fn quote(v: &str) -> String {
    serde_json::to_string(v).unwrap()
}
fn artifact_markdown(a: &Artifact) -> String {
    format!(
        "---\nid: {}\nkind: {}\ntitle: {}\nsummary: {}\ntags: {}\nscope: {}\ncreated_at: {}\nauthor: {}\nsupersedes: {}\nsources: {}\n---\n\n{}",
        a.id,
        a.kind.as_str(),
        quote(&a.title),
        quote(&a.summary),
        serde_json::to_string(&a.tags).unwrap(),
        a.scope.as_str(),
        a.created_at.to_rfc3339(),
        quote(&a.author),
        serde_json::to_string(&a.supersedes).unwrap(),
        serde_json::to_string(&a.sources).unwrap(),
        a.body
    )
}
fn tag_markdown(t: &Tag) -> String {
    format!(
        "---\npath: {}\ndescription: {}\nstatus: {}\nregistered_at: {}\n---\n",
        t.path,
        quote(&t.description),
        t.status,
        t.registered_at.to_rfc3339()
    )
}
fn frontmatter(s: &str) -> Result<(HashMap<String, String>, String)> {
    let r = s.strip_prefix("---\n").context("missing frontmatter")?;
    let (h, b) = r
        .split_once("\n---\n")
        .context("unterminated frontmatter")?;
    let mut m = HashMap::new();
    for l in h.lines() {
        let (k, v) = l.split_once(": ").context("invalid frontmatter")?;
        m.insert(k.into(), v.into());
    }
    Ok((m, b.trim_start().into()))
}
fn parse_artifact(s: &str) -> Result<Artifact> {
    let (m, body) = frontmatter(s)?;
    Ok(Artifact {
        id: m["id"].clone(),
        kind: ArtifactKind::parse(&m["kind"])?,
        title: serde_json::from_str(&m["title"])?,
        summary: serde_json::from_str(&m["summary"])?,
        tags: serde_json::from_str(&m["tags"])?,
        scope: Scope::parse(&m["scope"])?,
        created_at: m["created_at"].parse()?,
        author: serde_json::from_str(&m["author"])?,
        supersedes: serde_json::from_str(m.get("supersedes").map(String::as_str).unwrap_or("[]"))?,
        sources: serde_json::from_str(m.get("sources").map(String::as_str).unwrap_or("[]"))?,
        body,
    })
}
fn parse_tag(s: &str) -> Result<Tag> {
    let (m, _) = frontmatter(s)?;
    Ok(Tag {
        path: m["path"].clone(),
        description: serde_json::from_str(&m["description"])?,
        status: m["status"].clone(),
        registered_at: m["registered_at"].parse()?,
    })
}
fn fts_query(q: &str) -> String {
    if q.trim() == "*" {
        "*".into()
    } else {
        q.split_whitespace()
            .map(|w| format!("\"{}\"", w.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" OR ")
    }
}
fn looks_like_secret(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    [
        "-----begin private key-----",
        "ghp_",
        "github_pat_",
        "atatt",
        "authorization: bearer",
        "aws_secret_access_key",
    ]
    .iter()
    .any(|x| lower.contains(x))
        || lower.lines().any(|l| {
            (l.contains("token") || l.contains("password") || l.contains("api_key"))
                && l.contains('=')
                && l.split('=').nth(1).is_some_and(|v| v.trim().len() > 12)
        })
}
