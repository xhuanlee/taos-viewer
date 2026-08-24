mod commands;
mod error;
mod models;
mod state;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(state::AppState::new(dir.join("connections.json")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_connections,
            commands::save_connections,
            commands::connect,
            commands::disconnect,
            commands::test_connection,
            commands::execute_batch,
            commands::list_databases,
            commands::list_tables,
            commands::describe_table,
            commands::show_create_table,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
