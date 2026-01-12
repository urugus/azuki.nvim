//! Configuration and dictionary loading

use crate::dictionary::Dictionary;
use std::path::PathBuf;

/// Default dictionary paths to search
pub fn default_dictionary_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // XDG data home
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        paths.push(PathBuf::from(data_home).join("azuki/dict/SKK-JISYO.L"));
    }

    // Home directory fallback
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(&home).join(".local/share/azuki/dict/SKK-JISYO.L"));
        paths.push(PathBuf::from(&home).join(".azuki/dict/SKK-JISYO.L"));
    }

    // System paths
    paths.push(PathBuf::from("/usr/share/skk/SKK-JISYO.L"));
    paths.push(PathBuf::from("/usr/local/share/skk/SKK-JISYO.L"));

    paths
}

/// Find and load dictionary from default paths
pub fn load_dictionary() -> Option<Dictionary> {
    // Check environment variable first
    if let Ok(dict_path) = std::env::var("AZUKI_DICTIONARY") {
        match Dictionary::load(&dict_path) {
            Ok(dict) => {
                eprintln!("Loaded dictionary from AZUKI_DICTIONARY: {}", dict_path);
                return Some(dict);
            }
            Err(e) => {
                eprintln!("Failed to load dictionary from {}: {}", dict_path, e);
            }
        }
    }

    // Search default paths
    for path in default_dictionary_paths() {
        if path.exists() {
            match Dictionary::load(&path) {
                Ok(dict) => {
                    eprintln!("Loaded dictionary from: {}", path.display());
                    return Some(dict);
                }
                Err(e) => {
                    eprintln!("Failed to load dictionary from {}: {}", path.display(), e);
                }
            }
        }
    }

    eprintln!("No dictionary found. Running without dictionary (hiragana pass-through mode).");
    None
}
