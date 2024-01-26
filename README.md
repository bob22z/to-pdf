# to-pdf

## About

[Crates.io](https://crates.io/crates/to-pdf)

[API Docs](https://docs.rs/to-pdf)

## Installation

Add the following to Cargo.toml:

```toml
[dependencies]
to-pdf = { git = "https://github.com/bob22z/to-pdf" }
```

## Usage

```rust
use to_pdf::ToPdf;

// Initialize with custom fonts
let to_pdf = ToPdf::new(FONT_PATH)?;

// Export to Pdf with typst text
let pdf_content = to_pdf.to_pdf(TYPST_TEXT)?;

// Preview as Svg with typst text
let svg_content_list = to_pdf.to_svg(TYPST_TEXT)?;
```

## License

MIT
