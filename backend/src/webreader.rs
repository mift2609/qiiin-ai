use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use url::Url;

#[derive(Serialize)]
pub struct ReadResult {
    pub url: String,
    pub title: String,
    pub text: String,
    pub source: String,
}

fn v4_public(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    if o[0] == 0 || o[0] == 10 || o[0] == 127 || o[0] >= 224 { return false; }
    if o[0] == 169 && o[1] == 254 { return false; }
    if o[0] == 172 && (16..=31).contains(&o[1]) { return false; }
    if o[0] == 192 && o[1] == 168 { return false; }
    if o[0] == 100 && (64..=127).contains(&o[1]) { return false; }
    if o[0] == 192 && o[1] == 0 { return false; }
    if o[0] == 198 && (18..=19).contains(&o[1]) { return false; }
    true
}

fn ip_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v4_public(v),
        IpAddr::V6(v) => {
            if v == Ipv6Addr::LOCALHOST || v.is_unspecified() || v.is_multicast() { return false; }
            if let Some(mapped) = v.to_ipv4() { return v4_public(mapped); }
            let s = v.segments();
            !(s[0] & 0xfe00 == 0xfc00 || s[0] & 0xffc0 == 0xfe80)
        }
    }
}

pub fn validate(input: &str) -> Result<Url, String> {
    let u = Url::parse(input).map_err(|_| "URL tidak valid".to_string())?;
    match u.scheme() {
        "http" | "https" => {},
        _ => return Err("skema harus HTTP/HTTPS".into()),
    }
    let host = u.host_str().ok_or("host tidak ada")?.to_lowercase();
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") || host.ends_with(".internal") {
        return Err("host lokal/internal diblokir".into());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !ip_public(ip) { return Err("IP private/link-local diblokir".into()); }
    }
    let port = u.port_or_known_default().unwrap_or(80);
    let addrs: Vec<_> = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("DNS gagal: {e}"))?
        .map(|a| a.ip())
        .collect();
    if addrs.is_empty() || addrs.iter().any(|ip| !ip_public(*ip)) {
        return Err("resolusi DNS mengarah ke alamat internal".into());
    }
    Ok(u)
}

fn strip_block(input: &str, tag: &str) -> String {
    let mut s = input.to_string();
    loop {
        let begin = match s.to_lowercase().find(&format!("<{tag}")) { Some(i) => i, None => break };
        let end = match s.to_lowercase()[begin..].find(&format!("</{tag}>")) {
            Some(i) => begin + i + tag.len() + 3,
            None => break,
        };
        s.replace_range(begin..end, " ");
    }
    s
}

fn plain(html: &str) -> String {
    let mut s = strip_block(&strip_block(html, "script"), "style");
    let mut out = String::new();
    let mut inside = false;
    for ch in s.chars() {
        if ch == '<' { inside = true; continue; }
        if ch == '>' { inside = false; continue; }
        if !inside { out.push(ch); }
    }
    s = out;
    for (a, b) in [("&amp;", "&"), ("&lt;", "<"), ("&gt;", ">"), ("&quot;", "\""), ("&#39;", "'"), ("&nbsp;", " ")] {
        s = s.replace(a, b);
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub async fn read(_client: &reqwest::Client, input: &str) -> Result<ReadResult, String> {
    let mut u = validate(input)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(20))
        .build().map_err(|e| e.to_string())?;
    for _ in 0..4 {
        let resp = client.get(u.clone()).send().await.map_err(|e| format!("fetch gagal: {e}"))?;
        let status = resp.status();
        if status.is_redirection() {
            let loc = resp.headers().get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok()).ok_or("redirect tanpa Location")?;
            u = validate(&u.join(loc).map_err(|e| e.to_string())?.to_string())?;
            continue;
        }
        if !status.is_success() { return Err(format!("HTTP {}", status.as_u16())); }
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 { return Err("halaman terlalu besar (>1 MB)".into()); }
        let html = String::from_utf8_lossy(&bytes);
        let lower = html.to_lowercase();
        let title = lower.find("<title>").and_then(|a| {
            lower[a + 7..].find("</title>").map(|b| lower[a + 7..a + 7 + b].trim().to_string())
        }).unwrap_or_default();
        return Ok(ReadResult {
            url: u.to_string(),
            title,
            text: plain(&html),
            source: "web-reader".into(),
        });
    }
    Err("terlalu banyak redirect".into())
}
