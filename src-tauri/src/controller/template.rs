use std::path::Path;

use tauri::{command, AppHandle};

use crate::service::template::{self, TemplateKind, TemplateStatus};
use crate::utils::{resolve_template_path, TEMPLATE_PAT_FILE_NAME, TEMPLATE_PLI_FILE_NAME};

fn template_file_name(kind: TemplateKind) -> &'static str {
    match kind {
        TemplateKind::Pli => TEMPLATE_PLI_FILE_NAME,
        TemplateKind::Pat => TEMPLATE_PAT_FILE_NAME,
    }
}

#[command]
pub async fn save_template(
    app_handle: AppHandle,
    kind: TemplateKind,
    file_path: String,
) -> Result<(), String> {
    let dest = resolve_template_path(&app_handle, template_file_name(kind))?;

    template::save_template(kind, Path::new(&file_path), &dest).map_err(|e| e.to_string())
}

#[command]
pub async fn get_templates_status(app_handle: AppHandle) -> Result<TemplateStatus, String> {
    let pli_path = resolve_template_path(&app_handle, TEMPLATE_PLI_FILE_NAME)?;
    let pat_path = resolve_template_path(&app_handle, TEMPLATE_PAT_FILE_NAME)?;

    Ok(template::templates_status(&pli_path, &pat_path))
}
