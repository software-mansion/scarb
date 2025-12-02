use crate::doc_test::runner::{ExecutionOutcome, RunStrategy};
use crate::docs_generation::markdown::traits::WithItemDataCommon;
use crate::types::crate_type::Crate;
use crate::types::module_type::Module;
use cairo_lang_doc::parser::DocumentationCommentToken;
use std::collections::HashMap;
use std::str::from_utf8;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct CodeBlockId {
    pub item_full_path: String,
    pub close_token_idx: usize,
    /// Index of this block in the item's documentation.
    pub block_index: usize,
}

impl CodeBlockId {
    pub fn new(item_full_path: String, block_index: usize, close_token_idx: usize) -> Self {
        Self {
            item_full_path,
            block_index,
            close_token_idx,
        }
    }

    // TODO: (#2888): Display exact code block location when running doc-tests
    pub fn display_name(&self, total_blocks_in_item: usize) -> String {
        if total_blocks_in_item <= 1 {
            self.item_full_path.clone()
        } else {
            format!("{} (example {})", self.item_full_path, self.block_index)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeBlockAttribute {
    Cairo,
    Runnable,
    Ignore,
    NoRun,
    CompileFail,
    ShouldPanic,
    Other(String),
}

impl From<&str> for CodeBlockAttribute {
    fn from(string: &str) -> Self {
        match string.to_lowercase().as_str() {
            "cairo" => CodeBlockAttribute::Cairo,
            "runnable" => CodeBlockAttribute::Runnable,
            "ignore" => CodeBlockAttribute::Ignore,
            "no_run" | "no-run" => CodeBlockAttribute::NoRun,
            "should_panic" | "should-panic" => CodeBlockAttribute::ShouldPanic,
            "compile_fail" | "compile-fail" => CodeBlockAttribute::CompileFail,
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
    pub fn new(id: CodeBlockId, content: String, info_string: &str) -> Self {
        let attributes = Self::parse_attributes(info_string);
        Self {
            id,
            content,
            attributes,
        }
    }

    // TODO: default to Cairo unless specified otherwise
    fn is_cairo(&self) -> bool {
        if self.attributes.contains(&CodeBlockAttribute::Cairo) {
            return true;
        }
        false
    }

    pub fn run_strategy(&self) -> RunStrategy {
        if self.attributes.contains(&CodeBlockAttribute::Ignore) {
            return RunStrategy::Ignore;
        }
        // TODO: drop the `runnable` attribute requirement; default to runnable for Cairo blocks
        if !self.is_cairo() || !self.attributes.contains(&CodeBlockAttribute::Runnable) {
            return RunStrategy::Ignore;
        }
        match self.expected_outcome() {
            ExecutionOutcome::None => RunStrategy::Ignore,
            ExecutionOutcome::BuildSuccess => RunStrategy::Build,
            ExecutionOutcome::RunSuccess => RunStrategy::Execute,
            ExecutionOutcome::CompileError => RunStrategy::Build,
            ExecutionOutcome::RuntimeError => RunStrategy::Execute,
        }
    }

    pub fn expected_outcome(&self) -> ExecutionOutcome {
        if self.attributes.contains(&CodeBlockAttribute::Ignore) {
            return ExecutionOutcome::None;
        }
        if self.attributes.contains(&CodeBlockAttribute::CompileFail) {
            return ExecutionOutcome::CompileError;
        }
        if self.attributes.contains(&CodeBlockAttribute::ShouldPanic) {
            return ExecutionOutcome::RuntimeError;
        }
        if self.attributes.contains(&CodeBlockAttribute::NoRun) {
            return ExecutionOutcome::BuildSuccess;
        }
        ExecutionOutcome::RunSuccess
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

pub fn collect_code_blocks(crate_: &Crate<'_>) -> Vec<CodeBlock> {
    let mut runnable_code_blocks = Vec::new();
    collect_from_module(&crate_.root_module, &mut runnable_code_blocks);
    for module in &crate_.foreign_crates {
        collect_from_module(module, &mut runnable_code_blocks);
    }
    runnable_code_blocks.sort_by_key(|block| block.id.clone());
    runnable_code_blocks
}

/// Counts the number of code blocks per documented item. Used to generate display names
/// for code blocks, allowing to distinguish between multiple code blocks in the same item.
///
/// Returns the mapping from `item_full_path` to the number of code blocks in that item.
pub fn count_blocks_per_item(code_blocks: &[CodeBlock]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for block in code_blocks {
        *counts.entry(block.id.item_full_path.clone()).or_insert(0) += 1;
    }
    counts
}

fn collect_from_module(module: &Module<'_>, runnable_code_blocks: &mut Vec<CodeBlock>) {
    for &item_data in module.get_all_item_ids().values() {
        collect_from_item_data(item_data, runnable_code_blocks);
    }
    for &item_data in module.pub_uses.get_all_item_ids().values() {
        collect_from_item_data(item_data, runnable_code_blocks);
    }
}

fn collect_from_item_data(
    item_data: &dyn WithItemDataCommon,
    runnable_code_blocks: &mut Vec<CodeBlock>,
) {
    for block in &item_data.code_blocks() {
        runnable_code_blocks.push(block.clone());
    }
}

pub fn collect_code_blocks_from_tokens(
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
    let mut block_index: usize = 0;

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
                    if !body.is_empty() {
                        let id = CodeBlockId::new(full_path.to_string(), block_index, end_idx);
                        code_blocks.push(CodeBlock::new(
                            id,
                            body.to_string(),
                            &opening.info_string,
                        ));
                        block_index += 1;
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
