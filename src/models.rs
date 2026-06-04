use std::{collections::BTreeMap, sync::Mutex};

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::backend::load_data;

/// Configuration for the i18n plugin.
///
/// Allows customizing the default locale, and optionally specifying
/// a runtime path to load locale overrides from.
///
/// # Example
///
/// ```rust,ignore
/// use tauri_plugin_i18n::I18nConfig;
///
/// let config = I18nConfig {
///     default_locale: "fr".to_string(),
///     runtime_locales_path: Some("/path/to/locales".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct I18nConfig {
    /// The default locale to use (e.g. "en", "zh-CN").
    /// Defaults to "en".
    pub default_locale: String,
    /// Optional path to a directory of locale files to load at runtime.
    /// Files in this directory override bundled translations with the same keys.
    /// Defaults to `None` (runtime loading disabled).
    pub runtime_locales_path: Option<String>,
}

impl Default for I18nConfig {
    fn default() -> Self {
        Self {
            default_locale: "en".to_string(),
            runtime_locales_path: None,
        }
    }
}

#[derive(Debug)]
pub struct PluginI18n<R: Runtime> {
    pub app: AppHandle<R>,
    pub data: BTreeMap<String, BTreeMap<String, String>>,
    pub locale: Mutex<String>,
}

impl<R: Runtime> PluginI18n<R> {
    ///
    /// Initialize the i18n plugin with the given configuration.
    ///
    pub fn new(app: tauri::AppHandle<R>, config: I18nConfig) -> Self {
        let data = load_data(config.runtime_locales_path.as_deref());
        Self {
            app,
            data,
            locale: Mutex::new(config.default_locale),
        }
    }

    ///
    /// Gets the available locales
    ///
    pub fn available_locales(&self) -> Vec<String> {
        self.data.keys().map(|k| k.to_string()).collect()
    }

    ///
    /// Gets the translated string according to the current locale
    ///
    pub fn translate(&self, key: &str) -> Option<&str> {
        let locale = self.locale.lock().ok()?;

        self.data
            .get(&locale.to_string())?
            .get(key)
            .map(|k| k.as_str())
    }

    ///
    /// Returns the data used for translations
    ///
    pub fn get_translations_data(&self) -> BTreeMap<String, BTreeMap<String, String>> {
        self.data.clone()
    }

    ///
    /// Update the locale.
    /// eg: "zh-CN", "en-US"
    ///
    pub fn set_locale(&self, locale: &str) {
        let mut l = self.locale.lock().unwrap();
        *l = locale.to_string();
        let _ = self.app.emit("i18n:locale_changed", locale);
    }

    ///
    /// Get the current locale.
    /// eg: "zh-CN", "en-US"
    /// Default locale is "en".
    ///
    pub fn get_locale(&self) -> String {
        let locale = self.locale.lock();
        if let Ok(l) = locale {
            l.to_string()
        } else {
            "en".to_string()
        }
    }
}

pub trait PluginI18nExt<R: Runtime> {
    fn i18n(&self) -> &PluginI18n<R>;
}

impl<R: Runtime, T: Manager<R>> PluginI18nExt<R> for T {
    fn i18n(&self) -> &PluginI18n<R> {
        self.state::<PluginI18n<R>>().inner()
    }
}
