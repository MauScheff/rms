use std::{fs, io, path::Path};

use super::{RmsWorkbenchEffect, RmsWorkbenchEffectResult};

pub(crate) fn write_if_changed(path: &Path, contents: &str) -> io::Result<()> {
    if fs::read_to_string(path).is_ok_and(|current| current == contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

pub(crate) fn execute_rms_workbench_effect(effect: RmsWorkbenchEffect) -> RmsWorkbenchEffectResult {
    match effect {
        RmsWorkbenchEffect::WriteSemanticChangeRecord {
            record_path,
            record_contents,
        } => match write_if_changed(&record_path, &record_contents) {
            Ok(()) => RmsWorkbenchEffectResult::SemanticChangeRecordWritten,
            Err(_) => RmsWorkbenchEffectResult::SemanticChangeRecordWriteRejected,
        },
    }
}
