//! Coverage report generator for Vue compiler tests
//!
//! Usage:
//!   cargo run -p vize_test_runner --bin coverage          # Summary only
//!   cargo run -p vize_test_runner --bin coverage -- -v    # Show failing tests
//!   cargo run -p vize_test_runner --bin coverage -- -vv   # Show diffs

use std::path::PathBuf;
use vize_test_runner::{run_fixture_tests, CompilerMode};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let show_diff = args.iter().any(|a| a == "-vv");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = manifest_dir.parent().unwrap().join("fixtures");
    let expected_dir = manifest_dir.parent().unwrap().join("expected");

    let test_files = [
        ("vdom/element", CompilerMode::Vdom),
        ("vdom/component", CompilerMode::Vdom),
        ("vdom/directives", CompilerMode::Vdom),
        ("vdom/hoisting", CompilerMode::Vdom),
        ("vdom/patch-flags", CompilerMode::Vdom),
        ("vdom/v-if", CompilerMode::Vdom),
        ("vdom/v-for", CompilerMode::Vdom),
        ("vdom/v-bind", CompilerMode::Vdom),
        ("vdom/v-on", CompilerMode::Vdom),
        ("vdom/v-model", CompilerMode::Vdom),
        ("vdom/v-slot", CompilerMode::Vdom),
        ("vdom/v-show", CompilerMode::Vdom),
        ("vdom/v-once", CompilerMode::Vdom),
        ("vapor/element", CompilerMode::Vapor),
        ("vapor/component", CompilerMode::Vapor),
        ("vapor/v-if", CompilerMode::Vapor),
        ("vapor/v-for", CompilerMode::Vapor),
        ("vapor/v-bind", CompilerMode::Vapor),
        ("vapor/v-on", CompilerMode::Vapor),
        ("vapor/v-model", CompilerMode::Vapor),
        ("vapor/v-slot", CompilerMode::Vapor),
        ("vapor/v-show", CompilerMode::Vapor),
        ("vapor/edge-cases", CompilerMode::Vapor),
        ("sfc/basic", CompilerMode::Sfc),
        ("sfc/script-setup", CompilerMode::Sfc),
        ("sfc/patches", CompilerMode::Sfc),
    ];

    println!("Vue Compiler Coverage Report");
    println!("============================\n");

    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut total_skipped = 0;

    let mut vdom_passed = 0;
    let mut vdom_total = 0;
    let mut vapor_passed = 0;
    let mut vapor_total = 0;
    let mut sfc_passed = 0;
    let mut sfc_total = 0;

    for (path, mode) in &test_files {
        let fixture = fixtures_dir.join(format!("{}.toml", path));
        let expected = expected_dir.join(format!("{}.snap", path));

        let results = run_fixture_tests(&fixture, &expected);

        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results
            .iter()
            .filter(|r| !r.passed && r.error.is_some())
            .count();
        let skipped = results
            .iter()
            .filter(|r| !r.passed && r.error.is_none())
            .count();
        let total = results.len();

        total_passed += passed;
        total_failed += failed;
        total_skipped += skipped;

        match mode {
            CompilerMode::Vdom => {
                vdom_passed += passed;
                vdom_total += total;
            }
            CompilerMode::Vapor => {
                vapor_passed += passed;
                vapor_total += total;
            }
            CompilerMode::Sfc => {
                sfc_passed += passed;
                sfc_total += total;
            }
        }

        let pct = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let status = if passed == total {
            "\x1b[32m✓\x1b[0m"
        } else if passed > 0 {
            "\x1b[33m◐\x1b[0m"
        } else {
            "\x1b[31m✗\x1b[0m"
        };

        println!(
            "{} {:25} {:3}/{:3} ({:5.1}%)",
            status, path, passed, total, pct
        );

        // Show details if verbose
        if (verbose || show_diff) && failed > 0 {
            for result in &results {
                if !result.passed {
                    if let Some(ref err) = result.error {
                        if show_diff {
                            println!("    \x1b[31m✗\x1b[0m {}", result.name);
                            for line in err.lines().take(5) {
                                println!("      {}", line);
                            }
                        } else if verbose {
                            println!("    \x1b[31m✗\x1b[0m {}", result.name);
                        }
                    }
                }
            }
        }
    }

    println!("\n----------------------------");

    let vdom_pct = if vdom_total > 0 {
        (vdom_passed as f64 / vdom_total as f64) * 100.0
    } else {
        0.0
    };
    let vapor_pct = if vapor_total > 0 {
        (vapor_passed as f64 / vapor_total as f64) * 100.0
    } else {
        0.0
    };
    let sfc_pct = if sfc_total > 0 {
        (sfc_passed as f64 / sfc_total as f64) * 100.0
    } else {
        0.0
    };

    println!(
        "VDOM:   {:3}/{:3} ({:5.1}%)",
        vdom_passed, vdom_total, vdom_pct
    );
    println!(
        "Vapor:  {:3}/{:3} ({:5.1}%)",
        vapor_passed, vapor_total, vapor_pct
    );
    println!(
        "SFC:    {:3}/{:3} ({:5.1}%)",
        sfc_passed, sfc_total, sfc_pct
    );

    println!("\n============================");

    let total = total_passed + total_failed + total_skipped;
    let total_pct = if total > 0 {
        (total_passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    println!(
        "TOTAL:  {:3}/{:3} ({:5.1}%)",
        total_passed, total, total_pct
    );

    if total_failed > 0 {
        println!("\n{} tests failed", total_failed);
    }
}
