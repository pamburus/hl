// std imports
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

// third-party imports
use anyhow::{Result, anyhow};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json as json;
use sha2::{Digest, Sha256};

const THEME_DIR: &str = "etc/defaults/themes";
const THEME_SCHEMA_PATH: &str = "schema/json/theme.schema.v1.json";

fn main() {
    if let Err(e) = run() {
        eprintln!("{:?}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    build_capnp()?;
    set_git_build_info()?;
    set_theme_version()
}

fn set_git_build_info() -> Result<()> {
    let base_version = env!("CARGO_PKG_VERSION");

    // Parse the base version
    let Ok(mut version) = Version::parse(base_version) else {
        // If version doesn't parse, just use it as-is
        println!("cargo:rustc-env=HL_VERSION={}", base_version);
        return Ok(());
    };

    // Determine if we should add git info (only for pre-release builds)
    let should_add_git_info = !version.pre.is_empty();

    let final_version = if should_add_git_info {
        // Get commit hash
        let commit = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        // Check if working directory is dirty
        let is_dirty = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        // Build the metadata string
        let mut metadata_parts = Vec::new();

        // Add existing build metadata if any
        if !version.build.is_empty() {
            metadata_parts.push(version.build.to_string());
        }

        // Add commit hash
        if let Some(commit) = commit {
            metadata_parts.push(commit);
        }

        // Add dirty flag
        if is_dirty {
            metadata_parts.push("dirty".to_string());
        }

        // Construct the final version string
        if metadata_parts.is_empty() {
            version.to_string()
        } else {
            version.build = metadata_parts.join(".").parse()?;
            version.to_string()
        }
    } else {
        // For stable releases, just use the base version
        base_version.into()
    };

    // Always set HL_VERSION
    println!("cargo:rustc-env=VERSION={}", final_version);

    // Rerun build if git HEAD changes or working directory changes
    if should_add_git_info {
        println!("cargo:rerun-if-changed=.git/HEAD");
        println!("cargo:rerun-if-changed=.git/index");
    }

    Ok(())
}

fn build_capnp() -> Result<()> {
    for filename in ["index.capnp"] {
        let source_file = Path::new("schema").join(filename);
        let target_file = Path::new("src").join(filename.replace(".", "_") + ".rs");
        let hashes = HashInfo {
            source: hex::encode(file_hash(&source_file)?),
            target: hex::encode(file_hash(&target_file)?),
        };
        let hash_file = Path::new(".build").join("capnp").join(format!("{}.json", filename));
        if hash_file.is_file() {
            let file = File::open(&hash_file)
                .map_err(|e| anyhow!("Failed to open hash file {}: {}", hash_file.display(), e))?;
            if let Ok(stored_hashes) = json::from_reader::<_, HashInfo>(file) {
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
            .map_err(|e| anyhow!("Failed to compile capnp schema {}: {}", filename, e))?;

        std::fs::write(&hash_file, json::to_string_pretty(&hashes).unwrap())?;
    }
    Ok(())
}

fn set_theme_version() -> Result<()> {
    let schema_path = Path::new(THEME_SCHEMA_PATH);
    let file =
        File::open(schema_path).map_err(|e| anyhow!("Failed to open theme schema {}: {}", schema_path.display(), e))?;

    let schema: json::Value = json::from_reader(file)
        .map_err(|e| anyhow!("Failed to parse theme schema {}: {}", schema_path.display(), e))?;

    let version = schema
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'version' field in theme schema"))?;

    println!("cargo:rustc-env=HL_BUILD_THEME_VERSION={}", version);
    println!("cargo:rerun-if-changed={}", schema_path.display());

    // Update version and schema directive in all theme TOML files
    update_theme_toml_files(version)?;

    Ok(())
}

fn update_theme_toml_files(version: &str) -> Result<()> {
    let themes_dir = Path::new(THEME_DIR);
    let expected_version_line = format!("version = \"{}\"", version);

    // Calculate relative path from THEME_DIR to root
    let depth = themes_dir.components().count();
    let rel = "../".repeat(depth);

    // Use relative URL - release workflow will replace with absolute tagged URL
    let expected_schema_line = format!("#:schema {}{}", rel, THEME_SCHEMA_PATH);

    for entry in fs::read_dir(themes_dir)
        .map_err(|e| anyhow!("Failed to read themes directory {}: {}", themes_dir.display(), e))?
    {
        let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }

        println!("cargo:rerun-if-changed={}", path.display());

        let content =
            fs::read_to_string(&path).map_err(|e| anyhow!("Failed to read theme file {}: {}", path.display(), e))?;

        // Check if version needs updating
        let version_needs_update = content
            .lines()
            .any(|line| line.starts_with("version = \"") && line != expected_version_line);

        // Only update files if version differs
        if !version_needs_update {
            continue;
        }

        // Update both version and schema directive when version changes
        let new_content: Vec<String> = content
            .lines()
            .map(|line| {
                if line.starts_with("#:schema ") {
                    expected_schema_line.clone()
                } else if line.starts_with("version = \"") {
                    expected_version_line.clone()
                } else {
                    line.to_string()
                }
            })
            .collect();

        let mut file = File::create(&path)
            .map_err(|e| anyhow!("Failed to open theme file for writing {}: {}", path.display(), e))?;

        for (i, line) in new_content.iter().enumerate() {
            if i > 0 {
                writeln!(file)?;
            }
            write!(file, "{}", line)?;
        }
        writeln!(file)?;
    }

    Ok(())
}

fn file_hash(filename: &PathBuf) -> Result<Hash> {
    let mut hasher = Sha256::new();
    let file = File::open(filename).map_err(|e| anyhow!("Failed to open {}: {}", filename.display(), e))?;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| anyhow!("Failed to read line from {}: {}", filename.display(), e))?;
        hasher.update(line);
    }

    Ok(hasher.finalize().into())
}

type Hash = [u8; 32];

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
struct HashInfo {
    source: String,
    target: String,
}
