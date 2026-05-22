//! Strongly-typed identifiers.
//!
//! Each entity gets its own newtype around [`Uuid`] so the compiler rejects
//! passing, say, a [`JobId`] where a [`ProjectId`] is expected.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Declares a `Uuid` newtype with the standard derives and constructors.
macro_rules! typed_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generates a fresh random identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Wraps an existing [`Uuid`].
            #[must_use]
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }
    };
}

typed_id!(
    /// Identifies a registered code [`Project`](crate::model::Project).
    ProjectId
);
typed_id!(
    /// Identifies a single agent [`Job`](crate::model::Job).
    JobId
);
typed_id!(
    /// Identifies a [`Batch`](crate::model::Batch).
    BatchId
);
typed_id!(
    /// Identifies a [`BatchItem`](crate::model::BatchItem).
    BatchItemId
);
typed_id!(
    /// Identifies a [`PromptTemplate`](crate::model::PromptTemplate).
    TemplateId
);
typed_id!(
    /// Identifies one execution of a job or batch.
    RunId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_ids_are_unique() {
        assert_ne!(JobId::new(), JobId::new());
    }

    #[test]
    fn display_matches_inner_uuid() {
        let id = ProjectId::new();
        assert_eq!(id.to_string(), id.0.to_string());
    }
}
