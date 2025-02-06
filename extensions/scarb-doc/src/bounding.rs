use cairo_lang_semantic::items::visibility::Visibility;
use cairo_lang_semantic::Mutability;

#[derive(Clone)]
pub enum BoundingType {
    None,
    Parenthesis,
    Braces,
}

#[derive(Clone)]
pub enum BoundingPostfix {
    None,
    Arrow,     // ->
    EqualSign, // =
}

#[derive(Clone)]
pub enum SyntacticKind {
    Function,
    Struct,
    Enum,
    Constant,
    Trait,
    Impl,
    Type,
}

impl SyntacticKind {
    pub fn get_syntax(&self) -> &str {
        match self {
            SyntacticKind::Function => "fn",
            SyntacticKind::Struct => "struct",
            SyntacticKind::Enum => "enum",
            SyntacticKind::Constant => "const",
            SyntacticKind::Trait => "trait",
            SyntacticKind::Impl => "impl",
            SyntacticKind::Type => "type",
        }
    }
}

pub fn start_bounding(bounding_type: BoundingType) -> String {
    match bounding_type {
        BoundingType::Braces => "{",
        BoundingType::Parenthesis => "(",
        BoundingType::None => "",
    }
    .to_string()
}

pub fn end_bounding(bounding_type: BoundingType) -> String {
    match bounding_type {
        BoundingType::Braces => "}",
        BoundingType::Parenthesis => ")",
        BoundingType::None => "",
    }
    .to_string()
}

pub fn get_postfix(bounding_postfix: BoundingPostfix) -> String {
    match bounding_postfix {
        BoundingPostfix::None => "".to_string(),
        BoundingPostfix::Arrow => "->".to_string(),
        BoundingPostfix::EqualSign => "=".to_string(),
    }
}

pub fn get_syntactic_visibility(semantic_visibility: &Visibility) -> &str {
    match semantic_visibility {
        Visibility::Public => "pub ",
        Visibility::PublicInCrate => "pub(crate) ",
        Visibility::Private => "",
    }
}

pub fn get_syntactic_mutability(semantic_mutability: &Mutability) -> &str {
    match semantic_mutability {
        Mutability::Immutable => "",
        Mutability::Mutable => "mut ",
        Mutability::Reference => "ref",
    }
}

pub fn extract_and_format(input: &str) -> String {
    fn inner(input: &str) -> String {
        input
            .split(',') // Split by commas to handle multiple elements
            .map(|part| {
                // Strip leading and trailing whitespace and any outer parentheses/brackets
                let trimmed = part.trim_matches(|c: char| {
                    c == ' ' || c == '(' || c == ')' || c == '[' || c == ']'
                });
                // Find last occurrence of "::", returning either what's after or the whole string if not found
                trimmed.rsplit("::").next().unwrap_or(trimmed)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    let mut result = String::new();
    let mut nest_level = 0;
    let mut temp = String::new();

    // Process each character to maintain structure and to correctly apply transformation
    for c in input.chars() {
        match c {
            '(' | '[' => {
                // Opening brackets
                if !temp.is_empty() {
                    result.push_str(&inner(&temp));
                    temp.clear();
                }
                nest_level += 1;
                result.push(c);
            }
            ')' | ']' => {
                // Closing brackets
                if !temp.is_empty() {
                    result.push_str(&inner(&temp));
                    temp.clear();
                }
                nest_level -= 1;
                result.push(c);
            }
            ',' if nest_level > 0 => {
                // Commas within brackets
                if !temp.is_empty() {
                    result.push_str(&inner(&temp));
                    temp.clear();
                }
                result.push(c); // Append comma directly to the result string
                result.push(' '); // Maintain formatting with a space
            }
            _ => temp.push(c), // Accumulate characters in temporary string
        }
    }

    // Handle any remaining characters collected
    if !temp.is_empty() {
        result.push_str(&inner(&temp));
    }

    result
}
