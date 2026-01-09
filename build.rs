// std imports
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

// third-party imports
use anyhow::{Result, anyhow};
use const_str::join;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json as json;
use sha2::{Digest, Sha256};

const DEFAULTS_DIR: &str = "etc/defaults";
const THEME_DIR: &str = join!(&[DEFAULTS_DIR, "themes"], "/");
const SCHEMA_DIR: &str = "schema";
const JSON_SCHEMA_DIR: &str = join!(&[SCHEMA_DIR, "json"], "/");
const THEME_SCHEMA_PATH: &str = join!(&[JSON_SCHEMA_DIR, "theme.schema.v1.json"], "/");
const CAPNP_DIR: &str = SCHEMA_DIR;
const SRC_DIR: &str = "src";
const BUILD_CAPNP_DIR: &str = ".build/capnp";

fn main() {
    if let Err(e) = run() {
        eprintln!("{:?}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    build_capnp()?;
    set_git_build_info()?;
    update_schema_directives()?;
    let theme_version = set_theme_version()?;
    update_theme_versions(&theme_version)
}

fn set_git_build_info() -> Result<()> {
    let base_version = env!("CARGO_PKG_VERSION");

    // Parse the base version
    let Ok(mut version) = Version::parse(base_version) else {
        // If version doesn't parse, just use it as-is
        println!("cargo:rustc-env=VERSION={}", base_version);
        return Ok(());
    };

    // Determine if we should add git info (only for pre-release builds)
    let final_version = if version.pre.is_empty() {
        // For stable releases, just use the base version
        base_version.into()
    } else {
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
    };

    // Set VERSION
    println!("cargo:rustc-env=VERSION={}", final_version);

    Ok(())
}

fn build_capnp() -> Result<()> {
    for filename in ["index.capnp"] {
        let source_file = Path::new(CAPNP_DIR).join(filename);
        let target_file = Path::new(SRC_DIR).join(filename.replace(".", "_") + ".rs");
        let hashes = HashInfo {
            source: hex::encode(text_file_hash(&source_file)?),
            target: hex::encode(text_file_hash(&target_file)?),
        };
        let hash_file = Path::new(BUILD_CAPNP_DIR).join(format!("{}.json", filename));
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
            .src_prefix(CAPNP_DIR)
            .file(source_file)
            .output_path(SRC_DIR)
            .run()
            .map_err(|e| anyhow!("Failed to compile capnp schema {}: {}", filename, e))?;

        std::fs::write(&hash_file, json::to_string_pretty(&hashes).unwrap())?;
    }
    Ok(())
}

fn update_schema_directives() -> Result<()> {
    // Process all TOML files in etc/defaults recursively
    update_toml_schema_urls_in_dir(Path::new(DEFAULTS_DIR))?;
    Ok(())
}

fn update_toml_schema_urls_in_dir(dir: &Path) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|e| anyhow!("Failed to read directory {}: {}", dir.display(), e))? {
        let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            update_toml_schema_urls_in_dir(&path)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            println!("cargo:rerun-if-changed={}", path.display());
            update_toml_schema_url(&path)?;
        }
    }
    Ok(())
}

fn update_toml_schema_url(toml_path: &Path) -> Result<()> {
    const SCHEMA_PREFIX: &str = "#:schema ";

    let content = fs::read_to_string(toml_path)
        .map_err(|e| anyhow!("Failed to read TOML file {}: {}", toml_path.display(), e))?;

    // Find the #:schema directive
    let schema_line = content.lines().find(|line| line.trim().starts_with(SCHEMA_PREFIX));

    let Some(schema_line) = schema_line else {
        return Ok(());
    };

    let schema_url = schema_line
        .trim()
        .strip_prefix(SCHEMA_PREFIX)
        .ok_or_else(|| anyhow!("Invalid schema directive in {}", toml_path.display()))?
        .trim();

    // If it's already a relative path, nothing to do
    if !schema_url.starts_with("http://") && !schema_url.starts_with("https://") {
        return Ok(());
    }

    // Extract the schema file name from the URL
    let schema_filename = schema_url
        .rsplit('/')
        .next()
        .ok_or_else(|| anyhow!("Invalid schema URL: {}", schema_url))?;

    // Find the local schema file
    let local_schema_path = find_local_schema_file(schema_filename)?;

    // Compare sha256 hashes
    if let Ok(remote_hash) = fetch_and_hash_url(schema_url) {
        let local_hash = text_file_hash(&local_schema_path)?;

        // If hashes match, no need to update
        if remote_hash == local_hash {
            return Ok(());
        }
    }

    // Hashes differ or remote fetch failed - replace with relative path
    let relative_path = calculate_relative_path(toml_path, &local_schema_path)?;
    let new_schema_line = format!("#:schema {}", relative_path);

    // Only update if different
    if schema_line.trim() == new_schema_line.trim() {
        return Ok(());
    }

    // Replace the schema directive
    rewrite_file_lines(toml_path, |line| {
        if line.trim().starts_with(SCHEMA_PREFIX) {
            new_schema_line.clone()
        } else {
            line.to_string()
        }
    })
}

