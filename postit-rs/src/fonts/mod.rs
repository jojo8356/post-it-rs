use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::{Context, Result};
use fontdb::Database;
use egui::{FontDefinitions, FontData};
use tracing::{debug, warn, info};

/// Font manager for loading system and custom fonts
pub struct FontManager {
    db: Database,
    custom_fonts_dir: PathBuf,
    loaded_families: Vec<String>,
}

impl FontManager {
    pub fn new(custom_fonts_dir: PathBuf) -> Self {
        let mut db = Database::new();
        // Load system fonts
        db.load_system_fonts();
        
        Self {
            db,
            custom_fonts_dir,
            loaded_families: Vec::new(),
        }
    }

    /// Load custom fonts from the configured directory
    pub fn load_custom_fonts(&mut self) -> Result<()> {
        if !self.custom_fonts_dir.exists() {
            debug!("Custom fonts directory does not exist: {}", self.custom_fonts_dir.display());
            return Ok(());
        }

        let mut count = 0;
        for entry in walkdir::WalkDir::new(&self.custom_fonts_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            if matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "woff" | "woff2") {
                match self.load_font_file(path) {
                    Ok(families) => {
                        count += families.len();
                        self.loaded_families.extend(families);
                    }
                    Err(e) => {
                        warn!("Failed to load font {}: {}", path.display(), e);
                    }
                }
            }
        }
        info!("Loaded {} custom font families from {}", count, self.custom_fonts_dir.display());
        Ok(())
    }

    /// Load a single font file
    fn load_font_file(&mut self, path: &Path) -> Result<Vec<String>> {
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read font file: {}", path.display()))?;
        
        let mut families = Vec::new();
        // fontdb loads all faces from the file
        self.db.load_font_data(data);
        
        // Get the families that were added - check all faces after loading
        for face in self.db.faces() {
            // We can't easily track which faces came from this file, so just collect all
            for family in &face.families {
                let name = family.0.to_string();
                if !families.contains(&name) && !self.loaded_families.contains(&name) {
                    families.push(name);
                }
            }
        }
        
        Ok(families)
    }

    /// Get a font by family name, with fallback
    pub fn get_font(&self, family: &str, size: f32, _weight: crate::config::FontWeight) -> Option<egui::FontId> {
        // Check if family exists in our database
        let has_family = self.db.faces().any(|f| f.families.iter().any(|fam| fam.0 == family));
        if has_family {
            Some(egui::FontId::new(size, egui::FontFamily::Name(Arc::from(family))))
        } else {
            None
        }
    }

    /// Build egui FontDefinitions with all loaded fonts
    pub fn build_font_definitions(&self, config: &crate::config::Config) -> FontDefinitions {
        let mut fonts = FontDefinitions::default();
        
        // Configure title font if custom path specified
        if let Some(font_path) = config.title.font_path.to_str() {
            if !font_path.is_empty() {
                let family_name = "PostItTitleCustom";
                if let Ok(data) = std::fs::read(font_path) {
                    fonts.font_data.insert(
                        family_name.into(),
                        FontData::from_owned(data),
                    );
                    fonts.families
                        .entry(egui::FontFamily::Name(family_name.into()))
                        .or_default()
                        .insert(0, family_name.to_string());
                }
            }
        }
        
        // Configure body font if custom path specified
        if let Some(font_path) = config.body.font_path.to_str() {
            if !font_path.is_empty() {
                let family_name = "PostItBodyCustom";
                if let Ok(data) = std::fs::read(font_path) {
                    fonts.font_data.insert(
                        family_name.into(),
                        FontData::from_owned(data),
                    );
                    fonts.families
                        .entry(egui::FontFamily::Name(family_name.into()))
                        .or_default()
                        .insert(0, family_name.to_string());
                }
            }
        }
        
        fonts
    }

    /// List all available font families
    pub fn list_families(&self) -> Vec<String> {
        let mut families: Vec<String> = self.db.faces()
            .flat_map(|f| f.families.iter().map(|fam| fam.0.to_string()))
            .collect();
        families.sort();
        families.dedup();
        families
    }
}

/// Helper to load a font directly into egui context (for runtime loading)
/// Note: In egui 0.29, fonts must be set at startup via FontDefinitions
pub fn load_font_into_ctx(_ctx: &egui::Context, _font_path: &Path, _family_name: &str) -> Result<()> {
    // Runtime font loading not supported in egui 0.29 - use FontDefinitions at startup
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_font_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = FontManager::new(dir.path().to_path_buf());
        // Should not panic
        assert!(manager.loaded_families.is_empty());
    }
}