//! Differential fuzz for #4198: what a wrapped tool prints for its own `-h`/`--help`
//! must survive RTK.
//!
//! Two failure modes are covered, because the bug had two halves. RTK must not answer the
//! flag itself (its usage is recognisable by the `Usage: rtk ` line), and the filter must
//! not compress the page the tool printed -- `cargo build --help` came back as
//! `cargo build (0 crates compiled)`, 31 bytes of 3135.
//!
//! Tools that are not installed are skipped rather than failing, so the suite runs
//! anywhere and covers whatever the machine has. Commands are spawned directly, never
//! through a shell, so a `grep` shell function cannot stand in for the real binary.

use std::path::PathBuf;
use std::process::Command;

fn on_path(tool: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(tool))
        .find(|p| p.is_file())
}

struct Output {
    text: String,
    code: i32,
}

fn run(program: &PathBuf, args: &[&str]) -> Output {
    let out = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .env("MANWIDTH", "80")
        .env("COLUMNS", "80")
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Output {
        text,
        code: out.status.code().unwrap_or(-1),
    }
}

fn rtk() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rtk"))
}

/// `rtk <rtk_args>` must answer with what `<tool> <native_args>` answers.
struct Case {
    rtk_args: &'static [&'static str],
    tool: &'static str,
    native_args: &'static [&'static str],
}

const CASES: &[Case] = &[
    // `-h` spelled by a tool that means something else by it.
    Case {
        rtk_args: &["grep", "--help"],
        tool: "grep",
        native_args: &["--help"],
    },
    Case {
        rtk_args: &["ls", "--help"],
        tool: "ls",
        native_args: &["--help"],
    },
    Case {
        rtk_args: &["tree", "--help"],
        tool: "tree",
        native_args: &["--help"],
    },
    Case {
        rtk_args: &["wc", "--help"],
        tool: "wc",
        native_args: &["--help"],
    },
    Case {
        rtk_args: &["find", "--help"],
        tool: "find",
        native_args: &["--help"],
    },
    Case {
        rtk_args: &["rg", "--help"],
        tool: "rg",
        native_args: &["--help"],
    },
    // `-h` as a genuine help request, the half that only the filter guard catches.
    Case {
        rtk_args: &["cargo", "build", "-h"],
        tool: "cargo",
        native_args: &["build", "-h"],
    },
    Case {
        rtk_args: &["cargo", "test", "--help"],
        tool: "cargo",
        native_args: &["test", "--help"],
    },
    Case {
        rtk_args: &["cargo", "clippy", "--help"],
        tool: "cargo",
        native_args: &["clippy", "--help"],
    },
    Case {
        rtk_args: &["git", "log", "--help"],
        tool: "git",
        native_args: &["log", "--help"],
    },
    Case {
        rtk_args: &["git", "status", "--help"],
        tool: "git",
        native_args: &["status", "--help"],
    },
    Case {
        rtk_args: &["git", "diff", "--help"],
        tool: "git",
        native_args: &["diff", "--help"],
    },
    Case {
        rtk_args: &["go", "vet", "-h"],
        tool: "go",
        native_args: &["vet", "-h"],
    },
    Case {
        rtk_args: &["gh", "pr", "diff", "--help"],
        tool: "gh",
        native_args: &["pr", "diff", "--help"],
    },
    Case {
        rtk_args: &["kubectl", "get", "--help"],
        tool: "kubectl",
        native_args: &["get", "--help"],
    },
    Case {
        rtk_args: &["docker", "--help"],
        tool: "docker",
        native_args: &["--help"],
    },
    Case {
        rtk_args: &["npm", "--help"],
        tool: "npm",
        native_args: &["--help"],
    },
    Case {
        rtk_args: &["tsc", "--help"],
        tool: "tsc",
        native_args: &["--help"],
    },
    Case {
        rtk_args: &["prettier", "--help"],
        tool: "prettier",
        native_args: &["--help"],
    },
    Case {
        rtk_args: &["mvn", "--help"],
        tool: "mvn",
        native_args: &["--help"],
    },
    Case {
        rtk_args: &["pip", "list", "--help"],
        tool: "pip",
        native_args: &["list", "--help"],
    },
    Case {
        rtk_args: &["uv", "run", "--help"],
        tool: "uv",
        native_args: &["run", "--help"],
    },
    Case {
        rtk_args: &["deno", "test", "--help"],
        tool: "deno",
        native_args: &["test", "--help"],
    },
    Case {
        rtk_args: &["dotnet", "build", "--help"],
        tool: "dotnet",
        native_args: &["build", "--help"],
    },
];