fn find_local_schema_file(filename: &str) -> Result<PathBuf> {
    let schema_dir = Path::new(JSON_SCHEMA_DIR);
    let schema_path = schema_dir.join(filename);

    if schema_path.exists() {
        Ok(schema_path)
    } else {
        Err(anyhow!("Local schema file not found: {}", schema_path.display()))
    }
}

fn fetch_and_hash_url(url: &str) -> Result<Hash> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .build()
        .new_agent();

    let mut response = agent
        .get(url)
        .call()
        .map_err(|e| anyhow!("Failed to fetch URL {}: {}", url, e))?;

    text_reader_hash(BufReader::new(response.body_mut().as_reader()))
}

fn text_file_hash(path: &Path) -> Result<Hash> {
    let file = File::open(path).map_err(|e| anyhow!("Failed to open {}: {}", path.display(), e))?;
    text_reader_hash(file)
}

fn text_reader_hash<R: std::io::Read>(reader: R) -> Result<Hash> {
    let mut hasher = Sha256::new();
    for line in BufReader::new(reader).lines() {
        let line = line.map_err(|e| anyhow!("Failed to read line for hashing: {}", e))?;
        hasher.update(line);
        hasher.update(b"\n");
    }
    Ok(hasher.finalize().into())
}

fn rewrite_file_lines<F>(path: &Path, transform: F) -> Result<()>
where
    F: Fn(&str) -> String,
{
    let content = fs::read_to_string(path).map_err(|e| anyhow!("Failed to read file {}: {}", path.display(), e))?;

    let new_content: String = content.lines().map(transform).collect::<Vec<_>>().join("\n");

    let new_content = if content.ends_with('\n') {
        format!("{}\n", new_content)
    } else {
        new_content
    };

    fs::write(path, new_content).map_err(|e| anyhow!("Failed to write file {}: {}", path.display(), e))?;

    Ok(())
}

fn calculate_relative_path(from: &Path, to: &Path) -> Result<String> {
    let from_dir = from
        .parent()
        .ok_or_else(|| anyhow!("Failed to get parent directory of {}", from.display()))?;

    let from_components: Vec<_> = from_dir.components().collect();
    let to_components: Vec<_> = to.components().collect();

    // Find common prefix length
    let common_len = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // Build relative path
    let up_levels = from_components.len() - common_len;
    let mut rel_path = String::new();

    for _ in 0..up_levels {
        rel_path.push_str("../");
    }

    for component in &to_components[common_len..] {
        if let std::path::Component::Normal(comp) = component {
            if !rel_path.is_empty() && !rel_path.ends_with('/') {
                rel_path.push('/');
            }
            rel_path.push_str(comp.to_str().ok_or_else(|| anyhow!("Invalid path component"))?);
        }
    }

    Ok(rel_path)
}

fn set_theme_version() -> Result<String> {
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

    Ok(version.to_string())
}

fn update_theme_versions(schema_version: &str) -> Result<()> {
    const VERSION_PREFIX: &str = "version = \"";

    let themes_dir = Path::new(THEME_DIR);
    let expected_version_line = format!("version = \"{}\"", schema_version);

    for entry in fs::read_dir(themes_dir)
        .map_err(|e| anyhow!("Failed to read themes directory {}: {}", themes_dir.display(), e))?
    {
        let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }

        let content =
            fs::read_to_string(&path).map_err(|e| anyhow!("Failed to read theme file {}: {}", path.display(), e))?;

        // Check if version needs updating
        let version_needs_update = content
            .lines()
            .any(|line| line.starts_with(VERSION_PREFIX) && line != expected_version_line);

        // Only update files if version differs
        if !version_needs_update {
            continue;
        }

        // Update version line
        rewrite_file_lines(&path, |line| {
            if line.starts_with(VERSION_PREFIX) {
                expected_version_line.clone()
            } else {
                line.to_string()
            }
        })?;
    }

    Ok(())
}

type Hash = [u8; 32];

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
struct HashInfo {
    source: String,
    target: String,
}
