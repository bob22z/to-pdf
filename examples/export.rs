use std::{io::Write, path::PathBuf};

use to_pdf::ToPdf;

fn main() {
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
        .to_pdf(template.into(), Some(json.into()), now)
        .unwrap();

    let mut file = std::fs::File::create("export.pdf").unwrap();
    file.write_all(&pdf_content).unwrap();
}
