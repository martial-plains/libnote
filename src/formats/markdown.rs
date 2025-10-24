use regex::Regex;

use crate::{
    formats::NoteFormat,
    models::{Attachment, AttachmentType, Block, Inline, LinkTarget, Note},
};

#[derive(Debug)]
pub struct MarkdownFormat;

impl MarkdownFormat {
    /// Parse a note from its file name and raw content
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
        let name_part = path
            .rsplit_once(['/', '\\'])
            .map(|(_, name)| name)
            .unwrap_or(path);

        match name_part.rsplit_once('.') {
            Some((stem, _ext)) => stem.to_string(),
            None => name_part.to_string(),
        }
    }
}

impl NoteFormat for MarkdownFormat {
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

        let id = id_hint
            .map(Self::filename_stem)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Note { id, title, blocks }
    }

    fn serialize(&self, note: &Note) -> Vec<u8> {
        let mut output = String::new();
        output.push_str("# ");
        output.push_str(&note.title);
        output.push('\n');

        for block in &note.blocks {
            match block {
                Block::Heading(level, inlines) => {
                    output.push_str(&"#".repeat(*level as usize));
                    output.push(' ');
                    output.push_str(&serialize_inlines(inlines));
                    output.push('\n');
                }
                Block::Paragraph(inlines) => {
                    output.push_str(&serialize_inlines(inlines));
                    output.push('\n');
                }
                Block::List { ordered, items } => {
                    for (i, item) in items.iter().enumerate() {
                        let prefix = if *ordered {
                            format!("{}. ", i + 1)
                        } else {
                            "- ".to_string()
                        };
                        if let Block::Paragraph(inlines) = item {
                            output.push_str(&prefix);
                            output.push_str(&serialize_inlines(inlines));
                            output.push('\n');
                        } else {
                            output.push_str(&prefix);
                            output.push_str(&serialize_blocks(core::slice::from_ref(item)));
                        }
                    }
                }
                Block::Table { headers, rows } => {
                    for (i, header) in headers.iter().enumerate() {
                        if i > 0 {
                            output.push('|');
                        }
                        output.push(' ');
                        output.push_str(&serialize_inlines(header));
                        output.push(' ');
                    }
                    output.push('\n');

                    for i in 0..headers.len() {
                        if i > 0 {
                            output.push('|');
                        }
                        output.push_str(" --- ");
                    }
                    output.push('\n');

                    for row in rows {
                        for (i, cell) in row.iter().enumerate() {
                            if i > 0 {
                                output.push('|');
                            }
                            output.push(' ');
                            output.push_str(&serialize_inlines(cell));
                            output.push(' ');
                        }
                        output.push('\n');
                    }
                }
                Block::Image { alt_text, src } => {
                    output.push_str("![");
                    if let Some(alt) = alt_text {
                        output.push_str(alt);
                    }
                    output.push_str("](");
                    output.push_str(src);
                    output.push_str(")\n");
                }
                Block::CodeBlock { language, content } => {
                    output.push_str("```");
                    if let Some(lang) = language {
                        output.push_str(lang);
                    }
                    output.push('\n');
                    output.push_str(content);
                    output.push_str("```\n");
                }
                Block::MathBlock(content) => {
                    output.push_str("$$\n");
                    output.push_str(content);
                    output.push_str("\n$$\n");
                }
                Block::Quote(inner) => {
                    for b in inner {
                        let lines = serialize_blocks(core::slice::from_ref(b))
                            .lines()
                            .map(|l| format!("> {}", l))
                            .collect::<Vec<_>>()
                            .join("\n");
                        output.push_str(&lines);
                        output.push('\n');
                    }
                }

                Block::Attachment(Attachment {
                    id: src,
                    name: alt_text,
                    kind: _,
                }) => {
                    output.push_str("![");
                    output.push_str(alt_text);
                    output.push_str("](");
                    output.push_str(src);
                    output.push_str(")\n");
                }
            }
        }

        output.into_bytes()
    }

    fn extract_links(&self, note: &Note, attachments: &[Attachment]) -> Vec<LinkTarget> {
        let mut links = Vec::new();

        let is_attachment = |target: &str| attachments.iter().any(|a| a.id == target);

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
                    Inline::Bold(inner) | Inline::Italic(inner) | Inline::Strikethrough(inner) => {
                        process_inlines(inner, links, is_attachment)
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
                    Block::Paragraph(inlines) | Block::Heading(_, inlines) => {
                        process_inlines(inlines, links, is_attachment)
                    }
                    Block::Quote(inner_blocks) => {
                        process_blocks(inner_blocks, links, is_attachment)
                    }
                    Block::List { items, .. } => process_blocks(items, links, is_attachment),
                    Block::Table { headers, rows } => {
                        for row in headers.iter().chain(rows.iter().flatten()) {
                            process_inlines(row, links, is_attachment);
                        }
                    }
                    Block::Image { src, .. } => {
                        if is_attachment(src) {
                            links.push(LinkTarget::Attachment(src.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }

        process_blocks(&note.blocks, &mut links, &is_attachment);

        links
    }
}

fn serialize_blocks(blocks: &[Block]) -> String {
    let mut out = String::new();
    for b in blocks {
        match b {
            Block::Paragraph(inlines) => out.push_str(&serialize_inlines(inlines)),
            Block::Heading(l, inlines) => {
                out.push_str(&"#".repeat(*l as usize));
                out.push(' ');
                out.push_str(&serialize_inlines(inlines));
            }
            Block::List { .. } | Block::Quote(_) | Block::Table { .. } | Block::Image { .. } => {
                out.push_str(
                    &String::from_utf8(MarkdownFormat.serialize(&Note {
                        id: "".to_string(),
                        title: "".to_string(),
                        blocks: vec![b.clone()],
                    }))
                    .unwrap_or_default(),
                );
            }
            _ => {}
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
            blocks.push(Block::CodeBlock {
                language: if language.is_empty() {
                    None
                } else {
                    Some(language.to_string())
                },
                content,
            });
            continue;
        }

        if let Some(mut content) = trimmed.strip_prefix("$$") {
            if content.ends_with("$$") {
                content = &content[..content.len() - 2];
                blocks.push(Block::MathBlock(content.to_string()));
                continue;
            }

            let mut full_content = String::new();
            if !content.is_empty() {
                full_content.push_str(content);
            }

            for next in lines.by_ref() {
                let next_trimmed = next.trim_end();
                if let Some(stripped_end) = next_trimmed.strip_suffix("$$") {
                    if !full_content.is_empty() {
                        full_content.push('\n');
                    }
                    full_content.push_str(stripped_end);
                    break;
                } else {
                    if !full_content.is_empty() {
                        full_content.push('\n');
                    }
                    full_content.push_str(next_trimmed);
                }
            }

            if full_content.ends_with('\n') {
                full_content.pop();
            }

            blocks.push(Block::MathBlock(full_content));
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
                blocks.push(Block::Quote(parse_blocks(&inner)));
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
                blocks.push(Block::Image {
                    alt_text: alt.map(|s| s.to_string()),
                    src: src.to_string(),
                });
                continue;
            }
        }

        blocks.push(Block::Paragraph(parse_inlines(trimmed)));
    }

    blocks
}

fn serialize_inlines(inlines: &[Inline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) => output.push_str(t),
            Inline::Bold(inner) => {
                output.push_str("**");
                output.push_str(&serialize_inlines(inner));
                output.push_str("**");
            }
            Inline::Italic(inner) => {
                output.push('*');
                output.push_str(&serialize_inlines(inner));
                output.push('*');
            }
            Inline::Strikethrough(inner) => {
                output.push_str("~~");
                output.push_str(&serialize_inlines(inner));
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
            Inline::Code(code) => {
                output.push('`');
                output.push_str(code);
                output.push('`');
            }
            Inline::Math(content) => {
                output.push('$');
                output.push_str(content);
                output.push('$');
            }
        }
    }
    output
}

fn parse_markdown_header(line: &str) -> Option<Block> {
    let trimmed = line.trim_start();
    let mut chars = trimmed.chars().peekable();

    let mut level = 0;
    while let Some('#') = chars.peek() {
        chars.next();
        level += 1;
    }

    if level == 0 || chars.next() != Some(' ') {
        return None;
    }

    let content: String = chars.collect();

    Some(Block::Heading(
        level,
        vec![Inline::Text(content.trim().to_string())],
    ))
}

pub fn parse_list(input: &str) -> Option<Block> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() {
        return None;
    }

    let mut items = Vec::new();
    let mut i = 0;
    let mut ordered = None;

    while i < lines.len() {
        if let Some((item_block, next_index, item_ordered)) = parse_list_item(&lines, i) {
            if ordered.is_none() {
                ordered = Some(item_ordered);
            }

            items.push(item_block);
            i = next_index;
        } else {
            i += 1;
        }
    }

    if items.is_empty() {
        None
    } else {
        Some(Block::List {
            ordered: ordered.unwrap_or(false),
            items,
        })
    }
}

