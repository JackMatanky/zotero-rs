//! Controlled vocabulary value types for Zotero entities.
//!
//! Provides enumerations for item types, annotation kinds, attachment storage
//! modes, and collection parent relationships. Unknown API values are preserved
//! in each enum's catch-all variant to ensure lossless round-tripping.
//!
//! # Main Types
//!
//! - [`ItemType`]: Item classification (`journalArticle`, `book`, etc.).
//! - [`AnnotationType`]: PDF annotation kind (`highlight`, `underline`, etc.).
//! - [`LinkMode`]: Attachment storage mode (`imported_file`, `linked_file`,
//!   etc.).
//! - [`CollectionParent`]: Parent collection state
//!   ([`CollectionParent::TopLevel`] or [`CollectionParent::Parent`]).
//!
//! # Examples
//!
//! ```
//! use zotero_api::ItemType;
//!
//! let item_type = ItemType::from("journalArticle");
//! assert_eq!(item_type, ItemType::JournalArticle);
//! assert_eq!(item_type.as_str(), "journalArticle");
//! ```

use serde::{Deserialize, Serialize};

use crate::keys::CollectionKey;

open_string_enum! {
    /// Zotero item kind carried in the `itemType` field.
    ///
    /// Only variants this crate branches on are named explicitly. Every other
    /// Zotero item type, such as `webpage`, `bookSection`, or `thesis`,
    /// round-trips through [`ItemType::Other`] with its original API string
    /// preserved.
    pub enum ItemType {
        /// Annotation item (`annotation`).
        Annotation => "annotation",
        /// Artwork item (`artwork`).
        Artwork => "artwork",
        /// Attachment item (`attachment`).
        Attachment => "attachment",
        /// Audio recording item (`audioRecording`).
        AudioRecording => "audioRecording",
        /// Bill item (`bill`).
        Bill => "bill",
        /// Blog post item (`blogPost`).
        BlogPost => "blogPost",
        /// Book item (`book`).
        Book => "book",
        /// Book section item (`bookSection`).
        BookSection => "bookSection",
        /// Case item (`case`).
        Case => "case",
        /// Computer program item (`computerProgram`).
        ComputerProgram => "computerProgram",
        /// Conference paper item (`conferencePaper`).
        ConferencePaper => "conferencePaper",
        /// Dictionary entry item (`dictionaryEntry`).
        DictionaryEntry => "dictionaryEntry",
        /// Document item (`document`).
        Document => "document",
        /// Email item (`email`).
        Email => "email",
        /// Encyclopedia article item (`encyclopediaArticle`).
        EncyclopediaArticle => "encyclopediaArticle",
        /// Film item (`film`).
        Film => "film",
        /// Forum post item (`forumPost`).
        ForumPost => "forumPost",
        /// Hearing item (`hearing`).
        Hearing => "hearing",
        /// Instant message item (`instantMessage`).
        InstantMessage => "instantMessage",
        /// Interview item (`interview`).
        Interview => "interview",
        /// Journal article item (`journalArticle`).
        JournalArticle => "journalArticle",
        /// Letter item (`letter`).
        Letter => "letter",
        /// Magazine article item (`magazineArticle`).
        MagazineArticle => "magazineArticle",
        /// Manuscript item (`manuscript`).
        Manuscript => "manuscript",
        /// Map item (`map`).
        Map => "map",
        /// Newspaper article item (`newspaperArticle`).
        NewspaperArticle => "newspaperArticle",
        /// Note item (`note`).
        Note => "note",
        /// Patent item (`patent`).
        Patent => "patent",
        /// Podcast item (`podcast`).
        Podcast => "podcast",
        /// Preprint item (`preprint`).
        Preprint => "preprint",
        /// Presentation item (`presentation`).
        Presentation => "presentation",
        /// Radio broadcast item (`radioBroadcast`).
        RadioBroadcast => "radioBroadcast",
        /// Report item (`report`).
        Report => "report",
        /// Statute item (`statute`).
        Statute => "statute",
        /// Thesis item (`thesis`).
        Thesis => "thesis",
        /// TV broadcast item (`tvBroadcast`).
        TvBroadcast => "tvBroadcast",
        /// Video recording item (`videoRecording`).
        VideoRecording => "videoRecording",
        /// Webpage item (`webpage`).
        Webpage => "webpage",
    }
}

