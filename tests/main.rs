use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;

fn build_binary() -> String {
    let output = Command::new("cargo")
        .args(&["build"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to build binary");

    if !output.status.success() {
        panic!(
            "Failed to build: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    format!("{}/target/debug/trunk", env!("CARGO_MANIFEST_DIR"))
}

/// Helper: spawn child and return (child, receiver of stdout lines)
fn spawn_with_stdout(
    binary: &str,
    args: &[&str],
    path: &std::path::Path,
) -> (std::process::Child, mpsc::Receiver<String>) {
    let mut cmd = Command::new(binary);
    for a in args {
        cmd.arg(a);
    }
    cmd.arg(path);
    cmd.stdout(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn child");
    let stdout = child.stdout.take().expect("no stdout");

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let _ = tx.send(line.clone());
                }
                Err(_) => break,
            }
        }
    });

    (child, rx)
}

/// Default behavior: last 5 lines
#[test]
fn test_default_last_5_lines() {
    let mut f = NamedTempFile::new().unwrap();
    let mut content = String::new();
    for i in 1..=10 {
        content.push_str(&format!("line{}\n", i));
    }
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();

    let binary = build_binary();
    let output = Command::new(&binary)
        .arg(f.path())
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected: String = (6..=10).map(|i| format!("line{}\n", i)).collect();
    assert_eq!(stdout, expected);
}

/// -n option
#[test]
fn test_option_n() {
    let mut f = NamedTempFile::new().unwrap();
    for i in 1..=8 {
        writeln!(f, "line{}", i).unwrap();
    }
    f.flush().unwrap();

    let binary = build_binary();
    let output = Command::new(&binary)
        .args(&["-n", "3"])
        .arg(f.path())
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected: String = (6..=8).map(|i| format!("line{}\n", i)).collect();
    assert_eq!(stdout, expected);
}

/// -f (follow) option: spawn, append, and verify appended lines appear
#[test]
fn test_follow() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"first\nsecond\n").unwrap();
    f.flush().unwrap();

    let binary = build_binary();

    let mut child = Command::new(&binary)
        .args(&["-f"])
        .arg(f.path())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    let mut append = OpenOptions::new().append(true).open(f.path()).unwrap();
    append.write_all(b"followed_line\n").unwrap();
    append.flush().unwrap();

    let start = Instant::now();
    let mut line = String::new();

    // Poll stdout with timeout
    loop {
        line.clear();

        if stdout.read_line(&mut line).unwrap() > 0 {
            if line.contains("followed_line") {
                break;
            }
        }

        if start.elapsed() > Duration::from_secs(2) {
            break;
        }
    }
    let _ = child.kill();
    let _ = child.wait();

    println!("Line: {:#?}", line);

    assert!(line.contains("followed_line"));
}

/// -s (sieve) option: spawn with sieve, append mixed lines, expect only matching lines
#[test]
fn test_sieve() {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(b"first\nsecond\n").unwrap();
    f.flush().unwrap();

    let binary = build_binary();

    let mut child = Command::new(&binary)
        .args(&["-s", "INFO"])
        .arg(f.path())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    let mut append = OpenOptions::new().append(true).open(f.path()).unwrap();

    let infos = vec!["INFO: all good", "INFO: another message"];
    let non_infos = vec!["ERR: this is an error", "WARN: consider yourself"];

    let _ = append.write_all(infos.join("\n").as_bytes());
    let _ = append.write_all(b"\n");
    let _ = append.write_all(non_infos.join("\n").as_bytes());
    let _ = append.write_all(b"\n");

    append.flush().unwrap();

    let start = Instant::now();
    let mut output = Vec::new();
    let mut line = String::new();

    while start.elapsed() < Duration::from_secs(2) {
        line.clear();

        match stdout.read_line(&mut line) {
            Ok(0) => {
                // no data yet, avoid busy spin
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(_) => {
                output.push(line.trim_end().to_string());

                // stop early if we've seen all INFO lines
                if infos.iter().all(|i| output.iter().any(|o| o.contains(i))) {
                    break;
                }
            }
            Err(e) => panic!("stdout read failed: {e}"),
        }
    }

    let _ = child.kill();
    let _ = child.wait();

    println!("Line: {:#?}", output);

    for l in &infos {
        assert!(output.iter().any(|o| o.contains(l)));
    }

    for l in &non_infos {
        assert!(!output.iter().any(|o| o.contains(l)));
    }
}
