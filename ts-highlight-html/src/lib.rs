use std::{collections::HashMap, io, mem};

use tree_sitter_highlight::{HighlightConfiguration, Highlighter, HtmlRenderer};

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
];

pub struct Theme(pub HashMap<&'static str, String>);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("An io error occured while rendering HTML")]
    Io(#[from] io::Error),
    #[error("Could not highlight input due to tree sitter error")]
    TreeSitter(#[from] tree_sitter_highlight::Error),
}

pub mod theme {
    use once_cell::sync::Lazy;
    use std::collections::HashMap;

    pub struct Color(pub HashMap<&'static str, &'static str>);

    impl From<&Color> for super::Theme {
        fn from(Color(colors): &Color) -> Self {
            super::Theme(
                colors
                    .iter()
                    .map(|(&k, v)| (k, format!(r#"style="color: {v}""#)))
                    .collect(),
            )
        }
    }

    mod moonfly {
        macro_rules! colors {
            ($($name:ident = $value:expr);* $(;)?) => {
                $(pub const $name: &str = $value;)*
            };
        }

        colors! {
            VIOLET = "#d183e8";
            SKY = "#74b2ff";
            TURQUOISE = "#79dac8";
            GREEN = "#8cc85f";
            EMERALD = "#36c692";
            CRANBERRY = "#e2637f";
            WHITE = "#c6c6c6";
            PURPLE = "#ae81ff";
            BLUE = "#80a0ff";
            LIME = "#85dc85";
            KHAKI = "#c2c292";
            ORANGE = "#de935f";
            CORAL = "#f09479";
            GREY246 = "#949494";
            RED = "#ff5454";
            CRIMSON = "#ff5189";
        }
    }

    static MOONFLY_COLORS: Lazy<Color> = Lazy::new(|| {
        let mut colors = HashMap::new();
        colors.insert("annotation", moonfly::VIOLET);
        colors.insert("attribute", moonfly::SKY);
        colors.insert("constant", moonfly::TURQUOISE);
        colors.insert("constant.builtin", moonfly::GREEN);
        colors.insert("constant.macro", moonfly::VIOLET);
        colors.insert("constructor", moonfly::EMERALD);
        colors.insert("function.builtin", moonfly::SKY);
        colors.insert("function.macro", moonfly::SKY);
        colors.insert("include", moonfly::CRANBERRY);
        colors.insert("keyword.operator", moonfly::VIOLET);
        colors.insert("namespace", moonfly::TURQUOISE);
        colors.insert("parameter", moonfly::WHITE);
        colors.insert("punctuation.special", moonfly::CRANBERRY);
        colors.insert("symbol", moonfly::PURPLE);
        colors.insert("tag", moonfly::BLUE);
        colors.insert("tag.delimiter", moonfly::LIME);
        colors.insert("variable.builtin", moonfly::LIME);
        colors.insert("string", moonfly::KHAKI);
        colors.insert("number", moonfly::ORANGE);
        colors.insert("label", moonfly::TURQUOISE);
        colors.insert("boolean", moonfly::CORAL);
        colors.insert("character", moonfly::PURPLE);
        colors.insert("character.special", moonfly::CRANBERRY);
        colors.insert("comment", moonfly::GREY246);
        colors.insert("conditional", moonfly::VIOLET);
        colors.insert("debug", moonfly::CRANBERRY);
        colors.insert("define", moonfly::CRANBERRY);
        colors.insert("error", moonfly::RED);
        colors.insert("exception", moonfly::CRIMSON);
        colors.insert("field", moonfly::TURQUOISE);
        colors.insert("float", moonfly::ORANGE);
        colors.insert("function", moonfly::SKY);
        colors.insert("function.call", moonfly::SKY);
        colors.insert("keyword", moonfly::VIOLET);
        colors.insert("keyword.function", moonfly::VIOLET);
        colors.insert("keyword.return", moonfly::VIOLET);
        colors.insert("method", moonfly::SKY);
        colors.insert("method.call", moonfly::SKY);
        colors.insert("operator", moonfly::CRANBERRY);
        colors.insert("parameter.reference", moonfly::WHITE);
        colors.insert("preproc", moonfly::CRANBERRY);
        colors.insert("property", moonfly::TURQUOISE);
        colors.insert("punctuation.delimiter", moonfly::WHITE);
        colors.insert("punctuation.bracket", moonfly::WHITE);
        colors.insert("repeat", moonfly::VIOLET);
        colors.insert("storageclass", moonfly::CORAL);
        colors.insert("string.regex", moonfly::KHAKI);
        colors.insert("string.escape", moonfly::CRANBERRY);
        colors.insert("string.special", moonfly::CRANBERRY);
        colors.insert("tag.attribute", moonfly::TURQUOISE);
        colors.insert("title", moonfly::ORANGE);
        colors.insert("text.literal", moonfly::KHAKI);
        colors.insert("text.math", moonfly::CRANBERRY);
        colors.insert("text.reference", moonfly::ORANGE);
        colors.insert("text.environment", moonfly::CRANBERRY);
        colors.insert("text.environment.name", moonfly::EMERALD);
        colors.insert("text.note", moonfly::CRANBERRY);
        colors.insert("type", moonfly::EMERALD);
        colors.insert("type.builtin", moonfly::EMERALD);
        colors.insert("type.qualifier", moonfly::EMERALD);
        colors.insert("type.definition", moonfly::EMERALD);
        Color(colors)
    });

    pub static MOONFLY: Lazy<super::Theme> = Lazy::new(|| (&*MOONFLY_COLORS).into());
}

mod hi_cfg {
    include!(concat!(env!("OUT_DIR"), "/ts_config.rs"));
}

pub struct SyntaxConfig<'t> {
    configs: &'static HashMap<&'static str, HighlightConfiguration>,
    theme: &'t Theme,
}

impl<'t> SyntaxConfig<'t> {
    pub fn new(theme: &'t Theme) -> Self {
        Self {
            configs: &*hi_cfg::HI_CFGS,
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
            .render(events, text.as_bytes(), &|hi| match self
                .config
                .theme
                .0
                .get(HIGHLIGHT_NAMES[hi.0])
            {
                Some(style) => style.as_bytes(),
                None => "".as_bytes(),
            })?;

        Ok(mem::take(&mut self.ts_render.html))
    }
}
