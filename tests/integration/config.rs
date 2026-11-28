//! Loading of the configuration from files of the OS.

use hl::config;

use crate::support::repo::repo_file;

#[test]
fn test_load_toml_file() {
    let settings = config::at([repo_file("etc/defaults/config-k8s.toml")])
        .no_default(true)
        .load()
        .unwrap();
    assert_eq!(settings.fields.predefined.time.0.names, &["ts"]);
}

#[test]
fn test_load_yaml_file() {
    let settings = config::at([repo_file("src/testing/assets/configs/issue-288.yaml")])
        .no_default(true)
        .load()
        .unwrap();
    assert_eq!(settings.fields.predefined.level.variants.len(), 1);
    assert_eq!(settings.fields.predefined.level.variants[0].names, &["level"]);
}

#[test]
fn test_load_missing_file() {
    let err = config::at([repo_file("etc/defaults/missing.toml")])
        .no_default(true)
        .load()
        .unwrap_err();
    assert!(err.to_string().contains("missing.toml"), "{err}");
}

#[test]
fn test_load_auto() {
    config::load().unwrap();
}
