use generic_array::{typenum::U32, GenericArray};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn main() {
    build_capnp();
    build_lalrpop();
}

fn build_capnp() {
    for filename in ["index.capnp"] {
        let source_file = Path::new("schema").join(filename);
        let target_file = Path::new("src").join(filename.replace(".", "_") + ".rs");
        let hashes = HashInfo {
            source: hex::encode(file_hash(&source_file)),
            target: hex::encode(file_hash(&target_file)),
        };
        let hash_file = Path::new(".build").join("capnp").join(format!("{}.json", filename));
        if hash_file.is_file() {
            if let Ok(stored_hashes) = serde_json::from_reader::<_, HashInfo>(File::open(&hash_file).unwrap()) {
                if stored_hashes == hashes {
                    continue;
                }
            }
        }

        capnpc::CompilerCommand::new()
            .src_prefix("schema")
            .file(source_file)
            .output_path("src")
            .run()
            .expect("schema compiler command");

        std::fs::write(&hash_file, serde_json::to_string_pretty(&hashes).unwrap()).unwrap();
    }
}

fn build_lalrpop() {
    lalrpop::process_root().unwrap();
}

fn file_hash(filename: &PathBuf) -> GenericArray<u8, U32> {
    let mut hasher = Sha256::new();
    for line in BufReader::new(File::open(filename).unwrap()).lines() {
        hasher.update(line.unwrap());
    }
    hasher.finalize()
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
struct HashInfo {
    source: String,
    target: String,
}
