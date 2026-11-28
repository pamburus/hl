use std::path::Path;

use super::Reliability;

// ---

/// No file system is known to be local here, as there is no way to tell one apart.
pub fn classify(_path: &Path) -> Reliability {
    Reliability::NotConfirmed
}
