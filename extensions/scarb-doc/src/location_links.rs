use crate::db::ScarbDocDatabase;
use cairo_lang_defs::ids::{
    ImplItemId, LookupItemId, ModuleItemId, TopLevelLanguageElementId, TraitItemId,
};
use cairo_lang_diagnostics::DiagnosticsBuilder;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_filesystem::ids::{FileKind, FileLongId, VirtualFile};
use cairo_lang_formatter::{FormatterConfig, get_formatted_file};
use cairo_lang_parser::parser::Parser;
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::green::GreenNodeDetails;
use cairo_lang_syntax::node::kind::SyntaxKind;
use cairo_lang_syntax::node::{SyntaxNode, TypedSyntaxNode};
use cairo_lang_utils::Intern;

fn get_documentable_full_path(db: &ScarbDocDatabase, item_id: DocumentableItemId) -> String {
    match item_id {
        DocumentableItemId::LookupItem(item_id) => match item_id {
            LookupItemId::ModuleItem(item_id) => match item_id {
                ModuleItemId::Struct(item_id) => item_id.full_path(db),
                ModuleItemId::Enum(item_id) => item_id.full_path(db),
                ModuleItemId::Constant(item_id) => item_id.full_path(db),
                ModuleItemId::FreeFunction(item_id) => item_id.full_path(db),
                ModuleItemId::TypeAlias(item_id) => item_id.full_path(db),
                ModuleItemId::ImplAlias(item_id) => item_id.full_path(db),
                ModuleItemId::Trait(item_id) => item_id.full_path(db),
                ModuleItemId::Impl(item_id) => item_id.full_path(db),
                ModuleItemId::ExternType(item_id) => item_id.full_path(db),
                ModuleItemId::ExternFunction(item_id) => item_id.full_path(db),
                ModuleItemId::Submodule(item_id) => item_id.full_path(db),
                ModuleItemId::Use(item_id) => item_id.full_path(db),
            },
            LookupItemId::TraitItem(item_id) => match item_id {
                TraitItemId::Function(item_id) => item_id.full_path(db),
                TraitItemId::Constant(item_id) => item_id.full_path(db),
                TraitItemId::Type(item_id) => item_id.full_path(db),
                TraitItemId::Impl(item_id) => item_id.full_path(db),
            },
            LookupItemId::ImplItem(item_id) => match item_id {
                ImplItemId::Function(item_id) => item_id.full_path(db),
                ImplItemId::Constant(item_id) => item_id.full_path(db),
                ImplItemId::Type(item_id) => item_id.full_path(db),
                ImplItemId::Impl(item_id) => item_id.full_path(db),
            },
        },
        DocumentableItemId::Member(item_id) => item_id.full_path(db),
        DocumentableItemId::Variant(item_id) => item_id.full_path(db),
        DocumentableItemId::Crate(_) => "".to_string(),
    }
    .replace("::", "-")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocLocationLink {
    pub start: usize,
    pub end: usize,
    pub full_path: String,
}

impl DocLocationLink {
    pub fn new(
        start: usize,
        end: usize,
        item_id: DocumentableItemId,
        db: &ScarbDocDatabase,
    ) -> Self {
        Self {
            start,
            end,
            full_path: get_documentable_full_path(db, item_id),
        }
    }
}

fn collect_green_nodes(
    db: &dyn SyntaxGroup,
    syntax_node: &SyntaxNode,
    green_nodes: &mut Vec<(SyntaxKind, String)>,
) -> Vec<(SyntaxKind, String)> {
    let green_node = syntax_node.green_node(db);

    match &green_node.details {
        GreenNodeDetails::Token(text) => green_nodes.push((green_node.kind, text.to_string())),
        GreenNodeDetails::Node { .. } => {
            let syntax_node_children = syntax_node.get_children(db);
            syntax_node_children.iter().for_each(|child| {
                collect_green_nodes(db, child, green_nodes);
            });
        }
    }
    green_nodes.to_owned()
}

fn get_virtual_syntax_file_signature(signature: String) -> (SimpleParserDatabase, SyntaxNode) {
    let sig_db = SimpleParserDatabase::default();
    let virtual_file = FileLongId::Virtual(VirtualFile {
        parent: None,
        name: "string_to_format".into(),
        content: signature.clone().into(),
        code_mappings: [].into(),
        kind: FileKind::Module,
    })
    .intern(&sig_db);

    let syntax_file = Parser::parse_file(
        &sig_db,
        &mut DiagnosticsBuilder::default(),
        virtual_file,
        signature.as_str(),
    )
    .as_syntax_node();

    (sig_db, syntax_file)
}

fn get_offsets(
    signature1: Vec<(SyntaxKind, String)>,
    signature2: Vec<(SyntaxKind, String)>,
) -> Vec<(usize, i32)> {
    let mut offset_vector: Vec<(usize, i32)> = Vec::new();
    let mut original_length_tracker = 0;
    let mut signature1_index = 0;
    let mut signature2_index = 0;

    while signature1_index < signature1.len() && signature2_index < signature2.len() {
        let (kind1, ref string1) = signature1[signature1_index];
        let (kind2, ref string2) = signature2[signature2_index];

        if kind1 == kind2 {
            signature1_index += 1;

            if string1.len() != string2.len() {
                offset_vector.push((
                    original_length_tracker,
                    string2.len() as i32 - string1.len() as i32,
                ));
            }
            original_length_tracker += string1.len();
        } else {
            offset_vector.push((original_length_tracker, string2.len() as i32));
        }
        signature2_index += 1;
    }
    offset_vector
}

/// Moves location links based on differences created in signature formatting.   
fn move_location_links(
    mut location_links: Vec<DocLocationLink>,
    offset_vector: Vec<(usize, i32)>,
) -> Vec<DocLocationLink> {
    for link in &mut location_links {
        let mut new_start = link.start as i32;
        let mut new_end = link.end as i32;

        for (location, length) in offset_vector.iter() {
            if link.end < *location {
                break;
            }
            if link.start >= *location {
                new_start += length;
            }
            if link.end > *location {
                new_end += length;
            } else {
                break;
            }
        }
        link.start = new_start as usize;
        link.end = new_end as usize;
    }
    location_links
}

pub fn format_signature(
    signature: Option<String>,
    location_links: Vec<DocLocationLink>,
) -> (Option<String>, Vec<DocLocationLink>) {
    if let Some(signature) = signature {
        if !location_links.is_empty() {
            let (simple_db, syntax_file) = get_virtual_syntax_file_signature(signature);
            let formatted_file =
                get_formatted_file(&simple_db, &syntax_file, FormatterConfig::default());

            let (simple_db_formatted, syntax_file_formatted) =
                get_virtual_syntax_file_signature(formatted_file.clone());

            let nodes_original = collect_green_nodes(&simple_db, &syntax_file, &mut Vec::new());
            let nodes_formatted = collect_green_nodes(
                &simple_db_formatted,
                &syntax_file_formatted,
                &mut Vec::new(),
            );

            let offsets = get_offsets(nodes_original, nodes_formatted);
            (
                Some(formatted_file.trim_end().to_string()),
                move_location_links(location_links, offsets),
            )
        } else {
            let (simple_db, syntax_file) = get_virtual_syntax_file_signature(signature);
            (
                Some(
                    get_formatted_file(&simple_db, &syntax_file, FormatterConfig::default())
                        .trim_end()
                        .to_string(),
                ),
                location_links,
            )
        }
    } else {
        (signature, location_links)
    }
}

#[cfg(test)]
mod tests {
    use super::{DocLocationLink, format_signature, move_location_links};
    use indoc::indoc;

    #[test]
    fn test_move_location_links() {
        let links = vec![
            DocLocationLink {
                start: 5,
                end: 10,
                full_path: "xyz".to_string(),
            },
            DocLocationLink {
                start: 15,
                end: 20,
                full_path: "xyz".to_string(),
            },
        ];
        let offset_vector = vec![(0, 2), (10, 5)];
        let moved_links = move_location_links(links, offset_vector);

        assert_eq!(moved_links[0].start, 7);
        assert_eq!(moved_links[0].end, 12);
        assert_eq!(moved_links[1].start, 22);
        assert_eq!(moved_links[1].end, 27);
    }

    #[test]
    fn test_signature_formatter() {
        let signature = "fn this_function_has_a_very_long_signature(
    and_contains_a_linked_parameter: Circle,
    lorem: felt252, ipsum: felt252, and_another_linked_parameter_in_the_middle: Circle,
    dolor: felt252, sit: felt252, amet: felt252, yet_another_linked_parameter_at_the_end: Circle,
) -> Circle {}"
            .to_string();

        let links = vec![
            DocLocationLink {
                start: 81,
                end: 87,
                full_path: "hello_world-Circle".to_string(),
            },
            DocLocationLink {
                start: 169,
                end: 175,
                full_path: "hello_world-Circle".to_string(),
            },
            DocLocationLink {
                start: 267,
                end: 273,
                full_path: "hello_world-Circle".to_string(),
            },
            DocLocationLink {
                start: 280,
                end: 286,
                full_path: "hello_world-Circle".to_string(),
            },
        ];

        let (result, moved_links) = format_signature(Some(signature), links);
        assert_eq!(
            result.unwrap(),
            indoc! {
            "fn this_function_has_a_very_long_signature(
                and_contains_a_linked_parameter: Circle,
                lorem: felt252,
                ipsum: felt252,
                and_another_linked_parameter_in_the_middle: Circle,
                dolor: felt252,
                sit: felt252,
                amet: felt252,
                yet_another_linked_parameter_at_the_end: Circle,
            ) -> Circle {}"
            }
        );

        assert_eq!(moved_links[0].start, 81);
        assert_eq!(moved_links[0].end, 87);

        assert_eq!(moved_links[1].start, 177);
        assert_eq!(moved_links[1].end, 183);

        assert_eq!(moved_links[2].start, 287);
        assert_eq!(moved_links[2].end, 293);

        assert_eq!(moved_links[3].start, 300);
        assert_eq!(moved_links[3].end, 306);
    }
}
