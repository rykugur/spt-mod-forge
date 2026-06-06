// src/forge.rs
// Task 9: Forge API client (token from env only, blocking, list + versions + download, cache integration)
// Create: src/forge.rs per plan.
//
// Step 9.1: Define client. On new: read FORGE_API_TOKEN or return TokenRequired. Use reqwest::blocking::Client::new(). With header (per-request).
//
// Methods (minimal):
// pub fn list_mods(&self, sort: &str, spt_version: Option<&str>, search: Option<&str>, cache: Option<&Cache>) -> Result<Vec<ForgeMod>>
// pub fn get_versions(&self, forge_id: i64) -> Result<Vec<ForgeModVersion>>
// pub fn download_to(&self, url: &str, dest: &Path) -> Result<()>
//
// Hard-code query per design history (viewer-derived): https://forge.sp-tarkov.com/api/v0/mods?filter[spt_version]=...&sort=downloads
// (supports filter[name]/filter[guid] via search param; /api/v0/mod/{id}/versions for versions).
// Refine with real responses (unit test uses embedded sample json only).
//
// Cache wiring: list_mods consults cache.load_list (for sort/spt key) before net if enabled+provided.
// On net success, store_list (skips for search queries in v1; force via prior clear_lists() by caller).
// download_to streams/gets bytes and writes to provided dest (caller e.g. cache.download_path chooses the on-disk location).
//
// Handle 401 -> AppError::TokenRequired (nice msg).
// Rate limit: on 429 read Retry-After (or 1s), sleep, retry the request once. On second 429 -> Api error.
//
// For network/live: document "run manually with token". No cfg(feature) or live tests in this task (per "parse sample json in a test (no network)").
//
// Other errors -> Api or Http variants. No CWD. Uses blocking reqwest (features already declared).
//
// Step 9.2: #[test] fn parses_sample_mod_list() with json literal matching ForgeModResponse/ForgeMod.
// Step 9.3: Commit (git add src/forge.rs ONLY; supporting mod decl in models.rs left unstaged per pattern).
//
// After green: cargo check + targeted tests under nix develop.

use std::path::Path;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::StatusCode;

use crate::cache::Cache;
use crate::error::{AppError, Result};
use crate::{ForgeMod, ForgeModResponse, ForgeModVersion};

const API_BASE: &str = "https://forge.sp-tarkov.com/api/v0";

#[derive(Debug)]
pub struct ForgeClient {
    client: Client,
    token: String,
}

impl ForgeClient {
    /// Read FORGE_API_TOKEN (env only, never from config files). Returns TokenRequired on missing/empty.
    /// Uses blocking Client::new(); Authorization header added on each request.
    pub fn new() -> Result<Self> {
        let token = std::env::var("FORGE_API_TOKEN").map_err(|_| {
            AppError::TokenRequired(
                "FORGE_API_TOKEN missing. Export it or paste at prompt.".into(),
            )
        })?;
        if token.trim().is_empty() {
            return Err(AppError::TokenRequired(
                "FORGE_API_TOKEN is empty. Export it or paste at prompt.".into(),
            ));
        }
        let client = Client::new();
        Ok(Self { client, token })
    }

