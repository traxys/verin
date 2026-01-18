use std::{
    collections::HashMap,
    env,
    io::{BufWriter, Write},
    path::Path,
};

fn main() -> std::io::Result<()> {
    let languages = &[
        "json",
        "rust",
        "nix",
        "toml",
        "yaml",
        "linkerscript",
        "devicetree",
    ];
    let nvim_treesitter_queries = concat!(env!("NVIM_TREESITTER"), "/runtime/queries");

    let mut out_file = BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(
                Path::new(&env::var("OUT_DIR").expect("could not read out-dir"))
                    .join("ts_config.rs"),
            )?,
    );

    write!(
        out_file,
        r#"
        use std::collections::HashMap;
        use once_cell::sync::Lazy;
        use tree_sitter_highlight::HighlightConfiguration;

        pub static HI_CFGS: Lazy<HashMap<&'static str, HighlightConfiguration>> = Lazy::new(|| {{
            let mut configs = HashMap::new();
    "#
    )?;

    write!(
        out_file,
        r#"
            configs.insert("javascript", {{
                let mut cfg = HighlightConfiguration::new(
                    tree_sitter_javascript::LANGUAGE.into(),
                    "javascript",
                    include_str!("{nvim_treesitter_queries}/ecma/highlights.scm"),
                    include_str!("{nvim_treesitter_queries}/ecma/injections.scm"),
                    include_str!("{nvim_treesitter_queries}/ecma/locals.scm"),
                ).expect("Could not load language javascript");
                cfg.configure(crate::HIGHLIGHT_NAMES);
                cfg
            }});
        "#
    )?;

    write!(
        out_file,
        r#"
            configs.insert("asm", {{
                let mut cfg = HighlightConfiguration::new(
                    tree_sitter_asm::LANGUAGE.into(),
                    "asm",
                    include_str!("{nvim_treesitter_queries}/asm/highlights.scm"),
                    "",
                    include_str!("{nvim_treesitter_queries}/asm/injections.scm"),
                ).expect("Could not load language asm");
                cfg.configure(crate::HIGHLIGHT_NAMES);
                cfg
            }});
        "#
    )?;

    let mut alternate_module = HashMap::new();
    alternate_module.insert("toml", "toml_ng");

    for language in languages {
        let injections = if Path::new(nvim_treesitter_queries)
            .join(format!("{language}/injections.scm"))
            .exists()
        {
            format!(r#"include_str!("{nvim_treesitter_queries}/{language}/injections.scm")"#)
        } else {
            r#""""#.to_owned()
        };

        let module = alternate_module.get(language).unwrap_or(language);

        write!(
            out_file,
            r#"
            configs.insert("{language}", {{
                let mut cfg = HighlightConfiguration::new(
                    tree_sitter_{module}::LANGUAGE.into(),
                    "{language}",
                    include_str!("{nvim_treesitter_queries}/{language}/highlights.scm"),
                    {injections},
                    include_str!("{nvim_treesitter_queries}/{language}/locals.scm"),
                ).expect("Could not load language {language}");
                cfg.configure(crate::HIGHLIGHT_NAMES);
                cfg
            }});
        "#
        )?;
    }

    writeln!(out_file, "configs }});")?;

    Ok(())
}
