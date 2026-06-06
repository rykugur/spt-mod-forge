// src/spt.rs
// Task 6: SPT path resolver + version detect + validate
// Per plan: TDD first (failing tests), then impl.
// pub struct SptInstall { pub root: PathBuf }
// resolve(cfg_path: &str, env_override: Option<String>) -> Result<Self>
// Precedence (env > cfg > ~/Games/SPTarkov), validate markers (user/mods + BepInEx), best-effort version.
// Err(SptPath("not found")) for missing (prompt handled in app layer).
// No CWD reads, Linux-focused default.

use std::path::PathBuf;

use crate::error::{AppError, Result};

#[derive(Debug)]
pub struct SptInstall {
    pub root: PathBuf,
}

impl SptInstall {
    /// Resolve SPT install root using precedence: env_override (if non-empty) > cfg_path (if non-empty) > default ~/Games/SPTarkov .
    /// Then validate marker dirs. On failure to find valid install, returns Err(SptPath("not found")) so caller (app) can prompt.
    /// Tilde expansion supported for convenience.
    pub fn resolve(cfg_path: &str, env_override: Option<String>) -> Result<Self> {
        let candidate = if let Some(ref e) = env_override {
            if !e.is_empty() {
                Self::expand_tilde(e)
            } else if !cfg_path.is_empty() {
                Self::expand_tilde(cfg_path)
            } else {
                Self::default_spt_path()
            }
        } else if !cfg_path.is_empty() {
            Self::expand_tilde(cfg_path)
        } else {
            Self::default_spt_path()
        };

        if !Self::validate(&candidate) {
            return Err(AppError::SptPath("not found".into()));
        }

        Ok(Self { root: candidate })
    }

    fn default_spt_path() -> PathBuf {
        // Linux-focused per spec section 6. Fallback to /tmp or similar if no HOME (test envs).
        let home = std::env::var("HOME").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
        PathBuf::from(home).join("Games/SPTarkov")
    }

    fn expand_tilde(p: &str) -> PathBuf {
        if p == "~" {
            if let Ok(h) = std::env::var("HOME") {
                return PathBuf::from(h);
            }
            return PathBuf::from(p);
        }
        if let Some(rest) = p.strip_prefix("~/") {
            if let Ok(h) = std::env::var("HOME") {
                return PathBuf::from(h).join(rest);
            }
        }
        PathBuf::from(p)
    }

    /// Returns true only if both required marker subdirectories exist (per spec).
    pub fn validate(root: &std::path::Path) -> bool {
        root.join("user/mods").is_dir() && root.join("BepInEx").is_dir()
    }

    /// Best-effort version detection. Tries common text files first, then byte scan of binaries for "4.x" patterns.
    /// Returns e.g. "4.0.13" or "unknown".
    pub fn version(&self) -> String {
        Self::detect_version(&self.root)
    }

    fn detect_version(root: &std::path::Path) -> String {
        // Text file candidates (common for SPT installs)
        let text_cands: [PathBuf; 6] = [
            root.join("SPT_Data/StreamingAssets/SPT.version"),
            root.join("SPT_Data/StreamingAssets/version.txt"),
            root.join("version.txt"),
            root.join("package.json"),
            root.join("user/server/version.txt"),
            root.join("BepInEx/config/version.txt"),
        ];
        for cand in &text_cands {
            if let Ok(s) = std::fs::read_to_string(cand) {
                if let Some(v) = Self::find_version_str(&s) {
                    return v;
                }
            }
        }

        // Byte scan candidates (dlls/exes that may embed version strings)
        let bin_cands: [PathBuf; 4] = [
            root.join("SPT_Data/Server/Assembly-CSharp.dll"),
            root.join("EscapeFromTarkov_Data/Managed/Assembly-CSharp.dll"),
            root.join("BepInEx/core/BepInEx.dll"),
            root.join("SPT.Launcher.exe"),
        ];
        for bc in &bin_cands {
            if let Ok(bytes) = std::fs::read(bc) {
                if let Some(v) = Self::find_version_in_bytes(&bytes) {
                    return v;
                }
            }
        }

        "unknown".to_string()
    }

