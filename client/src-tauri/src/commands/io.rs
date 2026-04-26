use crate::CommandResult;

#[tauri::command]
pub async fn pick_file(
    filters: Option<Vec<(String, Vec<String>)>>,
) -> CommandResult<Option<String>> {
    let task = tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();

        // Add filters if provided
        if let Some(filter_list) = filters {
            for (name, extensions) in filter_list {
                dialog = dialog.add_filter(&name, &extensions);
            }
        }

        dialog
            .pick_file()
            .map(|path| path.to_string_lossy().to_string())
    });

    match task.await {
        Ok(result) => Ok(result),
        Err(_) => Err("File dialog failed".to_string()),
    }
}

#[tauri::command]
pub async fn pick_folder() -> CommandResult<Option<String>> {
    let task = tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .pick_folder()
            .map(|path| path.to_string_lossy().to_string())
    });

    match task.await {
        Ok(result) => Ok(result),
        Err(_) => Err("Folder dialog failed".to_string()),
    }
}
