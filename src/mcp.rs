use crate::{Artifact, ArtifactKind, NabuStore, Scope};
use anyhow::Result;
use chrono::Utc;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

pub fn serve(store: NabuStore) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let message: Value = match serde_json::from_str(&line?) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(id) = message.get("id").cloned() else {
            continue;
        };
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let response = match method {
            "initialize" => ok(
                id,
                json!({"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"nabu","version":env!("CARGO_PKG_VERSION")}}),
            ),
            "tools/list" => ok(id, json!({"tools": tools()})),
            "tools/call" => match call(&store, message.get("params").unwrap_or(&Value::Null)) {
                Ok(result) => ok(
                    id,
                    json!({"content":[{"type":"text","text":serde_json::to_string(&result)?}]}),
                ),
                Err(e) => ok(
                    id,
                    json!({"content":[{"type":"text","text":format!("nabu error: {e}")}],"isError":true}),
                ),
            },
            _ => error(id, -32601, "Method not found"),
        };
        writeln!(stdout, "{}", response)?;
        stdout.flush()?;
    }
    Ok(())
}
fn ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":result})
}
fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
}
fn tools() -> Vec<Value> {
    vec![
        tool(
            "nabu_tag_tree",
            "Browse registered tags cheaply before searching.",
            json!({"type":"object","properties":{"root":{"type":"string"},"depth":{"type":"integer","default":3}}}),
        ),
        tool(
            "nabu_search",
            "Search active artifacts. Results are compact; call nabu_get for bodies.",
            json!({"type":"object","required":["query"],"properties":{"query":{"type":"string"},"tags":{"type":"string"},"kind":{"type":"string","enum":["decision","gotcha","environment","orientation","preference","external"]},"limit":{"type":"integer","default":8},"include_superseded":{"type":"boolean","default":false}}}),
        ),
        tool(
            "nabu_get",
            "Fetch full artifacts by ID, including derived superseded_by ids.",
            json!({"type":"object","required":["ids"],"properties":{"ids":{"type":"array","items":{"type":"string"}}}}),
        ),
        tool(
            "nabu_publish",
            "Publish an immutable artifact. Never include credentials or secrets.",
            json!({"type":"object","required":["kind","title","summary","body","tags","scope"],"properties":{"kind":{"type":"string"},"title":{"type":"string"},"summary":{"type":"string"},"body":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}},"scope":{"type":"string","enum":["global","local"]},"supersedes":{"type":"array","items":{"type":"string"}},"sources":{"type":"array","items":{"type":"string"}},"author":{"type":"string"}}}),
        ),
        tool(
            "nabu_register_tag",
            "Register a tag; its parent tag must already be registered.",
            json!({"type":"object","required":["path","description"],"properties":{"path":{"type":"string"},"description":{"type":"string"}}}),
        ),
        tool(
            "nabu_supersede",
            "Publish a new artifact that supersedes old artifacts.",
            json!({"type":"object","required":["artifact"],"properties":{"artifact":{"type":"object"},"old_ids":{"type":"array","items":{"type":"string"}}}}),
        ),
        tool(
            "nabu_related",
            "Return supersession successors for an artifact.",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"}}}),
        ),
        tool(
            "nabu_recent",
            "List recent matching artifacts.",
            json!({"type":"object","properties":{"tags":{"type":"string"},"limit":{"type":"integer","default":8}}}),
        ),
    ]
}
fn call(s: &NabuStore, p: &Value) -> Result<Value> {
    let name = p["name"].as_str().unwrap_or("");
    let a = &p["arguments"];
    match name {
        "nabu_tag_tree" => Ok(json!(s.tags(a["root"].as_str(), false)?)),
        "nabu_search" => {
            let kind = a["kind"].as_str().map(ArtifactKind::parse).transpose()?;
            let limit = a["limit"].as_u64().unwrap_or(8) as usize;
            let tag = a["tags"].as_str();
            let hits = s.search(
                a["query"].as_str().unwrap_or(""),
                tag,
                kind.as_ref(),
                limit,
                a["include_superseded"].as_bool().unwrap_or(false),
            )?;
            let mut v = json!(hits);
            if tag.is_none() {
                v["nudge"] = json!(
                    "Search without a tag returned a reduced set. Use nabu_tag_tree then retry with tags."
                );
            }
            Ok(v)
        }
        "nabu_get" => {
            let mut output = Vec::new();
            for id in a["ids"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if let Some(artifact) = s.get(id)? {
                    output.push(json!({"artifact":artifact,"superseded_by":s.superseded_by(id)?}));
                }
            }
            Ok(json!(output))
        }
        "nabu_register_tag" => Ok(json!(s.register_tag(
            a["path"].as_str().unwrap_or(""),
            a["description"].as_str().unwrap_or("")
        )?)),
        "nabu_related" => Ok(
            json!({"id":a["id"],"superseded_by":s.superseded_by(a["id"].as_str().unwrap_or(""))?}),
        ),
        "nabu_recent" => Ok(json!(s.recent(
            a["tags"].as_str(),
            a["limit"].as_u64().unwrap_or(8) as usize
        )?)),
        "nabu_publish" => publish(s, a, Vec::new()),
        "nabu_supersede" => publish(
            s,
            &a["artifact"],
            a["old_ids"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        ),
        _ => anyhow::bail!("unknown tool `{name}`"),
    }
}
fn publish(s: &NabuStore, a: &Value, extra_supersedes: Vec<String>) -> Result<Value> {
    let mut supersedes: Vec<String> = a["supersedes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    supersedes.extend(extra_supersedes);
    let artifact = Artifact {
        id: String::new(),
        kind: ArtifactKind::parse(a["kind"].as_str().unwrap_or(""))?,
        title: a["title"].as_str().unwrap_or("").into(),
        summary: a["summary"].as_str().unwrap_or("").into(),
        body: a["body"].as_str().unwrap_or("").into(),
        tags: a["tags"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        scope: Scope::parse(a["scope"].as_str().unwrap_or("global"))?,
        created_at: Utc::now(),
        author: a["author"].as_str().unwrap_or("agent").into(),
        supersedes,
        sources: a["sources"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
    };
    let (artifact, created) = s.publish(artifact)?;
    Ok(json!({"artifact":artifact,"created":created}))
}