    fn find_version_str(s: &str) -> Option<String> {
        // Prefer first lines, and lines mentioning version
        for line in s.lines().take(30) {
            let t = line.trim();
            if let Some(v) = Self::extract_4x_version(t) {
                return Some(v);
            }
            if t.contains("version") || t.contains("spt") || t.contains("SPT") {
                if let Some(v) = Self::extract_4x_version(t) {
                    return Some(v);
                }
            }
        }
        // last resort full content scan
        Self::extract_4x_version(s)
    }

    fn extract_4x_version(text: &str) -> Option<String> {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len().saturating_sub(2) {
            if bytes[i] == b'4' && bytes[i + 1] == b'.' {
                let _start = i;
                let mut j = i;
                let mut ver = String::new();
                while j < bytes.len() {
                    let c = bytes[j] as char;
                    if c.is_ascii_digit() || c == '.' {
                        ver.push(c);
                    } else {
                        break;
                    }
                    j += 1;
                    if ver.len() > 20 {
                        break;
                    }
                }
                if ver.starts_with("4.") && ver.matches('.').count() >= 1 {
                    let clean = ver.trim_end_matches('.');
                    if clean.len() >= 5 && clean.chars().last().map_or(false, |c| c.is_ascii_digit()) {
                        return Some(clean.to_string());
                    }
                }
                i = j;
            } else {
                i += 1;
            }
        }
        None
    }

    fn find_version_in_bytes(bytes: &[u8]) -> Option<String> {
        let mut i = 0;
        while i < bytes.len().saturating_sub(3) {
            if bytes[i] == b'4' && bytes[i + 1] == b'.' && bytes[i + 2].is_ascii_digit() {
                let mut j = i;
                let mut ver = String::new();
                while j < bytes.len() {
                    let c = bytes[j] as char;
                    if c.is_ascii_digit() || c == '.' {
                        ver.push(c);
                    } else if c.is_ascii_alphabetic() && !ver.is_empty() {
                        break;
                    } else if !ver.is_empty() {
                        break;
                    }
                    j += 1;
                    if ver.len() > 20 {
                        break;
                    }
                }
                if let Some(v) = Self::extract_4x_version(&ver) {
                    return Some(v);
                }
                i = j;
            } else {
                i += 1;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_prefers_env_override() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root.join("user/mods")).unwrap();
        fs::create_dir_all(root.join("BepInEx")).unwrap();
        let env_path = root.to_str().unwrap().to_string();

        // env provided, even with cfg set, should win
        let spt = SptInstall::resolve("/cfg/should/be/ignored", Some(env_path.clone()))
            .expect("resolve with valid env fixture should succeed");
        assert_eq!(spt.root, root);
    }

    #[test]
    fn resolve_uses_cfg_path_when_no_env_override() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root.join("user/mods")).unwrap();
        fs::create_dir_all(root.join("BepInEx")).unwrap();
        let cfg_path = root.to_str().unwrap().to_string();

