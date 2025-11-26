use cairo_lang_doc::parser::DocumentationCommentToken;
use std::str::from_utf8;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CodeBlockId {
    pub item_full_path: String,
    pub close_token_idx: usize,
}

impl CodeBlockId {
    pub fn new(item_full_path: String, close_token_idx: usize) -> Self {
        Self {
            item_full_path,
            close_token_idx,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeBlockAttribute {
    Cairo,
    Runnable,
    Ignore,
    NoRun,
    Other(String),
}

impl From<&str> for CodeBlockAttribute {
    fn from(string: &str) -> Self {
        match string.to_lowercase().as_str() {
            "cairo" => CodeBlockAttribute::Cairo,
            "runnable" => CodeBlockAttribute::Runnable,
            "ignore" => CodeBlockAttribute::Ignore,
            "no_run" | "no-run" => CodeBlockAttribute::NoRun,
            _ => CodeBlockAttribute::Other(string.to_string()),
        }
    }
}

/// Represents code block extracted from doc comments.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlock {
    pub id: CodeBlockId,
    pub content: String,
    pub attributes: Vec<CodeBlockAttribute>,
}

impl CodeBlock {
    pub fn new(id: CodeBlockId, content: String, info_string: &String) -> Self {
        let attributes = Self::parse_attributes(info_string);
        Self {
            id,
            content,
            attributes,
        }
    }

    // TODO: default to Cairo unless specified otherwise?
    fn is_cairo(&self) -> bool {
        if self.attributes.contains(&CodeBlockAttribute::Cairo) {
            return true;
        }
        // Assume unknown attributes imply non-Cairo code.
        // !self.attributes.iter().any(|attr| matches!(attr, CodeBlockAttribute::Other(_)))
        false
    }

    // TODO: consider runnable by default unless specified otherwise?
    pub fn should_run(&self) -> bool {
        self.is_cairo() && self.attributes.contains(&CodeBlockAttribute::Runnable)
        //     && !self.attributes.contains(&CodeBlockAttribute::Ignore)
        //     && !self.attributes.contains(&CodeBlockAttribute::NoRun)
    }

    // TODO: implement building examples without running them
    #[allow(unused)]
    pub fn should_build(&self) -> bool {
        self.is_cairo() && !self.attributes.contains(&CodeBlockAttribute::Ignore)
    }

    fn parse_attributes(info_string: &str) -> Vec<CodeBlockAttribute> {
        info_string
            .split(',')
            .map(|attr| attr.trim())
            .filter(|attr| !attr.is_empty())
            .map(Into::into)
            .collect()
    }
}

/// Collects code blocks from documentation comment tokens.
pub fn collect_code_blocks(
    doc_tokens: &Option<Vec<DocumentationCommentToken>>,
    full_path: &str,
) -> Vec<CodeBlock> {
    let Some(tokens) = doc_tokens else {
        return Vec::new();
    };

    #[derive(Debug)]
    struct CodeFence {
        token_idx: usize,
        char: u8,
        len: usize,
        info_string: String,
    }

    let mut code_blocks = Vec::new();
    let mut current_fence: Option<CodeFence> = None;

    for (idx, token) in tokens.iter().enumerate() {
        let content = match token {
            DocumentationCommentToken::Content(content) => content.trim(),
            DocumentationCommentToken::Link(_) => continue,
        };
        if content.is_empty() {
            continue;
        }
        match current_fence {
            // Handle potential closing fence.
            Some(ref opening) => {
                if is_matching_closing_fence(content, opening.char, opening.len) {
                    let end_idx = idx;
                    let body = get_block_body(tokens, opening.token_idx + 1, end_idx);

                    // Skip empty code blocks.
                    let id = CodeBlockId::new(full_path.to_string(), end_idx);
                    if !body.is_empty() {
                        code_blocks.push(CodeBlock::new(
                            id,
                            body.to_string(),
                            &opening.info_string,
                        ));
                    }
                    current_fence = None;
                }
            }
            // Handle potential opening fence.
            None => {
                if let Some((len, char)) = scan_code_fence(content.as_bytes()) {
                    let bytes = content.as_bytes();
                    let after = &bytes[len..];
                    let info_string = from_utf8(after).unwrap_or("").trim().to_string();

                    current_fence = Some(CodeFence {
                        token_idx: idx,
                        char,
                        len,
                        info_string,
                    });
                }
            }
        }
    }
    // There may be an unterminated fence at this point, but this is allowed from the spec perspective, so we ignore it.
    code_blocks
}

fn get_block_body(
    tokens: &[DocumentationCommentToken],
    start_idx: usize,
    end_idx: usize,
) -> String {
    tokens[start_idx..end_idx]
        .iter()
        .filter_map(|token| match token {
            DocumentationCommentToken::Content(content) => Some(content.as_str()),
            DocumentationCommentToken::Link(_) => None,
        })
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string()
}

/// Checks if the given `content` is a closing fence matching the given opening fence.
fn is_matching_closing_fence(content: &str, opening_char: u8, opening_len: usize) -> bool {
    let bytes = content.as_bytes();
    let Some((len, ch)) = scan_code_fence(bytes) else {
        return false;
    };
    ch == opening_char
        && len >= opening_len
        && bytes[len..]
            .iter()
            .all(|&b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
}

/// Copied from `pulldown-cmark`:
/// https://github.com/pulldown-cmark/pulldown-cmark/blob/a574ea8a5e6fda7bc26542a612130a2b458a68a7/pulldown-cmark/src/scanners.rs#L744
fn scan_code_fence(data: &[u8]) -> Option<(usize, u8)> {
    let c = *data.first()?;
    if !(c == b'`' || c == b'~') {
        return None;
    }
    let i = 1 + scan_ch_repeat(&data[1..], c);
    if i >= 3 {
        if c == b'`' {
            let suffix = &data[i..];
            let next_line = i + scan_nextline(suffix);
            // FIXME: make sure this is correct
            if suffix[..(next_line - i)].contains(&b'`') {
                return None;
            }
        }
        Some((i, c))
    } else {
        None
    }
}

fn scan_ch_repeat(data: &[u8], c: u8) -> usize {
    data.iter().take_while(|&&b| b == c).count()
}

fn scan_nextline(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |x| x + 1)
}
