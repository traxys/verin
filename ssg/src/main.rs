use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::BufWriter,
    path::{Path, PathBuf},
};

use chrono::NaiveDate;
use clap::Parser;
use color_eyre::{
    eyre::{self, Context, ContextCompat},
    Result,
};
use glob::glob;
use liquid::Template;
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Debug, Clone)]
struct Metadata {
    date: String,
    title: String,
    page: String,
    summary: String,
    #[serde(default = "create_seven")]
    max_depth: u8,
}

fn create_seven() -> u8 {
    7
}

impl Metadata {
    fn date(&self, config: &Config) -> Result<NaiveDate> {
        Ok(NaiveDate::parse_from_str(&self.date, &config.date.input)?)
    }
}

mod html;

#[derive(Deserialize, Debug)]
struct Config {
    name: String,
    date: DateConfig,
}

#[derive(Deserialize, Debug)]
struct DateConfig {
    input: String,
    output: String,
}

fn parse_article(s: &str) -> Result<(Metadata, &str)> {
    let pattern = "/~";

    let idx = s
        .find(pattern)
        .with_context(|| "could not find separator for metadata")?;
    let (start, end) = s.split_at(idx);
    let end = &end[pattern.len()..];

    Ok((toml::from_str(start)?, end))
}

fn refresh(debug: bool) -> String {
    if debug {
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
        "#
        .into()
    } else {
        "".into()
    }
}

struct ArticleConfig<'a> {
    metadata: Metadata,
    output: PathBuf,
    syntax_conf: &'a SyntaxConfig<'a>,
    templates: &'a Templates,
    debug: bool,
    config: &'a Config,
}

fn render_article(cfg: ArticleConfig, body: &str) -> Result<()> {
    let template = cfg
        .templates
        .pages
        .get(&cfg.metadata.page)
        .ok_or_else(|| eyre::eyre!("Template `{}` does not exist", cfg.metadata.page))?;

    let date = cfg.metadata.date(cfg.config)?;

    let mut output = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(cfg.output)
            .context("Could not open output file")?,
    );

    let mut content = Vec::new();

    let body = pulldown_cmark::Parser::new(body);
    let headers = html::write_html(&mut content, body, cfg.syntax_conf)?;

    template.render_to(
        &mut output,
        &liquid::object!({
            "title": cfg.metadata.title,
            "date": date.format(&cfg.config.date.output).to_string(),
            "content": String::from_utf8(content).context("generated content was not UTF-8")?,
            "refresh": refresh(cfg.debug),
            "headers": headers,
            "max_depth": cfg.metadata.max_depth,
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

            let config: Config = toml::from_str(
                &std::fs::read_to_string(input.join("config.toml"))
                    .context("Could not read config.toml")?,
            )?;

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

            let mut articles = Vec::new();

            for entry in glob(&input.as_path().join("**/*.md").to_string_lossy())? {
                let entry = entry?;
                let entry = entry.to_string_lossy();
                let out = entry
                    .trim_start_matches(&*input.to_string_lossy())
                    .trim_start_matches('/');

                let input = std::fs::read_to_string(&*entry)?;

                let (metadata, body) = parse_article(&input)?;

                articles.push((metadata.clone(), Path::new(out).with_extension("html")));

                render_article(
                    ArticleConfig {
                        metadata,
                        output: output.join(out).with_extension("html"),
                        syntax_conf: &syntax_conf,
                        templates: &templates,
                        config: &config,
                        debug,
                    },
                    body,
                )?;
            }

            let index = templates
                .pages
                .get("index")
                .context("should provide an index.html")?;
            {
                struct ArticleInfo {
                    date: NaiveDate,
                    name: String,
                    page: String,
                    summary: String,
                }

                #[derive(Debug, Serialize)]
                struct ArticleInfoStr {
                    date: String,
                    name: String,
                    page: String,
                    summary: String,
                }

                let info: Result<Vec<_>, _> = articles
                    .into_iter()
                    .map(|(metadata, file)| -> Result<_> {
                        Ok(ArticleInfo {
                            date: metadata.date(&config)?,
                            name: metadata.title,
                            page: file.file_name().unwrap().to_string_lossy().to_string(),
                            summary: metadata.summary.trim_end().replace('\n', "<br/>"),
                        })
                    })
                    .collect();
                let mut info = info?;
                info.sort_unstable_by(|a, b| a.date.cmp(&b.date));

                let info_str: Vec<_> = info
                    .into_iter()
                    .map(|info| ArticleInfoStr {
                        name: info.name,
                        page: info.page,
                        summary: info.summary,
                        date: info.date.format(&config.date.output).to_string(),
                    })
                    .collect();

                let mut output = BufWriter::new(
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(output.join("index.html"))
                        .context("Could not open output file")?,
                );

                index.render_to(
                    &mut output,
                    &liquid::object!({
                        "blog_name": &config.name,
                        "refresh": refresh(debug),
                        "articles": info_str,
                    }),
                )?;
            }
        }
    }
    Ok(())
}
