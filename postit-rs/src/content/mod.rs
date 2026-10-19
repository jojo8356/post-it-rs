use std::path::Path;
use std::sync::{Arc, Mutex};
use anyhow::{Context, Result};
use rand::{rngs::StdRng, SeedableRng, Rng};
use tracing::{debug, info, warn};
use crate::config::{Config, ContentMode, PostItContent};
use crate::json_posts::{JsonPostItCollection, json_to_postit_content, select_weighted_random};

/// Content provider for post-it messages
pub struct ContentProvider {
    config: Arc<Mutex<Config>>,
    content_cache: Vec<PostItContent>,
    json_collection: Option<JsonPostItCollection>,
    sequential_index: usize,
    rng: StdRng,
}

impl ContentProvider {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        Self {
            config,
            content_cache: Vec::new(),
            json_collection: None,
            sequential_index: 0,
            rng: StdRng::from_entropy(),
        }
    }

    /// Reload content from source based on current config
    pub fn reload(&mut self) -> Result<()> {
        // Clone config values we need to avoid holding lock
        let (content_mode, content_file, json_file, json_category, sequential_index, title_enabled, title_text) = {
            let config = self.config.lock().unwrap();
            (
                config.general.content_mode,
                config.general.content_file.clone(),
                config.general.json_file.clone(),
                config.general.json_category.clone(),
                config.general.sequential_index,
                config.title.enabled,
                config.title.text.clone(),
            )
        };

        self.content_cache.clear();

        match content_mode {
            ContentMode::Fixed => {
                if !title_text.is_empty() || true {
                    self.content_cache.push(PostItContent {
                        title: if title_enabled && !title_text.is_empty() {
                            Some(title_text)
                        } else {
                            None
                        },
                        body: String::new(),
                    });
                }
            }
            ContentMode::Sequential | ContentMode::Random => {
                self.load_default_content();
            }
            ContentMode::File => {
                self.load_from_file(&content_file)?;
            }
            ContentMode::Json => {
                self.load_json(&json_file, &json_category)?;
            }
        }

        self.sequential_index = sequential_index.min(self.content_cache.len().saturating_sub(1));

        info!("Content provider reloaded: {} items, mode={:?}", self.content_cache.len(), content_mode);
        Ok(())
    }

    /// Load built-in default content
    fn load_default_content(&mut self) {
        let defaults = [
            ("Rappel", "Buvez de l'eau et étirez-vous !"),
            ("Rappel", "Pensez à commit & push avant le déjeuner"),
            ("Citation", "\"Le code est lu plus souvent qu'il n'est écrit.\" — Guido van Rossum"),
            ("Focus", "Une seule chose à la fois. Terminez-la."),
            ("Santé", "Regardez loin 20 secondes (règle 20-20-20)"),
            ("Productivité", "La méthode Pomodoro : 25min focus / 5min pause"),
            ("Git", "git add -p pour reviewer vos changements avant commit"),
            ("Rust", "let x = 5; // immutable by default"),
            ("Motivation", "Progrès > Perfection"),
            ("Astuce", "Ctrl+Shift+T rouvre l'onglet fermé par erreur"),
        ];

        for (title, body) in defaults {
            self.content_cache.push(PostItContent {
                title: Some(title.to_string()),
                body: body.to_string(),
            });
        }
    }

    /// Load content from external file
    fn load_from_file(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            warn!("Content file not found: {}, using defaults", path.display());
            self.load_default_content();
            return Ok(());
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read content file: {}", path.display()))?;

        for line in content.lines() {
            let item = PostItContent::from_line(line);
            if !item.body.is_empty() {
                self.content_cache.push(item);
            }
        }

        if self.content_cache.is_empty() {
            warn!("No valid content in file, using defaults");
            self.load_default_content();
        }

        Ok(())
    }

    /// Load content from JSON file with checkboxes
    fn load_json(&mut self, path: &Path, category: &str) -> Result<()> {
        if !path.exists() {
            warn!("JSON post-its file not found: {}, creating example", path.display());
            let collection = JsonPostItCollection::example();
            collection.save_to_file(path).ok();
            self.json_collection = Some(collection);
        } else {
            let collection = JsonPostItCollection::load_from_file(path)
                .with_context(|| format!("Failed to load JSON post-its: {}", path.display()))?;
            self.json_collection = Some(collection);
        }

        // Convert enabled JSON post-its to internal format
        if let Some(collection) = &self.json_collection {
            let category_filter = if category.is_empty() { None } else { Some(&category[..]) };
            let enabled = collection.get_enabled(category_filter);
            self.content_cache = json_to_postit_content(&enabled);
            
            info!("Loaded {} enabled JSON post-its (category: {:?})", 
                  self.content_cache.len(), category_filter);
        }

        if self.content_cache.is_empty() {
            warn!("No enabled JSON post-its, using defaults");
            self.load_default_content();
        }

        Ok(())
    }

    /// Get next post-it content based on mode
    pub fn next(&mut self) -> PostItContent {
        let config = self.config.lock().unwrap();
        let mode = config.general.content_mode;
        let json_category = config.general.json_category.clone();
        drop(config);

        if self.content_cache.is_empty() {
            self.reload().ok();
        }

        if self.content_cache.is_empty() {
            // Ultimate fallback
            return PostItContent {
                title: Some("Post-it".to_string()),
                body: "Bienvenue ! Configurez vos post-its dans le fichier de config.".to_string(),
            };
        }

        let item = match mode {
            ContentMode::Fixed => {
                // Always return first (or title/body from config)
                self.content_cache.first().cloned().unwrap_or_default()
            }
            ContentMode::Sequential => {
                let item = self.content_cache[self.sequential_index].clone();
                self.sequential_index = (self.sequential_index + 1) % self.content_cache.len();
                // Persist index
                if let Ok(mut config) = self.config.lock() {
                    config.general.sequential_index = self.sequential_index;
                }
                item
            }
            ContentMode::Random => {
                // For JSON mode, use weighted random selection
                if mode == ContentMode::Json {
                    if let Some(collection) = &self.json_collection {
                        let category_filter = if json_category.is_empty() { None } else { Some(&json_category[..]) };
                        let weighted = collection.get_weighted_enabled(category_filter);
                        if let Some(selected) = select_weighted_random(&weighted) {
                            return PostItContent {
                                title: selected.title.clone(),
                                body: selected.body.clone(),
                            };
                        }
                    }
                }
                // Fallback to simple random
                let idx = self.rng.gen_range(0..self.content_cache.len());
                self.content_cache[idx].clone()
            }
            ContentMode::File => {
                // For file mode, also random by default
                let idx = self.rng.gen_range(0..self.content_cache.len());
                self.content_cache[idx].clone()
            }
            ContentMode::Json => {
                // JSON mode with weighted random
                if let Some(collection) = &self.json_collection {
                    let category_filter = if json_category.is_empty() { None } else { Some(&json_category[..]) };
                    let weighted = collection.get_weighted_enabled(category_filter);
                    if let Some(selected) = select_weighted_random(&weighted) {
                        return PostItContent {
                            title: selected.title.clone(),
                            body: selected.body.clone(),
                        };
                    }
                }
                // Fallback
                let idx = self.rng.gen_range(0..self.content_cache.len());
                self.content_cache[idx].clone()
            }
        };

        debug!("Selected content: title={:?}, body_len={}", item.title, item.body.len());
        item
    }

    /// Get current content without advancing (for preview)
    pub fn current(&self) -> Option<PostItContent> {
        if self.content_cache.is_empty() {
            None
        } else {
            let config = self.config.lock().unwrap();
            let mode = config.general.content_mode;
            drop(config);

            match mode {
                ContentMode::Fixed | ContentMode::Sequential => {
                    Some(self.content_cache[self.sequential_index.min(self.content_cache.len() - 1)].clone())
                }
                ContentMode::Random | ContentMode::File | ContentMode::Json => {
                    // For random, return a sample
                    Some(self.content_cache[0].clone())
                }
            }
        }
    }

    /// Get all available content (for debugging/UI)
    pub fn all_content(&self) -> Vec<PostItContent> {
        self.content_cache.clone()
    }

    /// Get JSON collection for UI (if loaded)
    pub fn json_collection(&self) -> Option<&JsonPostItCollection> {
        self.json_collection.as_ref()
    }

    /// Get mutable JSON collection for UI
    pub fn json_collection_mut(&mut self) -> Option<&mut JsonPostItCollection> {
        self.json_collection.as_mut()
    }

    /// Save JSON collection back to file
    pub fn save_json(&mut self) -> Result<()> {
        if let Some(collection) = &self.json_collection {
            let config = self.config.lock().unwrap();
            let path = config.general.json_file.clone();
            drop(config);
            if !path.as_os_str().is_empty() {
                collection.save_to_file(&path)?;
            }
        }
        Ok(())
    }

    /// Set content mode at runtime
    pub fn set_mode(&mut self, mode: ContentMode) -> Result<()> {
        if let Ok(mut config) = self.config.lock() {
            config.general.content_mode = mode;
        }
        self.reload()
    }
}

