// Copyright 2015 Google Inc. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

//! HTML renderer that takes an iterator of events as input.

use std::{
    collections::HashMap,
    io::{self, Write},
};

use bstr::ByteSlice;
use color_eyre::Result;

use itertools::Itertools;
use pulldown_cmark::{
    Alignment, CodeBlockKind, CowStr,
    Event::{self, *},
    HeadingLevel, LinkType, Tag, TagEnd,
};
use pulldown_cmark_escape::{escape_href, escape_html, StrWrite, WriteWrapper};
use serde::Serialize;
use ts_highlight_html::{Renderer, SyntaxConfig};

#[derive(Debug, Serialize)]
pub struct HeadingInfo {
    pub level: u8,
    pub text: String,
    pub number: String,
}

struct HeadingStack {
    floor: u8,
    current: u8,
    stack: Vec<usize>,
}

impl HeadingStack {
    fn new() -> Self {
        Self {
            floor: 0,
            current: 0,
            stack: Vec::with_capacity(6),
        }
    }

    fn repr(&self) -> String {
        self.stack.iter().join(".")
    }

    fn effective_level(&self) -> u8 {
        self.stack.len() as u8
    }

    fn enter(&mut self, level: HeadingLevel) {
        let level = level as u8;

        if self.current == 0 {
            self.floor = level;
            self.current = level;
            self.stack.push(1);
            return;
        }

        let level = std::cmp::max(level, self.floor);

        match self.current.cmp(&level) {
            std::cmp::Ordering::Greater => {
                for _ in 0..(self.current - level) {
                    self.stack.pop();
                }
                let current = self
                    .stack
                    .pop()
                    .expect("invariant: if != 0 -> has something");
                self.stack.push(current + 1);
            }
            std::cmp::Ordering::Equal => {
                let current = self
                    .stack
                    .pop()
                    .expect("invariant: if != 0 -> has something");
                self.stack.push(current + 1);
            }
            std::cmp::Ordering::Less => {
                for _ in 0..(level - self.current) {
                    self.stack.push(1)
                }
            }
        };

        self.current = level;
    }
}

enum TableState {
    Head,
    Body,
}

struct HtmlWriter<'a, I, W> {
    /// Iterator supplying events.
    iter: I,

    /// Writer to write to.
    writer: W,

    /// Whether or not the last write wrote a newline.
    end_newline: bool,

    /// Whether if inside a metadata block (text should not be written)
    in_non_writing_block: bool,

    code: Option<CowStr<'a>>,
    syntax: Renderer<'a>,

    table_state: TableState,
    table_alignments: Vec<Alignment>,
    table_cell_index: usize,
    numbers: HashMap<CowStr<'a>, usize>,

    header_stack: HeadingStack,
    current_header: Option<String>,
    headers: Vec<HeadingInfo>,
}

