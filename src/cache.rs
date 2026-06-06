// src/cache.rs
// Task 8: Cache layer (lists + downloads, toggle, force, SPT_CACHE_DIR)
// Create: src/cache.rs per plan.
//
// Step 8.1-8.3 TDD: cache root resolution (respect enabled, config.dir, SPT_CACHE_DIR env, XDG fallback via config::default_cache_root).
// Methods: list_cache_key(sort, spt_ver) -> Path, store_list, load_list, download_path(forge_id, ver), store_download, clear_lists() (for force).
//
// Pure std::fs (create_dir_all, read/write, remove_dir_all) + serde (for lists as JSON) or raw bytes (downloads).
// No TTL, no network, no integration with spt/forge/install yet. "user controls freshness".
// Subdirs under root: lists/ + downloads/ . List key e.g. "{sort}-{spt_ver}.json".
// Toggle: if !enabled then store_* / load_* are no-ops (load returns None), no files written/read for data.
// clear_lists always executes (explicit force/purge action).
// Tests use temp cache roots (tempfile::tempdir like state.rs) + TEST_ENV_LOCK + XDG override + manual /tmp pid patterns (like config.rs).
// Never touches CWD. Respects config/env/XDG without duplicating XDG/project_dirs logic (reuses default_cache_root).
// Self-contained for `cargo test --lib cache` (with supporting mod decl + dep left unstaged at commit).
//
// After: cargo check + tests green + git add src/cache.rs only + commit.

use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::Result;
use crate::ForgeMod;

pub struct Cache {
    root: PathBuf,
    enabled: bool,
}

impl Cache {
    /// Build from loaded Config. Config's load already applies SPT_CACHE_DIR -> cache.dir (and file), XDG via helpers.
    /// We still implement explicit SPT_CACHE_DIR check in resolve for TDD coverage of "config.dir, SPT_CACHE_DIR env, XDG fallback".
    /// (The SPT check is tiny; XDG logic is not duplicated, we call default_cache_root.)
    pub fn from_config(cfg: &Config) -> Result<Self> {
        let enabled = cfg.cache.enabled;
        let root = resolve_root(&cfg.cache.dir)?;
        Ok(Self { root, enabled })
    }

    /// Test ctor: explicit temp root + enabled (bypasses all config/env/XDG).
    pub fn with_root(root: PathBuf, enabled: bool) -> Self {
        Self { root, enabled }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// lists/{sort}-{spt_ver}.json under root.
    /// (Use explicit .json in filename string, not with_extension, because spt_ver like "4.0.13" contains dots which Path treats as ext separator.)
    pub fn list_cache_key(&self, sort: &str, spt_ver: &str) -> PathBuf {
        let filename = format!("{}-{}.json", sort, spt_ver);
        self.root.join("lists").join(filename)
    }

    /// Serialize & store list data as pretty JSON. No-op (Ok) if !enabled. Ensures parent dir.
    pub fn store_list(&self, sort: &str, spt_ver: &str, data: &[ForgeMod]) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let path = self.list_cache_key(sort, spt_ver);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(data)
            .map_err(|e| crate::error::AppError::Other(format!("json serialize list: {}", e)))?;
        std::fs::write(&path, bytes)?;
        Ok(())
    }

    /// Load + deserialize list if file present + enabled. Returns None for miss / disabled / unreadable / bad json (graceful; user can force).
    pub fn load_list(&self, sort: &str, spt_ver: &str) -> Result<Option<Vec<ForgeMod>>> {
        if !self.enabled {
            return Ok(None);
        }
        let path = self.list_cache_key(sort, spt_ver);
        if !path.exists() {
            return Ok(None);
        }
        match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Vec<ForgeMod>>(&bytes) {
                Ok(items) => Ok(Some(items)),
                Err(_) => Ok(None),
            },
            Err(_) => Ok(None),
        }
    }

    /// downloads/{forge_id}-{ver}.zip under root (conventional ext for artifacts; bytes are opaque).
    /// (Use explicit .zip in filename string, not with_extension, because ver like "1.2.3-beta" contains dots which Path treats as ext separator.)
    pub fn download_path(&self, forge_id: i64, ver: &str) -> PathBuf {
        let filename = format!("{}-{}.zip", forge_id, ver);
        self.root.join("downloads").join(filename)
    }

    /// Write raw bytes for a download. No-op if !enabled. Ensures parent.
    pub fn store_download(&self, forge_id: i64, ver: &str, bytes: &[u8]) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let path = self.download_path(forge_id, ver);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
        Ok(())
    }

    /// Remove the lists/ subdir entirely (for --force / user controlled freshness). Always succeeds in intent (even if !enabled).
    /// Does not affect downloads/ or the root itself.
    pub fn clear_lists(&self) -> Result<()> {
        let lists_dir = self.root.join("lists");
        if lists_dir.exists() {
            std::fs::remove_dir_all(&lists_dir)?;
        }
        Ok(())
    }
}

