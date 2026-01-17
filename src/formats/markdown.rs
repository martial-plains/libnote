#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use std::fmt::Write;

use regex::Regex;

use crate::{
    formats::{NoteMetadata, NoteSerialization},
    models::{
        Alignment, Attachment, AttachmentType, Block, Blocks, ContainerBlock, DefinitionItem,
        Inline, LeafBlock, LinkTarget, ListStyle, Note, Numbering, NumberingStyle, NumberingType,
    },
};

#[derive(Debug)]
pub struct MarkdownFormat;

impl MarkdownFormat {
    /// Parse a note from its file name and raw content
    #[must_use]
    pub fn parse_note(&self, file_name: &str, content: &str) -> Note {
        let yaml_title = Self::extract_yaml_title(content);
        let title = yaml_title.unwrap_or_else(|| Self::strip_extension(file_name));
        let body = Self::strip_yaml_frontmatter(content);

        let blocks = parse_blocks(&body);

        Note {
            id: file_name.to_string(),
            title,
            blocks,
        }
    }

    fn extract_yaml_title(content: &str) -> Option<String> {
        let re = Regex::new(r"(?s)^---\s*(.*?)\s*---").ok()?;
        let caps = re.captures(content)?;
        let yaml_block = caps.get(1)?.as_str();

        for line in yaml_block.lines() {
            if let Some(title_val) = line.strip_prefix("title:") {
                return Some(title_val.trim().trim_matches('"').to_string());
            }
        }
        None
    }

    fn strip_yaml_frontmatter(content: &str) -> String {
        let re = Regex::new(r"(?s)^---\s*.*?\s*---\s*").unwrap();
        re.replace(content, "").trim().to_string()
    }

    fn strip_extension(name: &str) -> String {
        name.trim_end_matches(".md").to_string()
    }

    fn filename_stem(path: &str) -> String {
        let name_part = path.rsplit_once(['/', '\\']).map_or(path, |(_, name)| name);

        match name_part.rsplit_once('.') {
            Some((stem, _ext)) => stem.to_string(),
            None => name_part.to_string(),
        }
    }

    fn serialize_block(block: &Block) -> String {
        match block {
            Block::DefinitionList { items } => Self::serialize_definition_list(items),
            Block::FootnoteDefinition { label, content } => {
                Self::serialize_footnote(label, content)
            }
            Block::Container { container } => Self::serialize_container(container),
            Block::Leaf { leaf } => Self::serialize_leaf(leaf),
        }
    }

