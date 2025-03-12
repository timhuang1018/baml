//! Types and utilities for working with text, modifying source files, and `Ruff <-> LSP` type conversion.
mod range;
mod text_document;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use lsp_types::{PositionEncodingKind, Url};
pub(crate) use range::RangeExt;
pub(crate) use text_document::DocumentVersion;
pub use text_document::TextDocument;

/// A convenient enumeration for supported text encodings. Can be converted to [`lsp_types::PositionEncodingKind`].
// Please maintain the order from least to greatest priority for the derived `Ord` impl.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PositionEncoding {
    /// UTF 16 is the encoding supported by all LSP clients.
    #[default]
    UTF16,

    /// Second choice because UTF32 uses a fixed 4 byte encoding for each character (makes conversion relatively easy)
    UTF32,

    /// Ruff's preferred encoding
    UTF8,
}

/// A unique document ID, derived from a URL passed as part of an LSP request.
/// This document ID can point to either be a standalone Python file, a full notebook, or a cell within a notebook.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocumentKey(Url);

impl DocumentKey {
    /// Returns the URL associated with the key.
    pub(crate) fn url(&self) -> &Url {
        &self.0
    }

    /// A flexible constructor that can take any path delivered by the LSP
    /// client and convert it to a `DocumentKey`.
    /// 
    /// We sometimes see:
    ///   - file:///Users/someone/baml_src/test.baml
    ///   - /Users/someone/baml_src/test.baml
    ///   - test.baml
    ///   - /test.baml
    /// 
    /// These should all be converted to a URL with an absolute
    /// path appropriate for the user's system (e.g. Linux, MacOS, Windows)
    pub fn from_path(root_path: &Path, file_path: &Path) -> anyhow::Result<Self> {

        // Ensure that we have a relative path, by taking file_path
        // and stripping the root_path from it, if it's absolute.
        let relative_path = file_path.strip_prefix(root_path).unwrap_or(file_path);

        // Ensure our relative path doesn't begin with a path separator.
        let relative_path = relative_path.strip_prefix(std::path::MAIN_SEPARATOR_STR).unwrap_or(relative_path);

        let absolute_path = root_path.join(relative_path);
        let aboslute_url = Url::from_file_path(absolute_path).map_err(|_| anyhow::anyhow!("Could not convert path to URL"))?;
        Ok(DocumentKey(aboslute_url))
    }

    /// A flexible constructor that can take any URL delivered by the LSP.
    /// It uses the same logic as `DocumentKey::from_path`.
    pub fn from_url(root_path: &Path, url: &Url) -> anyhow::Result<Self> {
        Self::from_path(root_path, &PathBuf::from(&url.path()))
    }
}

impl std::fmt::Display for DocumentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<PositionEncoding> for lsp_types::PositionEncodingKind {
    fn from(value: PositionEncoding) -> Self {
        match value {
            PositionEncoding::UTF8 => lsp_types::PositionEncodingKind::UTF8,
            PositionEncoding::UTF16 => lsp_types::PositionEncodingKind::UTF16,
            PositionEncoding::UTF32 => lsp_types::PositionEncodingKind::UTF32,
        }
    }
}

impl TryFrom<&lsp_types::PositionEncodingKind> for PositionEncoding {
    type Error = ();

    fn try_from(value: &PositionEncodingKind) -> Result<Self, Self::Error> {
        Ok(if value == &PositionEncodingKind::UTF8 {
            PositionEncoding::UTF8
        } else if value == &PositionEncodingKind::UTF16 {
            PositionEncoding::UTF16
        } else if value == &PositionEncodingKind::UTF32 {
            PositionEncoding::UTF32
        } else {
            return Err(());
        })
    }
}
