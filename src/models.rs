// src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeMod {
    pub id: i64,
    pub guid: Option<String>,
    pub name: String,
    pub teaser: Option<String>,
    pub author: Option<String>,
    pub downloads: i64,
    pub updated_at: Option<String>,
    pub detail_url: Option<String>,
    pub source_url: Option<String>,
    // ... extend as we discover real fields from /api/v0/mods responses
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeModVersion {
    pub id: i64,
    pub version: String,
    pub spt_version: Option<String>,
    pub download_url: Option<String>,
    pub file_size: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ManagedMod {
    pub forge_id: i64,
    pub guid: Option<String>,
    pub name: String,
    pub last_known_version: Option<String>,
    pub desired_enabled: bool,
    pub last_installed_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InstalledFile {
    pub forge_id: i64,
    pub relative_path: String,
    pub is_directory: bool,
}

// Minimal example response wrapper the API uses (from the viewer code)
#[derive(Debug, Deserialize)]
pub struct ForgeModResponse {
    pub data: Vec<ForgeMod>,
    // meta etc later
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forge_mod_serde_roundtrip() {
        let m = ForgeMod { id: 42, guid: Some("com.foo.bar".into()), name: "TestMod".into(), teaser: None, author: None, downloads: 123, updated_at: None, detail_url: None, source_url: None };
        let json = serde_json::to_string(&m).unwrap();
        let back: ForgeMod = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 42);
        assert_eq!(back.name, "TestMod");
    }
}
