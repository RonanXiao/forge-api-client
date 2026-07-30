use crate::models::{AuthConfig, KeyValue};
use base64::Engine;

/// Apply auth config by returning extra headers and query params to merge.
pub fn apply_auth(auth: &AuthConfig) -> (Vec<KeyValue>, Vec<KeyValue>) {
    let mut headers = Vec::new();
    let mut query = Vec::new();
    match auth.auth_type.to_lowercase().as_str() {
        "bearer" if !auth.token.is_empty() => {
            headers.push(KeyValue::new(
                "Authorization",
                format!("Bearer {}", auth.token),
            ));
        }
        "basic" => {
            let raw = format!("{}:{}", auth.username, auth.password);
            let encoded = base64::engine::general_purpose::STANDARD.encode(raw.as_bytes());
            headers.push(KeyValue::new("Authorization", format!("Basic {encoded}")));
        }
        "apikey" | "api_key" | "api-key" => {
            if auth.key.is_empty() {
                return (headers, query);
            }
            if auth.add_to.eq_ignore_ascii_case("query") {
                query.push(KeyValue::new(&auth.key, &auth.value));
            } else {
                headers.push(KeyValue::new(&auth.key, &auth.value));
            }
        }
        _ => {}
    }
    (headers, query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_header() {
        let auth = AuthConfig {
            auth_type: "bearer".into(),
            token: "tok".into(),
            ..Default::default()
        };
        let (h, q) = apply_auth(&auth);
        assert!(q.is_empty());
        assert_eq!(h[0].key, "Authorization");
        assert_eq!(h[0].value, "Bearer tok");
    }

    #[test]
    fn basic_header() {
        let auth = AuthConfig {
            auth_type: "basic".into(),
            username: "u".into(),
            password: "p".into(),
            ..Default::default()
        };
        let (h, _) = apply_auth(&auth);
        assert!(h[0].value.starts_with("Basic "));
        let b64 = h[0].value.trim_start_matches("Basic ");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "u:p");
    }

    #[test]
    fn apikey_query() {
        let auth = AuthConfig {
            auth_type: "apikey".into(),
            key: "X-Key".into(),
            value: "v".into(),
            add_to: "query".into(),
            ..Default::default()
        };
        let (h, q) = apply_auth(&auth);
        assert!(h.is_empty());
        assert_eq!(q[0].key, "X-Key");
        assert_eq!(q[0].value, "v");
    }
}
