//! Architecture smoke test. Runs on every `cargo test`, takes ~2s.
//! Asserts that no business rule or wrong-layer dep sneaks in.

use std::fs;
use std::path::Path;

#[test]
fn domain_does_not_import_infra_or_mcp_or_web() {
    let src = read_src("src/domain");
    for forbidden in
        ["infra::", "crate::infra", "crate::mcp", "crate::web", "rusqlite", "fastembed"]
    {
        assert!(
            !src.contains(&format!("use {forbidden}")),
            "domain must not depend on {forbidden}"
        );
    }
}

#[test]
fn infra_does_not_depend_on_mcp_or_web() {
    let src = read_src("src/infra");
    assert!(!src.contains("crate::mcp"), "infra must not depend on mcp");
    assert!(!src.contains("crate::web"), "infra must not depend on web");
}

#[test]
fn mcp_tools_call_use_cases_not_repos_directly() {
    // mcp/tools_impl/* are thin delegates. They must use `crate::domain::use_cases::*`,
    // never `crate::infra::repos::*` directly.
    let src = read_src("src/mcp/tools_impl");
    assert!(!src.contains("crate::infra::repos"), "tools must not touch SQL directly");
}

#[test]
fn no_unsafe_in_src() {
    let _src = read_src("src");
    // Allow `unsafe { rusqlite::ffi::sqlite3_auto_extension(...) }` in pool.rs.
    let mut occurrences: Vec<(String, usize)> = Vec::new();
    for entry in walk("src") {
        let path = entry.path();
        let text = fs::read_to_string(&path).unwrap();
        // Count actual unsafe tokens, not the substring — comrak's
        // `options.render.unsafe_` field name is a false positive.
        let n = token_count_unsafe(&text);
        if n > 0 {
            // Allow the documented site in db/pool.rs.
            if !path.ends_with("db/pool.rs") {
                occurrences.push((path.display().to_string(), n));
            }
        }
    }
    assert!(occurrences.is_empty(), "unsafe code found outside db/pool.rs: {occurrences:?}");
}

/// Count `unsafe` keyword occurrences (word-boundary match), so identifiers
/// like comrak's `render.unsafe_` field don't count.
fn token_count_unsafe(text: &str) -> usize {
    fn is_word(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }
    let mut n = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while let Some(pos) = text[i..].find("unsafe") {
        let start = i + pos;
        let end = start + "unsafe".len();
        let before_ok = start == 0 || !is_word(bytes[start - 1]);
        let after_ok = end >= bytes.len() || !is_word(bytes[end]);
        if before_ok && after_ok {
            n += 1;
        }
        i = end;
    }
    n
}

fn read_src(dir: &str) -> String {
    let mut out = String::new();
    for entry in walk(dir) {
        let raw = fs::read_to_string(entry.path()).unwrap_or_default();
        // Strip `#[cfg(test)] mod tests { ... }` blocks — they are allowed
        // to import infra adapters directly because unit tests need concrete
        // handles, not the port traits.
        out.push_str(&strip_test_blocks(&raw));
    }
    out
}

/// Drop `#[cfg(test)]` (and plain `#[test]`-gated) submodules so that
/// unit tests inside `domain/` may still pull infra adapters without
/// tripping the architecture test.
fn strip_test_blocks(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut depth: i32 = 0;
    let mut skip = false;
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if skip {
            // Skip until balanced braces close, then resume.
            let c = bytes[i];
            if c == b'{' {
                depth += 1;
            } else if c == b'}' {
                depth -= 1;
                if depth == 0 {
                    skip = false;
                }
            }
            i += 1;
            continue;
        }
        // Detect `#[cfg(test)]` or `#[test]` attribute start.
        if i + 1 < bytes.len() && bytes[i] == b'#' && bytes[i + 1] == b'[' {
            if let Some(end) = src[i + 2..].find(']') {
                let attr = &src[i + 2..i + 2 + end];
                if attr.contains("test") {
                    skip = true;
                    depth = 0;
                    i = i + 2 + end + 1;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn walk(dir: &str) -> Vec<std::fs::DirEntry> {
    let mut out = Vec::new();
    let root = Path::new(dir);
    if !root.exists() {
        return out;
    }
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            out.extend(walk(entry.path().to_str().unwrap()));
        } else if entry.path().extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(entry);
        }
    }
    out
}
