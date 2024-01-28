use std::{
    collections::HashMap,
    fs, mem,
    path::{Path, PathBuf},
};

use comemo::Prehashed;
use ecow::{eco_format, EcoString};
use parking_lot::Mutex;
use time::OffsetDateTime;
use typst::{
    diag::{FileError, FileResult, StrResult},
    eval::Tracer,
    foundations::{Bytes, Datetime, Dict},
    model::Document,
    syntax::{FileId, Source, VirtualPath},
    text::{Font, FontBook},
    Library, World,
};

use crate::{
    fonts::{search_fonts, FontSlot},
    package::prepare_package,
};

pub struct ToPdf {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<FontSlot>,
    root: PathBuf,
    slots: Mutex<HashMap<FileId, FileSlot>>,
}

impl ToPdf {
    pub fn new(font_path: &PathBuf) -> StrResult<Self> {
        let (book, fonts) = search_fonts(font_path);

        let root = {
            let path = Path::new(".");
            path.canonicalize().map_err(|_| {
                eco_format!("root directory not found (searched at {})", path.display())
            })?
        };

        let library = Library::builder().with_inputs(Dict::default()).build();

        Ok(Self {
            library: Prehashed::new(library),
            book: Prehashed::new(book),
            fonts,
            root,
            slots: Mutex::new(HashMap::new()),
        })
    }

    pub fn to_pdf(
        &self,
        template_source: String,
        template_json: Option<String>,
        time: OffsetDateTime,
    ) -> StrResult<Vec<u8>> {
        let document = self.compile(template_source, template_json, time)?;

        Ok(export_pdf(document))
    }

    pub fn to_svg(
        &self,
        template_source: String,
        template_json: Option<String>,
        time: OffsetDateTime,
    ) -> StrResult<Vec<String>> {
        let document = self.compile(template_source, template_json, time)?;

        Ok(export_svg(document))
    }

    fn compile(
        &self,
        template_source: String,
        template_json: Option<String>,
        time: OffsetDateTime,
    ) -> StrResult<Document> {
        let world = self.with_source(template_source, template_json, time);

        let mut tracer = Tracer::new();
        typst::compile(&world, &mut tracer)
            .map_err(|errors| EcoString::from(format!("{:?}", errors)))
    }

    fn with_source(
        &self,
        template_source: String,
        template_json: Option<String>,
        time: OffsetDateTime,
    ) -> ToPdfWithSource<'_> {
        let main_file_id = FileId::new(None, VirtualPath::new("main.typ"));
        let main_source = Source::new(main_file_id, template_source);

        let main_json = template_json.unwrap_or("{}".into()).into_bytes().into();
        ToPdfWithSource {
            to_pdf: self,
            main_source,
            main_json,
            time,
        }
    }

    fn slot<F, T>(&self, id: FileId, f: F) -> T
    where
        F: FnOnce(&mut FileSlot) -> T,
    {
        let mut map = self.slots.lock();
        f(map.entry(id).or_insert_with(|| FileSlot::new(id)))
    }
}

fn export_pdf(document: Document) -> Vec<u8> {
    typst_pdf::pdf(&document, None, None)
}

fn export_svg(document: Document) -> Vec<String> {
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
    document
        .pages
        .par_iter()
        .map(|page| typst_svg::svg(&page.frame))
        .collect()
}

struct ToPdfWithSource<'a> {
    to_pdf: &'a ToPdf,
    main_source: Source,
    main_json: Bytes,
    time: OffsetDateTime,
}

impl<'a> World for ToPdfWithSource<'a> {
    fn library(&self) -> &Prehashed<Library> {
        &self.to_pdf.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.to_pdf.book
    }

