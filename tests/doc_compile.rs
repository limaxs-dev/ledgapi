//! Regression test for QA-hunter BUG-000007.
//!
//! `src/lib.rs` uses `#![doc = include_str!("../README.md")]` to surface
//! the README as the crate's rustdoc landing page. That only compiles
//! when the README's non-Rust blocks (notably the ASCII architecture
//! diagram, which contains box-drawing Unicode) are tagged with a
//! non-Rust fence language (e.g. ` ```text ` instead of ` ``` `).
//!
//! Before this test, the README rewrite added an untagged fence that
//! made the doc include fail to parse, which broke `cargo test --doc`
//! silently — the regular unit/integration suite still passed, so the
//! regression slipped through every CI gate except `cargo doc`.
//!
//! This test runs `cargo doc --no-deps` and asserts it exits 0. The
//! check is intentionally cheap (the doc is already part of the build
//! graph) and CI-friendly (no network, no extra tooling).

use std::path::Path;
use std::process::Command;

#[test]
fn readme_doc_include_compiles() {
    // Run from the workspace root so the include_str!("../README.md")
    // path resolves the same way it does for the real build.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    // src/lib.rs includes "../README.md" relative to the manifest dir
    // (i.e. the crate's own directory), not the workspace root. For a
    // single-crate workspace the two coincide, but be explicit.
    let readme_path = manifest_dir.join("README.md");
    assert!(
        readme_path.exists(),
        "README.md missing at {readme_path:?}",
    );

    let output = Command::new("cargo")
        .args(["doc", "--no-deps", "--offline"])
        .current_dir(manifest_dir)
        .env_remove("RUSTC_WRAPPER")
        // Without this, an untagged fenced block in README.md only
        // produces a `rustdoc::invalid_rust_codeblocks` *warning*, not
        // a hard error — so the suite would pass on a regression.
        .env("RUSTDOCFLAGS", "-D rustdoc::invalid_rust_codeblocks")
        .output()
        .expect("failed to spawn `cargo doc`");

    assert!(
        output.status.success(),
        "`cargo doc --no-deps` failed (exit {:?}). \
         README.md likely contains a fenced block that is not tagged as \
         a non-Rust language, so the `#![doc = include_str!]` in \
         src/lib.rs is being parsed as a Rust code block.\n\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
