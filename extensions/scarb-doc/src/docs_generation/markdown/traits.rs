use itertools::{chain, Itertools};
use std::fmt::Write;

use crate::docs_generation::{DocItem, PrimitiveDocItem, TopLevelDocItem};
use crate::types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, Module, Struct,
    Trait, TypeAlias,
};

pub trait TopLevelMarkdownDocItem: MarkdownDocItem + TopLevelDocItem {
    const ITEMS_SUMMARY_FILENAME: &'static str;

    fn filename(&self) -> String {
        format!("{}.md", self.full_path().replace("::", "-"))
    }

    fn md_ref(&self) -> String {
        format!("[{}](./{})", self.name(), self.filename())
    }

    fn generate_markdown_list_item(&self) -> String {
        format!("- {}\n", self.md_ref())
    }
}

macro_rules! impl_top_level_markdown_doc_item {
    ($t:ty, $filename:expr) => {
        impl TopLevelMarkdownDocItem for $t {
            const ITEMS_SUMMARY_FILENAME: &'static str = $filename;
        }
    };
}

impl_top_level_markdown_doc_item!(Constant, "constants.md");
impl_top_level_markdown_doc_item!(Enum, "enums.md");
impl_top_level_markdown_doc_item!(ExternFunction, "extern_functions.md");
impl_top_level_markdown_doc_item!(ExternType, "extern_types.md");
impl_top_level_markdown_doc_item!(FreeFunction, "free_functions.md");
impl_top_level_markdown_doc_item!(Impl, "impls.md");
impl_top_level_markdown_doc_item!(ImplAlias, "impl_aliases.md");
impl_top_level_markdown_doc_item!(Module, "modules.md");
impl_top_level_markdown_doc_item!(Struct, "structs.md");
impl_top_level_markdown_doc_item!(Trait, "traits.md");
impl_top_level_markdown_doc_item!(TypeAlias, "type_aliases.md");

pub trait MarkdownDocItem: DocItem {
    fn generate_markdown(&self, header_level: usize) -> String;
}

impl<T> MarkdownDocItem for T
where
    T: PrimitiveDocItem,
{
    fn generate_markdown(&self, header_level: usize) -> String {
        generate_markdown_from_item_data(self, header_level)
    }
}

impl MarkdownDocItem for Enum {
    fn generate_markdown(&self, header_level: usize) -> String {
        let mut markdown = generate_markdown_from_item_data(self, header_level);

        markdown += &generate_markdown_for_subitems(&self.variants, header_level);

        markdown
    }
}

impl MarkdownDocItem for Impl {
    fn generate_markdown(&self, header_level: usize) -> String {
        let mut markdown = generate_markdown_from_item_data(self, header_level);

        markdown += &generate_markdown_for_subitems(&self.impl_constants, header_level);
        markdown += &generate_markdown_for_subitems(&self.impl_functions, header_level);
        markdown += &generate_markdown_for_subitems(&self.impl_types, header_level);

        markdown
    }
}

impl MarkdownDocItem for Module {
    fn generate_markdown(&self, header_level: usize) -> String {
        let mut markdown = generate_markdown_from_item_data(self, header_level);

        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.submodules.iter().collect_vec(),
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.constants.iter().collect_vec(),
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.free_functions.iter().collect_vec(),
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.structs.iter().collect_vec(),
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.enums.iter().collect_vec(),
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.type_aliases.iter().collect_vec(),
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.impl_aliases.iter().collect_vec(),
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.traits.iter().collect_vec(),
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.impls.iter().collect_vec(),
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.extern_types.iter().collect_vec(),
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.extern_functions.iter().collect_vec(),
            header_level + 1,
        );

        markdown
    }
}

impl MarkdownDocItem for Struct {
    fn generate_markdown(&self, header_level: usize) -> String {
        let mut markdown = generate_markdown_from_item_data(self, header_level);

        markdown += &generate_markdown_for_subitems(&self.members, header_level);

        markdown
    }
}

impl MarkdownDocItem for Trait {
    fn generate_markdown(&self, header_level: usize) -> String {
        let mut markdown = generate_markdown_from_item_data(self, header_level);

        markdown += &generate_markdown_for_subitems(&self.trait_constants, header_level);
        markdown += &generate_markdown_for_subitems(&self.trait_functions, header_level);
        markdown += &generate_markdown_for_subitems(&self.trait_types, header_level);

        markdown
    }
}

pub fn generate_markdown_list_for_top_level_subitems<T: TopLevelMarkdownDocItem>(
    subitems: &[&T],
    header_level: usize,
) -> String {
    let mut markdown = String::new();

    if !subitems.is_empty() {
        let header = str::repeat("#", header_level);

        writeln!(&mut markdown, "{header} {}\n", T::HEADER).unwrap();
        for item in subitems {
            writeln!(&mut markdown, "{}", item.generate_markdown_list_item()).unwrap();
        }
    }

    markdown
}

fn generate_markdown_for_subitems<T: MarkdownDocItem + PrimitiveDocItem>(
    subitems: &[T],
    header_level: usize,
) -> String {
    let mut markdown = String::new();

    if !subitems.is_empty() {
        let header = str::repeat("#", header_level + 1);

        writeln!(&mut markdown, "{header} {}\n", T::HEADER).unwrap();
        for item in subitems {
            writeln!(
                &mut markdown,
                "{}",
                item.generate_markdown(header_level + 2)
            )
            .unwrap();
        }
    }

    markdown
}

fn generate_markdown_from_item_data(doc_item: &impl DocItem, header_level: usize) -> String {
    let mut markdown = String::new();

    let header = str::repeat("#", header_level);
    let sub_header = str::repeat("#", header_level + 2);

    writeln!(&mut markdown, "{header} {}\n", doc_item.name()).unwrap();

    let documentation = doc_item.doc();

    if let Some(module_level_doc) = &documentation.module_level_comments {
        writeln!(&mut markdown, "{sub_header} {}\n", module_level_doc).unwrap();
    }

    match (
        &documentation.prefix_comments,
        &documentation.inner_comments,
    ) {
        (None, None) => (),
        (prefix, inner) => {
            let common_docs = chain!(prefix, inner)
                .map(|comment| comment.trim_end().to_string())
                .collect::<Vec<_>>()
                .join(" ");
            writeln!(&mut markdown, "{common_docs}\n\n").unwrap();
        }
    }

    writeln!(
        &mut markdown,
        "Fully qualified path: `{}`\n",
        doc_item.full_path()
    )
    .unwrap();

    if let Some(sig) = &doc_item.signature() {
        if !sig.is_empty() {
            // TODO(#1457) add cairo support to mdbook
            writeln!(&mut markdown, "```rust\n{sig}\n```\n").unwrap();
        }
    }

    markdown
}
