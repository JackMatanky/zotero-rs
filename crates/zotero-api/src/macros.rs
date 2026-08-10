//! Shared macros for string-backed newtypes and open (unknown-value-preserving)
//! enums.

/// Generates a `String`-backed newtype with standard conversions.
///
/// Generates: `Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq,
/// PartialOrd, Serialize`, `#[serde(transparent)]`, `as_str()`, `Display`,
/// `From<String>`, `From<&str>`, `AsRef<str>`, `PartialEq<str>`,
/// `PartialEq<&str>`, `PartialEq<$name> for str`.
///
/// Accepts a leading visibility (e.g. `pub`, `pub(crate)`) applied to both the
/// generated struct and its `as_str()` accessor.
macro_rules! string_newtype {
    ($vis:vis $name:ident, $doc:expr) => {
        string_newtype!($vis $name, $doc,);
    };
    ($vis:vis $name:ident, $doc:expr, $($extra:ident),* $(,)?) => {
        #[doc = $doc]
        #[derive(
            Clone,
            Debug,
            Default,
            Deserialize,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            Serialize,
            $($extra,)*
        )]
        #[serde(transparent)]
        $vis struct $name(String);

        impl $name {
            /// Constructs a new instance from any string-like type.
            #[inline]
            $vis fn new<S: Into<String>>(value: S) -> Self {
                Self(value.into())
            }

            /// Returns the inner string slice.
            #[inline]
            $vis fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            #[inline]
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            #[inline]
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl AsRef<str> for $name {
            #[inline]
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl PartialEq<str> for $name {
            #[inline]
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }
        impl PartialEq<&str> for $name {
            #[inline]
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<$name> for str {
            #[inline]
            fn eq(&self, other: &$name) -> bool {
                self == other.as_str()
            }
        }
    };
}

/// Generates an "open" `String`-backed enum: named variants plus an automatic
/// `Other(String)` case that losslessly preserves any wire value this crate
/// does not name explicitly, so serialization round-trips without dropping
/// unrecognized values.
///
/// Generates: the enum (with the trailing `Other(String)` variant; do not
/// list it), `#[derive(Debug, Clone, PartialEq, Eq, Serialize,
/// Deserialize)]`, `#[serde(from = "String", into = "String")]`, an inherent
/// `as_str(&self) -> &str`, `From<String>`, `From<&str>`, and `From<$name> for
/// String`.
///
/// Each variant is written as `Variant => "wireValue",` and may carry its own
/// doc comment. Type-specific inherent methods (e.g. a `Default` impl or an
/// extra predicate method) are NOT generated. Write a separate `impl $name`
/// block after the macro invocation for those.
macro_rules! open_string_enum {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => $wire:expr,
            )+
        }
    ) => {
        $(#[$enum_meta])*
        #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        #[serde(from = "String", into = "String")]
        $vis enum $name {
            $(
                $(#[$variant_meta])*
                $variant,
            )+
            /// Any value not modeled above; carries the original API string.
            Other(String),
        }

        impl $name {
            /// Borrows the API string representation of this value.
            #[must_use]
            #[inline]
            $vis fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)+
                    Self::Other(value) => value,
                }
            }
        }

        impl From<String> for $name {
            #[inline]
            fn from(value: String) -> Self {
                match value.as_str() {
                    $($wire => Self::$variant,)+
                    _ => Self::Other(value),
                }
            }
        }

        impl From<&str> for $name {
            #[inline]
            fn from(value: &str) -> Self {
                Self::from(value.to_owned())
            }
        }

        impl From<$name> for String {
            #[inline]
            fn from(value: $name) -> Self {
                match value {
                    $name::Other(value) => value,
                    known => known.as_str().to_owned(),
                }
            }
        }
    };
}
