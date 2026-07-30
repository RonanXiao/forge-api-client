use crate::models::CookieEntry;
use cookie::Cookie;
use url::Url;
use uuid::Uuid;

/// Match cookies for a request URL.
pub fn cookies_for_url(jar: &[CookieEntry], url: &str) -> Vec<CookieEntry> {
    let Ok(parsed) = Url::parse(url) else {
        return vec![];
    };
    let host = parsed.host_str().unwrap_or("").to_lowercase();
    let path = if parsed.path().is_empty() {
        "/"
    } else {
        parsed.path()
    };
    let is_https = parsed.scheme() == "https";

    jar.iter()
        .filter(|c| {
            if c.secure && !is_https {
                return false;
            }
            domain_matches(&c.domain, &host) && path_matches(&c.path, path)
        })
        .cloned()
        .collect()
}

fn domain_matches(cookie_domain: &str, host: &str) -> bool {
    let d = cookie_domain.trim_start_matches('.').to_lowercase();
    let h = host.to_lowercase();
    h == d || h.ends_with(&format!(".{d}"))
}

fn path_matches(cookie_path: &str, req_path: &str) -> bool {
    let cp = if cookie_path.is_empty() {
        "/"
    } else {
        cookie_path
    };
    req_path.starts_with(cp)
}

/// Build Cookie header value from matching entries.
pub fn cookie_header(jar: &[CookieEntry], url: &str) -> Option<String> {
    let matched = cookies_for_url(jar, url);
    if matched.is_empty() {
        return None;
    }
    Some(
        matched
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

/// Parse Set-Cookie headers and merge into jar (by name+domain+path).
pub fn ingest_set_cookie(jar: &mut Vec<CookieEntry>, url: &str, set_cookie_values: &[String]) {
    let Ok(parsed) = Url::parse(url) else {
        return;
    };
    let default_domain = parsed.host_str().unwrap_or("").to_string();
    let default_path = {
        let p = parsed.path();
        if p.is_empty() {
            "/".into()
        } else if let Some(idx) = p.rfind('/') {
            if idx == 0 {
                "/".into()
            } else {
                p[..=idx].to_string()
            }
        } else {
            "/".into()
        }
    };

    for raw in set_cookie_values {
        let Ok(c) = Cookie::parse(raw.as_str()) else {
            continue;
        };
        let domain = c
            .domain()
            .map(|d| d.trim_start_matches('.').to_string())
            .unwrap_or_else(|| default_domain.clone());
        let path = c.path().unwrap_or(&default_path).to_string();
        let entry = CookieEntry {
            id: Uuid::new_v4().to_string(),
            name: c.name().to_string(),
            value: c.value().to_string(),
            domain,
            path,
            secure: c.secure().unwrap_or(false),
            http_only: c.http_only().unwrap_or(false),
            expires: None,
        };
        // Replace existing same name+domain+path
        if let Some(pos) = jar.iter().position(|e| {
            e.name == entry.name && e.domain == entry.domain && e.path == entry.path
        }) {
            entry.id.clone_into(&mut jar[pos].id);
            jar[pos] = CookieEntry {
                id: jar[pos].id.clone(),
                ..entry
            };
        } else {
            jar.push(entry);
        }
    }
}

pub fn delete_cookie(jar: &mut Vec<CookieEntry>, id: &str) -> bool {
    let before = jar.len();
    jar.retain(|c| c.id != id);
    jar.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CookieEntry {
        CookieEntry {
            id: "1".into(),
            name: "sid".into(),
            value: "abc".into(),
            domain: "example.com".into(),
            path: "/".into(),
            secure: false,
            http_only: false,
            expires: None,
        }
    }

    #[test]
    fn matches_subdomain() {
        let jar = vec![sample()];
        let m = cookies_for_url(&jar, "http://api.example.com/v1");
        assert_eq!(m.len(), 1);
        assert_eq!(cookie_header(&jar, "http://api.example.com/").unwrap(), "sid=abc");
    }

    #[test]
    fn secure_requires_https() {
        let mut c = sample();
        c.secure = true;
        let jar = vec![c];
        assert!(cookies_for_url(&jar, "http://example.com/").is_empty());
        assert_eq!(cookies_for_url(&jar, "https://example.com/").len(), 1);
    }

    #[test]
    fn ingest_and_delete() {
        let mut jar = vec![];
        ingest_set_cookie(
            &mut jar,
            "http://example.com/api",
            &["token=xyz; Path=/; HttpOnly".into()],
        );
        assert_eq!(jar.len(), 1);
        assert_eq!(jar[0].name, "token");
        assert_eq!(jar[0].value, "xyz");
        let id = jar[0].id.clone();
        assert!(delete_cookie(&mut jar, &id));
        assert!(jar.is_empty());
    }
}
