use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use crate::models::settings::{AppSettings, WebSearchProvider};
use crate::runtime::search::{SearchProvider, SerperProvider, TavilyProvider};

pub struct SearchRuntime {
    provider: Mutex<Option<Arc<dyn SearchProvider>>>,
    /// Avoid rebuilding `reqwest::blocking::Client` (owns a Tokio runtime) on every chat.
    fingerprint: Mutex<String>,
}

impl SearchRuntime {
    pub fn new() -> Self {
        Self {
            provider: Mutex::new(None),
            fingerprint: Mutex::new(String::new()),
        }
    }

    pub fn configure(&self, settings: &AppSettings) {
        let next_fp = config_fingerprint(settings);
        {
            let current = lock_recover(&self.fingerprint);
            if *current == next_fp {
                return;
            }
        }
        let settings = settings.clone();
        let next = crate::runtime::isolated::run_isolated(move || build_provider(&settings));
        let previous = {
            let mut provider = lock_recover(&self.provider);
            std::mem::replace(&mut *provider, next)
        };
        *lock_recover(&self.fingerprint) = next_fp;
        if previous.is_some() {
            crate::runtime::isolated::drop_isolated(previous);
        }
    }

    pub fn provider(&self) -> Option<Arc<dyn SearchProvider>> {
        lock_recover(&self.provider).clone()
    }

    pub fn is_available(&self) -> bool {
        self.provider().is_some()
    }
}

impl Default for SearchRuntime {
    fn default() -> Self {
        Self::new()
    }
}

fn config_fingerprint(settings: &AppSettings) -> String {
    if !settings.web_search_enabled {
        return "disabled".into();
    }
    match settings.web_search_provider {
        WebSearchProvider::Serper => {
            format!("serper:{}", settings.serper_api_key.trim())
        }
        WebSearchProvider::Tavily => {
            format!("tavily:{}", settings.tavily_api_key.trim())
        }
    }
}

fn build_provider(settings: &AppSettings) -> Option<Arc<dyn SearchProvider>> {
    if !settings.web_search_enabled {
        return None;
    }
    match settings.web_search_provider {
        WebSearchProvider::Serper => {
            let key = settings.serper_api_key.trim();
            if key.is_empty() {
                return None;
            }
            SerperProvider::new(key)
                .ok()
                .map(|provider| Arc::new(provider) as Arc<dyn SearchProvider>)
        }
        WebSearchProvider::Tavily => {
            let key = settings.tavily_api_key.trim();
            if key.is_empty() {
                return None;
            }
            TavilyProvider::new(key)
                .ok()
                .map(|provider| Arc::new(provider) as Arc<dyn SearchProvider>)
        }
    }
}

fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn shared_search_runtime() -> Arc<SearchRuntime> {
    static RUNTIME: OnceLock<Arc<SearchRuntime>> = OnceLock::new();
    Arc::clone(RUNTIME.get_or_init(|| Arc::new(SearchRuntime::new())))
}
