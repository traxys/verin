use std::{collections::HashMap, fs::OpenOptions, io::BufWriter, path::PathBuf};

use chrono::NaiveDate;
use clap::Parser;
use color_eyre::{
    eyre::{self, Context, ContextCompat},
    Result,
};
use glob::glob;
use liquid::Template;
use serde::Deserialize;
use ts_highlight_html::{theme, SyntaxConfig};

#[derive(Parser)]
enum Args {
    Build {
        input: PathBuf,
        output: PathBuf,
        #[clap(short, long)]
        debug: bool,
    },
}

#[derive(Deserialize, Debug)]
struct Metadata {
    date: String,
    title: String,
    page: String,
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
    templates: &Templates,
    debug: bool,
) -> Result<()> {
    let template = templates
        .pages
        .get(&metadata.page)
        .ok_or_else(|| eyre::eyre!("Template `{}` does not exist", metadata.page))?;

    let date = NaiveDate::parse_from_str(&metadata.date, "%d/%m/%Y")?;

    let mut output = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output)
            .context("Could not open output file")?,
    );

    let mut content = Vec::new();

    let body = pulldown_cmark::Parser::new(body);
    html::write_html(&mut content, body, syntax_conf).context("could not generate html")?;

    let refresh = if debug {
        r#"
        <script>
            let ws = new WebSocket("ws://localhost:4111");
            ws.onopen = function(_) {
                console.log("WS started");
            };

            ws.onmessage = function(_) {
                console.log("REFRESH");
                window.location = window.location;
            };

            ws.onerror = function(error) {
                console.log(`[error] WS error: ${error.message}`);
            };
        </script>
        "#.to_string()
    } else {
        "".into()
    };

    template.render_to(
        &mut output,
        &liquid::object!({
            "title": metadata.title,
            "date": date.format("%d %B %Y").to_string(),
            "content": String::from_utf8(content).context("generated content was not UTF-8")?,
            "refresh": refresh,
        }),
    )?;

    Ok(())
}

struct Templates {
    pages: HashMap<String, Template>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::from_args();

    let syntax_conf = SyntaxConfig::new(&*theme::MOONFLY);
    let mut templates = Templates {
        pages: HashMap::new(),
    };

    match args {
        Args::Build {
            input,
            output,
            debug,
        } => {
            std::fs::create_dir_all(&output)?;

            for entry in glob(&input.as_path().join("**/*.liquid").to_string_lossy())? {
                let entry = entry?;
                let template = liquid::ParserBuilder::with_stdlib()
                    .build()?
                    .parse_file(&entry)?;

                templates.pages.insert(
                    entry
                        .file_stem()
                        .expect("Template has no file stem, should not be possible")
                        .to_str()
                        .ok_or(eyre::eyre!("Template name should be valid UTF-8"))?
                        .to_owned(),
                    template,
                );
            }

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
                    &templates,
                    debug,
                )?;
            }
        }
    }
    Ok(())
}
