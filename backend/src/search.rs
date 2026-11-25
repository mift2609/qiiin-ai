use serde::Serialize;
use std::{collections::HashMap, sync::Mutex, time::{Duration, Instant}};
use url::Url;

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

fn clean(s: &str) -> String {
    let mut t = s.to_string();
    for (a, b) in [("&amp;", "&"), ("&lt;", "<"), ("&gt;", ">"), ("&quot;", "\""), ("&#39;", "'"), ("&nbsp;", " ")] {
        t = t.replace(a, b);
    }
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn ddg_target(href: &str) -> String {
    let href = href.replace("&amp;", "&");
    let parsed = Url::parse(&href).ok().or_else(|| {
        Url::parse("https://lite.duckduckgo.com").ok()
            .and_then(|base| base.join(&href).ok())
    });
    if let Some(u) = parsed {
        if u.host_str().map(|h| h.contains("duckduckgo.com")).unwrap_or(false) {
            if let Some((_, v)) = u.query_pairs().find(|(k, _)| k == "uddg") {
                return v.into_owned();
            }
        }
        return u.to_string();
    }
    if href.starts_with("//") { return format!("https:{href}"); }
    href
}

fn parse_ddg_lite(html: &str, limit: usize) -> Vec<SearchItem> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("result-link") {
        let tail = &rest[i..];
        let hs = match tail.find("href=\"") { Some(x) => x + 6, None => break };
        let he = match tail[hs..].find('"') { Some(x) => hs + x, None => break };
        let href = &tail[hs..he];
        let ts = match tail[he + 1..].find('>') { Some(x) => he + 2 + x, None => break };
        let te = match tail[ts..].find('<') { Some(x) => ts + x, None => break };
        let title = clean(&tail[ts..te]);
        let snippet = tail[te..].find("result-snippet").and_then(|j| {
            let t = &tail[te + j..];
            t.find('>').and_then(|a| t[a + 1..].find('<').map(|b| clean(&t[a + 1..a + 1 + b])))
        }).unwrap_or_default();
        out.push(SearchItem { title, url: ddg_target(href), snippet, date: None, provider: "ddg".into() });
        if out.len() >= limit { break; }
        rest = &tail[te..];
    }
    out
}

pub async fn run(
    client: &reqwest::Client,
    cfg: &SearchConfig,
    cache: &Mutex<SearchCache>,
    query: &str,
    limit: usize,
) -> Result<(Vec<SearchItem>, String), String> {
    let key = format!("{}|{}|{}", cfg.provider, query, limit);
    let ttl = Duration::from_secs(600);
    if let Some(v) = cache_get(cache, &key, ttl) {
        return Ok((v, "cache".into()));
    }

    let items: Vec<SearchItem> = match cfg.provider.as_str() {
        "brave" => {
            if cfg.api_key.is_empty() { return Err("Brave API key kosong".into()); }
            let resp = client
                .get("https://api.search.brave.com/res/v1/web/search")
                .query(&[("q", query), ("count", &limit.to_string()), ("extra_snippets", "false")])
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
            if cfg.api_key.is_empty() { return Err("Tavily API key kosong".into()); }
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
        "ddg" => {
            let resp = client.get("https://lite.duckduckgo.com/lite/")
                .query(&[("q", query)])
                .timeout(cfg.timeout)
                .send().await.map_err(|e| format!("ddg request gagal: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("ddg HTTP {}", resp.status().as_u16()));
            }
            let html = resp.text().await.map_err(|e| e.to_string())?;
            let items = parse_ddg_lite(&html, limit);
            if items.is_empty() { return Err("ddg tidak mengembalikan hasil".into()); }
            items
        }
        other => return Err(format!("provider search tidak dikenal: {other}")),
    };

    cache_put(cache, key, items.clone());
    Ok((items, cfg.provider.clone()))
}