impl ItemType {
    /// Returns `true` if this item type is eligible for search and embedding
    /// indexing.
    ///
    /// Excludes [`Attachment`](Self::Attachment), [`Note`](Self::Note), and
    /// [`Annotation`](Self::Annotation). These are auxiliary content, not
    /// standalone searchable items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zotero_api::ItemType;
    ///
    /// assert!(ItemType::JournalArticle.is_indexable());
    /// assert!(!ItemType::Attachment.is_indexable());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_indexable(&self) -> bool {
        !matches!(self, Self::Attachment | Self::Note | Self::Annotation)
    }
}

impl Default for ItemType {
    #[inline]
    fn default() -> Self {
        Self::Other(String::new())
    }
}

open_string_enum! {
    /// PDF annotation kind carried in the `annotationType` field.
    ///
    /// Falls back to [`AnnotationType::Other`] for annotation kinds this crate
    /// does not create, such as `image` or `ink`.
    pub enum AnnotationType {
        /// Text highlight annotation (`highlight`).
        Highlight => "highlight",
        /// Text underline annotation (`underline`).
        Underline => "underline",
        /// Standalone PDF note annotation (`note`).
        Note => "note",
    }
}

open_string_enum! {
    /// Creator role carried in the `creatorType` field.
    ///
    /// Zotero defines many item-type-specific creator roles. The common roles
    /// are named explicitly, while [`CreatorType::Other`] preserves anything
    /// else for round-tripping.
    pub enum CreatorType {
        /// Primary author or creator (`author`).
        Author => "author",
        /// Editor (`editor`).
        Editor => "editor",
        /// Translator (`translator`).
        Translator => "translator",
    }
}

open_string_enum! {
    /// Attachment storage mode carried in the `linkMode` field.
    pub enum LinkMode {
        /// File stored directly inside Zotero's storage directory
        /// (`imported_file`).
        ImportedFile => "imported_file",
        /// File linked from an external filesystem path (`linked_file`).
        LinkedFile => "linked_file",
        /// Web page or remote URL link (`linked_url`).
        LinkedUrl => "linked_url",
        /// Saved HTML snapshot or imported URL content (`imported_url`).
        ImportedUrl => "imported_url",
    }
}

/// Parent relationship for a Zotero collection.
///
/// On the wire, Zotero encodes this as either `false` (top-level) or a string
/// containing the parent collection's key. This enum deserializes both forms
/// transparently.
///
/// # Examples
///
/// ```
/// use zotero_api::{CollectionKey, CollectionParent};
///
/// let top: CollectionParent =
///     serde_json::from_value(serde_json::json!(false)).unwrap();
/// assert_eq!(top, CollectionParent::TopLevel);
///
/// let child: CollectionParent =
///     serde_json::from_value(serde_json::json!("ABC123")).unwrap();
/// assert_eq!(child, CollectionParent::Parent(CollectionKey::from("ABC123")));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "serde_json::Value", into = "serde_json::Value")]
pub enum CollectionParent {
    /// Top-level collection with no parent collection.
    #[default]
    TopLevel,
    /// Child collection belonging to a parent collection identified by
    /// [`CollectionKey`].
    Parent(CollectionKey),
}

impl From<serde_json::Value> for CollectionParent {
    #[inline]
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::String(s) if !s.is_empty() && s != "false" => {
                Self::Parent(CollectionKey::from(s))
            }
            _ => Self::TopLevel,
        }
    }
}

impl From<CollectionParent> for serde_json::Value {
    #[inline]
    fn from(value: CollectionParent) -> Self {
        match value {
            CollectionParent::TopLevel => Self::Bool(false),
            CollectionParent::Parent(key) => {
                Self::String(key.as_str().to_owned())
            }
        }
    }
}

