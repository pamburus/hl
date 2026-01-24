use super::*;

#[test]
#[cfg(feature = "clap")]
fn test_clap_parser_default() {
    let _ = ClapParser::<()>::default();
}