impl Default for PostItContent {
    fn default() -> Self {
        Self {
            title: None,
            body: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[test]
    fn test_content_provider_fixed() {
        let config = Arc::new(Mutex::new(Config::new()));
        let mut provider = ContentProvider::new(config);
        provider.reload().unwrap();

        let item = provider.next();
        assert!(!item.body.is_empty() || item.title.is_some());
    }

    #[test]
    fn test_content_provider_sequential() {
        let mut config = Config::new();
        config.general.content_mode = ContentMode::Sequential;
        let config = Arc::new(Mutex::new(config));

        let mut provider = ContentProvider::new(config.clone());
        provider.reload().unwrap();

        let first = provider.next();
        let second = provider.next();

        // Should cycle through
        assert_ne!(first.body, second.body);
    }

    #[test]
    fn test_content_provider_random() {
        let mut config = Config::new();
        config.general.content_mode = ContentMode::Random;
        let config = Arc::new(Mutex::new(config));

        let mut provider = ContentProvider::new(config);
        provider.reload().unwrap();

        // Just verify it returns something
        let item = provider.next();
        assert!(!item.body.is_empty() || item.title.is_some());
    }

    #[test]
    fn test_postit_content_parsing() {
        let c = PostItContent::from_line("Title|Body");
        assert_eq!(c.title, Some("Title".to_string()));
        assert_eq!(c.body, "Body");

        let c = PostItContent::from_line("|Body only");
        assert_eq!(c.title, None);
        assert_eq!(c.body, "Body only");

        let c = PostItContent::from_line("# Comment");
        assert!(c.body.is_empty());
    }
}