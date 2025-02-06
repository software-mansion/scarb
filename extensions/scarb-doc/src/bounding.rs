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
    ExternType,
    ExternFunction,
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
            SyntacticKind::ExternType => "extern type",
            SyntacticKind::ExternFunction => "extern fn",
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
        Mutability::Reference => "ref ",
    }
}

pub fn extract_and_format(input: &str) -> String {
    fn inner(input: &str) -> String {
        input
            .split(',')
            .map(|part| {
                let mut parts = part
                    .split("::")
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>();

                if parts.len() >= 2 && parts.last().unwrap().contains("<") {
                    let last = parts.pop().unwrap();
                    let generic_parts = last
                        .split::<&[_]>(&['<', '>', ':'])
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>();
                    if generic_parts.len() >= 2 {
                        let l = generic_parts.len();
                        parts.push(&generic_parts[l - 2]);
                        format!("{}<{}>", parts.join("::"), generic_parts[l - 1])
                    } else {
                        last.to_owned()
                    }
                } else {
                    parts.pop().unwrap_or(part).trim().to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    let mut result = String::new();
    let mut temp = String::new();
    let mut nest_level = 0;

    // Process each character
    for c in input.chars() {
        match c {
            '(' | '[' | '<' => {
                if nest_level == 0 && !temp.is_empty() {
                    result.push_str(&inner(&temp));
                    temp.clear();
                }
                nest_level += 1;
                result.push(c);
            }
            ')' | ']' | '>' => {
                if nest_level == 1 && !temp.is_empty() {
                    result.push_str(&inner(&temp));
                    temp.clear();
                }
                nest_level -= 1;
                result.push(c);
            }
            ',' if nest_level > 0 => {
                if !temp.is_empty() {
                    result.push_str(&inner(&temp));
                    temp.clear();
                }
                result.push(c);
                result.push(' ');
            }
            ',' => temp.push(c),
            _ => temp.push(c),
        }
    }

    if !temp.is_empty() {
        result.push_str(&inner(&temp));
    }

    result
}