/// Resolution order per plan literal: config.dir (non-empty) ? it : SPT_CACHE_DIR env (non-empty) ? it : XDG (default_cache_root).
/// Reuses config helper so no XDG/project_dirs duplication.
fn resolve_root(dir: &str) -> Result<PathBuf> {
    if !dir.is_empty() {
        return Ok(PathBuf::from(dir));
    }
    if let Ok(d) = std::env::var("SPT_CACHE_DIR") {
        if !d.is_empty() {
            return Ok(PathBuf::from(d));
        }
    }
    crate::config::default_cache_root()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::sync::Mutex;

    // Serialize env-mutating tests (SPT_CACHE_DIR + XDG_*) mirroring config.rs + state.rs.
    static TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn make_test_mod(id: i64, name: &str) -> ForgeMod {
        ForgeMod {
            id,
            guid: None,
            name: name.into(),
            teaser: None,
            author: None,
            downloads: 42,
            updated_at: None,
            detail_url: None,
            source_url: None,
        }
    }

    #[test]
    fn default_enabled_and_with_root_ctor() {
        let c = Cache::with_root(std::env::temp_dir().join("cache-test-1"), true);
        assert!(c.enabled());
        assert!(c.root().to_string_lossy().contains("cache-test-1"));

        let c_off = Cache::with_root(std::env::temp_dir().join("cache-test-2"), false);
        assert!(!c_off.enabled());
    }

    #[test]
    fn list_cache_key_and_download_path_format() {
        let tmp = tempfile::tempdir().expect("temp cache root");
        let c = Cache::with_root(tmp.path().to_path_buf(), true);

        let lkey = c.list_cache_key("most_downloaded", "4.0.13");
        let lstr = lkey.to_string_lossy();
        assert!(lkey.starts_with(tmp.path()), "key under root");
        assert!(lstr.contains("/lists/"), "lists subdir");
        assert!(lstr.ends_with("most_downloaded-4.0.13.json"), "key format");

        let dpath = c.download_path(12345, "1.2.3-beta");
        let dstr = dpath.to_string_lossy();
        assert!(dpath.starts_with(tmp.path()), "dpath under root");
        assert!(dstr.contains("/downloads/"), "downloads subdir");
        assert!(dstr.ends_with("12345-1.2.3-beta.zip"), "download filename");
    }

    #[test]
    fn store_load_list_roundtrip_json_via_temp_root() {
        let tmp = tempfile::tempdir().expect("temp");
        let c = Cache::with_root(tmp.path().to_path_buf(), true);

        let items = vec![make_test_mod(1, "Alpha"), make_test_mod(99, "Omega")];
        c.store_list("last_updated", "4.1.1", &items).expect("store_list");

        let key = c.list_cache_key("last_updated", "4.1.1");
        assert!(key.exists(), "file written");
        // spot check it's json
        let raw = fs::read_to_string(&key).expect("read raw");
        assert!(raw.contains("\"name\": \"Alpha\""), "pretty json content");

        let loaded = c.load_list("last_updated", "4.1.1").expect("load_list");
        assert!(loaded.is_some());
        let got = loaded.unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].id, 1);
        assert_eq!(got[0].name, "Alpha");
        assert_eq!(got[1].name, "Omega");

        // miss for other key
        assert!(c.load_list("most_downloaded", "4.1.1").expect("miss other").is_none());
    }

    #[test]
    fn disabled_toggle_noops_stores_and_loads_return_none() {
        let tmp = tempfile::tempdir().expect("temp");
        let c = Cache::with_root(tmp.path().to_path_buf(), false);

        let items = vec![make_test_mod(7, "Hidden")];
        c.store_list("foo", "0.0", &items).expect("store when off is ok (no-op)");
        let k = c.list_cache_key("foo", "0.0");
        assert!(!k.exists(), "no list file when disabled");

        assert!(c.load_list("foo", "0.0").expect("load off").is_none());

        c.store_download(7, "0.0", b"should-not-write").expect("dl store off ok");
        let dp = c.download_path(7, "0.0");
        assert!(!dp.exists(), "no download file when disabled");
    }

    #[test]
    fn store_download_writes_raw_bytes_and_creates_subdir() {
        let tmp = tempfile::tempdir().expect("temp");
        let c = Cache::with_root(tmp.path().to_path_buf(), true);

        let payload: &[u8] = b"\x50\x4b\x03\x04FAKEZIP\x00content-for-mod";
        c.store_download(4242, "3.3.3", payload).expect("store_download");

        let p = c.download_path(4242, "3.3.3");
        assert!(p.exists());
        let back = fs::read(&p).expect("read stored download");
        assert_eq!(back, payload);

        // parent subdir
        assert!(p.parent().unwrap().ends_with("downloads"));
    }

    #[test]
    fn clear_lists_removes_only_lists_for_force_and_works_when_disabled() {
        let tmp = tempfile::tempdir().expect("temp");
        let c_on = Cache::with_root(tmp.path().to_path_buf(), true);

        c_on.store_list("s1", "v1", &[make_test_mod(10, "L1")]).expect("seed list");
        c_on.store_download(10, "v1", b"dl1").expect("seed dl");
        let lk = c_on.list_cache_key("s1", "v1");
        let dk = c_on.download_path(10, "v1");
        assert!(lk.exists());
        assert!(dk.exists());

        c_on.clear_lists().expect("clear");
        assert!(!lk.exists(), "lists entry removed");
        assert!(dk.exists(), "downloads untouched by clear_lists");

        // even when disabled, clear_lists can be called (for explicit force)
        let c_off = Cache::with_root(tmp.path().to_path_buf(), false);
        // (no lists now, but calling is fine)
        c_off.clear_lists().expect("clear when off ok");
    }

    #[test]
    fn root_resolution_prefers_config_dir() {
        let mut cfg = crate::config::Config::default();
        cfg.cache.dir = "/absolute/custom/cache/root".into();
        cfg.cache.enabled = true;

        let c = Cache::from_config(&cfg).expect("from_config with dir");
        assert_eq!(c.root().to_string_lossy(), "/absolute/custom/cache/root");
        assert!(c.enabled());
    }

    #[test]
    fn root_resolution_uses_spt_cache_dir_env_when_config_dir_empty() {
        let _lock = TEST_ENV_LOCK.lock().unwrap();
        let orig_cache = env::var("SPT_CACHE_DIR").ok();
        let orig_xdg = env::var("XDG_CACHE_HOME").ok();

        env::set_var("SPT_CACHE_DIR", "/from/env/spt/cache");
        // make sure XDG would differ
        env::set_var("XDG_CACHE_HOME", "/xdg/should-be-ignored-here");

        let mut cfg = crate::config::Config::default();
        cfg.cache.dir.clear();

        let c = Cache::from_config(&cfg).expect("resolve via env");
        assert_eq!(c.root().to_string_lossy(), "/from/env/spt/cache");

        // restore
        match orig_cache {
            Some(v) => env::set_var("SPT_CACHE_DIR", v),
            None => env::remove_var("SPT_CACHE_DIR"),
        }
        match orig_xdg {
            Some(v) => env::set_var("XDG_CACHE_HOME", v),
            None => env::remove_var("XDG_CACHE_HOME"),
        }
    }

    #[test]
    fn root_resolution_falls_back_to_xdg_default_cache_root() {
        let _lock = TEST_ENV_LOCK.lock().unwrap();
        let orig_x = env::var("XDG_CACHE_HOME").ok();
        let orig_spt = env::var("SPT_CACHE_DIR").ok();

        let test_id = std::process::id();
        let base = std::env::temp_dir().join(format!("spt-mod-forge-test-cache-xdg-{}", test_id));
        env::set_var("XDG_CACHE_HOME", &base);
        env::remove_var("SPT_CACHE_DIR"); // ensure not overriding

        let mut cfg = crate::config::Config::default();
        cfg.cache.dir.clear();

        let c = Cache::from_config(&cfg).expect("xdg fallback");
        let root_s = c.root().to_string_lossy().to_string();
        // Must be under the XDG override we set (default_cache_root uses project_dirs().cache_dir()).
        assert!(
            root_s.starts_with(base.to_str().unwrap()),
            "expected under XDG_CACHE_HOME base, got: {}",
            root_s
        );

        // restore + cleanup
        match orig_x {
            Some(v) => env::set_var("XDG_CACHE_HOME", v),
            None => env::remove_var("XDG_CACHE_HOME"),
        }
        match orig_spt {
            Some(v) => env::set_var("SPT_CACHE_DIR", v),
            None => env::remove_var("SPT_CACHE_DIR"),
        }
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn enabled_from_config_is_respected_for_resolution_and_ops() {
        let mut cfg = crate::config::Config::default();
        cfg.cache.enabled = false;
        cfg.cache.dir = "/tmp/disabled-cache".into();

        let c = Cache::from_config(&cfg).expect("from disabled cfg");
        assert!(!c.enabled());
        assert_eq!(c.root().to_string_lossy(), "/tmp/disabled-cache");

        // ops noop
        c.store_list("x", "y", &[make_test_mod(1, "no")]).expect("noop store");
        assert!(c.load_list("x", "y").expect("noop load").is_none());
    }
}
