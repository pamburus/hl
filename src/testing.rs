//! Testing utilities for internal use.

use crate::{
    model::{Caller, RawValue, Record},
    settings::{DisplayVariant, Formatting, MessageFormat, MessageFormatting},
    theme::Theme,
    themecfg,
};
use encstr::EncodedString;
use std::sync::Arc;

/// Creates a basic test record with stable properties for testing.
///
/// Note: Be careful when using this in tests that rely on specific record values.
/// Consider creating a record directly in the test if the specific values matter.
pub fn record() -> Record<'static> {
    Record {
        message: Some(RawValue::String(EncodedString::raw("test message"))),
        caller: Caller::with_name("test_caller"),
        ..Default::default()
    }
}

/// Creates a test record with file and line information.
///
/// Note: Be careful when using this in tests that rely on specific record values.
/// Consider creating a record directly in the test if the specific values matter.
pub fn record_with_source() -> Record<'static> {
    let mut rec = record();
    rec.caller = Caller::with_file_line("test_file.rs", "42");
    rec
}

/// Returns a new Arc-wrapped theme from the test theme configuration
pub fn theme() -> Arc<Theme> {
    Arc::new(Theme::from(themecfg::testing::theme().unwrap()))
}

/// Test settings and utilities for creating stable test configurations.
pub mod settings {
    use super::*;
    use crate::settings::Punctuation;

    /// Creates a stable Formatting object for tests.
    /// This ensures tests don't break when default settings change.
    pub fn formatting() -> Formatting {
        Formatting {
            flatten: None,
            message: MessageFormatting {
                format: MessageFormat::AutoQuoted,
            },
            punctuation: punctuation(),
        }
    }

    /// Creates a Punctuation object with stable test values.
    /// This ensures tests don't break when default configuration changes.
    pub fn punctuation() -> Punctuation {
        Punctuation {
            logger_name_separator: ":".into(),
            field_key_value_separator: "=".into(),
            string_opening_quote: "'".into(),
            string_closing_quote: "'".into(),
            source_location_separator: "@ ".into(),
            caller_name_file_separator: " :: ".into(),
            hidden_fields_indicator: " ...".into(),
            level_left_separator: "|".into(),
            level_right_separator: "|".into(),
            input_number_prefix: "#".into(),
            input_number_left_separator: "".into(),
            input_number_right_separator: " | ".into(),
            input_name_left_separator: "".into(),
            input_name_right_separator: " | ".into(),
            input_name_clipping: "..".into(),
            input_name_common_part: "..".into(),
            array_separator: ", ".into(),
            message_delimiter: "::".into(),
        }
    }
}

/// Test utilities for ASCII variants.
///
/// This module provides utilities for testing the ASCII variant rendering
/// of log output. It includes functions to create records with selective
/// ASCII/UTF-8 variants for formatting characters.
pub mod ascii {
    use super::*;

    /// Creates a record with selective ASCII variants for testing ASCII mode.
    ///
    /// This function creates a Record and adds Punctuation with selective ASCII variants
    /// that can be used to test the ASCII mode functionality.
    pub fn record() -> (Record<'static>, Formatting) {
        let record = super::record_with_source();

        // Create formatting with selective variants for ASCII mode testing
        let mut formatting = settings::formatting();
        formatting.punctuation.source_location_separator = DisplayVariant::Selective {
            ascii: "-> ".to_string(),
            utf8: "→ ".to_string(),
        };
        formatting.punctuation.input_number_right_separator = DisplayVariant::Selective {
            ascii: " | ".to_string(),
            utf8: " │ ".to_string(),
        };

        (record, formatting)
    }
}
