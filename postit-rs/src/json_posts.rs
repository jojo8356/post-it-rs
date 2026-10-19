//! JSON-based post-it system with enable/disable checkboxes

use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Context, Result};

/// A post-it entry with enable/disable checkbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPostIt {
    /// Unique identifier
    pub id: String,
    /// Display title (optional)
    pub title: Option<String>,
    /// Main content
    pub body: String,
    /// Whether this post-it is enabled for random selection
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional category/tag
    #[serde(default)]
    pub category: String,
    /// Weight for random selection (higher = more likely)
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_true() -> bool { true }
fn default_weight() -> u32 { 1 }

/// Collection of post-its with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPostItCollection {
    /// Version of the format
    #[serde(default = "default_version")]
    pub version: u32,
    /// List of post-its
    pub posts: Vec<JsonPostIt>,
    /// Default category filter (empty = all)
    #[serde(default)]
    pub default_category: String,
}

fn default_version() -> u32 { 1 }

impl JsonPostItCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self {
            version: 1,
            posts: Vec::new(),
            default_category: String::new(),
        }
    }

    /// Load from JSON file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read JSON post-its file: {}", path.display()))?;
        let collection: JsonPostItCollection = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON post-its: {}", path.display()))?;
        Ok(collection)
    }

    /// Save to JSON file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize JSON post-its")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write JSON post-its: {}", path.display()))?;
        Ok(())
    }

    /// Get only enabled post-its, optionally filtered by category
    pub fn get_enabled(&self, category: Option<&str>) -> Vec<&JsonPostIt> {
        self.posts
            .iter()
            .filter(|p| p.enabled)
            .filter(|p| category.map_or(true, |c| c.is_empty() || p.category == c))
            .collect()
    }

    /// Get enabled post-its as weighted list for random selection
    pub fn get_weighted_enabled(&self, category: Option<&str>) -> Vec<(&JsonPostIt, u32)> {
        self.get_enabled(category)
            .into_iter()
            .map(|p| (p, p.weight))
            .collect()
    }

    /// Toggle enabled state of a post-it by ID
    pub fn toggle_enabled(&mut self, id: &str) -> bool {
        if let Some(post) = self.posts.iter_mut().find(|p| p.id == id) {
            post.enabled = !post.enabled;
            true
        } else {
            false
        }
    }

    /// Enable a post-it by ID
    pub fn enable(&mut self, id: &str) -> bool {
        if let Some(post) = self.posts.iter_mut().find(|p| p.id == id) {
            post.enabled = true;
            true
        } else {
            false
        }
    }

    /// Disable a post-it by ID
    pub fn disable(&mut self, id: &str) -> bool {
        if let Some(post) = self.posts.iter_mut().find(|p| p.id == id) {
            post.enabled = false;
            true
        } else {
            false
        }
    }

    /// Add a new post-it
    pub fn add(&mut self, post: JsonPostIt) {
        self.posts.push(post);
    }

    /// Remove a post-it by ID
    pub fn remove(&mut self, id: &str) -> bool {
        let len_before = self.posts.len();
        self.posts.retain(|p| p.id != id);
        self.posts.len() != len_before
    }

    /// Get all unique categories
    pub fn get_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self.posts
            .iter()
            .filter(|p| !p.category.is_empty())
            .map(|p| p.category.clone())
            .collect();
        categories.sort();
        categories.dedup();
        categories
    }

    /// Create example collection
    pub fn example() -> Self {
        Self {
            version: 1,
            default_category: String::new(),
            posts: vec![
                JsonPostIt {
                    id: "hydration".to_string(),
                    title: Some("💧 Hydratation".to_string()),
                    body: "Buvez un grand verre d'eau !".to_string(),
                    enabled: true,
                    category: "santé".to_string(),
                    weight: 3,
                },
                JsonPostIt {
                    id: "stretch".to_string(),
                    title: Some("🧘 Étirement".to_string()),
                    body: "Levez-vous et étirez-vous 30 secondes".to_string(),
                    enabled: true,
                    category: "santé".to_string(),
                    weight: 3,
                },
                JsonPostIt {
                    id: "eyes".to_string(),
                    title: Some("👀 Yeux".to_string()),
                    body: "Regardez loin 20 secondes (règle 20-20-20)".to_string(),
                    enabled: true,
                    category: "santé".to_string(),
                    weight: 2,
                },
                JsonPostIt {
                    id: "focus".to_string(),
                    title: Some("🎯 Focus".to_string()),
                    body: "Une seule tâche à la fois. Terminez-la.".to_string(),
                    enabled: true,
                    category: "productivité".to_string(),
                    weight: 2,
                },
                JsonPostIt {
                    id: "pomodoro".to_string(),
                    title: Some("🍅 Pomodoro".to_string()),
                    body: "25 min focus / 5 min pause. Répétez.".to_string(),
                    enabled: true,
                    category: "productivité".to_string(),
                    weight: 2,
                },
                JsonPostIt {
                    id: "git_commit".to_string(),
                    title: Some("📝 Git".to_string()),
                    body: "Committez vos changements régulièrement".to_string(),
                    enabled: true,
                    category: "dev".to_string(),
                    weight: 1,
                },
                JsonPostIt {
                    id: "code_review".to_string(),
                    title: Some("🔍 Code Review".to_string()),
                    body: "Relisez votre code avant de push".to_string(),
                    enabled: true,
                    category: "dev".to_string(),
                    weight: 1,
                },
                JsonPostIt {
                    id: "quote_knuth".to_string(),
                    title: Some("💭 Citation".to_string()),
                    body: "\"Premature optimization is the root of all evil.\" — Donald Knuth".to_string(),
                    enabled: true,
                    category: "citation".to_string(),
                    weight: 1,
                },
                JsonPostIt {
                    id: "quote_torvalds".to_string(),
                    title: Some("💭 Citation".to_string()),
                    body: "\"Talk is cheap. Show me the code.\" — Linus Torvalds".to_string(),
                    enabled: true,
                    category: "citation".to_string(),
                    weight: 1,
                },
                JsonPostIt {
                    id: "motivation".to_string(),
                    title: Some("🚀 Motivation".to_string()),
                    body: "Progrès > Perfection. Continuez !".to_string(),
                    enabled: true,
                    category: "motivation".to_string(),
                    weight: 2,
                },
            ],
        }
    }
}