    /// List mods (curated or search). If cache provided+enabled and no active search, consult load_list first.
    /// Query uses design endpoints: /mods?filter[spt_version]=...&sort=... (+ filter[name] for search).
    /// On successful net fetch, store back to cache for the (sort, spt) key when not searching.
    /// (Force refresh: caller does cache.clear_lists() before list_mods; search results bypass cache storage.)
    pub fn list_mods(
        &self,
        sort: &str,
        spt_version: Option<&str>,
        search: Option<&str>,
        cache: Option<&Cache>,
    ) -> Result<Vec<ForgeMod>> {
        let spt_ver = spt_version.unwrap_or("").trim();
        let search_q = search.and_then(|s| {
            let t = s.trim();
            if t.is_empty() { None } else { Some(t) }
        });
        let has_search = search_q.is_some();

        // Cache consult only for non-search (curated sorts keyed by sort+spt_ver)
        if !has_search {
            if let Some(c) = cache {
                if c.enabled() {
                    let key_ver = if spt_ver.is_empty() { "unknown" } else { spt_ver };
                    if let Some(items) = c.load_list(sort, key_ver)? {
                        return Ok(items);
                    }
                }
            }
        }

        // Build URL (viewer-derived query style)
        let mut url = format!("{}/mods", API_BASE);
        let mut qs: Vec<String> = Vec::new();
        if !spt_ver.is_empty() {
            qs.push(format!("filter[spt_version]={}", urlencode(spt_ver)));
        }
        if let Some(q) = search_q {
            qs.push(format!("filter[name]={}", urlencode(q)));
        }
        let sort_param = match sort {
            "most_downloaded" | "downloads" => "downloads",
            "last_updated" | "recently_updated" | "updated_at" => "updated_at",
            s if !s.is_empty() => s,
            _ => "downloads",
        };
        qs.push(format!("sort={}", sort_param));

        if !qs.is_empty() {
            url.push('?');
            url.push_str(&qs.join("&"));
        }

        let resp = self.do_get(&url)?;
        let api_resp: ForgeModResponse = resp
            .json()
            .map_err(|e| AppError::Api(format!("failed to decode mod list json: {}", e)))?;
        let items = api_resp.data;

        // Store result (curated only)
        if !has_search {
            if let Some(c) = cache {
                if c.enabled() {
                    let key_ver = if spt_ver.is_empty() { "unknown" } else { spt_ver };
                    let _ = c.store_list(sort, key_ver, &items); // best-effort; list success takes precedence
                }
            }
        }

        Ok(items)
    }

    /// Fetch versions list for a given forge mod id.
    /// Endpoint: /api/v0/mod/{id}/versions (page omitted for v1 minimal; assume default page or all).
    /// Response assumed wrapped like list ( { data: [...] } ); falls back gracefully in decode if needed later.
    pub fn get_versions(&self, forge_id: i64) -> Result<Vec<ForgeModVersion>> {
        let url = format!("{}/mod/{}/versions", API_BASE, forge_id);
        let resp = self.do_get(&url)?;

        #[derive(serde::Deserialize)]
        struct VersionsResp {
            data: Vec<ForgeModVersion>,
        }

        let v: VersionsResp = resp
            .json()
            .map_err(|e| AppError::Api(format!("failed to decode versions json: {}", e)))?;
        Ok(v.data)
    }

    /// Download the archive at url directly to dest path (creates parent dirs).
    /// For v1: simple get + bytes + write (streaming + progress in later tasks).
    /// dest is typically a cache.download_path(...) chosen by caller.
    /// Re-uses do_get (will surface auth/rate errors if a download url ever requires token, though usually direct).
    pub fn download_to(&self, url: &str, dest: &Path) -> Result<()> {
        let resp = self.do_get(url)?;
        let bytes = resp
            .bytes()
            .map_err(|e| AppError::Http(format!("failed to read download body: {}", e)))?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(dest, &bytes)?;
        Ok(())
    }

    /// Shared request helper: adds Bearer, handles 401->TokenRequired, 429 (header sleep + one retry),
    /// other non-2xx -> Api error (body included). Returns success Response ready for .json() or .bytes().
    fn do_get(&self, url: &str) -> Result<reqwest::blocking::Response> {
        let mut resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|e| AppError::Http(format!("http request error: {}", e)))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            return Err(AppError::TokenRequired(
                "FORGE_API_TOKEN invalid or expired (401). Re-export a valid token.".into(),
            ));
        }

        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            let sleep_secs = resp
                .headers()
                .get("retry-after")
                .and_then(|hv| hv.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1);
            std::thread::sleep(Duration::from_secs(sleep_secs));

            resp = self
                .client
                .get(url)
                .header("Authorization", format!("Bearer {}", self.token))
                .send()
                .map_err(|e| AppError::Http(format!("http retry error: {}", e)))?;

            if resp.status() == StatusCode::UNAUTHORIZED {
                return Err(AppError::TokenRequired(
                    "FORGE_API_TOKEN invalid or expired (401 on rate-limit retry).".into(),
                ));
            }
            if resp.status() == StatusCode::TOO_MANY_REQUESTS {
                return Err(AppError::Api(
                    "rate limited by Forge API after retry (429)".into(),
                ));
            }
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_else(|_| "<no body>".into());
            return Err(AppError::Api(format!("API {}: {}", status, body)));
        }

        Ok(resp)
    }
}

