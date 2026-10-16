// Shared helpers for integration tests: fixture loading and packet builders.
// Each tests/*.rs that wants these does `mod common;`, so not every helper is
// used by every test binary.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// One payload line from a tests/fixtures/**/*.hex file.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub file: String,
    pub line: usize,
    pub bytes: Vec<u8>,
    /// From a preceding `#! inputs N` directive.
    pub expect_inputs: Option<usize>,
}

impl Fixture {
    pub fn label(&self) -> String {
        format!("{}:{}", self.file, self.line)
    }
}

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Same grammar as `cargo xtask decode`: spaced bytes, one hex run, `0x..,`
/// lists, or a defmt line where only the last [...] is used.
pub fn parse_hex(line: &str) -> Result<Vec<u8>, String> {
    let body = match (line.rfind('['), line.rfind(']')) {
        (Some(a), Some(b)) if a < b => &line[a + 1..b],
        _ => line,
    };
    let mut out = Vec::new();
    for tok in body.split(|c: char| c.is_whitespace() || c == ',') {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        let tok = tok.strip_prefix("0x").unwrap_or(tok);
        if tok.len() % 2 != 0 {
            return Err(format!("odd-length token '{tok}'"));
        }
        for i in (0..tok.len()).step_by(2) {
            out.push(
                u8::from_str_radix(&tok[i..i + 2], 16)
                    .map_err(|_| format!("bad hex '{}'", &tok[i..i + 2]))?,
            );
        }
    }
    Ok(out)
}

/// Load every payload from every *.hex file in tests/fixtures/<sub>.
/// Panics on unreadable files or bad hex: a broken fixture is a test failure.
pub fn load(sub: &str) -> Vec<Fixture> {
    let dir = fixtures_dir().join(sub);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "hex"))
        .collect();
    files.sort();

    let mut out = Vec::new();
    for path in files {
        let name = format!("{sub}/{}", path.file_name().unwrap().to_string_lossy());
        let text = std::fs::read_to_string(&path).unwrap();
        let mut pending: Option<usize> = None;
        for (i, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if let Some(d) = line.strip_prefix("#!") {
                let mut it = d.split_whitespace();
                match (it.next(), it.next()) {
                    (Some("inputs"), Some(n)) => {
                        pending =
                            Some(n.parse().unwrap_or_else(|_| {
                                panic!("{name}:{}: bad directive '{line}'", i + 1)
                            }))
                    }
                    _ => panic!("{name}:{}: unknown directive '{line}'", i + 1),
                }
                continue;
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let bytes = parse_hex(line).unwrap_or_else(|e| panic!("{name}:{}: {e}", i + 1));
            out.push(Fixture {
                file: name.clone(),
                line: i + 1,
                bytes,
                expect_inputs: pending.take(),
            });
        }
        assert!(
            pending.is_none(),
            "{name}: directive with no payload after it"
        );
    }
    out
}

// ---- packet builders (shared by fuzz.rs) ----

pub fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut v = vec![value.len() as u8, tag];
    v.extend_from_slice(value);
    v
}

pub fn f1(records: &[Vec<u8>]) -> Vec<u8> {
    let mut v = vec![0xF1];
    for r in records {
        v.extend_from_slice(r);
    }
    v
}
