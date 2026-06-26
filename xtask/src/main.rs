use std::{fs::File, io::Write};

const OUTPUT_PATH: &str = "src/lib.rs";

const UCD_URL: &str = "https://www.unicode.org/Public/UCD/latest/ucd/UnicodeData.txt";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ucd_reader = ureq::get(UCD_URL).call()?.into_body().into_reader();

    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b';')
        .from_reader(ucd_reader);

    let mut writer = File::create(OUTPUT_PATH)?;
    for result in csv_reader.records() {
        let record = result?;

        let code_value = &record[0];

        let character_name = &record[1];

        if character_name.starts_with('<') {
            // TODO: figure out why these exist..
            continue;
        }

        writeln!(
            writer,
            r#"pub const {}: &str = "\u{{{}}}";"#,
            character_name.replace([' ', '-'], "_").to_uppercase(),
            code_value
        )?;
    }

    Ok(())
}
