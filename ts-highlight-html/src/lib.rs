use std::{collections::HashMap, io, mem};

use tree_sitter_highlight::{HighlightConfiguration, Highlighter, HtmlRenderer};

pub mod theme;

mod hi_cfg {
    include!(concat!(env!("OUT_DIR"), "/ts_config.rs"));
}

struct LanguageConfig {
    language: tree_sitter::Language,
    name: String,
    highlights_query: String,
    injections_query: String,
    locals_query: String,
}

impl LanguageConfig {
    pub fn new(
        language: tree_sitter::Language,
        name: &str,
        highlights_query: &str,
        injections_query: &str,
        locals_query: &str,
    ) -> Self {
        Self {
            language,
            name: name.into(),
            highlights_query: highlights_query.into(),
            injections_query: injections_query.into(),
            locals_query: locals_query.into(),
        }
    }

    fn to_highlighter(&self) -> HighlightConfiguration {
        HighlightConfiguration::new(
            self.language.clone(),
            &self.name,
            &self.highlights_query,
            &self.injections_query,
            &self.locals_query,
        )
        .unwrap_or_else(|e| panic!("Could not init {}: {}", self.name, e))
    }
}

pub const HIGHLIGHT_NAMES: &[&str] = &[
    "annotation",
    "attribute",
    "constant",
    "constant.builtin",
    "constant.macro",
    "constructor",
    "function.builtin",
    "function.macro",
    "include",
    "keyword.operator",
    "namespace",
    "parameter",
    "punctuation.special",
    "symbol",
    "tag",
    "tag.delimiter",
    "variable.builtin",
    "string",
    "number",
    "label",
    "boolean",
    "character",
    "character.special",
    "comment",
    "conditional",
    "debug",
    "define",
    "error",
    "exception",
    "field",
    "float",
    "function",
    "function.call",
    "keyword",
    "keyword.function",
    "keyword.return",
    "method",
    "method.call",
    "operator",
    "parameter.reference",
    "preproc",
    "property",
    "punctuation.delimiter",
    "punctuation.bracket",
    "repeat",
    "storageclass",
    "string.regex",
    "string.escape",
    "string.special",
    "tag.attribute",
    "title",
    "text.literal",
    "text.math",
    "text.reference",
    "text.environment",
    "text.environment.name",
    "text.note",
    "type",
    "type.builtin",
    "type.qualifier",
    "type.definition",
    "keyword.conditional",
    "module.builtin",
    "markup.environment",
    "namespace.builtin",
    "markup.list.checked",
    "keyword.exception",
    "markup.list",
    "variable.parameter",
    "markup.link.label",
    "number.float",
    "keyword.directive",
    "diff.delta",
    "markup.environment.name",
    "string.documentation",
    "markup.raw",
    "keyword.import",
    "markup.math",
    "keyword.debug",
    "markup.list.unchecked",
    "keyword.storage",
    "string.regexp",
    "keyword.directive.define",
    "markup.link.label.symbol",
    "diff.plus",
    "markup",
    "keyword.coroutine",
    "markup.heading",
    "markup.raw.markdown_inline",
    "variable.member",
    "markup.list.markdown",
    "function.method",
    "variable.parameter.builtin",
    "keyword.repeat",
    "diff.minus",
    "markup.link",
    "module",
    "variable",
    "function.method.call",
    "none",
];

pub struct Theme(pub HashMap<&'static str, String>);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("An io error occured while rendering HTML")]
    Io(#[from] io::Error),
    #[error("Could not highlight input due to tree sitter error")]
    TreeSitter(#[from] tree_sitter_highlight::Error),
}

pub struct SyntaxConfig<'t> {
    configs: HashMap<&'static str, HighlightConfiguration>,
    theme: &'t Theme,
}

impl<'t> SyntaxConfig<'t> {
    pub fn new(theme: &'t Theme) -> Self {
        Self {
            configs: hi_cfg::HI_CFGS
                .iter()
                .map(|(&k, v)| (k, v.to_highlighter()))
                .collect(),
            theme,
        }
    }
}

pub struct Renderer<'a> {
    config: &'a SyntaxConfig<'a>,
    highlighter: Highlighter,
    ts_render: HtmlRenderer,
}

impl<'a> Renderer<'a> {
    pub fn new(config: &'a SyntaxConfig<'a>) -> Self {
        Self {
            config,
            highlighter: Highlighter::new(),
            ts_render: HtmlRenderer::new(),
        }
    }

    pub fn render(&mut self, language: &str, text: &str) -> Result<Vec<u8>, Error> {
        let events = match self.config.configs.get(language) {
            None => {
                println!("[WARNING] `{language}` was not recognized, skipping highlight");
                return Ok(text.as_bytes().into());
            }
            Some(cfg) => self
                .highlighter
                .highlight(cfg, text.as_bytes(), None, |_| None)?,
        };

        self.ts_render.reset();

        self.ts_render
            .render(events, text.as_bytes(), &|hi, data| {
                if let Some(style) = self.config.theme.0.get(HIGHLIGHT_NAMES[hi.0]) {
                    data.extend_from_slice(style.as_bytes());
                }
            })?;

        Ok(mem::take(&mut self.ts_render.html))
    }
}
