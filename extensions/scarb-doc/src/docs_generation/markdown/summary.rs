pub mod content;
pub mod files;
pub mod group_files;

use crate::docs_generation::common::OutputFilesExtension;
use crate::docs_generation::markdown::context::MarkdownGenerationContext;
use crate::docs_generation::markdown::summary::content::{
    generate_foreign_crates_summary_content, generate_global_groups_summary_content,
    generate_module_summary_content,
};
use crate::docs_generation::markdown::summary::files::{
    generate_foreign_crates_summary_files, generate_modules_summary_files,
};
use crate::docs_generation::markdown::traits::{MarkdownDocItem, TopLevelMarkdownDocItem};
use crate::docs_generation::markdown::{BASE_HEADER_LEVEL, SummaryIndexMap};
use crate::types::crate_type::Crate;
use anyhow::Result;
use camino::Utf8PathBuf;
use group_files::generate_global_groups_summary_files;

pub fn generate_summary_file_content(
    crate_: &Crate,
    output_format: OutputFilesExtension,
    base_doc_url: Option<String>,
    workspace_root: Utf8PathBuf,
) -> Result<(SummaryIndexMap, Vec<(String, String)>)> {
    let mut summary_index_map = SummaryIndexMap::new();
    let context =
        MarkdownGenerationContext::from_crate(crate_, output_format, base_doc_url, workspace_root);

    generate_module_summary_content(
        &crate_.root_module,
        0,
        &mut summary_index_map,
        context.files_extension,
    );
    generate_foreign_crates_summary_content(
        &crate_.foreign_crates,
        &mut summary_index_map,
        context.files_extension,
    );
    generate_global_groups_summary_content(
        &crate_.groups,
        &mut summary_index_map,
        context.files_extension,
    );

    let mut summary_files = vec![(
        crate_.root_module.filename(context.files_extension),
        crate_.root_module.generate_markdown(
            &context,
            BASE_HEADER_LEVEL,
            None,
            &summary_index_map,
        )?,
    )];

    let module_item_summaries =
        &generate_modules_summary_files(&crate_.root_module, &context, &summary_index_map)?;
    summary_files.extend(module_item_summaries.to_owned());

    let foreign_modules_files = generate_foreign_crates_summary_files(
        &crate_.foreign_crates,
        &context,
        &summary_index_map,
    )?;

    summary_files.extend(foreign_modules_files);

    let groups_files =
        generate_global_groups_summary_files(&crate_.groups, &context, &summary_index_map)?;
    summary_files.extend(groups_files.to_owned());
    Ok((summary_index_map, summary_files))
}
