pub mod commands;
pub mod github_client;
pub mod parser;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_gh_cli_token,
            commands::verify_github_token,
            commands::preview_team_invitations,
            commands::run_team_invitations,
            commands::preview_repo_provisioning,
            commands::run_repo_provisioning,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