impl<'a, I, W> HtmlWriter<'a, I, W>
where
    I: Iterator<Item = Event<'a>>,
    W: Write,
{
    fn new(iter: I, writer: W, syntax: &'a SyntaxConfig) -> Self {
        Self {
            iter,
            writer,
            syntax: Renderer::new(syntax),
            code: None,
            end_newline: true,
            in_non_writing_block: false,
            table_state: TableState::Head,
            table_alignments: vec![],
            table_cell_index: 0,
            numbers: HashMap::new(),
            headers: Vec::new(),
            current_header: None,
            header_stack: HeadingStack::new(),
        }
    }

    /// Writes a new line.
    fn write_newline(&mut self) -> io::Result<()> {
        self.end_newline = true;
        self.writer.write(b"\n").map(|_| ())
    }

    /// Writes a buffer, and tracks whether or not a newline was written.
    #[inline]
    fn write(&mut self, s: &[u8]) -> io::Result<()> {
        if let Some(header) = &mut self.current_header {
            header.write_str(&*String::from_utf8_lossy(s))?
        }
        self.writer.write_all(s)?;

        if !s.is_empty() {
            self.end_newline = s.ends_with(b"\n");
        }
        Ok(())
    }

    fn run(mut self) -> Result<Vec<HeadingInfo>> {
        while let Some(event) = self.iter.next() {
            match event {
                Start(tag) => {
                    self.start_tag(tag)?;
                }
                End(tag) => {
                    self.end_tag(tag)?;
                }
                Text(text) => {
                    match self.code.as_deref() {
                        None | Some("") => {
                            if let Some(header) = &mut self.current_header {
                                escape_html(header, &text)?;
                            }
                            escape_html(WriteWrapper(&mut self.writer), &text)?;
                        }
                        Some(lang) => {
                            let rendered = self.syntax.render(lang, &text)?;
                            for line in rendered.lines() {
                                self.write(br#"<span class="newline">"#)?;
                                self.write(line)?;
                                self.write(b"\n")?;
                                self.write(br#"</span>"#)?;
                            }
                        }
                    }
                    self.end_newline = text.ends_with('\n');
                }
                Code(text) => {
                    self.write(b"<code>")?;
                    if let Some(header) = &mut self.current_header {
                        escape_html(header, &text)?;
                    }
                    escape_html(WriteWrapper(&mut self.writer), &text)?;
                    self.write(b"</code>")?;
                }
                Html(html) | InlineHtml(html) => {
                    self.write(html.as_bytes())?;
                }
                SoftBreak => {
                    self.write_newline()?;
                }
                HardBreak => {
                    self.write(b"<br />\n")?;
                }
                Rule => {
                    if self.end_newline {
                        self.write(b"<hr />\n")?;
                    } else {
                        self.write(b"\n<hr />\n")?;
                    }
                }
                FootnoteReference(name) => {
                    let len = self.numbers.len() + 1;
                    self.write(b"<sup class=\"footnote-reference\"><a href=\"#")?;
                    if let Some(header) = &mut self.current_header {
                        escape_html(header, &name)?;
                    }
                    escape_html(WriteWrapper(&mut self.writer), &name)?;
                    self.write(b"\">")?;
                    let number = *self.numbers.entry(name).or_insert(len);
                    write!(&mut self.writer, "{}", number)?;
                    self.write(b"</a></sup>")?;
                }
                TaskListMarker(true) => {
                    self.write(b"<input disabled=\"\" type=\"checkbox\" checked=\"\"/>\n")?;
                }
                TaskListMarker(false) => {
                    self.write(b"<input disabled=\"\" type=\"checkbox\"/>\n")?;
                }
            }
        }
        Ok(self.headers)
    }

    /// Writes the start of an HTML tag.
    fn start_tag(&mut self, tag: Tag<'a>) -> io::Result<()> {
        match tag {
            Tag::HtmlBlock => Ok(()),
            Tag::Paragraph => {
                if self.end_newline {
                    self.write(b"<p>")
                } else {
                    self.write(b"\n<p>")
                }
            }
            Tag::Heading {
                level,
                id: _,
                classes,
                attrs,
            } => {
                self.header_stack.enter(level);
                if self.end_newline {
                    self.end_newline = false;
                    self.write(b"<")?;
                } else {
                    self.write(b"\n<")?;
                }
                write!(&mut self.writer, "{}", level)?;
                self.write(format!(r#" id="header-{}" "#, self.header_stack.repr()).as_bytes())?;
                let mut classes = classes.iter();
                if let Some(class) = classes.next() {
                    self.write(b" class=\"")?;
                    if let Some(header) = &mut self.current_header {
                        escape_html(header, class)?;
                    }
                    escape_html(WriteWrapper(&mut self.writer), class)?;
                    for class in classes {
                        self.write(b" ")?;
                        if let Some(header) = &mut self.current_header {
                            escape_html(header, class)?;
                        }
                        escape_html(WriteWrapper(&mut self.writer), class)?;
                    }
                    self.write(b"\"")?;
                }
                for (attr, value) in attrs {
                    self.write(b" ")?;
                    escape_html(WriteWrapper(&mut self.writer), &attr)?;
                    if let Some(val) = value {
                        self.write(b"=\"")?;
                        escape_html(WriteWrapper(&mut self.writer), &val)?;
                        self.write(b"\"")?;
                    } else {
                        self.write(b"=\"\"")?;
                    }
                }
                self.write(b">")?;
                self.current_header = Some(String::new());
                Ok(())
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments;
                self.write(b"<table>")
            }
            Tag::TableHead => {
                self.table_state = TableState::Head;
                self.table_cell_index = 0;
                self.write(b"<thead><tr>")
            }
            Tag::TableRow => {
                self.table_cell_index = 0;
                self.write(b"<tr>")
            }
            Tag::TableCell => {
                match self.table_state {
                    TableState::Head => {
                        self.write(b"<th")?;
                    }
                    TableState::Body => {
                        self.write(b"<td")?;
                    }
                }
                match self.table_alignments.get(self.table_cell_index) {
                    Some(&Alignment::Left) => self.write(b" style=\"text-align: left\">"),
                    Some(&Alignment::Center) => self.write(b" style=\"text-align: center\">"),
                    Some(&Alignment::Right) => self.write(b" style=\"text-align: right\">"),
                    _ => self.write(b">"),
                }
            }
            Tag::BlockQuote => {
                if self.end_newline {
                    self.write(b"<blockquote>\n")
                } else {
                    self.write(b"\n<blockquote>\n")
                }
            }
            Tag::CodeBlock(info) => {
                if !self.end_newline {
                    self.write_newline()?;
                }
                match info {
                    CodeBlockKind::Fenced(info) => {
                        let lang = info.split(' ').next().unwrap();
                        if lang.is_empty() {
                            self.code = Some("".into());
                        } else {
                            self.code = Some(info.clone());
                        }
                        self.write(
                            br#"<pre style="background-color: #080808; color: #c6c6c6"><code>"#,
                        )
                    }
                    CodeBlockKind::Indented => self.write(b"<pre><code>"),
                }
            }
            Tag::List(Some(1)) => {
                if self.end_newline {
                    self.write(b"<ol>\n")
                } else {
                    self.write(b"\n<ol>\n")
                }
            }
            Tag::List(Some(start)) => {
                if self.end_newline {
                    self.write(b"<ol start=\"")?;
                } else {
                    self.write(b"\n<ol start=\"")?;
                }
                write!(&mut self.writer, "{}", start)?;
                self.write(b"\">\n")
            }
            Tag::List(None) => {
                if self.end_newline {
                    self.write(b"<ul>\n")
                } else {
                    self.write(b"\n<ul>\n")
                }
            }
            Tag::Item => {
                if self.end_newline {
                    self.write(b"<li>")
                } else {
                    self.write(b"\n<li>")
                }
            }
            Tag::Emphasis => self.write(b"<em>"),
            Tag::Strong => self.write(b"<strong>"),
            Tag::Strikethrough => self.write(b"<del>"),
            Tag::Link {
                link_type: LinkType::Email,
                dest_url,
                title,
                id: _,
            } => {
                self.write(b"<a href=\"mailto:")?;
                escape_href(WriteWrapper(&mut self.writer), &dest_url)?;
                if !title.is_empty() {
                    self.write(b"\" title=\"")?;
                    escape_html(WriteWrapper(&mut self.writer), &title)?;
                }
                self.write(b"\">")
            }
            Tag::Link {
                link_type: _,
                dest_url,
                title,
                id: _,
            } => {
                self.write(b"<a href=\"")?;
                escape_href(WriteWrapper(&mut self.writer), &dest_url)?;
                if !title.is_empty() {
                    self.write(b"\" title=\"")?;
                    escape_html(WriteWrapper(&mut self.writer), &title)?;
                }
                self.write(b"\">")
            }
            Tag::Image {
                link_type: _,
                dest_url,
                title,
                id: _,
            } => {
                self.write(b"<img src=\"")?;
                escape_href(WriteWrapper(&mut self.writer), &dest_url)?;
                self.write(b"\" alt=\"")?;
                self.raw_text()?;
                if !title.is_empty() {
                    self.write(b"\" title=\"")?;
                    escape_html(WriteWrapper(&mut self.writer), &title)?;
                }
                self.write(b"\" />")
            }
            Tag::FootnoteDefinition(name) => {
                if self.end_newline {
                    self.write(b"<div class=\"footnote-definition\" id=\"")?;
                } else {
                    self.write(b"\n<div class=\"footnote-definition\" id=\"")?;
                }
                escape_html(WriteWrapper(&mut self.writer), &*name)?;
                self.write(b"\"><sup class=\"footnote-definition-label\">")?;
                let len = self.numbers.len() + 1;
                let number = *self.numbers.entry(name).or_insert(len);
                write!(&mut self.writer, "{}", number)?;
                self.write(b"</sup>")
            }
            Tag::MetadataBlock(_) => {
                self.in_non_writing_block = true;
                Ok(())
            }
        }
    }

    fn end_tag(&mut self, tag: TagEnd) -> io::Result<()> {
        match tag {
            TagEnd::HtmlBlock => {}
            TagEnd::Paragraph => {
                self.write(b"</p>\n")?;
            }
            TagEnd::Heading(level) => {
                let text = self
                    .current_header
                    .take()
                    .expect("header end but did not start");
                self.headers.push(HeadingInfo {
                    level: self.header_stack.effective_level(),
                    text,
                    number: self.header_stack.repr(),
                });
                self.write(b"</")?;
                write!(&mut self.writer, "{}", level)?;
                self.write(b">\n")?;
            }
            TagEnd::Table => {
                self.write(b"</tbody></table>\n")?;
            }
            TagEnd::TableHead => {
                self.write(b"</tr></thead><tbody>\n")?;
                self.table_state = TableState::Body;
            }
            TagEnd::TableRow => {
                self.write(b"</tr>\n")?;
            }
            TagEnd::TableCell => {
                match self.table_state {
                    TableState::Head => {
                        self.write(b"</th>")?;
                    }
                    TableState::Body => {
                        self.write(b"</td>")?;
                    }
                }
                self.table_cell_index += 1;
            }
            TagEnd::BlockQuote => {
                self.write(b"</blockquote>\n")?;
            }
            TagEnd::CodeBlock => {
                self.code = None;
                self.write(b"</code></pre>")?;
            }
            TagEnd::List(true) => {
                self.write(b"</ol>\n")?;
            }
            TagEnd::List(false) => {
                self.write(b"</ul>\n")?;
            }
            TagEnd::Item => {
                self.write(b"</li>\n")?;
            }
            TagEnd::Emphasis => {
                self.write(b"</em>")?;
            }
            TagEnd::Strong => {
                self.write(b"</strong>")?;
            }
            TagEnd::Strikethrough => {
                self.write(b"</del>")?;
            }
            TagEnd::Link => {
                self.write(b"</a>")?;
            }
            TagEnd::FootnoteDefinition => {
                self.write(b"</div>\n")?;
            }
            TagEnd::Image => (), // shouldn't happen, handled in start
            TagEnd::MetadataBlock(_) => {
                self.in_non_writing_block = false;
            }
        }
        Ok(())
    }

    // run raw text, consuming end tag
    fn raw_text(&mut self) -> io::Result<()> {
        let mut nest = 0;
        while let Some(event) = self.iter.next() {
            match event {
                Start(_) => nest += 1,
                End(_) => {
                    if nest == 0 {
                        break;
                    }
                    nest -= 1;
                }
                Html(_) => {},
                InlineHtml(text) | Code(text) | Text(text) => {
                    escape_html(WriteWrapper(&mut self.writer), &text)?;
                    self.end_newline = text.ends_with('\n');
                }
                SoftBreak | HardBreak | Rule => {
                    self.write(b" ")?;
                }
                FootnoteReference(name) => {
                    let len = self.numbers.len() + 1;
                    let number = *self.numbers.entry(name).or_insert(len);
                    write!(&mut self.writer, "[{}]", number)?;
                }
                TaskListMarker(true) => self.write(b"[x]")?,
                TaskListMarker(false) => self.write(b"[ ]")?,
            }
        }
        Ok(())
    }
}

/// Iterate over an `Iterator` of `Event`s, generate HTML for each `Event`, and
/// write it out to a writable stream.
///
/// **Note**: using this function with an unbuffered writer like a file or socket
/// will result in poor performance. Wrap these in a
/// [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html) to
/// prevent unnecessary slowdowns.
///
/// # Examples
///
/// ```
/// use pulldown_cmark::{html, Parser};
/// use std::io::Cursor;
///
/// let markdown_str = r#"
/// hello
/// =====
///
/// * alpha
/// * beta
/// "#;
/// let mut bytes = Vec::new();
/// let parser = Parser::new(markdown_str);
///
/// html::write_html(Cursor::new(&mut bytes), parser);
///
/// assert_eq!(&String::from_utf8_lossy(&bytes)[..], r#"<h1>hello</h1>
/// <ul>
/// <li>alpha</li>
/// <li>beta</li>
/// </ul>
/// "#);
/// ```
pub fn write_html<'a, I, W>(
    writer: W,
    iter: I,
    syntax: &'a SyntaxConfig,
) -> Result<Vec<HeadingInfo>>
where
    I: Iterator<Item = Event<'a>>,
    W: Write,
{
    HtmlWriter::new(iter, writer, syntax).run()
}
