// third-party imports
use derive_more::{Deref, DerefMut};
use serde::Deserialize;

// relative imports
use super::Role;

// ---

/// Base style inheritance specification (v1 feature).
///
/// Specifies generic roles to inherit from (single or multiple).
/// Merge order is left-to-right (later roles override earlier ones).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deref, DerefMut)]
pub struct StyleBase(Vec<Role>);

impl StyleBase {
    /// Create an empty StyleBase (no roles).
    pub const fn new() -> Self {
        Self(Vec::new())
    }
}

impl From<Role> for StyleBase {
    fn from(role: Role) -> Self {
        Self(vec![role])
    }
}

impl From<Vec<Role>> for StyleBase {
    fn from(roles: Vec<Role>) -> Self {
        Self(roles)
    }
}

impl<'de> Deserialize<'de> for StyleBase {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, SeqAccess, Visitor};

        struct StyleBaseVisitor;

        impl<'de> Visitor<'de> for StyleBaseVisitor {
            type Value = StyleBase;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a role name or array of role names")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                let role: Role = serde_plain::from_str(value).map_err(de::Error::custom)?;
                Ok(StyleBase(vec![role]))
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut roles = Vec::new();
                while let Some(value) = seq.next_element::<String>()? {
                    let role: Role = serde_plain::from_str(&value).map_err(de::Error::custom)?;
                    roles.push(role);
                }
                Ok(StyleBase(roles))
            }
        }

        deserializer.deserialize_any(StyleBaseVisitor)
    }
}

impl std::fmt::Display for StyleBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, role) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{}", role)?;
        }
        write!(f, "]")
    }
}
