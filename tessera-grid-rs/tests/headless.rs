//! End-to-end checks for the headless CLI.

use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tessera-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tessera"))
}

#[test]
fn exports_png_gif_and_rle() {
    let dir = temp_dir("exports");
    let png = dir.join("out.png");
    let gif = dir.join("out.gif");
    let rle = dir.join("out.rle");

    let status = bin()
        .arg("--headless")
        .args(["--mode", "High Life", "--pattern", "Replicator"])
        .args(["--width", "40", "--height", "30", "--steps", "6"])
        .arg("--png")
        .arg(&png)
        .arg("--gif")
        .arg(&gif)
        .arg("--rle")
        .arg(&rle)
        .status()
        .expect("run tessera");

    assert!(status.success(), "headless run failed");
    assert!(size(&png) > 0, "PNG should not be empty");
    assert!(size(&gif) > 0, "GIF should not be empty");
    let encoded = std::fs::read_to_string(&rle).expect("read rle");
    assert!(encoded.starts_with("x = "), "RLE header missing: {encoded}");
    assert!(
        encoded.contains("rule = B36/S23"),
        "RLE should use the High Life rule: {encoded}"
    );

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn custom_rule_and_stats_work() {
    let output = bin()
        .args(["--rule", "B36/S23", "--steps", "3", "--stats"])
        .output()
        .expect("run tessera");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mode: Custom Rules"), "{stdout}");
    assert!(stdout.contains("rule: B36/S23"), "{stdout}");
    assert!(stdout.contains("generation: 3"), "{stdout}");
}

#[test]
fn rejects_unknown_mode() {
    let output = bin()
        .args(["--mode", "Not A Mode", "--stats"])
        .output()
        .expect("run tessera");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown mode"), "{stderr}");
}

#[test]
fn lists_all_modes() {
    let output = bin().arg("--list-modes").output().expect("run tessera");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 11);
    assert!(stdout.contains("Elementary 1D"));
    assert!(stdout.contains("Langton's Ant"));
    assert!(stdout.contains("Custom Rules"));
}

#[test]
fn gif_requires_recorded_frames() {
    let dir = temp_dir("gif");
    let gif = dir.join("out.gif");
    let output = bin().arg("--gif").arg(&gif).output().expect("run tessera");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("at least one recorded step"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn elementary_rules_run_headless() {
    let output = bin()
        .args(["--rule", "W110", "--width", "40", "--height", "20"])
        .args(["--steps", "19", "--analyze"])
        .output()
        .expect("run tessera");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mode: Elementary 1D"), "{stdout}");
    assert!(stdout.contains("rule: W110"), "{stdout}");
    assert!(stdout.contains("entropy:"), "{stdout}");
}

#[test]
fn out_of_range_wolfram_rules_are_rejected() {
    let output = bin()
        .args(["--rule", "W300", "--stats"])
        .output()
        .expect("run tessera");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("out of range"), "{stderr}");
}

#[test]
fn analysis_csv_and_cycle_detection_work() {
    let dir = temp_dir("analysis");
    let csv = dir.join("metrics.csv");

    let output = bin()
        .args(["--pattern", "Blinker", "--steps", "6", "--analyze"])
        .arg("--analysis-csv")
        .arg(&csv)
        .output()
        .expect("run tessera");
    assert!(output.status.success());

    let text = std::fs::read_to_string(&csv).expect("read csv");
    assert!(
        text.starts_with("generation,population,density,entropy"),
        "{text}"
    );
    assert_eq!(text.lines().count(), 7, "one header plus six generations");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cycle: period 2"), "{stdout}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn divergence_reports_a_growth_rate() {
    let output = bin()
        .args(["--pattern", "Random Soup", "--steps", "0"])
        .args(["--divergence", "30", "--seed", "1"])
        .output()
        .expect("run tessera");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# divergence:"), "{stdout}");
    assert!(stdout.contains("# growth rate:"), "{stdout}");
}

#[test]
fn the_elementary_sweep_covers_every_rule() {
    let dir = temp_dir("sweep");
    let csv = dir.join("sweep.csv");

    let output = bin()
        .args(["--sweep-elementary", "--width", "64", "--height", "48"])
        .args(["--steps", "40", "--analysis-csv"])
        .arg(&csv)
        .output()
        .expect("run tessera");
    assert!(output.status.success());

    let text = std::fs::read_to_string(&csv).expect("read sweep");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 257, "header plus 256 rules");
    assert!(lines[0].starts_with("rule,"), "{}", lines[0]);
    assert!(lines[1].starts_with("0,"), "rule 0 comes first");
    assert!(lines[256].starts_with("255,"), "rule 255 comes last");

    // Rule 0 clears the grid and is a fixed point from the first generation.
    let rule_zero: Vec<&str> = lines[1].split(',').collect();
    assert_eq!(rule_zero[6], "1", "rule 0 reaches a period-1 cycle");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_closed_stdout_does_not_panic() {
    use std::process::Stdio;

    let mut child = bin()
        .args(["--sweep-elementary", "--width", "96", "--height", "72"])
        .args(["--steps", "60"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tessera");
    // Close the read end so the child's writes fail with EPIPE.
    drop(child.stdout.take());
    let output = child.wait_with_output().expect("wait for tessera");
    assert!(
        output.status.success(),
        "a closed pipe must not abort the CLI: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}
