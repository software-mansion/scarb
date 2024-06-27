use anyhow::Result;

use crate::types::{Module, Trait, TraitConstant, TraitFunction, TraitType};

use std::{fmt::Write, path::Path};

pub trait Markdown {
    fn md_ref(&self) -> String;
    fn generate_markdown(&self) -> Result<String>;
    fn generate_markdown_brief(&self) -> String;
    #[allow(dead_code)]
    fn generate_markdown_list_item(&self) -> String {
        format!("- {}\n", self.md_ref())
    }
}

pub trait ToC {
    fn generate_toc(&self) -> String;
}

impl Markdown for Module {
    fn md_ref(&self) -> String {
        format!(
            "[{}]({}.md)",
            self.name,
            self.full_path.to_lowercase().replace(' ', "-")
        )
    }

    fn generate_markdown(&self) -> Result<String> {
        let mut markdown = String::new();

        writeln!(&mut markdown, "# Module {}\n", self.full_path_ref())?;

        writeln!(&mut markdown, "{}", self.generate_toc())?;

        writeln!(&mut markdown, "## Submodules\n").unwrap();
        for submodule in &self.submodules {
            writeln!(&mut markdown, "\n{}\n", submodule.generate_markdown_brief())?;
        }

        writeln!(&mut markdown, "## Traits\n").unwrap();
        for trait_ in &self.traits {
            writeln!(&mut markdown, "\n{}\n", trait_.generate_markdown()?)?;
        }

        Ok(markdown)
    }

    fn generate_markdown_brief(&self) -> String {
        format!(
            "### {}\n{}",
            self.md_ref(),
            self.doc.as_ref().unwrap_or(&String::new())
        )
    }

    fn generate_markdown_list_item(&self) -> String {
        format!("- {}\n", self.md_ref())
    }
}

macro_rules! add_section {
    ($markdown:expr, $title:expr, $condition:expr) => {
        if $condition {
            writeln!(
                $markdown,
                "- [{}](#{})",
                $title,
                $title.to_lowercase().replace(' ', "-")
            )
            .unwrap();
        }
    };
}

impl ToC for Module {
    fn generate_toc(&self) -> String {
        let mut markdown = String::new();

        add_section!(&mut markdown, "Submodules", !self.submodules.is_empty());
        add_section!(&mut markdown, "Structs", !self.structs.is_empty());
        add_section!(&mut markdown, "Enums", !self.enums.is_empty());
        add_section!(&mut markdown, "Functions", !self.free_functions.is_empty());
        add_section!(&mut markdown, "Constants", !self.constants.is_empty());
        add_section!(&mut markdown, "Type Aliases", !self.type_aliases.is_empty());
        add_section!(&mut markdown, "Impl Aliases", !self.impl_aliases.is_empty());
        add_section!(&mut markdown, "Traits", !self.traits.is_empty());
        add_section!(&mut markdown, "Impls", !self.impls.is_empty());
        add_section!(&mut markdown, "Extern Types", !self.extern_types.is_empty());
        add_section!(
            &mut markdown,
            "Extern Functions",
            !self.extern_functions.is_empty()
        );

        markdown
    }
}

impl Module {
    pub fn full_path_ref(&self) -> String {
        let parts: Vec<&str> = self.full_path.split("::").collect();

        let mut curr_path = String::new();
        parts
            .iter()
            .enumerate()
            .map(|(index, &part)| {
                if index > 0 {
                    curr_path.push_str("::");
                }
                curr_path.push_str(part);

                if index != parts.len() - 1 {
                    format!("[{}]({}.md)", part, curr_path)
                } else {
                    part.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("::")
    }

    pub fn save_to_file_recursive(&self, directory: &Path) -> Result<()> {
        let markdown = self.generate_markdown()?;

        std::fs::create_dir_all(directory)?;
        std::fs::write(directory.join(format!("{}.md", self.full_path)), markdown)?;

        for submodule in &self.submodules {
            submodule.save_to_file_recursive(directory)?;
        }

        Ok(())
    }
}

impl Markdown for Trait {
    fn md_ref(&self) -> String {
        format!(
            "[{}]({}.md#{})",
            self.item_data.name,
            self.item_data.full_path.to_lowercase(),
            self.item_data.name
        )
    }

    fn generate_markdown(&self) -> Result<String> {
        let mut markdown = String::new();

        writeln!(&mut markdown, "### {}\n", self.item_data.name)?;
        writeln!(
            &mut markdown,
            "{}",
            self.item_data.doc.as_ref().unwrap_or(&String::new())
        )?;

        writeln!(
            &mut markdown,
            "```rust\n{}\n```\n",
            self.item_data.signature
        )?;

        if !self.trait_constants.is_empty() {
            writeln!(&mut markdown, "#### Trait Constants\n",)?;
            for trait_const in &self.trait_constants {
                writeln!(&mut markdown, "{}", trait_const.generate_markdown()?)?;
            }
        }

        if !self.trait_types.is_empty() {
            writeln!(&mut markdown, "#### Trait Types\n",)?;
            for trait_type in &self.trait_types {
                writeln!(&mut markdown, "{}", trait_type.generate_markdown()?)?;
            }
        }

        if !self.trait_functions.is_empty() {
            writeln!(&mut markdown, "#### Trait Functions\n",)?;
            for trait_func in &self.trait_functions {
                writeln!(&mut markdown, "{}", trait_func.generate_markdown()?)?;
            }
        }

        Ok(markdown)
    }

    fn generate_markdown_brief(&self) -> String {
        format!(
            "### {}\n{}",
            self.md_ref(),
            self.item_data.doc.as_ref().unwrap_or(&String::new())
        )
    }
}

macro_rules! impl_md {
    ($t:ty) => {
        impl Markdown for $t {
            fn md_ref(&self) -> String {
                format!(
                    "[{}]({}.md#{})",
                    self.item_data.name,
                    self.item_data.full_path.to_lowercase(),
                    self.item_data.name
                )
            }

            fn generate_markdown(&self) -> Result<String> {
                let mut markdown = String::new();

                writeln!(&mut markdown, "{}", self.generate_markdown_brief())?;
                writeln!(
                    &mut markdown,
                    "```rust\n{}\n```\n",
                    self.item_data.text_without_trivia
                )?;

                Ok(markdown)
            }

            fn generate_markdown_brief(&self) -> String {
                format!(
                    "##### {}\n{}",
                    self.item_data.name,
                    self.item_data.doc.as_ref().unwrap_or(&String::new())
                )
            }
        }
    };
}

impl_md!(TraitConstant);
impl_md!(TraitType);
impl_md!(TraitFunction);