/// Tag source carried in Zotero's numeric `type` field.
///
/// Zotero uses `0` for user-created tags and `1` for tags assigned
/// automatically on import.
///
/// # Examples
///
/// ```
/// use zotero_api::TagOrigin;
///
/// assert_eq!(TagOrigin::from(0), TagOrigin::User);
/// assert_eq!(TagOrigin::from(1), TagOrigin::Automatic);
/// assert_eq!(TagOrigin::from(42), TagOrigin::Other(42));
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize,
)]
#[serde(from = "u8", into = "u8")]
pub enum TagOrigin {
    /// Tag explicitly created by a user (`0`).
    #[default]
    User,
    /// Tag assigned automatically on import or export (`1`).
    Automatic,
    /// Any origin value outside Zotero's documented `0`/`1`; carries the
    /// original integer.
    Other(u8),
}

impl From<u8> for TagOrigin {
    #[inline]
    fn from(value: u8) -> Self {
        match value {
            0 => Self::User,
            1 => Self::Automatic,
            other => Self::Other(other),
        }
    }
}

impl From<TagOrigin> for u8 {
    #[inline]
    fn from(value: TagOrigin) -> Self {
        match value {
            TagOrigin::User => 0,
            TagOrigin::Automatic => 1,
            TagOrigin::Other(other) => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod item_type {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn round_trips_known_and_unknown_item_types() {
            let article = ItemType::JournalArticle;
            let article_str: String = article.clone().into();
            assert_eq!(article_str, "journalArticle");
            assert_eq!(ItemType::from(article_str), article);

            let custom = ItemType::from("customWebpage".to_owned());
            let custom_str: String = custom.clone().into();
            assert_eq!(custom_str, "customWebpage");
            assert_eq!(custom, ItemType::Other("customWebpage".to_owned()));
        }

        #[test]
        fn round_trips_all_35_zotero_item_types() {
            let types = vec![
                (ItemType::Annotation, "annotation"),
                (ItemType::Artwork, "artwork"),
                (ItemType::Attachment, "attachment"),
                (ItemType::AudioRecording, "audioRecording"),
                (ItemType::Bill, "bill"),
                (ItemType::BlogPost, "blogPost"),
                (ItemType::Book, "book"),
                (ItemType::BookSection, "bookSection"),
                (ItemType::Case, "case"),
                (ItemType::ComputerProgram, "computerProgram"),
                (ItemType::ConferencePaper, "conferencePaper"),
                (ItemType::DictionaryEntry, "dictionaryEntry"),
                (ItemType::Document, "document"),
                (ItemType::Email, "email"),
                (ItemType::EncyclopediaArticle, "encyclopediaArticle"),
                (ItemType::Film, "film"),
                (ItemType::ForumPost, "forumPost"),
                (ItemType::Hearing, "hearing"),
                (ItemType::InstantMessage, "instantMessage"),
                (ItemType::Interview, "interview"),
                (ItemType::JournalArticle, "journalArticle"),
                (ItemType::Letter, "letter"),
                (ItemType::MagazineArticle, "magazineArticle"),
                (ItemType::Manuscript, "manuscript"),
                (ItemType::Map, "map"),
                (ItemType::NewspaperArticle, "newspaperArticle"),
                (ItemType::Note, "note"),
                (ItemType::Patent, "patent"),
                (ItemType::Podcast, "podcast"),
                (ItemType::Preprint, "preprint"),
                (ItemType::Presentation, "presentation"),
                (ItemType::RadioBroadcast, "radioBroadcast"),
                (ItemType::Report, "report"),
                (ItemType::Statute, "statute"),
                (ItemType::Thesis, "thesis"),
                (ItemType::TvBroadcast, "tvBroadcast"),
                (ItemType::VideoRecording, "videoRecording"),
                (ItemType::Webpage, "webpage"),
            ];

            for (variant, expected_str) in types {
                assert_eq!(variant.as_str(), expected_str);
                let stringified: String = variant.clone().into();
                assert_eq!(stringified, expected_str);
                assert_eq!(ItemType::from(expected_str), variant);
            }
        }

        #[test]
        fn defaults_to_other_variant() {
            assert_eq!(ItemType::default(), ItemType::Other(String::new()));
        }

        #[test]
        fn is_indexable_excludes_attachments_notes_and_annotations() {
            for item_type in
                [ItemType::Attachment, ItemType::Note, ItemType::Annotation]
            {
                assert!(
                    !item_type.is_indexable(),
                    "{item_type:?} must not be indexable"
                );
            }
            assert!(ItemType::JournalArticle.is_indexable());
            assert!(ItemType::Other("webpage".to_owned()).is_indexable());
        }
    }

    mod annotation_type {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn round_trips_known_and_unknown_annotation_types() {
            let highlight = AnnotationType::Highlight;
            let highlight_str: String = highlight.clone().into();
            assert_eq!(highlight_str, "highlight");
            assert_eq!(AnnotationType::from(highlight_str), highlight);

            let ink = AnnotationType::from("ink".to_owned());
            let ink_str: String = ink.clone().into();
            assert_eq!(ink_str, "ink");
            assert_eq!(ink, AnnotationType::Other("ink".to_owned()));
        }
    }

    mod creator_type {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn creator_type_round_trips_author_editor_and_other() {
            for (value, expected) in [
                ("author", CreatorType::Author),
                ("editor", CreatorType::Editor),
                ("reviewer", CreatorType::Other("reviewer".to_owned())),
            ] {
                let creator_type = CreatorType::from(value.to_owned());
                let serialized: String = creator_type.clone().into();

                assert_eq!(creator_type, expected, "case {value}");
                assert_eq!(serialized, value, "case {value}");
            }
        }
    }

    mod tag_origin {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn converts_user_automatic_and_other_variants() {
            assert_eq!(TagOrigin::from(0), TagOrigin::User);
            assert_eq!(TagOrigin::from(1), TagOrigin::Automatic);
            assert_eq!(TagOrigin::from(42), TagOrigin::Other(42));

            let user_num: u8 = TagOrigin::User.into();
            let auto_num: u8 = TagOrigin::Automatic.into();
            let other_num: u8 = TagOrigin::Other(42).into();
            assert_eq!(user_num, 0);
            assert_eq!(auto_num, 1);
            assert_eq!(other_num, 42);
        }
    }

    mod link_mode {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn round_trips_known_and_unknown_link_modes() {
            let imported = LinkMode::ImportedFile;
            let imported_str: String = imported.clone().into();
            assert_eq!(imported_str, "imported_file");
            assert_eq!(LinkMode::from(imported_str), imported);

            let custom = LinkMode::from("custom_mode".to_owned());
            let custom_str: String = custom.clone().into();
            assert_eq!(custom_str, "custom_mode");
            assert_eq!(custom, LinkMode::Other("custom_mode".to_owned()));
        }
    }

    mod collection_parent {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn serializes_top_level_as_false_and_parent_as_key() {
            let top_level: serde_json::Value =
                CollectionParent::TopLevel.into();
            assert_eq!(top_level, serde_json::json!(false));

            let parent: serde_json::Value =
                CollectionParent::Parent(CollectionKey::from("PARENT01"))
                    .into();
            assert_eq!(parent, serde_json::json!("PARENT01"));
        }

        #[test]
        fn treats_false_null_and_string_false_as_top_level() {
            assert_eq!(
                CollectionParent::from(serde_json::json!(false)),
                CollectionParent::TopLevel
            );
            assert_eq!(
                CollectionParent::from(serde_json::Value::Null),
                CollectionParent::TopLevel
            );
            assert_eq!(
                CollectionParent::from(serde_json::json!("false")),
                CollectionParent::TopLevel
            );
        }
    }
}
