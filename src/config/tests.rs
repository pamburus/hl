use super::*;

use maplit::hashmap;

use crate::level::{InfallibleLevel, Level};

#[test]
fn test_default() {
    assert_eq!(default().theme, "uni");
}

#[test]
fn test_load_k8s() {
    let settings = super::at(["etc/defaults/config-k8s.yaml"]).load().unwrap();
    assert_eq!(settings.fields.predefined.time.0.names, &["ts"]);
    assert_eq!(settings.fields.predefined.message.0.names, &["msg"]);
    assert_eq!(settings.fields.predefined.level.variants.len(), 2);
}

#[test]
fn test_issue_288() {
    let settings = super::at(["src/testing/assets/configs/issue-288.yaml"]).load().unwrap();
    assert_eq!(settings.fields.predefined.level.variants.len(), 1);
    let variant = &settings.fields.predefined.level.variants[0];
    assert_eq!(variant.names, vec!["level".to_owned()]);
    assert_eq!(
        variant.values,
        hashmap! {
            InfallibleLevel::new(Level::Debug) => vec!["dbg".to_owned()],
            InfallibleLevel::new(Level::Info) => vec!["INF".to_owned()],
            InfallibleLevel::new(Level::Warning) => vec!["wrn".to_owned()],
            InfallibleLevel::new(Level::Error) => vec!["ERR".to_owned()],
        }
    );
}

#[test]
fn test_load_auto() {
    super::load().unwrap();
}
