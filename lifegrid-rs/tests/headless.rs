//! End-to-end checks for the headless CLI.

use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lifegrid-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lifegrid"))
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
        .expect("run lifegrid");

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
        .expect("run lifegrid");
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
        .expect("run lifegrid");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown mode"), "{stderr}");
}

#[test]
fn lists_all_modes() {
    let output = bin().arg("--list-modes").output().expect("run lifegrid");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 10);
    assert!(stdout.contains("Langton's Ant"));
    assert!(stdout.contains("Custom Rules"));
}

#[test]
fn gif_requires_recorded_frames() {
    let dir = temp_dir("gif");
    let gif = dir.join("out.gif");
    let output = bin().arg("--gif").arg(&gif).output().expect("run lifegrid");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("at least one recorded step"));
    std::fs::remove_dir_all(dir).ok();
}

fn size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}
