use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex, time::{Duration, Instant}};

#[derive(Clone)]
pub struct SearchConfig {
    pub provider: String,
    pub api_key: String,
    pub timeout: Duration,
}

#[derive(Clone, Serialize)]
pub struct SearchItem {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub date: Option<String>,
    pub provider: String,
}

#[derive(Default)]
pub struct SearchCache {
    map: HashMap<String, (Instant, Vec<SearchItem>)>,
}

fn cache_get(cache: &Mutex<SearchCache>, key: &str, ttl: Duration) -> Option<Vec<SearchItem>> {
    let mut c = cache.lock().ok()?;
    match c.map.get(key) {
        Some((at, v)) if at.elapsed() < ttl => Some(v.clone()),
        _ => None,
    }
}

fn cache_put(cache: &Mutex<SearchCache>, key: String, value: Vec<SearchItem>) {
    if let Ok(mut c) = cache.lock() {
        if c.map.len() > 256 { c.map.clear(); }
        c.map.insert(key, (Instant::now(), value));
    }
}

fn s(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

pub async fn run(
    client: &reqwest::Client,
    cfg: &SearchConfig,
    cache: &Mutex<SearchCache>,
    query: &str,
    limit: usize,
) -> Result<(Vec<SearchItem>, String), String> {
    if cfg.api_key.is_empty() {
        return Err("search provider belum dikonfigurasi (API key kosong)".into());
    }
    let key = format!("{}|{}|{}", cfg.provider, query, limit);
    let ttl = Duration::from_secs(600);
    if let Some(v) = cache_get(cache, &key, ttl) {
        return Ok((v, "cache".into()));
    }

    let items: Vec<SearchItem> = match cfg.provider.as_str() {
        "brave" => {
            let resp = client
                .get("https://api.search.brave.com/res/v1/web/search")
                .query(&[("q", query), ("count", &limit.to_string())])
                .header("X-Subscription-Token", &cfg.api_key)
                .header("Accept", "application/json")
                .timeout(cfg.timeout)
                .send().await.map_err(|e| format!("brave request gagal: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("brave HTTP {}", resp.status().as_u16()));
            }
            let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            let empty: Vec<serde_json::Value> = Vec::new();
            let results = body.get("web").and_then(|w| w.get("results"))
                .and_then(|r| r.as_array()).unwrap_or(&empty);
            results.iter().take(limit).map(|r| SearchItem {
                title: s(r, "title"), url: s(r, "url"),
                snippet: s(r, "description"), date: None,
                provider: "brave".into(),
            }).collect()
        }
        "tavily" => {
            let resp = client.post("https://api.tavily.com/search")
                .json(&serde_json::json!({
                    "api_key": cfg.api_key, "query": query, "max_results": limit
                }))
                .timeout(cfg.timeout)
                .send().await.map_err(|e| format!("tavily request gagal: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("tavily HTTP {}", resp.status().as_u16()));
            }
            let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            let empty: Vec<serde_json::Value> = Vec::new();
            let results = body.get("results").and_then(|r| r.as_array()).unwrap_or(&empty);
            results.iter().take(limit).map(|r| SearchItem {
                title: s(r, "title"), url: s(r, "url"),
                snippet: s(r, "content"),
                date: r.get("published_date").and_then(|x| x.as_str()).map(str::to_string),
                provider: "tavily".into(),
            }).collect()
        }
        other => return Err(format!("provider search tidak dikenal: {other}")),
    };

    cache_put(cache, key, items.clone());
    Ok((items, cfg.provider.clone()))
}
