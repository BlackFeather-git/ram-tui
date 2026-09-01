//! Integration test: run `ram --once --json` and validate JSON schema.

use std::process::Command;

#[test]
fn test_json_snapshot_schema() {
    // Build the binary first
    let status = Command::new("cargo")
        .args(["build", "-p", "cli"])
        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/..")
        .status()
        .expect("failed to build cli");
    assert!(status.success(), "cargo build failed");

    let output = Command::new("cargo")
        .args(["run", "-p", "cli", "--", "--once", "--json"])
        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/..")
        .output()
        .expect("failed to run cli --once --json");

    assert!(
        output.status.success(),
        "cli exited with error: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("output is not valid JSON");

    // Validate top-level fields
    assert!(json.get("timestamp").is_some(), "missing 'timestamp'");
    assert!(json.get("hostname").is_some(), "missing 'hostname'");
    assert_eq!(json["os"], "Linux");
    assert!(json.get("version").is_some(), "missing 'version'");

    // Validate memory object
    let mem = &json["memory"];
    assert!(mem.get("total").is_some(), "missing memory.total");
    assert!(mem.get("available").is_some(), "missing memory.available");
    assert!(mem.get("used").is_some(), "missing memory.used");
    assert!(mem.get("swap_used").is_some(), "missing memory.swap_used");
    assert!(mem.get("swap_total").is_some(), "missing memory.swap_total");
    assert!(mem.get("swap_desc").is_some(), "missing memory.swap_desc");
    assert!(mem.get("valid").is_some(), "missing memory.valid");

    // Validate top_processes array
    let procs = json["top_processes"]
        .as_array()
        .expect("top_processes is not array");
    for proc in procs {
        assert!(proc.get("name").is_some(), "process missing 'name'");
        assert!(proc.get("rss").is_some(), "process missing 'rss'");
        assert!(proc.get("count").is_some(), "process missing 'count'");
        // pid can be null
        assert!(proc.get("pid").is_some(), "process missing 'pid'");
    }
}

#[test]
fn test_once_mode_exits_cleanly() {
    let output = Command::new("cargo")
        .args(["run", "-p", "cli", "--", "--once"])
        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/..")
        .output()
        .expect("failed to run cli --once");

    assert!(
        output.status.success(),
        "cli --once exited with error: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RAM"), "output should contain 'RAM'");
}

#[test]
fn test_help_flag() {
    let output = Command::new("cargo")
        .args(["run", "-p", "cli", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/..")
        .output()
        .expect("failed to run cli --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ram-tui") || stdout.contains("ram"));
    assert!(stdout.contains("--rate"));
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--once"));
    assert!(stdout.contains("--sort"));
}

#[test]
fn test_sort_flag_integration() {
    for sort_mode in ["rss", "pss", "uss", "name"] {
        let output = Command::new("cargo")
            .args(["run", "-p", "cli", "--", "--once", "--sort", sort_mode])
            .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/..")
            .output()
            .expect("failed to run cli with --sort");

        assert!(
            output.status.success(),
            "failed on sort mode {}: {}",
            sort_mode,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_spark_and_debug_flags() {
    let output = Command::new("cargo")
        .args(["run", "-p", "cli", "--", "--once", "--spark", "--debug"])
        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/..")
        .output()
        .expect("failed to run cli with --spark and --debug");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("TREND (60s)"),
        "should contain sparkline trend header when --spark is passed"
    );
}

#[test]
fn test_zero_emoji_invariant() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let src_dirs = [
        "cli/src",
        "collector_linux/src",
        "core_render/src",
        "ui/src",
    ];

    fn is_emoji(ch: char) -> bool {
        matches!(ch,
            '\u{1F600}'..='\u{1F64F}' | // Emoticons
            '\u{1F300}'..='\u{1F5FF}' | // Misc Symbols and Pictographs
            '\u{1F680}'..='\u{1F6FF}' | // Transport and Map
            '\u{1F1E0}'..='\u{1F1FF}' | // Regional Indicator Symbols (flags)
            '\u{1F900}'..='\u{1F9FF}' | // Supplemental Symbols and Pictographs
            '\u{1FA70}'..='\u{1FAFF}'   // Symbols and Pictographs Extended-A
        )
    }

    for dir in src_dirs {
        let p = manifest_dir.join(dir);
        if let Ok(entries) = std::fs::read_dir(&p) {
            for entry in entries.filter_map(|e| e.ok()) {
                let file_path = entry.path();
                if file_path.extension().is_some_and(|ext| ext == "rs") {
                    let content = std::fs::read_to_string(&file_path).unwrap();
                    for (line_no, line) in content.lines().enumerate() {
                        for ch in line.chars() {
                            assert!(
                                !is_emoji(ch),
                                "Zero-Emoji Invariant Violated in {}:{}: found emoji character '{ch}' (U+{:04X})",
                                file_path.display(),
                                line_no + 1,
                                ch as u32
                            );
                        }
                    }
                }
            }
        }
    }
}