    fn serialize_definition_list(items: &[DefinitionItem]) -> String {
        let mut out = String::new();
        for item in items {
            out.push_str(&serialize_inlines(&item.term));
            out.push_str(":\n");
            for def_block in &item.definition {
                for line in serialize_blocks(std::slice::from_ref(def_block)).lines() {
                    out.push_str("  ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        out
    }

    fn serialize_footnote(label: &str, content: &[Block]) -> String {
        let mut out = String::new();
        write!(&mut out, "[^{label}]: ").unwrap();
        let content_str = serialize_blocks(content);
        out.push_str(&content_str.replace('\n', "\n    "));
        out.push('\n');
        out
    }

    fn serialize_container(container: &ContainerBlock) -> String {
        match container {
            ContainerBlock::List { style, items } => Self::serialize_list(style, items),
            ContainerBlock::Table { headers, rows, .. } => Self::serialize_table(headers, rows),
            ContainerBlock::Quote { blocks } => Self::serialize_quote(blocks),
            ContainerBlock::Div { children, .. } => Self::serialize_div(children),
        }
    }

    fn serialize_list(style: &crate::models::ListStyle, items: &[Vec<Block>]) -> String {
        let mut out = String::new();

        for (i, item_blocks) in items.iter().enumerate() {
            let prefix = match style {
                crate::models::ListStyle::Ordered { .. } => format!("{}. ", i + 1),
                crate::models::ListStyle::Unordered { bullet } => format!("{bullet} "),
            };

            if let Some((first, rest)) = item_blocks.split_first() {
                let first_serialized = serialize_blocks(core::slice::from_ref(first));
                let mut lines = first_serialized.lines();
                if let Some(first_line) = lines.next() {
                    out.push_str(&prefix);
                    out.push_str(first_line);
                    out.push('\n');
                }
                for line in lines {
                    out.push_str("  ");
                    out.push_str(line);
                    out.push('\n');
                }

                for block in rest {
                    for nested_line in serialize_blocks(core::slice::from_ref(block)).lines() {
                        out.push_str("  ");
                        out.push_str(nested_line);
                        out.push('\n');
                    }
                }
            }
        }
        out
    }

    fn serialize_table(headers: &[Vec<Inline>], rows: &[Vec<Vec<Inline>>]) -> String {
        let mut out = String::new();

        // headers
        for (i, header) in headers.iter().enumerate() {
            if i > 0 {
                out.push('|');
            }
            out.push(' ');
            out.push_str(&serialize_inlines(header));
            out.push(' ');
        }
        out.push('\n');

        // separator
        for i in 0..headers.len() {
            if i > 0 {
                out.push('|');
            }
            out.push_str(" --- ");
        }
        out.push('\n');

        // rows
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i > 0 {
                    out.push('|');
                }
                out.push(' ');
                out.push_str(&serialize_inlines(cell));
                out.push(' ');
            }
            out.push('\n');
        }

        out
    }

    fn serialize_quote(blocks: &[Block]) -> String {
        let mut out = String::new();
        for b in blocks {
            let lines = serialize_blocks(core::slice::from_ref(b))
                .lines()
                .map(|l| format!("> {l}"))
                .collect::<Vec<_>>()
                .join("\n");
            out.push_str(&lines);
            out.push('\n');
        }
        out
    }

    fn serialize_div(children: &[Block]) -> String {
        children
            .iter()
            .map(|c| serialize_blocks(std::slice::from_ref(c)))
            .collect()
    }

    fn serialize_leaf(leaf: &LeafBlock) -> String {
        match leaf {
            LeafBlock::Heading { level, content } => {
                format!(
                    "{} {}\n",
                    "#".repeat(*level as usize),
                    serialize_inlines(content)
                )
            }
            LeafBlock::Paragraph { content } => format!("{}\n", serialize_inlines(content)),
            LeafBlock::Image { alt_text, src } => {
                format!("![{}]({})\n", alt_text.clone().unwrap_or_default(), src)
            }
            LeafBlock::CodeBlock { language, content } => {
                let lang = language.as_deref().unwrap_or("");
                format!("```{lang}\n{content}\n```\n")
            }
            LeafBlock::MathBlock { content } => format!("$$\n{content}\n$$\n"),
            LeafBlock::Attachment {
                attachment: Attachment { src, name, kind: _ },
            } => format!("![{name}]({src})\n"),
            LeafBlock::HorizontalRule => String::from("---\n"),
        }
    }
}

impl NoteSerialization for MarkdownFormat {
    fn deserialize(&self, data: &[u8], id_hint: Option<&str>) -> Note {
        let input = core::str::from_utf8(data).unwrap_or("");

        let yaml_title = Self::extract_yaml_title(input);
        let body = Self::strip_yaml_frontmatter(input);

        let mut title = yaml_title.clone().unwrap_or_else(|| {
            let hint = id_hint.unwrap_or_default();
            Self::filename_stem(hint)
        });

        let mut lines = body.lines();
        let mut body_lines = Vec::new();

        for line in lines.by_ref() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if yaml_title.is_none()
                && let Some(stripped) = trimmed.strip_prefix("# ")
            {
                title = stripped.trim().to_string();
                continue;
            }

            body_lines.push(line);
            break;
        }

        body_lines.extend(lines);
        let clean_body = body_lines.join("\n");

        let blocks = parse_blocks(&clean_body);

        let id = id_hint.map_or_else(|| uuid::Uuid::new_v4().to_string(), Self::filename_stem);

        Note { id, title, blocks }
    }

    fn serialize(&self, note: &Note) -> Vec<u8> {
        let mut output = String::new();
        output.push_str("# ");
        output.push_str(&note.title);
        output.push('\n');

        for block in &note.blocks {
            output.push_str(&Self::serialize_block(block));
        }

        output.into_bytes()
    }
}

impl NoteMetadata for MarkdownFormat {
    fn extract_links(&self, note: &Note, attachments: &[Attachment]) -> Vec<LinkTarget> {
        fn process_inlines(
            inlines: &[Inline],
            links: &mut Vec<LinkTarget>,
            is_attachment: &dyn Fn(&str) -> bool,
        ) {
            for inline in inlines {
                match inline {
                    Inline::Link { text: _, target } => {
                        if is_attachment(target) {
                            links.push(LinkTarget::Attachment(target.clone()));
                        } else {
                            links.push(LinkTarget::Note(target.clone()));
                        }
                    }
                    Inline::Image { alt_text: _, src } => {
                        if is_attachment(src) {
                            links.push(LinkTarget::Attachment(src.clone()));
                        }
                    }
                    Inline::Bold { content: inner }
                    | Inline::Italic { content: inner }
                    | Inline::Strikethrough { content: inner } => {
                        process_inlines(inner, links, is_attachment);
                    }
                    _ => {}
                }
            }
        }

        fn process_blocks(
            blocks: &[Block],
            links: &mut Vec<LinkTarget>,
            is_attachment: &dyn Fn(&str) -> bool,
        ) {
            for block in blocks {
                match block {
                    Block::Leaf { leaf: leaf_block } => match leaf_block {
                        LeafBlock::Paragraph { content: inlines } => {
                            process_inlines(inlines, links, is_attachment);
                        }
                        LeafBlock::Heading {
                            content: inlines, ..
                        } => process_inlines(inlines, links, is_attachment),

                        LeafBlock::Image { src, .. } => {
                            if is_attachment(src) {
                                links.push(LinkTarget::Attachment(src.clone()));
                            }
                        }
                        _ => {}
                    },

                    Block::Container {
                        container: container_block,
                    } => match container_block {
                        ContainerBlock::Quote {
                            blocks: inner_blocks,
                        } => process_blocks(inner_blocks, links, is_attachment),
                        ContainerBlock::List { items, .. } => {
                            for item in items {
                                process_blocks(item, links, is_attachment);
                            }
                        }
                        ContainerBlock::Table { headers, rows, .. } => {
                            for row in headers.iter().chain(rows.iter().flatten()) {
                                process_inlines(row, links, is_attachment);
                            }
                        }

                        ContainerBlock::Div { .. } => {}
                    },

                    _ => {}
                }
            }
        }

        let mut links = Vec::new();

        let is_attachment = |target: &str| attachments.iter().any(|a| a.src == target);

        process_blocks(&note.blocks, &mut links, &is_attachment);

        links
    }
}

fn serialize_blocks(blocks: &[Block]) -> String {
    let mut out = String::new();
    for b in blocks {
        match b {
            Block::Container { container } => match container {
                ContainerBlock::List { .. }
                | ContainerBlock::Quote { .. }
                | ContainerBlock::Table { .. } => {
                    out.push_str(
                        &String::from_utf8(MarkdownFormat.serialize(&Note {
                            id: String::new(),
                            title: String::new(),
                            blocks: vec![b.clone()],
                        }))
                        .unwrap_or_default(),
                    );
                }
                ContainerBlock::Div { children, .. } => {
                    for child in children {
                        out.push_str(&serialize_blocks(std::slice::from_ref(child)));
                    }
                }
            },
            Block::Leaf { leaf } => match leaf {
                LeafBlock::Image { .. } => {
                    out.push_str(
                        &String::from_utf8(MarkdownFormat.serialize(&Note {
                            id: String::new(),
                            title: String::new(),
                            blocks: vec![b.clone()],
                        }))
                        .unwrap_or_default(),
                    );
                }
                LeafBlock::Paragraph { content: inlines } => {
                    out.push_str(&serialize_inlines(inlines));
                }
                LeafBlock::Heading {
                    level,
                    content: inlines,
                } => {
                    out.push_str(&"#".repeat(*level as usize));
                    out.push(' ');
                    out.push_str(&serialize_inlines(inlines));
                }

                LeafBlock::HorizontalRule => out.push_str("---\n"),

                _ => {}
            },

            Block::DefinitionList { items } => {
                for item in items {
                    out.push_str(&serialize_inlines(&item.term));
                    out.push_str(":\n");
                    for def_block in &item.definition {
                        let def_serialized = serialize_blocks(std::slice::from_ref(def_block));
                        for line in def_serialized.lines() {
                            out.push_str("  ");
                            out.push_str(line);
                            out.push('\n');
                        }
                    }
                }
            }

            Block::FootnoteDefinition { label, content } => {
                write!(&mut out, "[^{label}]: ").unwrap();
                let content_str = serialize_blocks(content);
                write!(&mut out, "{}", content_str.replace('\n', "\n    ")).unwrap();
                writeln!(&mut out).unwrap();
            }
        }
        out.push('\n');
    }
    out
}

pub fn parse_blocks(input: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines = input.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(stripped) = trimmed.strip_prefix("```") {
            let language = stripped.trim();
            let mut content = String::new();
            for next in lines.by_ref() {
                if next.trim() == "```" {
                    break;
                }
                content.push_str(next);
                content.push('\n');
            }

            if content.ends_with('\n') {
                content.pop();
            }

            blocks.push(Block::code_block(
                if language.is_empty() {
                    None
                } else {
                    Some(language.to_string())
                },
                content,
            ));

            continue;
        }

        if let Some(mut content) = trimmed.strip_prefix("$$") {
            if content.ends_with("$$") {
                content = &content[..content.len() - 2];
                blocks.push(Block::math_block(content.to_string()));
                continue;
            }

            let mut full_content = String::new();
            if !content.is_empty() {
                full_content.push_str(content);
            }

            for next in lines.by_ref() {
                let next_trimmed = next.trim_end();
                if !full_content.is_empty() {
                    full_content.push('\n');
                }
                if let Some(stripped_end) = next_trimmed.strip_suffix("$$") {
                    full_content.push_str(stripped_end);
                    break;
                }
                full_content.push_str(next_trimmed);
            }

            if full_content.ends_with('\n') {
                full_content.pop();
            }

            blocks.push(Block::math_block(full_content));
            continue;
        }

        if let Some(header) = parse_markdown_header(trimmed) {
            blocks.push(header);
            continue;
        }

        if trimmed.starts_with('>') {
            let mut quote_lines = vec![trimmed.trim_start_matches('>').trim_start().to_string()];

            while let Some(next_line) = lines.peek() {
                if next_line.trim_start().starts_with('>') {
                    let clean_line = next_line
                        .trim_start()
                        .trim_start_matches('>')
                        .trim_start()
                        .to_string();
                    quote_lines.push(clean_line);
                    lines.next();
                } else {
                    break;
                }
            }

            let inner = quote_lines.join("\n").trim().to_string();

            if !inner.is_empty() {
                blocks.push(Block::quote(parse_blocks(&inner)));
            }

            continue;
        }

        if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            let mut list_lines = vec![line];
            while let Some(&next_line) = lines.peek() {
                let next_trimmed = next_line.trim_start();
                if next_trimmed.starts_with("- ")
                    || next_trimmed.starts_with("* ")
                    || next_trimmed.starts_with("+ ")
                    || next_trimmed
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit())
                    || next_trimmed.is_empty()
                {
                    list_lines.push(lines.next().unwrap());
                } else {
                    break;
                }
            }