    fn main(&self) -> Source {
        self.main_source.clone()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.to_pdf.slot(id, |slot| slot.source(&self.to_pdf.root))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id.vpath().as_rooted_path().to_str() == Some("/main.json") {
            Ok(self.main_json.clone())
        } else {
            self.to_pdf.slot(id, |slot| slot.file(&self.to_pdf.root))
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.to_pdf.fonts[index].get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let offset = offset.unwrap_or(0);
        let offset = time::UtcOffset::from_hms(offset.try_into().ok()?, 0, 0).ok()?;
        let time = self.time.checked_to_offset(offset)?;
        Some(Datetime::Date(time.date()))
    }
}

/// Holds the processed data for a file ID.
///
/// Both fields can be populated if the file is both imported and read().
#[derive(Clone)]
struct FileSlot {
    /// The slot's file id.
    id: FileId,
    /// The lazily loaded and incrementally updated source file.
    source: SlotCell<Source>,
    /// The lazily loaded raw byte buffer.
    file: SlotCell<Bytes>,
}

/// Lazily processes data for a file.
#[derive(Clone)]
struct SlotCell<T> {
    /// The processed data.
    data: Option<FileResult<T>>,
    /// A hash of the raw file contents / access error.
    fingerprint: u128,
    /// Whether the slot has been accessed in the current compilation.
    accessed: bool,
}

impl FileSlot {
    /// Create a new path slot.
    fn new(id: FileId) -> Self {
        Self {
            id,
            file: SlotCell::new(),
            source: SlotCell::new(),
        }
    }

    /// Retrieve the source for this file.
    fn source(&mut self, project_root: &Path) -> FileResult<Source> {
        self.source.get_or_init(
            || system_path(project_root, self.id),
            |data, prev| {
                let text = decode_utf8(&data)?;
                if let Some(mut prev) = prev {
                    prev.replace(text);
                    Ok(prev)
                } else {
                    Ok(Source::new(self.id, text.into()))
                }
            },
        )
    }

    /// Retrieve the file's bytes.
    fn file(&mut self, project_root: &Path) -> FileResult<Bytes> {
        self.file.get_or_init(
            || system_path(project_root, self.id),
            |data, _| Ok(data.into()),
        )
    }
}

impl<T: Clone> SlotCell<T> {
    /// Creates a new, empty cell.
    fn new() -> Self {
        Self {
            data: None,
            fingerprint: 0,
            accessed: false,
        }
    }

    /// Gets the contents of the cell or initialize them.
    fn get_or_init(
        &mut self,
        path: impl FnOnce() -> FileResult<PathBuf>,
        f: impl FnOnce(Vec<u8>, Option<T>) -> FileResult<T>,
    ) -> FileResult<T> {
        // If we accessed the file already in this compilation, retrieve it.
        if mem::replace(&mut self.accessed, true) {
            if let Some(data) = &self.data {
                return data.clone();
            }
        }

        // Read and hash the file.
        let result = path().and_then(|p| read(&p));
        let fingerprint = typst::util::hash128(&result);

        // If the file contents didn't change, yield the old processed data.
        if mem::replace(&mut self.fingerprint, fingerprint) == fingerprint {
            if let Some(data) = &self.data {
                return data.clone();
            }
        }

        let prev = self.data.take().and_then(Result::ok);
        let value = result.and_then(|data| f(data, prev));
        self.data = Some(value.clone());

        value
    }
}

/// Resolves the path of a file id on the system, downloading a package if
/// necessary.
fn system_path(project_root: &Path, id: FileId) -> FileResult<PathBuf> {
    // Determine the root path relative to which the file path
    // will be resolved.
    let buf;
    let mut root = project_root;
    if let Some(spec) = id.package() {
        buf = prepare_package(spec)?;
        root = &buf;
    }

    // Join the path to the root. If it tries to escape, deny
    // access. Note: It can still escape via symlinks.
    id.vpath().resolve(root).ok_or(FileError::AccessDenied)
}

/// Decode UTF-8 with an optional BOM.
fn decode_utf8(buf: &[u8]) -> FileResult<&str> {
    // Remove UTF-8 BOM.
    Ok(std::str::from_utf8(
        buf.strip_prefix(b"\xef\xbb\xbf").unwrap_or(buf),
    )?)
}

/// Read a file.
fn read(path: &Path) -> FileResult<Vec<u8>> {
    let f = |e| FileError::from_io(e, path);
    if fs::metadata(path).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(path).map_err(f)
    }
}
