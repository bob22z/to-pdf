use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use fontdb::{Database, Source};
use typst::text::{Font, FontBook, FontInfo};

/// Search everything that is available.
pub(crate) fn search_fonts(font_path: &Path) -> (FontBook, Vec<FontSlot>) {
    let mut book = FontBook::new();
    let mut fonts = Vec::default();
    let mut db = Database::new();

    db.load_fonts_dir(font_path);

    // System fonts have second priority.
    db.load_system_fonts();

    for face in db.faces() {
        let path = match &face.source {
            Source::File(path) | Source::SharedFile(path, _) => path,
            // We never add binary sources to the database, so there
            // shouln't be any.
            Source::Binary(_) => continue,
        };

        let info = db
            .with_face_data(face.id, FontInfo::new)
            .expect("database must contain this font");

        if let Some(info) = info {
            book.push(info);
            fonts.push(FontSlot {
                path: path.clone(),
                index: face.index,
                font: OnceLock::new(),
            });
        }
    }

    (book, fonts)
}

/// Holds details about the location of a font and lazily the font itself.
pub(crate) struct FontSlot {
    /// The path at which the font can be found on the system.
    path: PathBuf,
    /// The index of the font in its collection. Zero if the path does not point
    /// to a collection.
    index: u32,
    /// The lazily loaded font.
    font: OnceLock<Option<Font>>,
}

impl FontSlot {
    /// Get the font for this slot.
    pub(crate) fn get(&self) -> Option<Font> {
        self.font
            .get_or_init(|| {
                let data = fs::read(&self.path).ok()?.into();
                Font::new(data, self.index)
            })
            .clone()
    }
}
