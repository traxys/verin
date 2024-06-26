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
use pulldown_cmark::Options;
use serde::{Deserialize, Serialize};
use ts_highlight_html::{theme, SyntaxConfig};

#[derive(Parser)]
enum Args {
    Build {
        input: PathBuf,
        output: PathBuf,
        /// Listen on the refresh server for refresh requests
        #[clap(short, long)]
        debug: bool,
        #[clap(long, default_value = "4111")]
        refresh_port: u16,
        /// Generate a RSS feed
        #[clap(short, long)]
        rss: bool,
    },
    /// Start the refresh server used for debug mode
    ///
    /// Whenever a refresh request occurs the server sends the request to the webpage by websocket.
    StartRefreshServer {
        /// Port on which the websockets listen
        #[clap(short = 'r', long, default_value = "4111")]
        refresh_port: u16,
        /// Port on which the server listens for refresh requests
        #[clap(short = 'p', long, default_value = "4112")]
        request_port: u16,
    },
    TriggerRefresh {
        #[clap(short, long, default_value = "4112")]
        port: u16,
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
    fn date(&self, config: &DateConfig) -> Result<NaiveDate> {
        Ok(NaiveDate::parse_from_str(&self.date, &config.input)?)
    }
}

mod html;
mod refresh;

#[derive(Deserialize, Debug)]
struct ChannelData {
    title: String,
    link: String,
    description: String,

    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    copyright: Option<String>,
    #[serde(default)]
    managing_editor: Option<String>,
    #[serde(default)]
    webmaster: Option<String>,
    #[serde(default)]
    pub_date: Option<String>,
    #[serde(default)]
    last_build_date: Option<String>,
    #[serde(default)]
    categories: Vec<rss::Category>,
    #[serde(default)]
    generator: Option<String>,
    #[serde(default)]
    docs: Option<String>,
    #[serde(default)]
    cloud: Option<rss::Cloud>,
    #[serde(default)]
    rating: Option<String>,
    #[serde(default)]
    ttl: Option<String>,
    #[serde(default)]
    image: Option<rss::Image>,
    #[serde(default)]
    text_input: Option<rss::TextInput>,
    #[serde(default)]
    skip_hours: Vec<String>,
    #[serde(default)]
    skip_days: Vec<String>,
}

impl From<ChannelData> for rss::Channel {
    fn from(
        ChannelData {
            title,
            link,
            description,
            language,
            copyright,
            managing_editor,
            webmaster,
            pub_date,
            last_build_date,
            categories,
            generator,
            docs,
            cloud,
            rating,
            ttl,
            image,
            text_input,
            skip_hours,
            skip_days,
        }: ChannelData,
    ) -> Self {
        rss::Channel {
            title,
            link,
            description,
            language,
            copyright,
            managing_editor,
            webmaster,
            pub_date,
            last_build_date,
            categories,
            generator,
            docs,
            cloud,
            rating,
            ttl,
            image,
            text_input,
            skip_hours,
            skip_days,
            ..rss::Channel::default()
        }
    }
}

#[derive(Deserialize, Debug)]
struct Config {
    name: String,
    date: DateConfig,
    #[serde(default)]
    rss: Option<ChannelData>,
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

fn refresh(debug: bool, port: u16) -> String {
    if debug {
        format!(
            r#"
        <script>
            let ws = new WebSocket("ws://localhost:{port}");
            ws.onopen = function(_) {{
                console.log("WS started");
            }};

            ws.onmessage = function(_) {{
                console.log("REFRESH");
                document.location.reload()
            }};

            ws.onerror = function(error) {{
                console.log(`[error] WS error: ${{error.message}}`);
            }};
        </script>
        "#
        )
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

fn render_article(cfg: ArticleConfig, body: &str, refresh_port: u16) -> Result<()> {
    let template = cfg
        .templates
        .pages
        .get(&cfg.metadata.page)
        .ok_or_else(|| eyre::eyre!("Template `{}` does not exist", cfg.metadata.page))?;

    let date = cfg.metadata.date(&cfg.config.date)?;

    let mut output = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(cfg.output)
            .context("Could not open output file")?,
    );

    let mut content = Vec::new();

    let body = pulldown_cmark::Parser::new_ext(body, Options::ENABLE_MATH);
    let headers = html::write_html(&mut content, body, cfg.syntax_conf)?;

    template.render_to(
        &mut output,
        &liquid::object!({
            "title": cfg.metadata.title,
            "date": date.format(&cfg.config.date.output).to_string(),
            "content": String::from_utf8(content).context("generated content was not UTF-8")?,
            "refresh": refresh(cfg.debug, refresh_port),
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
    let args = Args::parse();

    let syntax_conf = SyntaxConfig::new(&theme::TOKYO_NIGHT);
    let mut templates = Templates {
        pages: HashMap::new(),
    };

    match args {
        Args::Build {
            input,
            output,
            debug,
            refresh_port,
            rss,
        } => {
            std::fs::create_dir_all(&output)?;

            let input = input
                .canonicalize()
                .context("failed to canonicalize input")?;

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

                let out = entry
                    .strip_prefix(&input)
                    .context("could not remove leading dir from file")?;

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
                    refresh_port,
                )?;
            }

            if let Some(not_found) = templates.pages.get("not_found") {
                let mut output = BufWriter::new(
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(output.join("404.html"))
                        .context("Could not open output file")?,
                );

                not_found.render_to(
                    &mut output,
                    &liquid::object!({
                        "blog_name": &config.name,
                        "refresh": refresh(debug, refresh_port),
                    }),
                )?;
            }

            let index = templates
                .pages
                .get("index")
                .context("should provide an index.html")?;
            {
                struct ArticleInfo<'a> {
                    date: NaiveDate,
                    name: &'a str,
                    page: String,
                    summary: String,
                }

                #[derive(Debug, Serialize)]
                struct ArticleInfoStr<'a> {
                    date: String,
                    name: &'a str,
                    page: String,
                    summary: String,
                }

                let info: Result<Vec<_>, _> = articles
                    .iter()
                    .map(|(metadata, file)| -> Result<_> {
                        Ok(ArticleInfo {
                            date: metadata.date(&config.date)?,
                            name: &metadata.title,
                            page: file.file_name().unwrap().to_string_lossy().to_string(),
                            summary: metadata.summary.trim_end().replace('\n', "<br/>"),
                        })
                    })
                    .collect();
                let mut info = info?;
                info.sort_unstable_by(|a, b| b.date.cmp(&a.date));

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
                        "refresh": refresh(debug, refresh_port),
                        "articles": info_str,
                    }),
                )?;
            }

            if rss {
                let mut channel: rss::Channel = config
                    .rss
                    .context(
                        "specifying --rss requires to have an `rss` section in the configuration",
                    )?
                    .into();

                channel.set_items(
                    articles
                        .into_iter()
                        .map(|(metadata, path)| rss::Item {
                            pub_date: Some(
                                chrono::NaiveDateTime::new(
                                    metadata.date(&config.date).unwrap(),
                                    Default::default(),
                                )
                                .and_utc()
                                .to_rfc2822(),
                            ),
                            title: Some(metadata.title),
                            link: Some(format!("{}/{}", channel.link, path.to_str().unwrap())),
                            description: Some(metadata.summary),
                            ..Default::default()
                        })
                        .collect::<Vec<_>>(),
                );

                let feed = OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(output.join("rss.xml"))?;

                channel.pretty_write_to(feed, b' ', 4)?;
            }
        }
        Args::StartRefreshServer {
            refresh_port,
            request_port,
        } => refresh::refresh_server(refresh_port, request_port)?,
        Args::TriggerRefresh { port } => refresh::trigger_refresh(port)?,
    }
    Ok(())
}