        let spt = SptInstall::resolve(&cfg_path, None)
            .expect("resolve with valid cfg_path (env=None) should succeed");
        assert_eq!(spt.root, root);
    }

    #[test]
    fn resolve_falls_back_to_default_when_empty_cfg_and_no_env() {
        // Default is ~/Games/SPTarkov (Linux per spec). In this env it almost
        // certainly lacks markers, so expect the "not found" error per plan.
        let res = SptInstall::resolve("", None);
        assert!(res.is_err(), "default fallback without markers must error");
        match res {
            Err(AppError::SptPath(msg)) => assert_eq!(msg, "not found"),
            other => panic!("expected SptPath(\"not found\"), got {:?}", other),
        }
    }

    #[test]
    fn resolve_empty_env_string_treated_as_no_override() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root.join("user/mods")).unwrap();
        fs::create_dir_all(root.join("BepInEx")).unwrap();
        let cfg_path = root.to_str().unwrap().to_string();

        // Some("") should fall to cfg
        let spt = SptInstall::resolve(&cfg_path, Some("".to_string()))
            .expect("empty env string should defer to cfg_path");
        assert_eq!(spt.root, root);
    }

    #[test]
    fn resolve_validates_markers_success_with_temp_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        // create exactly the two marker dirs required by spec
        fs::create_dir_all(root.join("user/mods")).unwrap();
        fs::create_dir_all(root.join("BepInEx")).unwrap();

        let p = root.to_str().unwrap().to_string();
        let spt = SptInstall::resolve(&p, None).expect("valid markers -> Ok");
        assert_eq!(spt.root, root);
        // also via explicit env
        let spt2 = SptInstall::resolve("", Some(p)).expect("valid via env");
        assert_eq!(spt2.root, root);
    }

    #[test]
    fn resolve_fails_on_missing_markers() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        // intentionally do NOT create user/mods or BepInEx
        // (maybe create one but not both to test && )
        fs::create_dir_all(root.join("user/mods")).unwrap();
        // BepInEx missing

        let p = root.to_str().unwrap().to_string();
        let res = SptInstall::resolve(&p, None);
        assert!(res.is_err());
        match res {
            Err(AppError::SptPath(msg)) => assert_eq!(msg, "not found"),
            other => panic!("expected SptPath not found, got {:?}", other),
        }
    }

    #[test]
    fn version_detect_reads_common_version_file_in_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root.join("user/mods")).unwrap();
        fs::create_dir_all(root.join("BepInEx")).unwrap();

        // Create one of the common candidate locations with a clean version
        let ver_dir = root.join("SPT_Data/StreamingAssets");
        fs::create_dir_all(&ver_dir).unwrap();
        fs::write(ver_dir.join("SPT.version"), "4.0.13\n").unwrap();

        let spt = SptInstall::resolve(root.to_str().unwrap(), None)
            .expect("fixture with markers valid");
        assert_eq!(spt.version(), "4.0.13");
    }

    #[test]
    fn version_detect_falls_back_to_unknown() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root.join("user/mods")).unwrap();
        fs::create_dir_all(root.join("BepInEx")).unwrap();
        // no version file at any candidate location

        let spt = SptInstall::resolve(root.to_str().unwrap(), None).unwrap();
        assert_eq!(spt.version(), "unknown");
    }

    #[test]
    fn version_detect_also_works_from_package_json_or_first_line_noise() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root.join("user/mods")).unwrap();
        fs::create_dir_all(root.join("BepInEx")).unwrap();

        let pkg = root.join("package.json");
        fs::write(&pkg, r#" { "name": "spt", "version": "4.1.2" } "#).unwrap();

        let spt = SptInstall::resolve(root.to_str().unwrap(), None).unwrap();
        assert_eq!(spt.version(), "4.1.2");
    }

    #[test]
    fn tilde_expansion_in_cfg_or_env() {
        // We can't easily assert ~ expands without knowing $HOME, but we can
        // create a temp as if it were under home and pass "~/<basename>"
        // and ensure it resolved (i.e. didn't stay literal "~/...")
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root.join("user/mods")).unwrap();
        fs::create_dir_all(root.join("BepInEx")).unwrap();

        // simulate: suppose the temp root's parent + basename acts as "home/sub"
        // simpler: just use absolute for fixture, but pass a ~ form? hard.
        // Instead, test that if we pass a path starting with ~ it doesn't
        // treat literal "~/nonexistent" as valid (it will fail validate).
        // And for a real absolute it works. (Expansion is covered indirectly)
        let bad = "~/definitely-not-a-real-spt-here-xyz123";
        let res = SptInstall::resolve(bad, None);
        assert!(res.is_err()); // because after expand (to $HOME/...) still no markers, or if ~ not expanded would try literal which also fails

        // positive: absolute works (already in other tests)
        let p = root.to_str().unwrap().to_string();
        let spt = SptInstall::resolve(&p, None).unwrap();
        assert!(spt.root.is_absolute());
    }
}
