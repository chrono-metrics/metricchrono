use std::fs;
use std::path::PathBuf;

use metricchrono_core::{
    ladder_values, ConsensusResultDocument, LadderDocument, TickVectorDocument,
};

#[test]
fn rust_round_trips_ladder_schema_fixture() {
    let text = read_fixture("ladder.v1.json");
    let doc: LadderDocument = serde_json::from_str(&text).unwrap();
    let ladder = doc.clone().into_ladder().unwrap();
    assert_eq!(ladder.len(), 3);
    assert_eq!(
        ladder_values(1.0, ladder.tiers()).unwrap(),
        vec![10.0, 4.0, 2.0]
    );

    let encoded = serde_json::to_string(&doc).unwrap();
    let decoded: LadderDocument = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn rust_round_trips_tick_vector_and_consensus_fixtures() {
    let tick_doc: TickVectorDocument =
        serde_json::from_str(&read_fixture("tick_vector.v1.json")).unwrap();
    assert_eq!(tick_doc.metricchrono_schema, "tick_vector.v1");
    assert_eq!(tick_doc.ticks, vec![10.0, 4.0, 2.0]);

    let consensus_doc: ConsensusResultDocument =
        serde_json::from_str(&read_fixture("consensus_result.v1.json")).unwrap();
    assert_eq!(consensus_doc.metricchrono_schema, "consensus_result.v1");
    assert_eq!(consensus_doc.consensus, vec![2.5, 0.5]);
}

#[test]
fn rust_rejects_unknown_schema_fields() {
    let invalid = r#"{"metricchrono_schema":"tick_vector.v1","ticks":[1],"extra":true}"#;
    assert!(serde_json::from_str::<TickVectorDocument>(invalid).is_err());
}

fn read_fixture(name: &str) -> String {
    fs::read_to_string(repo_root().join("tests/golden").join(name)).unwrap()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}
