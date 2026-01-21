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

macro_rules! colors {
    ($($name:ident = $value:expr);* $(;)?) => {
        paste::paste! {
            $(pub const [< $name:upper >]: &str = $value;)*
        }
    };
}

mod moonfly {
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

#[allow(unused)]
mod tokio_night {
    colors! {
        none = "NONE";
        bg_dark = "#16161e";
        bg = "#1a1b26";
        bg_highlight = "#292e42";
        terminal_black = "#414868";
        fg = "#c0caf5";
        fg_dark = "#a9b1d6";
        fg_gutter = "#3b4261";
        dark3 = "#545c7e";
        comment = "#565f89";
        dark5 = "#737aa2";
        blue0 = "#3d59a1";
        blue = "#7aa2f7";
        cyan = "#7dcfff";
        blue1 = "#2ac3de";
        blue2 = "#0db9d7";
        blue5 = "#89ddff";
        blue6 = "#b4f9f8";
        blue7 = "#394b70";
        magenta = "#bb9af7";
        magenta2 = "#ff007c";
        purple = "#9d7cd8";
        orange = "#ff9e64";
        yellow = "#e0af68";
        green = "#9ece6a";
        green1 = "#73daca";
        green2 = "#41a6b5";
        teal = "#1abc9c";
        red = "#f7768e";
        red1 = "#db4b4b";
        git_change = "#6183bb";
        git_add = "#449dab";
        git_delet = "#914c54";
        gitSigns_add = "#266d6a";
        gitSigns_change = "#536c9e";
        gitSigns_delete = "#b2555b";
    }
}

macro_rules! set_many {
    ($map:expr; $($name:expr => $value:expr),* $(,)?) => {
        $(
            $map.insert($name, $value);
        )*
    };
}

enum Group {
    Link(&'static str),
    Color(&'static str),
    None,
}

fn resolve_group(groups: &HashMap<&'static str, Group>, group: &str) -> Option<&'static str> {
    let mut group = groups.get(group).unwrap();
    loop {
        match group {
            Group::Link(l) => group = groups.get(l).unwrap(),
            Group::Color(s) => return Some(s),
            Group::None => return None,
        }
    }
}

static TOKYO_NIGHT_COLORS: Lazy<Color> = Lazy::new(|| {
    use tokio_night::*;
    use Group::Color as S;
    use Group::Link as L;

    let mut groups = HashMap::new();
    set_many!(groups;
        // Neovim groups
        "Comment" => S(COMMENT),
        "DiffAdd" => Group::None, // bg, darken
        "DiffChange" => Group::None, // bg, darken
        "DiffDelete" => Group::None, // bg, darken
        "Title" => S(BLUE), // bold

        "Constant" => S(ORANGE),
        "String" => S(GREEN),
        "Character" => S(GREEN),
        "Number" => L("Constant"),
        "Boolean" => L("Constant"),
        "Float" => L("Number"),

        "Identifier" => S(MAGENTA), // style
        "Function" => S(BLUE), // style

        "PreProc" => S(CYAN),
        "Include" => L("PreProc"),
        "Define" => L("PreProc"),
        "Macro" => L("PreProc"),

        "Type" => S(BLUE1),
        "StorageClass" => L("Type"),
        "Typedef" => L("Type"),

        "Statement" => S(MAGENTA),
        "Conditional" => L("Statement"),
        "Repeat" => L("Statement"),
        "Label" => L("Statement"),
        "Exception" => L("Statement"),

        "Special" => S(BLUE1),
        "SpecialChar" => L("Special"),
        "Delimiter" => L("Special"),
        "Debug" => S(ORANGE),

        // Tree-sitter groups
        "@annotation" => L("PreProc"),
        "@attribute" => L("PreProc"),
        "@boolean" => L("Boolean"),
        "@character" => L("Character"),
        "@character.special" => L("SpecialChar"),
        "@comment" => L("Comment"),
        "@keyword.conditional" => L("Conditional"),
        "@constant" => L("Constant"),
        "@constant.builtin" => L("Special"),
        "@constant.macro" => L("Define"),
        "@keyword.debug" => L("Debug"),
        "@keyword.directive.define" => L("Define"),
        "@keyword.exception" => L("Exception"),
        "@number.float" => L("Float"),
        "@function" => L("Function"),
        "@function.builtin" => L("Special"),
        "@function.call" => L("@function"),
        "@function.macro" => L("Macro"),
        "@keyword.import" => L("Include"),
        "@keyword.coroutine" => L("@keyword"),
        "@keyword.operator" => L("@operator"),
        "@keyword.return" => L("@keyword"),
        "@function.method" => L("Function"),
        "@function.method.call" => L("@function.method"),
        "@namespace.builtin" => L("@variable.builtin"),
        "@none" => Group::None,
        "@number" => L("Number"),
        "@keyword.directive" => L("PreProc"),
        "@keyword.repeat" => L("Repeat"),
        "@keyword.storage" => L("StorageClass"),
        "@string" => L("String"),
        "@markup.link.label" => L("SpecialChar"),
        "@markup.link.label.symbol" => L("Identifier"),
        "@tag" => L("Label"),
        "@tag.attribute" => L("@property"),
        "@tag.delimiter" => L("Delimiter"),
        "@markup" => L("@none"),
        "@markup.environment" => L("Macro"),
        "@markup.environment.name" => L("Type"),
        "@markup.raw" => L("String"),
        "@markup.math" => L("Special"),
        "@markup.heading" => L("Title"),
        "@type" => L("Type"),
        "@type.definition" => L("Typedef"),
        "@type.qualifier" => L("@keyword"),

        // Misc
        "@operator" => S(BLUE5),

        // Punctuation
        "@punctuation.delimiter" => S(BLUE5),
        "@punctuation.bracket" => S(FG_DARK),
        "@punctuation.special" => S(BLUE5),
        "@markup.list" => S(BLUE5),
        "@markup.list.markdown" => S(ORANGE),

        // Literals
        "@string.documentation" => S(YELLOW),
        "@string.regexp" => S(BLUE6),
        "@string.escape" => S(MAGENTA),

        // Functions
        "@constructor" => S(MAGENTA),
        "@variable.parameter" => S(YELLOW),
        "@variable.parameter.builtin" => S(YELLOW), // lighten

        // Keywords
        "@keyword" => S(PURPLE),
        "@keyword.function" => S(MAGENTA),

        "@label" => S(BLUE),

        // Types
        "@type.builtin" => S(BLUE1), // darken
        "@variable.member" => S(GREEN1),
        "@property" => S(GREEN1),

        // Identifiers
        "@variable" => S(FG),
        "@variable.builtin" => S(RED),
        "@module.builtin" => S(RED),

        // Text
        "@markup.raw.markdown_inline" => S(BLUE),
        "@markup.link" => S(TEAL),

        "@markup.list.unchecked" => S(BLUE),
        "@markup.list.checked" => S(GREEN1),

        "@diff.plus" => L("DiffAdd"),
        "@diff.minus" => L("DiffDelete"),
        "@diff.delta" => L("DiffChange"),

        "@module" => L("Include"),
    );

    let mut colors = HashMap::new();
    let mut ignore = vec![
        "include",
        "namespace",
        "parameter",
        "symbol",
        "conditional",
        "debug",
        "define",
        "error",
        "exception",
        "field",
        "float",
        "method",
        "method.call",
        "parameter.reference",
        "preproc",
        "repeat",
        "storageclass",
        "string.regex",
        "string.special",
        "title",
        "text.literal",
        "text.math",
        "text.reference",
        "text.environment",
        "text.environment.name",
        "text.note",
    ];

    for group in groups.keys().filter_map(|g| g.strip_prefix('@')) {
        assert!(
            crate::HIGHLIGHT_NAMES.contains(&group),
            "group {group} not highlighted"
        );

        match resolve_group(&groups, &format!("@{group}")) {
            Some(color) => {
                colors.insert(group, color);
            }
            _ => ignore.push(group),
        }
    }

    for hi in crate::HIGHLIGHT_NAMES {
        if !colors.contains_key(hi) && !ignore.contains(hi) {
            eprintln!("warn: no colors for {hi}");
        }
    }

    Color(colors)
});

static MOONFLY_COLORS: Lazy<Color> = Lazy::new(|| {
    let mut colors = HashMap::new();
    set_many!(colors;
        "annotation" => moonfly::VIOLET,
        "attribute" => moonfly::SKY,
        "constant" => moonfly::TURQUOISE,
        "constant.builtin" => moonfly::GREEN,
        "constant.macro" => moonfly::VIOLET,
        "constructor" => moonfly::EMERALD,
        "function.builtin" => moonfly::SKY,
        "function.macro" => moonfly::SKY,
        "include" => moonfly::CRANBERRY,
        "keyword.operator" => moonfly::VIOLET,
        "namespace" => moonfly::TURQUOISE,
        "parameter" => moonfly::WHITE,
        "punctuation.special" => moonfly::CRANBERRY,
        "symbol" => moonfly::PURPLE,
        "tag" => moonfly::BLUE,
        "tag.delimiter" => moonfly::LIME,
        "variable.builtin" => moonfly::LIME,
        "string" => moonfly::KHAKI,
        "number" => moonfly::ORANGE,
        "label" => moonfly::TURQUOISE,
        "boolean" => moonfly::CORAL,
        "character" => moonfly::PURPLE,
        "character.special" => moonfly::CRANBERRY,
        "comment" => moonfly::GREY246,
        "conditional" => moonfly::VIOLET,
        "keyword.conditional" => moonfly::VIOLET,
        "debug" => moonfly::CRANBERRY,
        "define" => moonfly::CRANBERRY,
        "error" => moonfly::RED,
        "exception" => moonfly::CRIMSON,
        "field" => moonfly::TURQUOISE,
        "float" => moonfly::ORANGE,
        "function" => moonfly::SKY,
        "function.call" => moonfly::SKY,
        "keyword" => moonfly::VIOLET,
        "keyword.function" => moonfly::VIOLET,
        "keyword.return" => moonfly::VIOLET,
        "method" => moonfly::SKY,
        "method.call" => moonfly::SKY,
        "operator" => moonfly::CRANBERRY,
        "parameter.reference" => moonfly::WHITE,
        "preproc" => moonfly::CRANBERRY,
        "property" => moonfly::TURQUOISE,
        "punctuation.delimiter" => moonfly::WHITE,
        "punctuation.bracket" => moonfly::WHITE,
        "repeat" => moonfly::VIOLET,
        "storageclass" => moonfly::CORAL,
        "string.regex" => moonfly::KHAKI,
        "string.escape" => moonfly::CRANBERRY,
        "string.special" => moonfly::CRANBERRY,
        "tag.attribute" => moonfly::TURQUOISE,
        "title" => moonfly::ORANGE,
        "text.literal" => moonfly::KHAKI,
        "text.math" => moonfly::CRANBERRY,
        "text.reference" => moonfly::ORANGE,
        "text.environment" => moonfly::CRANBERRY,
        "text.environment.name" => moonfly::EMERALD,
        "text.note" => moonfly::CRANBERRY,
        "type" => moonfly::EMERALD,
        "type.builtin" => moonfly::EMERALD,
        "type.qualifier" => moonfly::EMERALD,
        "type.definition" => moonfly::EMERALD,
    );
    Color(colors)
});

pub static MOONFLY: Lazy<super::Theme> = Lazy::new(|| (&*MOONFLY_COLORS).into());
pub static TOKYO_NIGHT: Lazy<super::Theme> = Lazy::new(|| (&*TOKYO_NIGHT_COLORS).into());
