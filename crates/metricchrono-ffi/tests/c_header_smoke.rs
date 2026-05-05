use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn c_header_smoke_compiles() {
    let temp = env::temp_dir().join(format!("metricchrono-c-smoke-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    let source = temp.join("smoke.c");
    let object = temp.join("smoke.o");
    fs::write(
        &source,
        r#"
#include "metricchrono.h"

int main(void) {
  MCTier tier;
  MCLadder *ladder = 0;
  double out[2] = {0.0, 0.0};
  size_t len = 0;
  if (mc_error_message(MC_STATUS_OK) == 0) return 1;
  if (mc_tier_new(0.5, 1.0, 0.0, 1.0, &tier) != MC_STATUS_OK) return 2;
  if (mc_ladder_new(&tier, 1, &ladder) != MC_STATUS_OK) return 3;
  if (mc_ladder_len(ladder, &len) != MC_STATUS_OK) return 4;
  if (mc_ladder_distance_owned(ladder, 1.2, out, 2) != MC_STATUS_OK) return 5;
  mc_ladder_free(ladder);
  return 0;
}
"#,
    )
    .unwrap();
    let status = Command::new("cc")
        .arg("-I")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("include"))
        .arg("-c")
        .arg(&source)
        .arg("-o")
        .arg(&object)
        .status()
        .expect("failed to run cc");
    assert!(status.success());
}
