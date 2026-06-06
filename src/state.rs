// src/state.rs
// Task 7: SQLite state for managed_mods + installed_files manifest.
// Exact schema from design doc (PK/FK/UNIQUE, INTEGER for bools, TEXT timestamps with defaults).
// TDD: in-memory first (new_in_memory + init_schema + CRUD + query back + counts).
// Then diff: compute_pending for commit logic.
// Real path support via config dir (state.db under XDG config).
// Uses rusqlite(bundled) + crate::error::Result + crate models.
// Supporting mod decl + config helper left unstaged per plan pattern (git add src/state.rs only at commit).

use rusqlite::{Connection, params};
use crate::error::Result;
use crate::ManagedMod;
use crate::InstalledFile;

pub struct StateDb {
    conn: Connection,
}

impl StateDb {
    /// For tests: in-memory DB (rusqlite :memory: / open_in_memory).
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self { conn })
    }

    /// Open (or create) the DB file at the given path.
    /// Caller (or open_default) responsible for ensuring parent dir exists.
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    /// Open using the real path under XDG config dir (state.db), ensuring dir + schema.
    /// Per plan: "Commit with real file open path too (using config dir)".
    pub fn open_default() -> Result<Self> {
        let path = crate::config::state_db_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = Self::open(&path)?;
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize exact schema from the design doc / plan.
    /// Uses IF NOT EXISTS so safe to call multiple times / on open.
    pub fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS managed_mods (
                forge_id INTEGER PRIMARY KEY,
                guid TEXT UNIQUE,
                name TEXT,
                last_known_version TEXT,
                desired_enabled INTEGER NOT NULL DEFAULT 0,
                last_installed_version TEXT,
                added_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS installed_files (
                id INTEGER PRIMARY KEY,
                forge_id INTEGER NOT NULL,
                relative_path TEXT NOT NULL,
                is_directory INTEGER NOT NULL,
                installed_at TEXT DEFAULT (datetime('now')),
                UNIQUE(forge_id, relative_path),
                FOREIGN KEY(forge_id) REFERENCES managed_mods(forge_id)
            );
        "#)?;
        Ok(())
    }

    /// Upsert a managed mod (by forge_id PK). Supports "insert a mod" in tests.
    /// Maps Rust bool <-> INTEGER for desired_enabled.
    pub fn upsert_managed_mod(&self, m: &ManagedMod) -> Result<()> {
        let desired_i: i64 = if m.desired_enabled { 1 } else { 0 };
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO managed_mods
                (forge_id, guid, name, last_known_version, desired_enabled, last_installed_version)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                m.forge_id,
                &m.guid,
                &m.name,
                &m.last_known_version,
                desired_i,
                &m.last_installed_version,
            ],
        )?;
        Ok(())
    }

    /// Set desired_enabled for an existing managed mod (0/1 in DB).
    pub fn set_desired(&self, forge_id: i64, enabled: bool) -> Result<()> {
        let val: i64 = if enabled { 1 } else { 0 };
        self.conn.execute(
            "UPDATE managed_mods SET desired_enabled = ?1 WHERE forge_id = ?2",
            params![val, forge_id],
        )?;
        Ok(())
    }

    pub fn get_all_managed(&self) -> Result<Vec<ManagedMod>> {
        let mut stmt = self.conn.prepare(
            "SELECT forge_id, guid, name, last_known_version, desired_enabled, last_installed_version FROM managed_mods ORDER BY forge_id"
        )?;
        let rows = stmt.query_map([], |row| {
            let desired_i: i64 = row.get(4)?;
            Ok(ManagedMod {
                forge_id: row.get(0)?,
                guid: row.get(1)?,
                name: row.get(2)?,
                last_known_version: row.get(3)?,
                desired_enabled: desired_i != 0,
                last_installed_version: row.get(5)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Record (replace) the installed file manifest for a mod.
    /// paths: Vec<(relative_path, is_directory)>
    /// Also updates last_installed_version to last_known_version (for diff/version tracking on commit).
    pub fn record_installed_files(&self, forge_id: i64, paths: Vec<(String, bool)>) -> Result<()> {
        // Replace semantics for the manifest of this mod.
        self.conn.execute(
            "DELETE FROM installed_files WHERE forge_id = ?1",
            params![forge_id],
        )?;
        if !paths.is_empty() {
            let mut stmt = self.conn.prepare(
                "INSERT INTO installed_files (forge_id, relative_path, is_directory) VALUES (?1, ?2, ?3)"
            )?;
            for (rel, is_dir) in &paths {
                let is_dir_i: i64 = if *is_dir { 1 } else { 0 };
                stmt.execute(params![forge_id, rel, is_dir_i])?;
            }
        }
        // On recording install, sync last_installed_version <- last_known_version (if present).
        self.conn.execute(
            "UPDATE managed_mods SET last_installed_version = last_known_version WHERE forge_id = ?1",
            params![forge_id],
        )?;
        Ok(())
    }

    pub fn get_installed_for(&self, forge_id: i64) -> Result<Vec<InstalledFile>> {
        let mut stmt = self.conn.prepare(
            "SELECT forge_id, relative_path, is_directory FROM installed_files WHERE forge_id = ?1 ORDER BY relative_path"
        )?;
        let rows = stmt.query_map(params![forge_id], |row| {
            let is_dir_i: i64 = row.get(2)?;
            Ok(InstalledFile {
                forge_id: row.get(0)?,
                relative_path: row.get(1)?,
                is_directory: is_dir_i != 0,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Clear the recorded installed_files rows for this mod (for uninstall + commit).
    /// Does not change last_installed_version (caller or later record will manage); after this get_installed_for returns [] and compute_pending will not see has_installed.
    /// Supporting for Task 10 (left unstaged at commit of only install.rs per pattern).
    pub fn clear_installed_files(&self, forge_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM installed_files WHERE forge_id = ?1",
            params![forge_id],
        )?;
        // Best to also clear the last_installed_version so re-enable will treat as fresh install (no false "version match").
        self.conn.execute(
            "UPDATE managed_mods SET last_installed_version = NULL WHERE forge_id = ?1",
            params![forge_id],
        )?;
        Ok(())
    }

    /// Compute the diff for "commit": what needs install vs uninstall based on desired state vs recorded installed_files.
    /// - to_enable (pending install): desired_enabled=true AND (has no installed_files rows OR version mismatch between last_known and last_installed)
    /// - to_disable (pending uninstall): desired_enabled=false AND has installed_files rows
    ///   Per plan semantics + "On commit: Compare desired_enabled vs. what is currently recorded as installed."
    pub fn compute_pending(&self) -> Result<(Vec<ManagedMod>, Vec<ManagedMod>)> {
        let all = self.get_all_managed()?;
        let mut to_enable: Vec<ManagedMod> = Vec::new();
        let mut to_disable: Vec<ManagedMod> = Vec::new();

        for m in all {
            let installed = self.get_installed_for(m.forge_id)?;
            let has_installed = !installed.is_empty();

            let version_mismatch = match (&m.last_known_version, &m.last_installed_version) {
                (Some(known), Some(installed_v)) => known != installed_v,
                (Some(_), None) => true,
                _ => false,
            };

            if m.desired_enabled {
                if !has_installed || version_mismatch {
                    to_enable.push(m);
                }
            } else if has_installed {
                to_disable.push(m);
            }
        }

        Ok((to_enable, to_disable))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::sync::Mutex;

    // Serialize env-mutating XDG tests (mirrors pattern in config.rs).
    static TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn in_memory_init_schema_exact_and_insert_query_counts() {
        let db = StateDb::new_in_memory().expect("new_in_memory");
        db.init_schema().expect("init_schema");

        // Insert a mod desired=true (via upsert as the "insert" path)
        let m = ManagedMod {
            forge_id: 123,
            guid: Some("com.example.testmod".into()),
            name: "Test Mod".into(),
            last_known_version: Some("1.2.3".into()),
            desired_enabled: true,
            last_installed_version: None,
        };
        db.upsert_managed_mod(&m).expect("upsert");

        // Record 3 files (mix of file + dir)
        let files = vec![
            ("user/mods/TestMod.dll".to_string(), false),
            ("user/mods/TestMod/".to_string(), true),
            ("BepInEx/plugins/TestMod/Plugin.dll".to_string(), false),
        ];
        db.record_installed_files(123, files).expect("record files");

        // Query back
        let all = db.get_all_managed().expect("get_all");
        assert_eq!(all.len(), 1);
        let got = &all[0];
        assert_eq!(got.forge_id, 123);
        assert_eq!(got.name, "Test Mod");
        assert!(got.desired_enabled);
        assert_eq!(got.last_known_version.as_deref(), Some("1.2.3"));
        // After record, last_installed should be synced to last_known
        assert_eq!(got.last_installed_version.as_deref(), Some("1.2.3"));

        let inst = db.get_installed_for(123).expect("get_installed");
        assert_eq!(inst.len(), 3);
        assert_eq!(inst[0].relative_path, "BepInEx/plugins/TestMod/Plugin.dll");
        assert!(!inst[0].is_directory);
        assert_eq!(inst[1].relative_path, "user/mods/TestMod.dll");
        assert!(!inst[1].is_directory);
        assert_eq!(inst[2].relative_path, "user/mods/TestMod/");
        assert!(inst[2].is_directory);

        // set_desired flip
        db.set_desired(123, false).expect("set_desired");
        let all2 = db.get_all_managed().expect("get_all after set");
        assert!(!all2[0].desired_enabled);
    }

    #[test]
    fn compute_pending_two_mod_cases_install_and_uninstall() {
        let db = StateDb::new_in_memory().expect("new_in_memory");
        db.init_schema().expect("init");

        // Mod A: desired=true, but no files recorded yet => should be pending install
        let ma = ManagedMod {
            forge_id: 1,
            guid: Some("guid.a".into()),
            name: "ModA".into(),
            last_known_version: Some("1.0".into()),
            desired_enabled: true,
            last_installed_version: None,
        };
        db.upsert_managed_mod(&ma).expect("upsert A");

        // Mod B: desired=false (or set false), but has files recorded => pending uninstall
        let mb = ManagedMod {
            forge_id: 2,
            guid: Some("guid.b".into()),
            name: "ModB".into(),
            last_known_version: Some("2.0".into()),
            desired_enabled: true, // temp true
            last_installed_version: None,
        };
        db.upsert_managed_mod(&mb).expect("upsert B");
        db.record_installed_files(2, vec![("some/file.txt".to_string(), false)]).expect("record B files");
        db.set_desired(2, false).expect("set B not desired");

        let (to_enable, to_disable) = db.compute_pending().expect("compute_pending");

        // ModA: desired + no files => in to_enable
        assert_eq!(to_enable.len(), 1);
        assert_eq!(to_enable[0].forge_id, 1);
        assert_eq!(to_enable[0].name, "ModA");

        // ModB: not desired + has files => in to_disable
        assert_eq!(to_disable.len(), 1);
        assert_eq!(to_disable[0].forge_id, 2);
        assert_eq!(to_disable[0].name, "ModB");

        // Sanity: after recording files for A, and if versions match, A should no longer be pending
        db.record_installed_files(1, vec![("a/file".to_string(), false)]).expect("record A now");
        let (to_e2, to_d2) = db.compute_pending().expect("compute after record A");
        assert!(to_e2.is_empty(), "A should no longer be pending install after files recorded (versions synced)");
        assert_eq!(to_d2.len(), 1); // B still
    }

    #[test]
    fn compute_pending_version_mismatch_triggers_install() {
        let db = StateDb::new_in_memory().expect("new_in_memory");
        db.init_schema().expect("init");

        let m = ManagedMod {
            forge_id: 99,
            guid: None,
            name: "VerMod".into(),
            last_known_version: Some("1.1".into()),
            desired_enabled: true,
            last_installed_version: Some("1.0".into()), // mismatch
        };
        db.upsert_managed_mod(&m).expect("upsert");
        // Record files (which would normally sync last_installed, but we force mismatch by re-upsert after)
        db.record_installed_files(99, vec![("f".to_string(), false)]).expect("record");
        // Re-upsert with new known but leave installed as-is (simulates update available)
        let m2 = ManagedMod {
            forge_id: 99,
            guid: None,
            name: "VerMod".into(),
            last_known_version: Some("1.1".into()),
            desired_enabled: true,
            last_installed_version: Some("1.0".into()),
        };
        db.upsert_managed_mod(&m2).expect("re-upsert mismatch");

        let (to_e, to_d) = db.compute_pending().expect("compute");
        assert_eq!(to_e.len(), 1, "version mismatch should cause pending install even if files present");
        assert!(to_d.is_empty());
    }

    #[test]
    fn real_file_open_and_schema_persists() {
        // Use a real temp file path (not :memory:) to test open(path) + persistence + config dir pattern.
        let tmp = tempfile::tempdir().expect("tempdir for real db");
        let db_path = tmp.path().join("state.db");

        {
            let db = StateDb::open(&db_path).expect("open real path");
            db.init_schema().expect("init on real");
            let m = ManagedMod {
                forge_id: 42,
                guid: Some("real.guid".into()),
                name: "RealFileMod".into(),
                last_known_version: None,
                desired_enabled: true,
                last_installed_version: None,
            };
            db.upsert_managed_mod(&m).expect("upsert real");
            db.record_installed_files(42, vec![("real/path".to_string(), false)]).expect("record real");
        }

        // Reopen the *same file* and verify data persisted (tests file-backed, not mem)
        {
            let db2 = StateDb::open(&db_path).expect("reopen real path");
            // init again is noop due to IF NOT EXISTS
            db2.init_schema().expect("init again");
            let all = db2.get_all_managed().expect("get after reopen");
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].forge_id, 42);
            assert_eq!(all[0].name, "RealFileMod");
            let inst = db2.get_installed_for(42).expect("inst after reopen");
            assert_eq!(inst.len(), 1);
            assert_eq!(inst[0].relative_path, "real/path");
        }
    }

    #[test]
    fn open_default_uses_config_dir_pattern() {
        let _lock = TEST_ENV_LOCK.lock().unwrap();

        // Save/restore XDG to point to a temp base (never touches real user config or CWD)
        let orig_xdg = env::var("XDG_CONFIG_HOME").ok();
        let test_id = std::process::id();
        let base = std::env::temp_dir().join(format!("spt-mod-forge-test-state-{}", test_id));
        let cfg_dir = base.join("spt-mod-forge");
        fs::create_dir_all(&cfg_dir).expect("create temp config dir for state test");

        env::set_var("XDG_CONFIG_HOME", &base);

        // Exercise the path derivation (via open_default which calls state_db_path + ensures dir + schema)
        let db = StateDb::open_default().expect("open_default should derive from config dir + succeed");
        // Verify the file was created under the XDG override we set
        let expected_db = cfg_dir.join("state.db");
        assert!(expected_db.exists(), "state.db should exist under the XDG_CONFIG_HOME path");

        // Quick smoke: schema + one op works on the real file
        let m = ManagedMod {
            forge_id: 7,
            guid: None,
            name: "CfgDirMod".into(),
            last_known_version: None,
            desired_enabled: false,
            last_installed_version: None,
        };
        db.upsert_managed_mod(&m).expect("upsert via open_default");

        // Restore
        match orig_xdg {
            Some(v) => env::set_var("XDG_CONFIG_HOME", v),
            None => env::remove_var("XDG_CONFIG_HOME"),
        }
        // cleanup
        let _ = fs::remove_file(&expected_db);
        let _ = fs::remove_dir(&cfg_dir);
        let _ = fs::remove_dir(&base);
    }
}
