//! File paths stored in a document reach the file system only through [`resolve`]: a relative path
//! is taken from the document's folder, and only local files, or files on the network share the
//! document itself is on, are opened (see `craft_docpath`). A refused path is handled like a missing
//! file, with the reason.
//!
//! The only file paths a `.figcraft` document stores are its library links
//! (`doc.libraries.enabled[].path`; images are embedded and fonts are named by family). Paths the
//! user picks in this session (Add library, the control channel) are not stored-document paths and
//! are read as given ([`crate::library::Cache::choose`]).

use std::fmt;
use std::path::{Path, PathBuf};

pub use craft_docpath::Refused;

/// Why a stored path is not followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotFollowed {
    /// The path is not a local file, or is not on the document's own share.
    Refused(Refused),
    /// The path is relative and the document has no file to take it from.
    NoDocumentFolder,
}

impl fmt::Display for NotFollowed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(r) => r.fmt(f),
            Self::NoDocumentFolder => f.write_str("the path is relative and the file has not been saved"),
        }
    }
}

impl std::error::Error for NotFollowed {}

/// The file to open for `stored`, a path or `file:` URL stored in the document saved at `document`
/// (None: a document without a file). A relative `document` is taken from the working directory,
/// where it was opened from.
pub fn resolve(stored: &str, document: Option<&Path>) -> Result<String, NotFollowed> {
    let document = document.filter(|d| !d.as_os_str().is_empty()).map(|d| std::path::absolute(d).unwrap_or_else(|_| d.to_path_buf()));
    let path: PathBuf = craft_docpath::resolve(stored, document.as_deref()).map_err(NotFollowed::Refused)?;
    // A path that is still relative would be taken from the working directory, which the
    // document does not choose.
    if path.is_relative() {
        return Err(NotFollowed::NoDocumentFolder);
    }
    path.into_os_string().into_string().map_err(|_| NotFollowed::Refused(Refused::Invalid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_are_taken_from_the_document_folder() {
        let dir = std::env::temp_dir().join("figs");
        let doc = dir.join("app.figcraft");
        assert_eq!(resolve("kit.figcraft", Some(&doc)), Ok(dir.join("kit.figcraft").to_string_lossy().into_owned()));
        assert_eq!(resolve("kit.figcraft", None), Err(NotFollowed::NoDocumentFolder));
        assert_eq!(resolve(r"\\192.0.2.1\share\kit.figcraft", Some(&doc)), Err(NotFollowed::Refused(Refused::Network)));
        assert_eq!(resolve("file://192.0.2.1/s/kit.figcraft", Some(&doc)), Err(NotFollowed::Refused(Refused::Network)));
    }
}
