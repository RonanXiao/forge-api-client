use crate::models::KeyValue;
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

static VAR_RE: OnceLock<Regex> = OnceLock::new();

fn var_re() -> &'static Regex {
    VAR_RE.get_or_init(|| Regex::new(r"\{\{\s*([a-zA-Z0-9_.-]+)\s*\}\}").expect("regex"))
}

/// Build variable map from enabled key-values (later entries override earlier).
pub fn vars_from_kv(items: &[KeyValue]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for kv in items.iter().filter(|k| k.enabled && !k.key.is_empty()) {
        map.insert(kv.key.clone(), kv.value.clone());
    }
    map
}

/// Interpolate `{{variable}}` placeholders. Unknown variables are left unchanged.
pub fn interpolate(input: &str, vars: &HashMap<String, String>) -> String {
    var_re()
        .replace_all(input, |caps: &regex::Captures| {
            let name = &caps[1];
            vars.get(name)
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
}

/// Interpolate all string fields of a request-like structure.
pub fn interpolate_kv_list(list: &[KeyValue], vars: &HashMap<String, String>) -> Vec<KeyValue> {
    list.iter()
        .map(|kv| KeyValue {
            key: interpolate(&kv.key, vars),
            value: interpolate(&kv.value, vars),
            enabled: kv.enabled,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_known_vars() {
        let mut vars = HashMap::new();
        vars.insert("host".into(), "api.example.com".into());
        vars.insert("token".into(), "abc".into());
        assert_eq!(
            interpolate("https://{{host}}/v1?t={{ token }}", &vars),
            "https://api.example.com/v1?t=abc"
        );
    }

    #[test]
    fn leaves_missing_vars() {
        let vars = HashMap::new();
        assert_eq!(interpolate("{{missing}}", &vars), "{{missing}}");
    }

    #[test]
    fn later_kv_overrides() {
        let items = vec![
            KeyValue::new("a", "1"),
            KeyValue {
                key: "a".into(),
                value: "2".into(),
                enabled: true,
            },
        ];
        let map = vars_from_kv(&items);
        assert_eq!(map.get("a").map(String::as_str), Some("2"));
    }
}
