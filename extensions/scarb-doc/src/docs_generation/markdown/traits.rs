use itertools::Itertools;
use std::fmt::Write;

use crate::docs_generation::{DocItem, PrimitiveDocItem, TopLevelDocItem};
use crate::types::{Enum, Impl, Module, Struct, Trait};

pub trait TopLevelMarkdownDocItem: MarkdownDocItem + TopLevelDocItem {
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

impl<T> TopLevelMarkdownDocItem for T where T: MarkdownDocItem + TopLevelDocItem {}

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

        markdown += &generate_markdown_for_subitems(&self.variants, "Variants", header_level);

        markdown
    }
}

impl MarkdownDocItem for Impl {
    fn generate_markdown(&self, header_level: usize) -> String {
        let mut markdown = generate_markdown_from_item_data(self, header_level);

        markdown +=
            &generate_markdown_for_subitems(&self.impl_constants, "Impl Constants", header_level);
        markdown +=
            &generate_markdown_for_subitems(&self.impl_functions, "Impl Functions", header_level);
        markdown += &generate_markdown_for_subitems(&self.impl_types, "Impl Types", header_level);

        markdown
    }
}

impl MarkdownDocItem for Module {
    fn generate_markdown(&self, header_level: usize) -> String {
        let mut markdown = generate_markdown_from_item_data(self, header_level);

        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.submodules.iter().collect_vec(),
            "Submodules",
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.constants.iter().collect_vec(),
            "Constants",
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.free_functions.iter().collect_vec(),
            "Free functions",
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.structs.iter().collect_vec(),
            "Structs",
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.enums.iter().collect_vec(),
            "Enums",
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.type_aliases.iter().collect_vec(),
            "Type aliases",
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.impl_aliases.iter().collect_vec(),
            "Impl aliases",
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.traits.iter().collect_vec(),
            "Traits",
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.impls.iter().collect_vec(),
            "Impls",
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.extern_types.iter().collect_vec(),
            "Extern types",
            header_level + 1,
        );
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.extern_functions.iter().collect_vec(),
            "Extern functions",
            header_level + 1,
        );

        markdown
    }
}

impl MarkdownDocItem for Struct {
    fn generate_markdown(&self, header_level: usize) -> String {
        let mut markdown = generate_markdown_from_item_data(self, header_level);

        markdown += &generate_markdown_for_subitems(&self.members, "Members", header_level);

        markdown
    }
}

impl MarkdownDocItem for Trait {
    fn generate_markdown(&self, header_level: usize) -> String {
        let mut markdown = generate_markdown_from_item_data(self, header_level);

        markdown +=
            &generate_markdown_for_subitems(&self.trait_constants, "Trait Constants", header_level);
        markdown +=
            &generate_markdown_for_subitems(&self.trait_functions, "Trait Functions", header_level);
        markdown += &generate_markdown_for_subitems(&self.trait_types, "Trait Types", header_level);

        markdown
    }
}

pub fn generate_markdown_list_for_top_level_subitems(
    subitems: &[&impl TopLevelMarkdownDocItem],
    name: &str,
    header_level: usize,
) -> String {
    let mut markdown = String::new();

    if !subitems.is_empty() {
        let header = str::repeat("#", header_level);

        writeln!(&mut markdown, "{header} {name}\n").unwrap();
        for item in subitems {
            writeln!(&mut markdown, "{}", item.generate_markdown_list_item()).unwrap();
        }
    }

    markdown
}

fn generate_markdown_for_subitems(
    subitems: &[impl MarkdownDocItem + PrimitiveDocItem],
    name: &str,
    header_level: usize,
) -> String {
    let mut markdown = String::new();

    if !subitems.is_empty() {
        let header = str::repeat("#", header_level + 1);

        writeln!(&mut markdown, "{header} {name}\n").unwrap();
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

fn generate_markdown_from_item_data(doc_item: &dyn DocItem, header_level: usize) -> String {
    let mut markdown = String::new();

    let header = str::repeat("#", header_level);

    writeln!(&mut markdown, "{header} {}\n", doc_item.name()).unwrap();

    if let Some(doc) = doc_item.doc() {
        writeln!(&mut markdown, "{doc}\n").unwrap();
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