impl Default for JsonPostItCollection {
    fn default() -> Self {
        Self::example()
    }
}

/// Convert JSON post-its to internal format
pub fn json_to_postit_content(posts: &[&JsonPostIt]) -> Vec<crate::config::PostItContent> {
    posts.iter().map(|p| crate::config::PostItContent {
        title: p.title.clone(),
        body: p.body.clone(),
    }).collect()
}

/// Weighted random selection
pub fn select_weighted_random<'a>(items: &[(&'a JsonPostIt, u32)]) -> Option<&'a JsonPostIt> {
    use rand::{rngs::StdRng, SeedableRng, Rng};
    
    if items.is_empty() {
        return None;
    }
    
    let total_weight: u32 = items.iter().map(|(_, w)| *w).sum();
    if total_weight == 0 {
        return None;
    }
    
    let mut rng = StdRng::from_entropy();
    let mut roll = rng.gen_range(0..total_weight);
    
    for (item, weight) in items {
        if roll < *weight {
            return Some(item);
        }
        roll -= *weight;
    }
    
    items.last().map(|(item, _)| *item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_json_collection_example() {
        let collection = JsonPostItCollection::example();
        assert!(!collection.posts.is_empty());
        assert!(collection.posts.iter().all(|p| p.enabled));
    }

    #[test]
    fn test_get_enabled() {
        let collection = JsonPostItCollection::example();
        let enabled = collection.get_enabled(None);
        assert_eq!(enabled.len(), collection.posts.len());
    }

    #[test]
    fn test_toggle_enabled() {
        let mut collection = JsonPostItCollection::example();
        let id = collection.posts[0].id.clone();
        assert!(collection.toggle_enabled(&id));
        assert!(!collection.posts[0].enabled);
        assert!(collection.toggle_enabled(&id));
        assert!(collection.posts[0].enabled);
    }

    #[test]
    fn test_categories() {
        let collection = JsonPostItCollection::example();
        let cats = collection.get_categories();
        assert!(cats.contains(&"santé".to_string()));
        assert!(cats.contains(&"productivité".to_string()));
    }

    #[test]
    fn test_save_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("posts.json");
        
        let collection = JsonPostItCollection::example();
        collection.save_to_file(&path).unwrap();
        
        let loaded = JsonPostItCollection::load_from_file(&path).unwrap();
        assert_eq!(loaded.posts.len(), collection.posts.len());
    }

    #[test]
    fn test_weighted_random() {
        let collection = JsonPostItCollection::example();
        let weighted = collection.get_weighted_enabled(None);
        assert!(!weighted.is_empty());
        
        // Run multiple times to verify it doesn't crash
        for _ in 0..100 {
            let _ = select_weighted_random(&weighted);
        }
    }
}