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
let font_path = PathBuf::from(".");
let to_pdf = ToPdf::new(&font_path).unwrap();

// Typst template & data
let template = r#"
    #let forecast(day) = block[
      #box(square(
        width: 2cm,
        inset: 8pt,
        fill: if day.weather == "sunny" {
          yellow
        } else {
          aqua
        },
        align(
          bottom + right,
          strong(day.weather),
        ),
      ))
      #h(6pt)
      #set text(22pt, baseline: -8pt)
      #day.temperature °#day.unit
    ]

    #forecast(json("main.json"))
"#;

let json = r#"
    {
      "weather": "sunny",
      "temperature": "23",
      "unit": "c"
    }
"#;


let now = time::OffsetDateTime::now_utc();

// Export to Pdf with typst text
let pdf_content = to_pdf
    .to_pdf(content.into(), Some(main_json.into()), now)
    .unwrap();

// Preview as Svg with typst text
let svg_content = to_pdf
    .to_svg(content.into(), Some(main_json.into()), now)
    .unwrap();
```

## License

MIT
