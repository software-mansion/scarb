use cairo_lang_doc::parser::DocumentationCommentToken;
use std::str::from_utf8;

/// Represents code block extracted from doc comments.
#[derive(Debug, Clone, PartialEq)]
pub struct DocCodeBlock {
    pub code: String,
    pub language: String,
    pub attributes: Vec<String>,
    pub item_full_path: String,
    pub close_token_idx: usize,
}

impl DocCodeBlock {
    pub fn new(
        code: String,
        info_string: &String,
        item_full_path: String,
        close_token_idx: usize,
    ) -> Self {
        let (language, attributes) = Self::parse_info_string(info_string);
        Self {
            code,
            language,
            attributes,
            item_full_path,
            close_token_idx,
        }
    }

    pub fn is_runnable(&self) -> bool {
        self.language == "cairo" &&
        self.attributes.iter().any(|attr| attr == "runnable")
    }

    /// Parses info string into language and attributes. The results are lowercased.
    fn parse_info_string(info_string: &str) -> (String, Vec<String>) {
        let parts: Vec<_> = info_string
            .trim()
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        if parts.is_empty() {
            return (String::new(), Vec::new());
        }
        let language = parts[0].to_string();
        let attributes = parts[1..].iter().map(|s| s.to_string()).collect();
        (language, attributes)
    }
}

/// Collects code blocks from documentation comment tokens.
pub fn collect_code_blocks(
    doc_tokens: &Option<Vec<DocumentationCommentToken>>,
    full_path: &str,
) -> Vec<DocCodeBlock> {
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
                    if !body.trim().is_empty() {
                        code_blocks.push(DocCodeBlock::new(
                            body,
                            &opening.info_string,
                            full_path.to_string(),
                            end_idx,
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

/// Copied from https://github.com/pulldown-cmark/pulldown-cmark/blob/a574ea8a5e6fda7bc26542a612130a2b458a68a7/pulldown-cmark/src/scanners.rs#L744
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
