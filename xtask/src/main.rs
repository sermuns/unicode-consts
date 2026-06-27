use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    str::FromStr,
};

const OUTPUT_PATH: &str = "src/lib.rs";

const UCD_URL: &str = "https://www.unicode.org/Public/UCD/latest/ucd/UnicodeData.txt";
const BLOCKS_URL: &str = "https://www.unicode.org/Public/UCD/latest/ucd/Blocks.txt";

#[derive(Debug)]
struct UnicodeBlock {
    // start: u32,
    end: u32,
    snake_case_name: String,
}

impl FromStr for UnicodeBlock {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (range, name) = s.split_once(';').unwrap();

        let (_start_str, end_str) = range.split_once("..").unwrap();

        // let start = u32::from_str_radix(start_str, 16)?;
        let end = u32::from_str_radix(end_str, 16)?;

        Ok(UnicodeBlock {
            // start,
            end,
            // don't know if we can avoid alloc
            snake_case_name: name.trim().replace([' ', '-'], "_").to_lowercase(),
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = File::create(OUTPUT_PATH)?;

    let ucd_reader = ureq::get(UCD_URL).call()?.into_body().into_reader();

    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b';')
        .from_reader(ucd_reader);

    let unicode_blocks_reader = ureq::get(BLOCKS_URL).call()?.into_body().into_reader();
    let mut unicode_blocks_lines = BufReader::new(unicode_blocks_reader)
        .lines()
        // consume the metadata comment at start of file
        .skip_while(|line| {
            let line = line.as_ref().unwrap();
            line.is_empty() || line.starts_with('#')
        });

    // FIXME: mother of unwraps
    let mut current_unicode_block = UnicodeBlock::from_str(&unicode_blocks_lines.next().unwrap()?)?;

    writeln!(
        writer,
        "pub mod {} {{",
        current_unicode_block.snake_case_name
    )?;

    for result in csv_reader.records() {
        let record = result?;

        let code_value_str = &record[0];

        let code_value = u32::from_str_radix(code_value_str, 16).unwrap();
        if code_value > current_unicode_block.end {
            // println!(
            //     "code value {} is outside of current block {:?}, moving to next block",
            //     code_value, current_unicode_block
            // );
            current_unicode_block =
                UnicodeBlock::from_str(&unicode_blocks_lines.next().unwrap().unwrap()).unwrap();
            writeln!(
                writer,
                "}}\npub mod {} {{",
                current_unicode_block.snake_case_name
            )?;
        }

        let character_name = &record[1];

        if character_name.starts_with('<') {
            // TODO: figure out why these exist..
            continue;
        }

        writeln!(
            writer,
            r#"    pub const {}: &str = "\u{{{}}}";"#,
            character_name.replace([' ', '-'], "_").to_uppercase(),
            code_value_str
        )?;
    }

    writeln!(writer, "}}")?;

    Ok(())
}
