use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
pub struct Doc {
    pub title: String,
    pub url: String,
    pub content: String,
    pub created_at: String,
}

pub struct Knowledge {
    path: String,
    docs: Vec<Doc>,
}

pub fn open(path: &str) -> Knowledge {
    let docs = fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<Doc>>(&s).ok())
        .unwrap_or_default();
    Knowledge { path: path.to_string(), docs }
}

impl Knowledge {
    fn save(&self) -> Result<(), String> {
        let text = serde_json::to_string(&self.docs).map_err(|e| e.to_string())?;
        fs::write(&self.path, text).map_err(|e| e.to_string())
    }

    pub fn add(&mut self, title: &str, url: &str, content: &str, created_at: &str) -> Result<usize, String> {
        if content.trim().is_empty() { return Err("content kosong".into()); }
        if content.len() > 200_000 { return Err("dokumen terlalu besar".into()); }
        if self.docs.iter().any(|d| d.content == content) { return Ok(0); }
        self.docs.push(Doc {
            title: title.to_string(), url: url.to_string(),
            content: content.to_string(), created_at: created_at.to_string(),
        });
        if self.docs.len() > 500 { self.docs.remove(0); }
        self.save()?;
        Ok(1)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Doc>, String> {
        let terms: Vec<String> = query.to_lowercase().split_whitespace()
            .filter(|t| t.len() > 1).map(str::to_string).collect();
        if terms.is_empty() { return Ok(vec![]); }
        let mut scored: Vec<(usize, Doc)> = self.docs.iter().filter_map(|d| {
            let hay = format!("{} {} {}", d.title, d.url, d.content).to_lowercase();
            let hits = terms.iter().filter(|t| hay.contains(t.as_str())).count();
            if hits > 0 { Some((hits, d.clone())) } else { None }
        }).collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(scored.into_iter().take(limit).map(|x| x.1).collect())
    }
}
