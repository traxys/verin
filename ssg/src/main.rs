use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::PathBuf,
};

use clap::Parser;
use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use glob::glob;
use serde::Deserialize;
use ts_highlight_html::{theme, SyntaxConfig};

#[derive(Parser)]
enum Args {
    Build { input: PathBuf, output: PathBuf },
}

#[derive(Deserialize, Debug)]
struct Metadata {
    date: String,
    title: String,
}

mod html;

fn parse_article(s: &str) -> Result<(Metadata, &str)> {
    let pattern = "/~";

    let idx = s
        .find(pattern)
        .with_context(|| "could not find separator for metadata")?;
    let (start, end) = s.split_at(idx);
    let end = &end[pattern.len()..];

    Ok((toml::from_str(start)?, end))
}

fn render_article(
    metadata: Metadata,
    body: &str,
    output: PathBuf,
    syntax_conf: &SyntaxConfig,
) -> Result<()> {
    let mut output = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output)
            .context("Could not open output file")?,
    );

    writeln!(
        output,
        "<!-- date = {}, title = {} -->",
        metadata.date, metadata.title
    )?;

    let body = pulldown_cmark::Parser::new(body);
    html::write_html(&mut output, body, syntax_conf).context("could not generate html")?;

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::from_args();

    let syntax_conf = SyntaxConfig::new(&*theme::MOONFLY);

    match args {
        Args::Build { input, output } => {
            std::fs::create_dir_all(&output)?;

            for entry in glob(&input.as_path().join("**/*.md").to_string_lossy())? {
                let entry = entry?;
                let entry = entry.to_string_lossy();
                let out = entry
                    .trim_start_matches(&*input.to_string_lossy())
                    .trim_start_matches('/');

                let input = std::fs::read_to_string(&*entry)?;

                let (metadata, body) = parse_article(&input)?;

                render_article(
                    metadata,
                    body,
                    output.join(out).with_extension("html"),
                    &syntax_conf,
                )?;
            }
        }
    }
    Ok(())
}