            let list_text = list_lines.join("\n");
            if let Some(list) = parse_list(&list_text) {
                blocks.push(list);
            }
            continue;
        }

        if trimmed.contains('|')
            && let Some(table) = parse_table(trimmed)
        {
            blocks.push(table);
            continue;
        }

        if let Some((alt, src)) = parse_image(trimmed) {
            let image_line = format!("![{}]({})", alt.unwrap_or(""), src);
            if trimmed == image_line {
                blocks.push(Block::image(
                    alt.map(std::string::ToString::to_string),
                    src.to_string(),
                ));
                continue;
            }
        }

        blocks.push(Block::paragraph(parse_inlines(trimmed)));
    }

    blocks
}

fn serialize_inlines(inlines: &[Inline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text } => output.push_str(text),
            Inline::Bold { content } => {
                output.push_str("**");
                output.push_str(&serialize_inlines(content));
                output.push_str("**");
            }
            Inline::Italic { content } => {
                output.push('*');
                output.push_str(&serialize_inlines(content));
                output.push('*');
            }
            Inline::Strikethrough { content } => {
                output.push_str("~~");
                output.push_str(&serialize_inlines(content));
                output.push_str("~~");
            }
            Inline::Link { text, target } => {
                output.push('[');
                output.push_str(&serialize_inlines(text));
                output.push_str("](");
                output.push_str(target);
                output.push(')');
            }
            Inline::Image { alt_text, src } => {
                output.push_str("![");
                if let Some(alt) = alt_text {
                    output.push_str(alt);
                }
                output.push_str("](");
                output.push_str(src);
                output.push(')');
            }
            Inline::Code { code } => {
                output.push('`');
                output.push_str(code);
                output.push('`');
            }
            Inline::Math { content } => {
                output.push('$');
                output.push_str(content);
                output.push('$');
            }
            Inline::LineBreak => output.push_str("  \n"),
            Inline::Superscript { content } => {
                output.push('^');
                output.push_str(&serialize_inlines(content));
            }
            Inline::Subscript { content } => {
                output.push('_');
                output.push_str(&serialize_inlines(content));
            }
            Inline::FootnoteReference { label } => {
                output.push_str("[^");
                output.push_str(label);
                output.push(']');
            }
        }
    }
    output
}

fn parse_markdown_header(line: &str) -> Option<Block> {
    let trimmed = line.trim_start();
    let mut chars = trimmed.chars().peekable();

    let mut level = 0;
    while matches!(chars.peek(), Some('#')) {
        chars.next();
        level += 1;
    }

    if level == 0 || chars.next() != Some(' ') {
        return None;
    }

    let content: String = chars.collect();

    Some(Block::heading(
        level,
        vec![Inline::Text {
            text: content.trim().to_string(),
        }],
    ))
}

