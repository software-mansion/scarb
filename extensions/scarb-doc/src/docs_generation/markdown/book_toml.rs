use indoc::formatdoc;

use crate::AdditionalMetadata;

pub fn generate_book_toml_content(package_metadata: &AdditionalMetadata) -> String {
    formatdoc! {
        r#"
            [book]
            authors = {:?}
            language = "en"
            multilingual = false
            src = "src"
            title = "{} - Cairo"

            [output.html]
            no-section-label = true

            [output.html.playground]
            runnable = false

            [output.html.fold]
            enable = true
            level = 0
        "#,
        package_metadata.authors.clone().unwrap_or_else(|| vec!["<unknown>".to_string()]),
        package_metadata.name
    }
}
