use std::{
    fs,
    path::{Path, PathBuf},
};

const SECRET_CANARIES: [&str; 4] = [
    "cookie-secret-value",
    "token-id-secret-canary",
    "video-token-secret-canary",
    "upstream-secret",
];

const FORBIDDEN_TRACE_FIELDS: [&str; 7] = [
    "cookie",
    "token",
    "url",
    "uuid",
    "sig",
    "response_body",
    "display_name",
];

#[test]
fn production_sources_contain_no_secret_canary_or_sensitive_trace_field() {
    let roots = production_source_roots();
    let files = roots
        .iter()
        .flat_map(|root| rust_files(root))
        .collect::<Vec<_>>();

    for file in files {
        let source = fs::read_to_string(&file).expect("production Rust source should be readable");
        for canary in SECRET_CANARIES {
            assert!(!source.contains(canary), "secret canary found in {file:?}");
        }
        assert!(!source.contains("dbg!("), "dbg! found in {file:?}");
        scan_trace_lines(&file, &source);
    }
}

fn production_source_roots() -> [PathBuf; 2] {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crates = manifest.parent().expect("protocol-cli is under crates");
    [manifest.join("src"), crates.join("canvas-core/src")]
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_owned()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).expect("source directory should be readable") {
            let path = entry.expect("source entry should be readable").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files
}

fn scan_trace_lines(file: &Path, source: &str) {
    for line in source.lines().filter(|line| line.contains("tracing::")) {
        let lower = line.to_ascii_lowercase();
        for field in FORBIDDEN_TRACE_FIELDS {
            assert!(
                !lower.contains(field),
                "sensitive tracing field {field} found in {file:?}"
            );
        }
    }
}
