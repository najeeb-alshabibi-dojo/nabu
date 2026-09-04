use chrono::Utc;
use nabu::{Artifact, ArtifactKind, NabuStore, Scope};
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
