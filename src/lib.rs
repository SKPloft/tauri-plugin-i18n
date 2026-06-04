use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

mod backend;
mod commands;
mod error;
mod models;

pub use error::{Error, Result};

/// Initializes the plugin with the given configuration.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_i18n::I18nConfig;
///
/// // Use defaults
/// tauri::Builder::default()
///     .plugin(tauri_plugin_i18n::init(I18nConfig::default()))
///
/// // Customize locale path
/// tauri::Builder::default()
///     .plugin(tauri_plugin_i18n::init(I18nConfig {
///         default_locale: "fr".to_string(),
///         runtime_locales_path: Some("/path/to/locales".to_string()),
///     }))
/// ```
pub fn init<R: Runtime>(config: I18nConfig) -> TauriPlugin<R> {
    Builder::new("i18n")
        .invoke_handler(tauri::generate_handler![
            commands::load_translations,
            commands::translate,
            commands::set_locale,
            commands::get_locale,
            commands::get_available_locales,
        ])
        .setup(move |app, _api| {
            app.manage(PluginI18n::new(app.clone(), config.clone()));

            Ok(())
        })
        .build()
}