#[must_use]
pub fn parse_list(input: &str) -> Option<Block> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() {
        return None;
    }

    let mut items: Vec<Blocks> = Vec::new();
    let mut i = 0;
    let mut list_style: Option<ListStyle> = None;

    while i < lines.len() {
        if let Some((item_blocks, next_index, style)) = parse_list_item(&lines, i) {
            if list_style.is_none() {
                list_style = Some(style);
            }
            items.push(item_blocks);
            i = next_index;
        } else {
            i += 1;
        }
    }

    if items.is_empty() {
        None
    } else {
        Some(Block::list(
            list_style.unwrap_or(ListStyle::Unordered { bullet: b'-' }),
            items,
        ))
    }
}

fn parse_list_item(lines: &[&str], start_index: usize) -> Option<(Blocks, usize, ListStyle)> {
    if start_index >= lines.len() {
        return None;
    }

    let line = lines[start_index];
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();

    let (list_style, content) = if let Some(stripped) = trimmed.strip_prefix("- ") {
        (ListStyle::Unordered { bullet: b'-' }, stripped)
    } else if let Some(stripped) = trimmed.strip_prefix("* ") {
        (ListStyle::Unordered { bullet: b'*' }, stripped)
    } else if let Some(stripped) = trimmed.strip_prefix("+ ") {
        (ListStyle::Unordered { bullet: b'+' }, stripped)
    } else if let Some(dot_pos) = trimmed.find('.') {
        if trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit())
            && trimmed[dot_pos + 1..].starts_with(' ')
        {
            (
                ListStyle::Ordered {
                    numbering: Numbering {
                        kind: NumberingType::Decimal,
                        style: NumberingStyle::Dot,
                    },
                },
                &trimmed[dot_pos + 2..],
            )
        } else {
            return None;
        }
    } else {
        return None;
    };

    let mut item_blocks: Blocks = Vec::new();

    item_blocks.push(Block::paragraph(parse_inlines(content.trim())));

    let mut nested_lines = Vec::new();
    let mut i = start_index + 1;

    while i < lines.len() {
        let next_line = lines[i];
        let next_trimmed = next_line.trim_start();
        let next_indent = next_line.len() - next_trimmed.len();

        if next_trimmed.is_empty() {
            i += 1;
        } else if next_indent > indent {
            nested_lines.push(next_line);
            i += 1;
        } else {
            break;
        }
    }

    if !nested_lines.is_empty() {
        let nested_input = nested_lines.join("\n");
        let nested_blocks = parse_blocks(&nested_input);
        item_blocks.extend(nested_blocks);
    }

    Some((item_blocks, i, list_style))
}

