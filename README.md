# Verin

## Features

- No implicit dependency on Javascript. You can make a website with or without javascript, as you like the most.
- No dependency on styling. You can style your website exactly as you like.
- No need to declare all your pages. Just create a markdown file, add some metadata and start writing.
- Auto-refresh for development. By supplying the `--debug` flag a small snippet of javascript can be inserted in the pages to auto refresh the pages.
- Syntax highlight of code blocks using tree-sitter. Many languages are supported like `rust`, `javascript`, `yaml`, `asm`, ...

## Usage

You may create a `posts` directory at the root. This directory must contain a `index.liquid` and a `config.toml`.

The `config.toml` is of the form:

```toml
name = "<website name>"

[date]
input = "<date format in metadat (chrono format strings)>"
output = "<date format in articles (chrono format strings)>"
```

In the `index` template you have access to the following variables:

- `blog_name`: the `name` in the `config.toml`
- `articles`: a list of articles with the following fields:
  - `page`: the name of the page of the article
  - `name`: the title of the article
  - `date`: the date of the article (formatted according to `date.output`)
  - `summary`
  - `refresh`: the javascript snippet that allows for reloading on save. Empty on release.

### Templates

All liquid (`*.liquid`) files are automatically picked up by Verin. These are mostly used for article genaration.

In articles you have access to the following variables:

- `title`
- `date` (same as in the index)
- `refresh` (same as in the index)
- `content`: The html content of the article

### Articles

All markdown (`*.md`) files in the `posts` directory will be transformed into pages.

They must start with some metadata, delimited by the `/~` sequence.

The following information is required (in a toml format):

- `title`
- `date` (formatted according to `date.input`)
- `page`: a template (the name of the file without the extension) to be used for this article.
- `summary`

### Building

In order to build your static website you can run `verin build <posts-dir> <output-dir> [--debug]`.

### Refreshing

Verin adds some machinery to support refreshing the generated webpages on changes.
You can run `verin start-refresh-server` to start the server that will allow webpages to receive refresh signals.
The command `verin trigger-refresh` allows to connect to the refresh server, that will forward the requests to the web pages.

## Name

Following a number of static site generators `Verin` is named from a literary character, Verin Mathwin from the Wheel of Time.
