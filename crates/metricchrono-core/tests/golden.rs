use std::fs;
use std::path::PathBuf;

use metricchrono_core::{geometric_ladder, ladder_values, tick_distance, Tier};

const EPS: f64 = 1e-12;

#[test]
fn rust_matches_shared_tick_golden_vectors() {
    for row in read_csv("fixtures/golden_ticks.csv").into_iter().skip(1) {
        let tier = Tier::new(
            parse_f64(&row[2]),
            parse_f64(&row[3]),
            parse_f64(&row[4]),
            parse_f64(&row[5]),
        )
        .unwrap();
        let actual = tick_distance(parse_f64(&row[1]), tier);
        let expected = parse_f64(&row[6]);
        assert_close(&row[0], actual, expected);
    }
}

#[test]
fn rust_matches_shared_ladder_golden_vectors() {
    for row in read_csv("fixtures/golden_ladders.csv").into_iter().skip(1) {
        let ladder = geometric_ladder(
            parse_f64(&row[2]),
            parse_f64(&row[3]),
            parse_f64(&row[4]),
            row[5].parse().unwrap(),
            parse_f64(&row[6]),
            parse_f64(&row[7]),
        )
        .unwrap();
        let actual = ladder_values(parse_f64(&row[1]), &ladder).unwrap();
        let expected: Vec<f64> = row[8].split(';').map(parse_f64).collect();
        assert_eq!(actual.len(), expected.len(), "{}", row[0]);
        for (idx, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert_close(&format!("{}[{idx}]", row[0]), *actual, expected);
        }
    }
}

fn read_csv(relative: &str) -> Vec<Vec<String>> {
    let path = repo_root().join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split(',').map(str::to_owned).collect())
        .collect()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn parse_f64(value: &str) -> f64 {
    value.parse::<f64>().unwrap()
}

fn assert_close(name: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= EPS,
        "{name}: expected {expected}, got {actual}"
    );
}