/// Minimal URL-encoder sufficient for our controlled values (SPT versions "4.x.y", simple search terms, known sorts).
/// Real API calls with odd chars in search will be refined post real-response inspection.
fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "%20".to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Serialize env tests (like config/cache) to avoid races on FORGE_API_TOKEN across parallel tests.
    static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn new_without_token_yields_token_required() {
        let _lock = TEST_ENV_LOCK.lock().unwrap();
        let orig = env::var("FORGE_API_TOKEN").ok();
        env::remove_var("FORGE_API_TOKEN");

        let res = ForgeClient::new();
        assert!(res.is_err());
        match res {
            Err(AppError::TokenRequired(msg)) => {
                assert!(msg.contains("FORGE_API_TOKEN"));
            }
            other => panic!("expected TokenRequired, got {:?}", other),
        }

        // restore
        if let Some(v) = orig {
            env::set_var("FORGE_API_TOKEN", v);
        }
    }

    #[test]
    fn new_with_empty_token_yields_token_required() {
        let _lock = TEST_ENV_LOCK.lock().unwrap();
        let orig = env::var("FORGE_API_TOKEN").ok();
        env::set_var("FORGE_API_TOKEN", "   ");

        let res = ForgeClient::new();
        assert!(res.is_err());
        match res {
            Err(AppError::TokenRequired(msg)) => {
                assert!(msg.contains("empty"));
            }
            other => panic!("expected TokenRequired for empty, got {:?}", other),
        }

        // restore
        match orig {
            Some(v) => env::set_var("FORGE_API_TOKEN", v),
            None => env::remove_var("FORGE_API_TOKEN"),
        }
    }

    #[test]
    fn parses_sample_mod_list() {
        // Sample json literal (from spec history / known response shape; matches ForgeModResponse + ForgeMod fields).
        // Used for no-network unit test. Real API may have additional fields (ignored by Deserialize) or renames (refined later).
        let sample = r#"{
  "data": [
    {
      "id": 42,
      "guid": "com.spt.example.mod",
      "name": "Example Mod",
      "teaser": "Does a thing for SPT",
      "author": "SomeAuthor",
      "downloads": 98765,
      "updated_at": "2026-05-01T12:00:00Z",
      "detail_url": "https://forge.sp-tarkov.com/mods/42",
      "source_url": "https://github.com/example/spt-example-mod"
    },
    {
      "id": 99,
      "guid": null,
      "name": "Another",
      "teaser": null,
      "author": null,
      "downloads": 10,
      "updated_at": null,
      "detail_url": null,
      "source_url": null
    }
  ]
}"#;

        let parsed: ForgeModResponse = serde_json::from_str(sample).expect("sample must deserialize to ForgeModResponse");
        assert_eq!(parsed.data.len(), 2);

        let m0 = &parsed.data[0];
        assert_eq!(m0.id, 42);
        assert_eq!(m0.name, "Example Mod");
        assert_eq!(m0.guid.as_deref(), Some("com.spt.example.mod"));
        assert_eq!(m0.downloads, 98765);
        assert_eq!(m0.author.as_deref(), Some("SomeAuthor"));

        let m1 = &parsed.data[1];
        assert_eq!(m1.id, 99);
        assert_eq!(m1.name, "Another");
        assert!(m1.guid.is_none());
    }

    // NOTE on network / live usage (per plan):
    // - No network tests here (sample parse only).
    // - To manually exercise with real token + net (e.g. for endpoint response discovery):
    //     FORGE_API_TOKEN=xxx nix develop --command cargo test --lib -- --quiet
    //   or simply run the binary:
    //     FORGE_API_TOKEN=xxx nix develop --command cargo run --
    //   (Later tasks will wire into app; rate/401 paths can be observed by providing bad/omitted token.)
    // - If adding live tests later: guard with #[cfg(feature = "network")] or #[ignore].
}

// Ensure the module at least typechecks in lib context (the tests above + methods).
// (cargo test --lib will run the parses_sample_mod_list and new_ token tests.)
