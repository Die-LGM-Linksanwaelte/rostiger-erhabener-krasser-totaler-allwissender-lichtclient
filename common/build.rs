use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

fn hash_directory(dir: &Path, hasher: &mut DefaultHasher) {

    let mut entries : Vec<_> = fs::read_dir(dir).unwrap().map(|r| r.unwrap()).collect();

    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            hash_directory(&path, hasher);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read(&path).unwrap();
            content.hash(hasher);
        }
    }
}

fn main() {
    let mut hasher = DefaultHasher::new();

    hash_directory(Path::new("src"), &mut hasher);
    let hash_str = hasher.finish().to_string();

    println!("cargo:rustc-env=PROTOCOL_HASH={}", hash_str);

    println!("cargo:rerun-if-changed=src");
}