#[test]
fn a_wrapped_tools_own_help_survives_rtk() {
    let rtk = rtk();
    let mut checked = 0usize;
    let mut skipped = Vec::new();

    for case in CASES {
        let Some(tool) = on_path(case.tool) else {
            skipped.push(case.tool);
            continue;
        };
        let label = format!("rtk {}", case.rtk_args.join(" "));
        let mine = run(&rtk, case.rtk_args);
        let native = run(&tool, case.native_args);
        checked += 1;

        assert!(
            !mine.text.contains("Usage: rtk "),
            "{label} answered with RTK's own usage:\n{}",
            &mine.text[..mine.text.len().min(200)]
        );
        assert_eq!(
            mine.code, native.code,
            "{label} exit code drifted from the tool's"
        );
        // A summarising filter collapses the page to a line or two. Exact bytes are not
        // asserted: `man` renders to the terminal it thinks it has, and a tool may print
        // its own name from argv[0].
        assert!(
            mine.text.len() * 2 >= native.text.len(),
            "{label} returned {} bytes of the tool's {}:\n{}",
            mine.text.len(),
            native.text.len(),
            &mine.text[..mine.text.len().min(200)]
        );
    }

    assert!(
        checked > 0,
        "no wrapped tool was available to compare against"
    );
    if !skipped.is_empty() {
        eprintln!("skipped (not installed): {}", skipped.join(", "));
    }
}

/// The reported bug, end to end: `-h` is grep's `--no-filename`, so the output must be
/// grep's matches, byte for byte, and not a help page.
#[test]
fn grep_dash_h_searches_and_drops_the_filename() {
    let Some(grep) = on_path("grep") else {
        eprintln!("skipped: grep not installed");
        return;
    };
    let dir = std::env::temp_dir().join("rtk_4198_fuzz");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let (a, b) = (dir.join("a.txt"), dir.join("b.txt"));
    std::fs::write(&a, "alpha\nbeta\n").expect("write a");
    std::fs::write(&b, "alpha\ngamma\n").expect("write b");
    let (a, b) = (
        a.to_string_lossy().into_owned(),
        b.to_string_lossy().into_owned(),
    );

    for flags in [
        vec!["-h", "alpha", &a, &b],
        vec!["-rh", "alpha", &a, &b],
        vec!["alpha", "-h", &a, &b],
        vec!["-h", "-n", "alpha", &a, &b],
    ] {
        let mine = run(&rtk(), &[&["grep"], flags.as_slice()].concat());
        let native = run(&grep, &flags);
        assert_eq!(
            mine.text,
            native.text,
            "rtk grep {} diverged from grep",
            flags.join(" ")
        );
        assert_eq!(mine.code, native.code, "exit code for {}", flags.join(" "));
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// The inverse of the passthrough: a tool whose `-h` is its own flag must still be
/// filtered. Dropping `tree` from `DASH_H_IS_NOT_HELP` fails here, which is what stops a
/// future edit from silently turning compression off with every other check still green.
/// `ls` is along for the ride rather than proving the table: it renders human sizes itself
/// and strips `-h` before building the child, so the guard never sees that flag.
#[test]
fn a_tools_own_dash_h_is_still_filtered() {
    let dir = std::env::temp_dir().join("rtk_4198_filtered");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(dir.join("one.txt"), "x\n").expect("write");
    std::fs::write(dir.join("two.txt"), "y\n").expect("write");
    let path = dir.to_string_lossy().into_owned();

    for (tool, args) in [("ls", vec!["-h", &path]), ("tree", vec!["-h", &path])] {
        let Some(native) = on_path(tool) else {
            continue;
        };
        let mine = run(&rtk(), &[&[tool], args.as_slice()].concat());
        let theirs = run(&native, &args);
        assert!(
            !mine.text.is_empty(),
            "rtk {tool} {} produced nothing",
            args.join(" ")
        );
        assert_ne!(
            mine.text,
            theirs.text,
            "rtk {tool} {} was passed through unfiltered; is `{tool}` still in \
             DASH_H_IS_NOT_HELP?",
            args.join(" ")
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// RTK's own commands keep their help: releasing everything would satisfy the checks
/// above and leave no way to read RTK's usage.
#[test]
fn rtks_own_help_is_still_rtks() {
    let rtk = rtk();
    for args in [
        vec!["--help"],
        vec!["-h"],
        vec!["gain", "--help"],
        vec!["proxy", "-h"],
        vec!["init", "--help"],
        vec!["help", "grep"],
    ] {
        let out = run(&rtk, &args);
        assert!(
            out.text.contains("Usage: rtk "),
            "rtk {} should print RTK's own usage, got:\n{}",
            args.join(" "),
            &out.text[..out.text.len().min(200)]
        );
    }
}
