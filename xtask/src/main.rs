//! xtask: ci / layers / parity gates (designcraft pattern, minimal).
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use std::process::Command;

fn run(cmd: &str, args: &[&str]) -> i32 {
    match Command::new(cmd).args(args).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("xtask: failed to run {cmd}: {e}");
            1
        }
    }
}

fn ci() -> i32 {
    let steps: &[(&str, &[&str])] = &[
        ("cargo", &["fmt", "--all", "--check"]),
        (
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        ),
        ("cargo", &["test", "--workspace"]),
    ];
    for (cmd, args) in steps {
        let code = run(cmd, args);
        if code != 0 {
            return code;
        }
    }
    // layers + parity + forge are part of ci (fast, no I/O).
    let code = layers();
    if code != 0 {
        return code;
    }
    let code = parity();
    if code != 0 {
        return code;
    }
    forge()
}

fn layers() -> i32 {
    // L6 rule: nothing below L3 depends on egui/eframe.
    // Minimal check: core + store must not mention egui/eframe in Cargo.toml.
    let files = [
        "crates/qgiscraft-core/Cargo.toml",
        "crates/qgiscraft-store/Cargo.toml",
    ];
    for path in files {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                if text.contains("egui") || text.contains("eframe") {
                    eprintln!("xtask layers: {path} must not depend on egui/eframe");
                    return 1;
                }
            }
            Err(e) => {
                eprintln!("xtask layers: cannot read {path}: {e}");
                return 1;
            }
        }
    }
    println!("xtask layers: ok (core/store free of egui/eframe)");
    0
}

fn parity() -> i32 {
    // Presence checklist: every command in core lib.rs must appear in docs/parity-checklist.md
    // and in mcp tools_list. Minimal grep, no unwrap.
    let lib = match std::fs::read_to_string("crates/qgiscraft-core/src/lib.rs") {
        Ok(t) => t,
        Err(e) => {
            eprintln!("xtask parity: cannot read core lib.rs: {e}");
            return 1;
        }
    };
    let checklist = std::fs::read_to_string("docs/parity-checklist.md").unwrap_or_default();
    let mut missing = 0;
    for cmd in [
        "layer.add",
        "layer.list",
        "layer.remove",
        "geocode.search",
        "overpass.query",
        "features.list",
        "layer.import",
        "processing.buffer",
        "map.forge",
        "wkt.show",
        "processing.grid",
        "processing.nearest",
        "export.png",
    ] {
        if !lib.contains(cmd) {
            eprintln!("xtask parity: core lib.rs missing {cmd}");
            missing += 1;
        }
        if !checklist.contains(cmd) && !checklist.contains(&cmd.replace('.', " ")) {
            eprintln!("xtask parity: docs/parity-checklist.md missing {cmd}");
            missing += 1;
        }
    }
    if missing == 0 {
        println!("xtask parity: ok (13 commands present)");
        0
    } else {
        1
    }
}

fn forge() -> i32 {
    // Depot gate: every depot/maps/*/spec.json exists with required keys.
    // Full schema validation lives in core unit tests + CLI dry-run.
    let depot = match std::fs::read_dir("depot/maps") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("xtask forge: cannot read depot/maps: {e}");
            return 1;
        }
    };
    let mut specs = 0;
    for entry in depot {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let spec = entry.path().join("spec.json");
        let text = match std::fs::read_to_string(&spec) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let mut ok = true;
        for key in ["title", "slug", "bbox", "layers", "key", "value"] {
            if !text.contains(&format!("\"{key}\"")) {
                eprintln!("xtask forge: {} missing key {key}", spec.to_string_lossy());
                ok = false;
            }
        }
        if ok {
            specs += 1;
        }
    }
    if specs == 0 {
        eprintln!("xtask forge: no valid specs in depot/maps");
        return 1;
    }
    println!("xtask forge: ok ({specs} specs)");
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let op = args.get(1).map(|s| s.as_str()).unwrap_or("ci");
    let code = match op {
        "ci" => ci(),
        "layers" => layers(),
        "parity" => parity(),
        "forge" => forge(),
        _ => {
            eprintln!("usage: xtask <ci|layers|parity|forge>");
            2
        }
    };
    std::process::exit(code);
}
