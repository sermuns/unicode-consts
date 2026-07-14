use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use toml::{Table, value::Array};

const LIB_OUTPUT_PATH: &str = "src/lib.rs";
const CARGO_TOML_OUTPUT_PATH: &str = "Cargo.toml";

const UCD_URL: &str = "https://www.unicode.org/Public/UCD/latest/ucd/UnicodeData.txt";
const BLOCKS_URL: &str = "https://www.unicode.org/Public/UCD/latest/ucd/Blocks.txt";

// TODO:
// const ALIASES_URL: &str = "https://www.unicode.org/Public/UCD/latest/ucd/NameAliases.txt";

#[derive(Debug)]
struct UnicodeBlock {
    start: u32,
    end: u32,
    snake_case_name: String,
}

#[derive(Deserialize, Serialize)]
struct CargoToml {
    package: Package,
    features: Option<Table>,
}

#[derive(Deserialize, Serialize)]
struct Package {
    name: String,
    version: String,
    edition: String,
    description: String,
    repository: String,
    license: String,
    categories: Vec<String>,
}

impl FromStr for UnicodeBlock {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (range, name) = s.split_once(';').unwrap();

        let (start_str, end_str) = range.split_once("..").unwrap();

        let start = u32::from_str_radix(start_str, 16)?;
        let end = u32::from_str_radix(end_str, 16)?;

        let snake_case_name = as_snake_case(name);

        Ok(UnicodeBlock {
            start,
            end,
            snake_case_name,
        })
    }
}

fn as_snake_case(s: &str) -> String {
    s.trim().replace([' ', '-'], "_").to_lowercase()
}

fn enter_unicode_block(
    writer: &mut impl Write,
    unicode_block: &UnicodeBlock,
    features: &mut Table,
) -> std::io::Result<()> {
    println!("doing unicode block '{}'", unicode_block.snake_case_name);

    writeln!(
        writer,
        "/// {:04X}..{:04X}",
        unicode_block.start, unicode_block.end
    )?;

    writeln!(
        writer,
        r#"#[cfg(feature = "{}")]"#,
        unicode_block.snake_case_name,
    )?;

    writeln!(writer, "pub mod {} {{", unicode_block.snake_case_name)?;

    features.insert(
        unicode_block.snake_case_name.clone(),
        toml::Value::Array(Array::new()),
    );

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut features = Table::new();

    let mut lib_writer = File::create(LIB_OUTPUT_PATH)?;
    writeln!(lib_writer, "#![no_std]").unwrap();

    let ucd_reader = ureq::get(UCD_URL).call()?.into_body().into_reader();
    println!("downloading '{}'.", UCD_URL);

    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b';')
        .from_reader(ucd_reader);
    println!("creating CSV reader for '{}'.", UCD_URL);

    let unicode_blocks_reader = ureq::get(BLOCKS_URL).call()?.into_body().into_reader();
    println!("downloading '{}'.", BLOCKS_URL);

    let mut unicode_blocks_lines = BufReader::new(unicode_blocks_reader)
        .lines()
        // consume the metadata comment at start of file
        .skip_while(|line| {
            let line = line.as_ref().unwrap();
            line.is_empty() || line.starts_with('#')
        });
    println!("creating line reader for '{}'.", BLOCKS_URL);

    // FIXME: mother of unwraps
    let mut current_unicode_block = UnicodeBlock::from_str(&unicode_blocks_lines.next().unwrap()?)?;
    enter_unicode_block(&mut lib_writer, &current_unicode_block, &mut features)?;

    for result in csv_reader.records() {
        let record = result?;

        let code_value_str = &record[0];

        let code_value = u32::from_str_radix(code_value_str, 16).unwrap();
        if code_value > current_unicode_block.end {
            writeln!(lib_writer, "}}\n")?;
            current_unicode_block =
                UnicodeBlock::from_str(&unicode_blocks_lines.next().unwrap().unwrap()).unwrap();
            enter_unicode_block(&mut lib_writer, &current_unicode_block, &mut features)?;
        }

        let character_name = &record[1];

        if character_name.starts_with('<') {
            // TODO: figure out why these exist..
            continue;
        }

        let character = char::from_u32(code_value).unwrap();
        if !character.is_whitespace() && !character.is_control() {
            writeln!(lib_writer, r#"    #[doc = "\u{{{}}}"]"#, code_value_str)?;
        }

        writeln!(
            lib_writer,
            r#"    pub const {}: &str = "\u{{{}}}";"#,
            character_name.replace([' ', '-'], "_").to_uppercase(),
            code_value_str
        )?;
    }

    writeln!(lib_writer, "}}")?;

    let mut cargo_toml: CargoToml = toml::from_str(&fs::read_to_string(CARGO_TOML_OUTPUT_PATH)?)?;
    cargo_toml.features = Some(features);

    let cargo_toml_string = toml::to_string(&cargo_toml)?;
    fs::write(CARGO_TOML_OUTPUT_PATH, cargo_toml_string)?;

    Ok(())
}
