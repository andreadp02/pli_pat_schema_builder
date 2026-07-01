use std::path::Path;

use calamine::{open_workbook, Reader, Xlsx};
use serde::{Deserialize, Serialize};

use crate::service::excel::{PAT_SHEET, PLI_SHEET};
use crate::AppError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemplateKind {
    Pli,
    Pat,
}

impl TemplateKind {
    fn expected_sheet(self) -> &'static str {
        match self {
            TemplateKind::Pli => PLI_SHEET,
            TemplateKind::Pat => PAT_SHEET,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateStatus {
    pub pli: bool,
    pub pat: bool,
}

/// Validate an uploaded ADM template and copy it into the app data dir (overwriting any existing
/// one). A wrong/renamed file is rejected here so generation never fails with a cryptic error.
pub fn save_template(kind: TemplateKind, file_path: &Path, dest: &Path) -> Result<(), AppError> {
    if !file_path.is_file() {
        return Err(AppError::Processing("Selected file does not exist".to_string()));
    }

    let ext = file_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !ext.eq_ignore_ascii_case("xlsx") {
        return Err(AppError::Processing(
            "Only .xlsx files are supported".to_string(),
        ));
    }

    let workbook: Xlsx<_> = open_workbook(file_path)
        .map_err(|e| AppError::Processing(format!("Not a valid .xlsx file: {e}")))?;

    let expected = kind.expected_sheet();
    if !workbook.sheet_names().iter().any(|name| name == expected) {
        return Err(AppError::Processing(format!(
            "Wrong template: the expected sheet '{expected}' was not found"
        )));
    }

    std::fs::copy(file_path, dest).map_err(|e| AppError::Io(e.to_string()))?;
    Ok(())
}

pub fn templates_status(pli_path: &Path, pat_path: &Path) -> TemplateStatus {
    TemplateStatus {
        pli: pli_path.is_file(),
        pat: pat_path.is_file(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn tmp_path(name: &str) -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("template_test_{id}_{name}"))
    }

    /// Minimal .xlsx containing a sheet with the given name.
    fn xlsx_with_sheet(sheet_name: &str) -> PathBuf {
        let path = tmp_path("src.xlsx");
        let mut book = umya_spreadsheet::new_file();
        book.new_sheet(sheet_name).unwrap();
        umya_spreadsheet::writer::xlsx::write(&book, &path).unwrap();
        path
    }

    #[test]
    fn rejects_non_xlsx() {
        let src = tmp_path("fake.txt");
        std::fs::write(&src, b"not excel").unwrap();
        assert!(save_template(TemplateKind::Pli, &src, &tmp_path("dest.xlsx")).is_err());
    }

    #[test]
    fn rejects_wrong_template() {
        // A real xlsx, but it's the PLI sheet fed into the PAT slot.
        let src = xlsx_with_sheet(PLI_SHEET);
        assert!(save_template(TemplateKind::Pat, &src, &tmp_path("dest.xlsx")).is_err());
    }

    #[test]
    fn saves_matching_template() {
        let src = xlsx_with_sheet(PLI_SHEET);
        let dest = tmp_path("dest.xlsx");
        save_template(TemplateKind::Pli, &src, &dest).unwrap();
        assert!(dest.is_file());
    }
}