fn parse_list_item(lines: &[&str], start_index: usize) -> Option<(Block, usize, bool)> {
    if start_index >= lines.len() {
        return None;
    }

    let line = lines[start_index];
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();

    let (ordered, content) = if let Some(stripped) = trimmed.strip_prefix("- ") {
        (false, stripped)
    } else if let Some(stripped) = trimmed.strip_prefix("* ") {
        (false, stripped)
    } else if let Some(stripped) = trimmed.strip_prefix("+ ") {
        (false, stripped)
    } else if let Some(dot_pos) = trimmed.find('.') {
        if trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit())
            && trimmed[dot_pos + 1..].starts_with(' ')
        {
            (true, &trimmed[dot_pos + 2..])
        } else {
            return None;
        }
    } else {
        return None;
    };

    let item_paragraph = Block::Paragraph(parse_inlines(content));

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
        if let Some(nested_list) = parse_list(&nested_input) {
            return Some((
                Block::List {
                    ordered,
                    items: vec![item_paragraph, nested_list],
                },
                i,
                ordered,
            ));
        }
    }

    Some((item_paragraph, i, ordered))
}

fn parse_table(input: &str) -> Option<Block> {
    let lines: Vec<_> = input.lines().filter(|l| l.contains('|')).collect();
    if lines.len() < 2 {
        return None;
    }

    let headers = lines[0]
        .split('|')
        .map(|c| parse_inlines(c.trim()))
        .collect::<Vec<_>>();
    let rows = lines[1..]
        .iter()
        .map(|row| {
            row.split('|')
                .map(|c| parse_inlines(c.trim()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    Some(Block::Table { headers, rows })
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

pub fn parse_inlines(input: &str) -> Vec<Inline> {
    let mut result = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.peek().cloned() {
        match c {
            '*' => {
                chars.next();
                if chars.peek() == Some(&'*') {
                    chars.next();
                    let content = parse_until(&mut chars, "**");
                    consume_delimiter(&mut chars, "**");
                    result.push(Inline::Bold(parse_inlines(&content)));
                } else {
                    let content = parse_until(&mut chars, "*");
                    consume_delimiter(&mut chars, "*");
                    result.push(Inline::Italic(parse_inlines(&content)));
                }
            }

            '~' => {
                chars.next();
                if chars.peek() == Some(&'~') {
                    chars.next();
                    let content = parse_until(&mut chars, "~~");
                    consume_delimiter(&mut chars, "~~");
                    result.push(Inline::Strikethrough(parse_inlines(&content)));
                } else {
                    result.push(Inline::Text("~".to_string()));
                }
            }

            '`' => {
                chars.next();
                let content = parse_until(&mut chars, "`");
                consume_delimiter(&mut chars, "`");
                result.push(Inline::Code(content));
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
                result.push(Inline::Text("!".to_string()));
            }

            '$' => {
                chars.next();
                let content = parse_until(&mut chars, "$");
                consume_delimiter(&mut chars, "$");
                result.push(Inline::Math(content));
            }

            '[' => {
                chars.next();
                if chars.peek() == Some(&'[') {
                    chars.next();
                    let text = parse_until(&mut chars, "]]");
                    consume_delimiter(&mut chars, "]]");
                    result.push(Inline::Link {
                        text: vec![Inline::Text(text.clone())],
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
                        result.push(Inline::Text(format!("[{}]", text)));
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
                result.push(Inline::Text(text));
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
        if delimiter_matches(chars, &delim_chars) {
            for _ in 0..delim_len {
                chars.next();
            }
            break;
        } else {
            buffer.push(chars.next().unwrap());
        }
    }

    buffer
}

pub fn extract_attachments(blocks: &[Block]) -> Vec<Attachment> {
    let mut attachments = Vec::new();

    fn push_attachment(attachments: &mut Vec<Attachment>, path: &str) {
        let name = path
            .split(['/', '\\'])
            .next_back()
            .unwrap_or(path)
            .to_string();

        let kind = match path.rsplit('.').next().map(|s| s.to_lowercase()) {
            Some(ext)
                if ext == "png"
                    || ext == "jpg"
                    || ext == "jpeg"
                    || ext == "gif"
                    || ext == "bmp"
                    || ext == "webp" =>
            {
                AttachmentType::Image
            }
            Some(ext) if ext == "mp3" || ext == "wav" || ext == "ogg" || ext == "flac" => {
                AttachmentType::Audio
            }
            Some(ext) if ext == "mp4" || ext == "mkv" || ext == "mov" || ext == "avi" => {
                AttachmentType::Video
            }
            Some(ext)
                if ext == "pdf" || ext == "doc" || ext == "docx" || ext == "txt" || ext == "md" =>
            {
                AttachmentType::Document
            }
            Some(other) => AttachmentType::Other(other.to_string()),
            None => AttachmentType::Other("unknown".to_string()),
        };

        attachments.push(Attachment {
            id: path.to_string(),
            name,
            kind,
        });
    }

    for block in blocks {
        match block {
            Block::Image { src, .. } => push_attachment(&mut attachments, src),

            Block::Paragraph(inlines) | Block::Heading(_, inlines) => {
                for inline in inlines {
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

            Block::Quote(inner) => attachments.extend(extract_attachments(inner)),

            Block::List { items, .. } => {
                for item in items {
                    attachments.extend(extract_attachments(core::slice::from_ref(item)));
                }
            }

            Block::Table { rows, .. } => {
                for row in rows {
                    for cell in row {
                        for inline in cell {
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
            }

            Block::Attachment(att) => attachments.push(att.clone()),

            _ => {}
        }
    }

    attachments
}

fn delimiter_matches<I>(chars: &mut core::iter::Peekable<I>, delimiter: &[char]) -> bool
where
    I: Iterator<Item = char> + Clone,
{
    let mut clone = chars.clone();
    for &d in delimiter {
        match clone.next() {
            Some(c) if c == d => continue,
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
                Block::Heading(2, vec![Inline::Text("Heading Example".to_string())]),
                Block::Paragraph(vec![
                    Inline::Text("This is a ".to_string()),
                    Inline::Bold(vec![Inline::Text("bold".to_string())]),
                    Inline::Text(" and ".to_string()),
                    Inline::Italic(vec![Inline::Text("italic".to_string())]),
                    Inline::Text(" text.".to_string()),
                ]),
                Block::Image {
                    alt_text: Some("Alt text".to_string()),
                    src: "image.png".to_string(),
                },
                Block::List {
                    ordered: false,
                    items: vec![
                        Block::Paragraph(vec![Inline::Text("Item 1".to_string())]),
                        Block::Paragraph(vec![Inline::Text("Item 2".to_string())]),
                    ],
                },
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
            assert!(h.is_some(), "Failed to parse header: {:?}", input);

            let block = h.unwrap();
            if let Block::Heading(level, content) = block {
                assert_eq!(
                    level, expected_level,
                    "Wrong heading level for input: {:?}",
                    input
                );
                assert_eq!(
                    content,
                    vec![Inline::Text(expected_text.into())],
                    "Wrong content for input: {:?}",
                    input
                );
            } else {
                panic!(
                    "Expected a Heading block for input: {:?}, got: {:?}",
                    input, block
                );
            }
        }
    }

    #[test]
    fn test_parse_unordered_list() {
        let line = "- Item 1";
        let result = parse_list(line);
        assert!(result.is_some());

        if let Block::List { items, ordered } = result.unwrap() {
            assert!(!ordered, "Expected unordered list");
            assert_eq!(items.len(), 1);

            match &items[0] {
                Block::Paragraph(inlines) => {
                    assert_eq!(inlines, &vec![Inline::Text("Item 1".into())]);
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

        if let Block::List { items, ordered } = result.unwrap() {
            assert!(ordered, "Expected ordered list");
            assert_eq!(items.len(), 1);

            match &items[0] {
                Block::Paragraph(inlines) => {
                    assert_eq!(inlines, &vec![Inline::Text("First item".into())]);
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

        if let Some(Block::Table { headers, rows }) = result {
            assert_eq!(
                headers,
                vec![
                    vec![Inline::Text("Header 1".into())],
                    vec![Inline::Text("Header 2".into())]
                ]
            );

            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0],
                vec![
                    vec![Inline::Text("Value 1".into())],
                    vec![Inline::Text("Value 2".into())]
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

        match &blocks[0] {
            Block::CodeBlock { language, content } => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert_eq!(content, "let x = 42;\nprintln!(\"{}\", x);\n");
            }
            _ => panic!("Expected a CodeBlock"),
        }
    }

    #[test]
    fn test_parse_code_block_no_language() {
        let input = "```\nHello world\n```";
        let blocks = parse_blocks(input);

        assert_eq!(blocks.len(), 1);

        match &blocks[0] {
            Block::CodeBlock { language, content } => {
                assert!(language.is_none());
                assert_eq!(content, "Hello world\n");
            }
            _ => panic!("Expected a CodeBlock"),
        }
    }

    #[test]
    fn test_parse_math_block_single_line() {
        let input = "$$x^2 + y^2 = z^2$$";
        let blocks = parse_blocks(input);

        assert_eq!(blocks.len(), 1);

        match &blocks[0] {
            Block::MathBlock(content) => {
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

        match &blocks[0] {
            Block::MathBlock(content) => {
                assert_eq!(content, "x^2 + y^2 = z^2\nx + y = z");
            }
            _ => panic!("Expected a MathBlock"),
        }
    }

    #[test]
    fn test_parse_plain_text() {
        let input = "Just plain text.";
        let expected = vec![Inline::Text("Just plain text.".into())];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_italic() {
        let input = "This is *italic* text.";
        let expected = vec![
            Inline::Text("This is ".into()),
            Inline::Italic(vec![Inline::Text("italic".into())]),
            Inline::Text(" text.".into()),
        ];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_bold() {
        let input = "This is **bold** text.";
        let expected = vec![
            Inline::Text("This is ".into()),
            Inline::Bold(vec![Inline::Text("bold".into())]),
            Inline::Text(" text.".into()),
        ];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_link() {
        let input = "Check out [Rust](https://www.rust-lang.org)!";
        let expected = vec![
            Inline::Text("Check out ".into()),
            Inline::Link {
                text: vec![Inline::Text("Rust".into())],
                target: "https://www.rust-lang.org".into(),
            },
            Inline::Text("!".into()),
        ];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_nested_formatting() {
        let input = "**bold and *italic inside***";
        let expected = vec![Inline::Bold(vec![
            Inline::Text("bold and ".into()),
            Inline::Italic(vec![Inline::Text("italic inside".into())]),
        ])];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_mixed_content() {
        let input = "*italic* and **bold**, then [link](url)";
        let expected = vec![
            Inline::Italic(vec![Inline::Text("italic".into())]),
            Inline::Text(" and ".into()),
            Inline::Bold(vec![Inline::Text("bold".into())]),
            Inline::Text(", then ".into()),
            Inline::Link {
                text: vec![Inline::Text("link".into())],
                target: "url".into(),
            },
        ];
        assert_eq!(parse_inlines(input), expected);
    }

    #[test]
    fn test_parse_nested_list() {
        let input = "- Item 1\n  - Nested 1\n  - Nested 2\n- Item 2";
        let result = parse_list(input).unwrap();

        if let Block::List { items, ordered } = result {
            assert!(!ordered);
            assert_eq!(items.len(), 2);

            match &items[0] {
                Block::List {
                    items: inner_items,
                    ordered: inner_ordered,
                } => {
                    assert!(!inner_ordered);
                    assert_eq!(inner_items.len(), 2);
                    match &inner_items[0] {
                        Block::Paragraph(inlines) => {
                            assert_eq!(inlines, &vec![Inline::Text("Item 1".into())]);
                        }
                        _ => panic!("Expected paragraph"),
                    }
                }
                _ => panic!("Expected nested list"),
            }

            match &items[1] {
                Block::Paragraph(inlines) => {
                    assert_eq!(inlines, &vec![Inline::Text("Item 2".into())]);
                }
                _ => panic!("Expected paragraph"),
            }
        } else {
            panic!("Expected top-level list");
        }
    }

    #[test]
    fn test_serialization() {
        let format = MarkdownFormat;
        let note = sample_note();

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
        let markdown = r#"# Sample Note
## Heading Example
This is a **bold** and *italic* text.
![Alt text](image.png)
- Item 1
- Item 2
"#;

        let note = format.deserialize(markdown.as_bytes(), Some("test-id"));

        assert_eq!(note.id, "test-id");
        assert_eq!(note.title, "Sample Note");

        match &note.blocks[0] {
            Block::Heading(level, inlines) => {
                assert_eq!(*level, 2);
                assert_eq!(inlines, &vec![Inline::Text("Heading Example".to_string())]);
            }
            _ => panic!("Expected heading"),
        }

        match &note.blocks[1] {
            Block::Paragraph(inlines) => {
                assert_eq!(inlines.len(), 5);
                match &inlines[1] {
                    Inline::Bold(b) => assert_eq!(b, &vec![Inline::Text("bold".to_string())]),
                    _ => panic!("Expected bold"),
                }
                match &inlines[3] {
                    Inline::Italic(i) => assert_eq!(i, &vec![Inline::Text("italic".to_string())]),
                    _ => panic!("Expected italic"),
                }
            }
            _ => panic!("Expected paragraph"),
        }

        match &note.blocks[2] {
            Block::Image { alt_text, src } => {
                assert_eq!(alt_text, &Some("Alt text".to_string()));
                assert_eq!(src, "image.png");
            }
            _ => panic!("Expected image"),
        }

        match &note.blocks[3] {
            Block::List { ordered, items } => {
                assert!(!ordered);
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
            blocks: vec![Block::CodeBlock {
                language: Some("rust".into()),
                content: "let x = 42;\nprintln!(\"{}\", x);\n".into(),
            }],
        };

        let serialized = format.serialize(&note);
        let deserialized = format.deserialize(&serialized, Some(&note.id));

        assert_eq!(deserialized.id, note.id);
        assert_eq!(deserialized.title, note.title);
        assert_eq!(deserialized.blocks.len(), note.blocks.len());

        match &deserialized.blocks[0] {
            Block::CodeBlock { language, content } => {
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
            blocks: vec![Block::MathBlock("x^2 + y^2 = z^2\nx + y = z".into())],
        };

        let serialized = format.serialize(&note);
        let deserialized = format.deserialize(&serialized, Some(&note.id));

        assert_eq!(deserialized.id, note.id);
        assert_eq!(deserialized.title, note.title);
        assert_eq!(deserialized.blocks.len(), note.blocks.len());

        match &deserialized.blocks[0] {
            Block::MathBlock(content) => {
                assert_eq!(content, "x^2 + y^2 = z^2\nx + y = z");
            }
            _ => panic!("Expected MathBlock"),
        }
    }

    #[test]
    fn test_parse_wiki_links() {
        let input = "This links to [[Note A]] and [[Note B]] in the text.";
        let inlines = parse_inlines(input);

        let expected = vec![
            Inline::Text("This links to ".to_string()),
            Inline::Link {
                text: vec![Inline::Text("Note A".to_string())],
                target: "Note A".to_string(),
            },
            Inline::Text(" and ".to_string()),
            Inline::Link {
                text: vec![Inline::Text("Note B".to_string())],
                target: "Note B".to_string(),
            },
            Inline::Text(" in the text.".to_string()),
        ];

        assert_eq!(inlines, expected);
    }

    #[test]
    fn test_extract_wiki_links() {
        let format = MarkdownFormat;

        let note = Note {
            id: "1".to_string(),
            title: "Wiki Links".to_string(),
            blocks: vec![Block::Paragraph(vec![
                Inline::Text("Links: ".to_string()),
                Inline::Link {
                    text: vec![Inline::Text("Note1".to_string())],
                    target: "Note1".to_string(),
                },
                Inline::Text(", ".to_string()),
                Inline::Link {
                    text: vec![Inline::Text("Note2".to_string())],
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
        let blocks = vec![Block::Attachment(Attachment {
            id: "file1.pdf".to_string(),
            name: "file1.pdf".to_string(),
            kind: AttachmentType::Document,
        })];

        let attachments = extract_attachments(&blocks);

        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].id, "file1.pdf");
        assert_eq!(attachments[0].name, "file1.pdf");
        assert_eq!(attachments[0].kind, AttachmentType::Document);
    }

    #[test]
    fn test_extract_multiple_attachment_blocks() {
        let blocks = vec![
            Block::Attachment(Attachment {
                id: "doc1.txt".to_string(),
                name: "doc1.txt".to_string(),
                kind: AttachmentType::Document,
            }),
            Block::Attachment(Attachment {
                id: "image.png".to_string(),
                name: "image.png".to_string(),
                kind: AttachmentType::Image,
            }),
        ];

        let attachments = extract_attachments(&blocks);

        assert_eq!(attachments.len(), 2);

        assert!(
            attachments
                .iter()
                .any(|a| a.id == "doc1.txt" && a.name == "doc1.txt")
        );
        assert!(
            attachments
                .iter()
                .any(|a| a.id == "image.png" && a.name == "image.png")
        );
    }
}