fn parse_table(input: &str) -> Option<Block> {
    let mut lines: Vec<&str> = input.lines().filter(|l| l.contains('|')).collect();
    if lines.len() < 2 {
        return None;
    }

    let headers = lines[0]
        .split('|')
        .map(|c| parse_inlines(c.trim()))
        .collect::<Vec<_>>();

    let alignments = if lines.len() >= 2
        && lines[1]
            .trim()
            .chars()
            .all(|c| c == '-' || c == ':' || c == '|' || c.is_whitespace())
    {
        let alignment_line = lines.remove(1);
        let detected = alignment_line
            .split('|')
            .map(|cell| {
                let cell = cell.trim();
                match (cell.starts_with(':'), cell.ends_with(':')) {
                    (true, true) => Alignment::Center,
                    (true, false) => Alignment::Left,
                    (false, true) => Alignment::Right,
                    _ => Alignment::default(),
                }
            })
            .collect::<Vec<_>>();
        Some(detected)
    } else {
        None
    };

    let rows = lines[1..]
        .iter()
        .map(|row| {
            row.split('|')
                .map(|c| parse_inlines(c.trim()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let caption = None;

    Some(Block::table(headers, rows, alignments, caption))
}

fn parse_image(line: &str) -> Option<(Option<&str>, &str)> {
    let start = line.find("![")?;
    let end_alt = line[start..].find(']')? + start;
    let alt = &line[start + 2..end_alt];
    let paren_start = line[end_alt..].find('(')? + end_alt + 1;
    let paren_end = line[paren_start..].find(')')? + paren_start;
    let src = &line[paren_start..paren_end];

    Some((if alt.is_empty() { None } else { Some(alt) }, src))
}

#[must_use]
pub fn parse_inlines(input: &str) -> Vec<Inline> {
    let mut result = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.peek().copied() {
        match c {
            '*' => {
                chars.next();
                if chars.peek().is_some_and(|&c| c == '*') {
                    chars.next();
                    let content = parse_until(&mut chars, "**");
                    consume_delimiter(&mut chars, "**");
                    result.push(Inline::Bold {
                        content: parse_inlines(&content),
                    });
                } else {
                    let content = parse_until(&mut chars, "*");
                    consume_delimiter(&mut chars, "*");
                    result.push(Inline::Italic {
                        content: parse_inlines(&content),
                    });
                }
            }

            '~' => {
                chars.next();
                if chars.peek().is_some_and(|&c| c == '~') {
                    chars.next();
                    let content = parse_until(&mut chars, "~~");
                    consume_delimiter(&mut chars, "~~");
                    result.push(Inline::Strikethrough {
                        content: parse_inlines(&content),
                    });
                } else {
                    result.push(Inline::Text {
                        text: "~".to_string(),
                    });
                }
            }

            '`' => {
                chars.next();
                let content = parse_until(&mut chars, "`");
                consume_delimiter(&mut chars, "`");
                result.push(Inline::Code { code: content });
            }

            '!' => {
                let mut clone = chars.clone();
                clone.next();
                if clone.peek() == Some(&'[') {
                    chars.next();
                    chars.next();
                    let alt_text = parse_until(&mut chars, "]");
                    consume_delimiter(&mut chars, "]");
                    if chars.peek() == Some(&'(') {
                        chars.next();
                        let src = parse_until(&mut chars, ")");
                        consume_delimiter(&mut chars, ")");
                        result.push(Inline::Image {
                            alt_text: if alt_text.is_empty() {
                                None
                            } else {
                                Some(alt_text)
                            },
                            src,
                        });
                        continue;
                    }
                }
                chars.next();
                result.push(Inline::Text {
                    text: "!".to_string(),
                });
            }

            '$' => {
                chars.next();
                let content = parse_until(&mut chars, "$");
                consume_delimiter(&mut chars, "$");
                result.push(Inline::Math { content });
            }

            '[' => {
                chars.next();
                if chars.peek() == Some(&'[') {
                    chars.next();
                    let text = parse_until(&mut chars, "]]");
                    consume_delimiter(&mut chars, "]]");
                    result.push(Inline::Link {
                        text: vec![Inline::Text { text: text.clone() }],
                        target: text,
                    });
                } else {
                    let text = parse_until(&mut chars, "]");
                    consume_delimiter(&mut chars, "]");
                    if chars.peek() == Some(&'(') {
                        chars.next();
                        let target = parse_until(&mut chars, ")");
                        consume_delimiter(&mut chars, ")");
                        result.push(Inline::Link {
                            text: parse_inlines(&text),
                            target,
                        });
                    } else {
                        result.push(Inline::Text {
                            text: format!("[{text}]"),
                        });
                    }
                }
            }

            _ => {
                let mut text = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '*'
                        || next == '`'
                        || next == '['
                        || next == '!'
                        || next == '$'
                        || next == '~'
                    {
                        break;
                    }
                    text.push(chars.next().unwrap());
                }
                result.push(Inline::Text { text });
            }
        }
    }

    result
}

fn parse_until<I>(chars: &mut core::iter::Peekable<I>, delimiter: &str) -> String
where
    I: Iterator<Item = char> + Clone,
{
    let mut buffer = String::new();
    let delim_chars: Vec<char> = delimiter.chars().collect();
    let delim_len = delim_chars.len();

    while chars.peek().is_some() {
        if delimiter_matches(chars.clone(), &delim_chars) {
            for _ in 0..delim_len {
                chars.next();
            }
            break;
        }
        buffer.push(chars.next().unwrap());
    }

    buffer
}

#[must_use]
pub fn extract_attachments(blocks: &[Block]) -> Vec<Attachment> {
    fn push_attachment(attachments: &mut Vec<Attachment>, path: &str) {
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_string();

        let kind = match path.rsplit('.').next().map(str::to_lowercase) {
            Some(ext) if ["png", "jpg", "jpeg", "gif", "bmp", "webp"].contains(&ext.as_str()) => {
                AttachmentType::Image
            }
            Some(ext) if ["mp3", "wav", "ogg", "flac"].contains(&ext.as_str()) => {
                AttachmentType::Audio
            }
            Some(ext) if ["mp4", "mkv", "mov", "avi"].contains(&ext.as_str()) => {
                AttachmentType::Video
            }
            Some(ext) if ["pdf", "doc", "docx", "txt", "md"].contains(&ext.as_str()) => {
                AttachmentType::Document
            }
            Some(other) => AttachmentType::Other { mime: other },
            None => AttachmentType::Other {
                mime: "unknown".to_string(),
            },
        };

        attachments.push(Attachment {
            src: path.to_string(),
            name,
            kind,
        });
    }

    let mut attachments = Vec::new();

    for block in blocks {
        match block {
            Block::Container { container } => match container {
                ContainerBlock::Quote { blocks: inner } => {
                    attachments.extend(extract_attachments(inner));
                }

                ContainerBlock::List { items, .. } => {
                    for item_blocks in items {
                        attachments.extend(extract_attachments(item_blocks));
                    }
                }

                ContainerBlock::Table { rows, headers, .. } => {
                    for cell_row in headers.iter().chain(rows.iter().flatten()) {
                        for inline in cell_row {
                            match inline {
                                Inline::Image { src, .. } => push_attachment(&mut attachments, src),
                                Inline::Link { target, .. }
                                    if target.starts_with("![[") && target.ends_with("]]") =>
                                {
                                    let path = &target[3..target.len() - 2];
                                    push_attachment(&mut attachments, path);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                ContainerBlock::Div { .. } => {}
            },

            Block::Leaf { leaf } => match leaf {
                LeafBlock::Image { src, .. } => push_attachment(&mut attachments, src),
                LeafBlock::Attachment { attachment } => attachments.push(attachment.clone()),
                LeafBlock::Paragraph { content } | LeafBlock::Heading { content, .. } => {
                    for inline in content {
                        match inline {
                            Inline::Image { src, .. } => push_attachment(&mut attachments, src),
                            Inline::Link { target, .. }
                                if target.starts_with("![[") && target.ends_with("]]") =>
                            {
                                let path = &target[3..target.len() - 2];
                                push_attachment(&mut attachments, path);
                            }
                            Inline::Bold { content }
                            | Inline::Italic { content }
                            | Inline::Strikethrough { content } => {
                                for inner in content {
                                    if let Inline::Image { src, .. } = inner {
                                        push_attachment(&mut attachments, src);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },

            _ => {}
        }
    }

    attachments
}

fn delimiter_matches<I>(mut chars: core::iter::Peekable<I>, delimiter: &[char]) -> bool
where
    I: Iterator<Item = char> + Clone,
{
    for &d in delimiter {
        match chars.next() {
            Some(c) if c == d => {}
            _ => return false,
        }
    }
    true
}

fn consume_delimiter<I>(chars: &mut core::iter::Peekable<I>, delimiter: &str)
where
    I: Iterator<Item = char>,
{
    for d in delimiter.chars() {
        if chars.peek() == Some(&d) {
            chars.next();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::Inline;

    use super::*;

    fn sample_note() -> Note {
        Note {
            id: "test-id".to_string(),
            title: "Sample Note".to_string(),
            blocks: vec![
                Block::heading(
                    2,
                    vec![Inline::Text {
                        text: "Heading Example".to_string(),
                    }],
                ),
                Block::paragraph(vec![
                    Inline::Text {
                        text: "This is a ".to_string(),
                    },
                    Inline::Bold {
                        content: vec![Inline::Text {
                            text: "bold".to_string(),
                        }],
                    },
                    Inline::Text {
                        text: " and ".to_string(),
                    },
                    Inline::Italic {
                        content: vec![Inline::Text {
                            text: "italic".to_string(),
                        }],
                    },
                    Inline::Text {
                        text: " text.".to_string(),
                    },
                ]),
                Block::image(Some("Alt text".to_string()), "image.png".to_string()),
                Block::list(
                    ListStyle::Unordered { bullet: b'-' },
                    vec![
                        vec![Block::paragraph(vec![Inline::Text {
                            text: "Item 1".to_string(),
                        }])],
                        vec![Block::paragraph(vec![Inline::Text {
                            text: "Item 2".to_string(),
                        }])],
                    ],
                ),
            ],
        }
    }

    #[test]
    fn test_parse_all_heading_levels() {
        let cases = vec![
            ("# Heading 1", 1, "Heading 1"),
            ("## Heading 2", 2, "Heading 2"),
            ("### Heading 3", 3, "Heading 3"),
            ("#### Heading 4", 4, "Heading 4"),
            ("##### Heading 5", 5, "Heading 5"),
            ("###### Heading 6", 6, "Heading 6"),
        ];

        for (input, expected_level, expected_text) in cases {
            let h = parse_markdown_header(input);
            assert!(h.is_some(), "Failed to parse header: {input:?}");

            let block = h.unwrap();
            if let Some(LeafBlock::Heading { level, content }) = block.as_heading() {
                assert_eq!(
                    level, expected_level,
                    "Wrong heading level for input: {input:?}"
                );
                assert_eq!(
                    content,
                    vec![Inline::Text {
                        text: expected_text.into()
                    }],
                    "Wrong content for input: {input:?}"
                );
            } else {
                panic!("Expected a Heading block for input: {input:?}, got: {block:?}");
            }
        }
    }

    #[test]
    fn test_parse_unordered_list() {
        let line = "- Item 1";
        let result = parse_list(line);
        assert!(result.is_some());

        if let Some(ContainerBlock::List { items, style }) = result.unwrap().as_list() {
            assert!(!style.is_ordered(), "Expected unordered list");
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].len(), 1);

            match &items[0][0].as_paragraph() {
                Some(LeafBlock::Paragraph { content }) => {
                    assert_eq!(
                        content,
                        &vec![Inline::Text {
                            text: "Item 1".into()
                        }]
                    );
                }
                _ => panic!("Expected Paragraph block"),
            }
        } else {
            panic!("Expected List block");
        }
    }

    #[test]
    fn test_parse_ordered_list() {
        let line = "1. First item";
        let result = parse_list(line);
        assert!(result.is_some());

        if let Some(ContainerBlock::List { items, style }) = result.unwrap().as_list() {
            assert!(style.is_ordered(), "Expected ordered list");
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].len(), 1);

            match &items[0][0].as_paragraph() {
                Some(LeafBlock::Paragraph { content }) => {
                    assert_eq!(
                        content,
                        &vec![Inline::Text {
                            text: "First item".into()
                        }]
                    );
                }
                _ => panic!("Expected Paragraph block"),
            }
        } else {
            panic!("Expected List block");
        }
    }

    #[test]
    fn test_parse_list_invalid() {
        let line = "* Valid unordered list";
        assert!(parse_list(line).is_some());

        let line2 = "2) Also not valid";
        assert!(parse_list(line2).is_none());
    }

    #[test]
    fn test_parse_simple_table() {
        let input = "\
Header 1 | Header 2
Value 1  | Value 2";

        let result = parse_table(input);
        assert!(result.is_some());

        if let Some(ContainerBlock::Table { headers, rows, .. }) = result.unwrap().as_table() {
            assert_eq!(
                headers,
                vec![
                    vec![Inline::Text {
                        text: "Header 1".into()
                    }],
                    vec![Inline::Text {
                        text: "Header 2".into()
                    }]
                ]
            );

            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0],
                vec![
                    vec![Inline::Text {
                        text: "Value 1".into()
                    }],
                    vec![Inline::Text {
                        text: "Value 2".into()
                    }]
                ]
            );
        } else {
            panic!("Expected Table block");
        }
    }

    #[test]
    fn test_parse_table_too_short() {
        let input = "Only one row | not enough";
        let result = parse_table(input);
        assert!(result.is_none(), "Expected None for short table input");
    }

    #[test]
    fn test_parse_image_with_alt() {
        let line = "Here is an image ![alt text](image.png)";
        let result = parse_image(line);
        assert!(result.is_some());

        let (alt, src) = result.unwrap();
        assert_eq!(alt, Some("alt text"));
        assert_eq!(src, "image.png");
    }

    #[test]
    fn test_parse_image_without_alt() {
        let line = "Look: ![](no-alt.png)";
        let result = parse_image(line);
        assert!(result.is_some());

        let (alt, src) = result.unwrap();
        assert_eq!(alt, None);
        assert_eq!(src, "no-alt.png");
    }

    #[test]
    fn test_parse_image_invalid_format() {
        let line = "No image here";
        assert!(parse_image(line).is_none());

        let malformed = "![alt text](missing-end";
        assert!(parse_image(malformed).is_none());
    }

    #[test]
    fn test_parse_code_block() {
        let input = "```rust\nlet x = 42;\nprintln!(\"{}\", x);\n```";
        let blocks = parse_blocks(input);

        assert_eq!(blocks.len(), 1);

        match &blocks[0].as_code_block() {
            Some(LeafBlock::CodeBlock { language, content }) => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert_eq!(content, "let x = 42;\nprintln!(\"{}\", x);");
            }
            _ => panic!("Expected a CodeBlock"),
        }
    }

    #[test]
    fn test_parse_code_block_no_language() {
        let input = "```\nHello world\n```";
        let blocks = parse_blocks(input);

        assert_eq!(blocks.len(), 1);

        match &blocks[0].as_code_block() {
            Some(LeafBlock::CodeBlock { language, content }) => {
                assert!(language.is_none());
                assert_eq!(content, "Hello world");
            }
            _ => panic!("Expected a CodeBlock"),
        }
    }

    #[test]
    fn test_parse_math_block_single_line() {
        let input = "$$x^2 + y^2 = z^2$$";
        let blocks = parse_blocks(input);

        assert_eq!(blocks.len(), 1);

        match &blocks[0].as_math_block() {
            Some(LeafBlock::MathBlock { content }) => {
                assert_eq!(content, "x^2 + y^2 = z^2");
            }
            _ => panic!("Expected a MathBlock"),
        }
    }

    #[test]
    fn test_parse_math_block_multi_line() {
        let input = "$$\nx^2 + y^2 = z^2\nx + y = z\n$$";
        let blocks = parse_blocks(input);

        assert_eq!(blocks.len(), 1);

        match &blocks[0].as_math_block() {
            Some(LeafBlock::MathBlock { content }) => {
                assert_eq!(content, "x^2 + y^2 = z^2\nx + y = z");
            }
            _ => panic!("Expected a MathBlock"),
        }
    }

    #[test]
    fn test_parse_plain_text() {
        let input = "Just plain text.";
        let expected = vec![Inline::Text {
            text: "Just plain text.".into(),
        }];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_italic() {
        let input = "This is *italic* text.";
        let expected = vec![
            Inline::Text {
                text: "This is ".into(),
            },
            Inline::Italic {
                content: vec![Inline::Text {
                    text: "italic".into(),
                }],
            },
            Inline::Text {
                text: " text.".into(),
            },
        ];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_bold() {
        let input = "This is **bold** text.";
        let expected = vec![
            Inline::Text {
                text: "This is ".into(),
            },
            Inline::Bold {
                content: vec![Inline::Text {
                    text: "bold".into(),
                }],
            },
            Inline::Text {
                text: " text.".into(),
            },
        ];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_link() {
        let input = "Check out [Rust](https://www.rust-lang.org)!";
        let expected = vec![
            Inline::Text {
                text: "Check out ".into(),
            },
            Inline::Link {
                text: vec![Inline::Text {
                    text: "Rust".into(),
                }],
                target: "https://www.rust-lang.org".into(),
            },
            Inline::Text { text: "!".into() },
        ];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_nested_formatting() {
        let input = "**bold and *italic inside***";
        let expected = vec![Inline::Bold {
            content: vec![
                Inline::Text {
                    text: "bold and ".into(),
                },
                Inline::Italic {
                    content: vec![Inline::Text {
                        text: "italic inside".into(),
                    }],
                },
            ],
        }];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_mixed_content() {
        let input = "*italic* and **bold**, then [link](url)";
        let expected = vec![
            Inline::Italic {
                content: vec![Inline::Text {
                    text: "italic".into(),
                }],
            },
            Inline::Text {
                text: " and ".into(),
            },
            Inline::Bold {
                content: vec![Inline::Text {
                    text: "bold".into(),
                }],
            },
            Inline::Text {
                text: ", then ".into(),
            },
            Inline::Link {
                text: vec![Inline::Text {
                    text: "link".into(),
                }],
                target: "url".into(),
            },
        ];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_nested_list() {
        let input = "- Item 1\n  - Nested 1\n  - Nested 2\n- Item 2";
        let result = parse_list(input).unwrap();

        if let Some(ContainerBlock::List { items, style }) = result.as_list() {
            assert!(!style.is_ordered());
            assert_eq!(items.len(), 2);

            let first_item = &items[0];
            assert!(
                first_item[0].is_paragraph(),
                "Expected first block in item 1 to be a paragraph"
            );

            if let Some(ContainerBlock::List {
                items: inner_items,
                style: inner_style,
            }) = &first_item[1].as_list()
            {
                assert!(!inner_style.is_ordered());
                assert_eq!(inner_items.len(), 2);
                match &inner_items[0][0].as_paragraph() {
                    Some(LeafBlock::Paragraph { content }) => {
                        assert_eq!(
                            content,
                            &vec![Inline::Text {
                                text: "Nested 1".into()
                            }]
                        );
                    }
                    _ => panic!("Expected Paragraph in nested list"),
                }
            } else {
                panic!("Expected nested list inside first item");
            }

            let second_item = &items[1];
            match &second_item[0].as_paragraph() {
                Some(LeafBlock::Paragraph { content }) => {
                    assert_eq!(
                        content,
                        &vec![Inline::Text {
                            text: "Item 2".into()
                        }]
                    );
                }
                _ => panic!("Expected paragraph in second item"),
            }
        } else {
            panic!("Expected top-level list");
        }
    }

    #[test]
    fn test_serialization() {
        let format = MarkdownFormat;
        let note = sample_note();

        dbg!(note.clone());

        let serialized = format.serialize(&note);
        let serialized_str = String::from_utf8(serialized).expect("Invalid UTF-8");

        assert!(serialized_str.contains("# Sample Note"));
        assert!(serialized_str.contains("**bold**"));
        assert!(serialized_str.contains("*italic*"));
        assert!(serialized_str.contains("## Heading Example"));
        assert!(serialized_str.contains("- Item 1"));
        assert!(serialized_str.contains("- Item 2"));
        assert!(serialized_str.contains("![Alt text](image.png)"));
    }

    #[test]
    fn test_deserialization() {
        let format = MarkdownFormat;
        let markdown = r"# Sample Note
## Heading Example
This is a **bold** and *italic* text.
![Alt text](image.png)
- Item 1
- Item 2
";

        let note = format.deserialize(markdown.as_bytes(), Some("test-id"));

        assert_eq!(note.id, "test-id");
        assert_eq!(note.title, "Sample Note");

        match &note.blocks[0].as_heading() {
            Some(LeafBlock::Heading { level, content }) => {
                assert_eq!(*level, 2);
                assert_eq!(
                    content,
                    &vec![Inline::Text {
                        text: "Heading Example".to_string()
                    }]
                );
            }
            _ => panic!("Expected heading"),
        }

        match &note.blocks[1].as_paragraph() {
            Some(LeafBlock::Paragraph { content: inlines }) => {
                assert_eq!(inlines.len(), 5);
                match &inlines[1] {
                    Inline::Bold { content: b } => assert_eq!(
                        b,
                        &vec![Inline::Text {
                            text: "bold".to_string()
                        }]
                    ),
                    _ => panic!("Expected bold"),
                }
                match &inlines[3] {
                    Inline::Italic { content: i } => assert_eq!(
                        i,
                        &vec![Inline::Text {
                            text: "italic".to_string()
                        }]
                    ),
                    _ => panic!("Expected italic"),
                }
            }
            _ => panic!("Expected paragraph"),
        }

        match &note.blocks[2].as_image() {
            Some(LeafBlock::Image { alt_text, src }) => {
                assert_eq!(alt_text, &Some("Alt text".to_string()));
                assert_eq!(src, "image.png");
            }
            _ => panic!("Expected image"),
        }

        match &note.blocks[3].as_list() {
            Some(ContainerBlock::List { style, items }) => {
                assert!(!style.is_ordered());
                assert_eq!(items.len(), 2);
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_round_trip() {
        let format = MarkdownFormat;
        let original_note = sample_note();

        let serialized = format.serialize(&original_note);
        let deserialized = format.deserialize(&serialized, Some(&original_note.id));

        assert_eq!(deserialized.id, original_note.id);
        assert_eq!(deserialized.title, original_note.title);
        assert_eq!(deserialized.blocks.len(), original_note.blocks.len());
    }

    #[test]
    fn test_code_block_round_trip() {
        let format = MarkdownFormat;

        let note = Note {
            id: "test-id".into(),
            title: "Code Note".into(),
            blocks: vec![Block::code_block(
                Some("rust".into()),
                "let x = 42;\nprintln!(\"{}\", x);\n".into(),
            )],
        };

        let serialized = format.serialize(&note);
        let deserialized = format.deserialize(&serialized, Some(&note.id));

        assert_eq!(deserialized.id, note.id);
        assert_eq!(deserialized.title, note.title);
        assert_eq!(deserialized.blocks.len(), note.blocks.len());

        match &deserialized.blocks[0].as_code_block() {
            Some(LeafBlock::CodeBlock { language, content }) => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert_eq!(content, "let x = 42;\nprintln!(\"{}\", x);\n");
            }
            _ => panic!("Expected CodeBlock"),
        }
    }

    #[test]
    fn test_math_block_round_trip() {
        let format = MarkdownFormat;

        let note = Note {
            id: "test-id".into(),
            title: "Math Note".into(),
            blocks: vec![Block::math_block("x^2 + y^2 = z^2\nx + y = z".into())],
        };

        let serialized = format.serialize(&note);
        let deserialized = format.deserialize(&serialized, Some(&note.id));

        assert_eq!(deserialized.id, note.id);
        assert_eq!(deserialized.title, note.title);
        assert_eq!(deserialized.blocks.len(), note.blocks.len());

        match &deserialized.blocks[0] {
            Block::Leaf { leaf } => match leaf {
                LeafBlock::MathBlock { content } => {
                    assert_eq!(content, "x^2 + y^2 = z^2\nx + y = z");
                }
                _ => panic!("Expected Math leaf"),
            },
            _ => panic!("Expected MathBlock"),
        }
    }

    #[test]
    fn test_parse_wiki_links() {
        let input = "This links to [[Note A]] and [[Note B]] in the text.";
        let inlines = parse_inlines(input);

        let expected = vec![
            Inline::Text {
                text: "This links to ".to_string(),
            },
            Inline::Link {
                text: vec![Inline::Text {
                    text: "Note A".to_string(),
                }],
                target: "Note A".to_string(),
            },
            Inline::Text {
                text: " and ".to_string(),
            },
            Inline::Link {
                text: vec![Inline::Text {
                    text: "Note B".to_string(),
                }],
                target: "Note B".to_string(),
            },
            Inline::Text {
                text: " in the text.".to_string(),
            },
        ];

        assert_eq!(inlines, expected);
    }

    #[test]
    fn test_extract_wiki_links() {
        let format = MarkdownFormat;

        let note = Note {
            id: "1".to_string(),
            title: "Wiki Links".to_string(),
            blocks: vec![Block::paragraph(vec![
                Inline::Text {
                    text: "Links: ".to_string(),
                },
                Inline::Link {
                    text: vec![Inline::Text {
                        text: "Note1".to_string(),
                    }],
                    target: "Note1".to_string(),
                },
                Inline::Text {
                    text: ", ".to_string(),
                },
                Inline::Link {
                    text: vec![Inline::Text {
                        text: "Note2".to_string(),
                    }],
                    target: "Note2".to_string(),
                },
            ])],
        };

        let links = format.extract_links(&note, &[]);
        let expected = vec![
            LinkTarget::Note("Note1".to_string()),
            LinkTarget::Note("Note2".to_string()),
        ];

        assert_eq!(links, expected);
    }

    #[test]
    fn test_extract_single_attachment_block() {
        let blocks = vec![Block::attachment(Attachment {
            src: "file1.pdf".to_string(),
            name: "file1.pdf".to_string(),
            kind: AttachmentType::Document,
        })];

        let attachments = extract_attachments(&blocks);

        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].src, "file1.pdf");
        assert_eq!(attachments[0].name, "file1.pdf");
        assert_eq!(attachments[0].kind, AttachmentType::Document);
    }

    #[test]
    fn test_extract_multiple_attachment_blocks() {
        let blocks = vec![
            Block::attachment(Attachment {
                src: "doc1.txt".to_string(),
                name: "doc1.txt".to_string(),
                kind: AttachmentType::Document,
            }),
            Block::attachment(Attachment {
                src: "image.png".to_string(),
                name: "image.png".to_string(),
                kind: AttachmentType::Image,
            }),
        ];

        let attachments = extract_attachments(&blocks);

        assert_eq!(attachments.len(), 2);

        assert!(
            attachments
                .iter()
                .any(|a| a.src == "doc1.txt" && a.name == "doc1.txt")
        );
        assert!(
            attachments
                .iter()
                .any(|a| a.src == "image.png" && a.name == "image.png")
        );
    }
}
