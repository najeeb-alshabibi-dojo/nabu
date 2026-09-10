use chrono::Utc;
use nabu::{Artifact, ArtifactKind, NabuStore, Scope};
use std::{fs, sync::Arc, thread};
use tempfile::tempdir;

fn artifact(title: &str, body: &str, supersedes: Vec<String>) -> Artifact {
    Artifact {
        id: String::new(),
        kind: ArtifactKind::Gotcha,
        title: title.into(),
        summary: "A concise durable fact".into(),
        tags: vec!["system::java".into()],
        scope: Scope::Global,
        created_at: Utc::now(),
        author: "test".into(),
        supersedes,
        sources: vec![],
        body: body.into(),
    }
}
#[test]
fn publish_is_immutable_deduplicated_and_supersedable() {
    let d = tempdir().unwrap();
    let s = NabuStore::open(d.path()).unwrap();
    s.register_tag("system", "machine tools").unwrap();
    s.register_tag("system::java", "Java tooling").unwrap();
    let (a, created) = s
        .publish(artifact("Java problem", "JDK 25 breaks Lombok.", vec![]))
        .unwrap();
    assert!(created);
    let (again, created) = s
        .publish(artifact("Java problem", "JDK 25 breaks Lombok.", vec![]))
        .unwrap();
    assert!(!created);
    assert_eq!(a.id, again.id);
    let (b, _) = s
        .publish(artifact(
            "Java fixed",
            "Use JDK 21 instead.",
            vec![a.id.clone()],
        ))
        .unwrap();
    assert_eq!(s.superseded_by(&a.id).unwrap(), vec![b.id]);
    assert!(
        s.search("Lombok", Some("system"), None, 8, false)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        s.search("Lombok", Some("system"), None, 8, true)
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn parents_and_secrets_are_enforced() {
    let d = tempdir().unwrap();
    let s = NabuStore::open(d.path()).unwrap();
    assert!(s.register_tag("system::java", "Java").is_err());
    s.register_tag("system", "machine tools").unwrap();
    s.register_tag("system::java", "Java").unwrap();
    let mut a = artifact("safe", "ghp_abcdefghijklmnopqrstuvwxyz123456", vec![]);
    assert!(s.publish(a.clone()).is_err());
    a.body = "safe".into();
    assert!(s.publish(a).is_ok());
}

#[test]
fn get_resolves_unambiguous_id_prefixes() {
    let d = tempdir().unwrap();
    let s = NabuStore::open(d.path()).unwrap();
    s.register_tag("system", "machine tools").unwrap();
    s.register_tag("system::java", "Java tooling").unwrap();

    let (artifact, _) = s
        .publish(artifact("Java problem", "JDK 25 breaks Lombok.", vec![]))
        .unwrap();
    let prefix = &artifact.id[..6];

    assert_eq!(s.get(&artifact.id).unwrap().unwrap().id, artifact.id);
    assert_eq!(s.get(prefix).unwrap().unwrap().id, artifact.id);
    assert!(s.get("").unwrap().is_none());
}

#[test]
fn get_rejects_ambiguous_id_prefixes_and_canonicalizes_supersedes() {
    let d = tempdir().unwrap();
    let s = NabuStore::open(d.path()).unwrap();
    s.register_tag("system", "machine tools").unwrap();
    s.register_tag("system::java", "Java tooling").unwrap();

    let (first, _) = s.publish(artifact("First", "First body.", vec![])).unwrap();
    let (second, _) = s.publish(artifact("Second", "Second body.", vec![])).unwrap();

    assert!(s.get("nb_").unwrap().is_none());

    let prefix = &first.id[..6];
    let (replacement, _) = s
        .publish(artifact("Replacement", "Replacement body.", vec![prefix.into()]))
        .unwrap();

    assert_eq!(replacement.supersedes, vec![first.id.clone()]);
    assert_eq!(s.superseded_by(prefix).unwrap(), vec![replacement.id]);
    assert!(s.get(&second.id).unwrap().is_some());
}

#[test]
fn concurrent_store_processes_can_open_and_publish() {
    let d = tempdir().unwrap();
    let root = Arc::new(d.path().to_path_buf());
    NabuStore::open(&*root).unwrap();
    fs::write(root.join(".gitignore"), ".index/\n.lock\n").unwrap();
    let store = NabuStore::open(&*root).unwrap();
    assert!(
        fs::read_to_string(root.join(".gitignore"))
            .unwrap()
            .lines()
            .any(|line| line == ".store.lock")
    );
    store.register_tag("system", "machine tools").unwrap();
    store.register_tag("system::java", "Java tooling").unwrap();

    let workers: Vec<_> = (0..10)
        .map(|index| {
            let root = Arc::clone(&root);
            thread::spawn(move || {
                let store = NabuStore::open(&*root).unwrap();
                store
                    .publish(artifact(
                        &format!("Concurrent artifact {index}"),
                        &format!("durable concurrent memory {index}"),
                        vec![],
                    ))
                    .unwrap()
            })
        })
        .collect();

    for worker in workers {
        assert!(worker.join().unwrap().1);
    }
    assert_eq!(
        NabuStore::open(&*root)
            .unwrap()
            .recent(None, 20)
            .unwrap()
            .len(),
        10
    );
}
