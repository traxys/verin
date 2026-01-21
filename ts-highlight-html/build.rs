use std::{collections::HashMap, env, path::Path};

use quote::{format_ident, quote};

fn main() {
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

    let mut add_configs = quote! {
        configs.insert("javascript", {
            let mut cfg = HighlightConfiguration::new(
                tree_sitter_javascript::LANGUAGE.into(),
                "javascript",
                include_str!(concat!(#nvim_treesitter_queries, "/ecma/highlights.scm")),
                include_str!(concat!(#nvim_treesitter_queries, "/ecma/injections.scm")),
                include_str!(concat!(#nvim_treesitter_queries, "/ecma/locals.scm")),
            ).expect("Could not load language javascript");
            cfg.configure(crate::HIGHLIGHT_NAMES);
            cfg
        });

        configs.insert("asm", {
            let mut cfg = HighlightConfiguration::new(
                tree_sitter_asm::LANGUAGE.into(),
                "asm",
                include_str!(concat!(#nvim_treesitter_queries, "/asm/highlights.scm")),
                "",
                include_str!(concat!(#nvim_treesitter_queries, "/asm/injections.scm")),
            ).expect("Could not load language asm");
            cfg.configure(crate::HIGHLIGHT_NAMES);
            cfg
        });

        configs.insert("vvk", {
            let mut cfg = HighlightConfiguration::new(
                tree_sitter_vvk::LANGUAGE.into(),
                "vvk",
                r#"
                    (comment) @comment

                    [
                         "["
                         "]"
                         "{"
                         "}"
                    ] @punctuation.bracket

                    [
                        "mod"
                    ] @keyword.import

                    (directive) @keyword.modifier

                    (string_literal) @string

                    (target
                        name: (identifier) @function.call)

                    (assign_statement
                        name: (identifier) @variable)

                    (argument
                        name: (identifier) @variable.member)
                "#,
                "",
                "",
            ).expect("Could not load language vvk");
            cfg.configure(crate::HIGHLIGHT_NAMES);
            cfg
        });
    };

    let mut alternate_module = HashMap::new();
    alternate_module.insert("toml", "toml_ng");

    for language in languages {
        let injections = if Path::new(nvim_treesitter_queries)
            .join(format!("{language}/injections.scm"))
            .exists()
        {
            let path = format!("{nvim_treesitter_queries}/{language}/injections.scm");
            quote! {
                include_str!(#path)
            }
        } else {
            quote! {
                ""
            }
        };

        let module = format_ident!(
            "tree_sitter_{}",
            alternate_module.get(language).unwrap_or(language)
        );

        add_configs = quote! {
            #add_configs

            configs.insert(#language, {{
                let mut cfg = HighlightConfiguration::new(
                    #module::LANGUAGE.into(),
                    #language,
                    include_str!(concat!(#nvim_treesitter_queries, "/", #language, "/highlights.scm")),
                    #injections,
                    include_str!(concat!(#nvim_treesitter_queries, "/", #language, "/locals.scm")),
                ).expect(concat!("Could not load language ", #language));
                cfg.configure(crate::HIGHLIGHT_NAMES);
                cfg
            }});
        };
    }

    let output = quote! {
        use std::collections::HashMap;
        use once_cell::sync::Lazy;
        use tree_sitter_highlight::HighlightConfiguration;

        pub static HI_CFGS: Lazy<HashMap<&'static str, HighlightConfiguration>> = Lazy::new(|| {
            let mut configs = HashMap::new();

            #add_configs

            configs
        });
    };

    let syntax_tree = syn::parse2(output).unwrap();
    let formatted = prettyplease::unparse(&syntax_tree);

    std::fs::write(
        Path::new(&env::var("OUT_DIR").expect("could not read out-dir")).join("ts_config.rs"),
        formatted,
    )
    .unwrap();
}
