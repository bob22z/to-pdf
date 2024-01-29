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
    let svg_list = to_pdf
        .to_svg(template.into(), Some(json.into()), now)
        .unwrap();

    let mut html = String::default();
    html.push_str("<html>");
    html.push_str("<body>");

    for svg_file in svg_list.into_iter() {
        html.push_str(&svg_file);
    }

    html.push_str("</body>");
    html.push_str("</html>");

    let mut file = std::fs::File::create("preview.html").unwrap();
    file.write_all(html.as_bytes()).unwrap();
}
