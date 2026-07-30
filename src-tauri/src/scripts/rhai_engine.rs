use super::{
    tools_base64_decode, tools_base64_encode, tools_date_format, tools_json_parse,
    tools_json_stringify, tools_md5, tools_sha256, tools_timestamp, tools_uuid, ScriptContext,
};
use crate::models::{KeyValue, RequestBody};
use rhai::{Dynamic, Engine, Map, Scope};
use std::sync::{Arc, Mutex};

pub fn run(code: &str, ctx: &Arc<Mutex<ScriptContext>>, _is_pre: bool) -> Result<(), String> {
    let timeout_ms = ctx
        .lock()
        .map(|g| g.permissions.timeout_ms)
        .unwrap_or(5_000)
        .max(1);
    // Bound work roughly with timeout (ops/sec heuristic + hard floor/ceiling)
    let max_ops = ((timeout_ms as u64).saturating_mul(50_000)).clamp(10_000, 5_000_000);

    let mut engine = Engine::new();
    engine.set_max_expr_depths(64, 64);
    engine.set_max_operations(max_ops);

    // Capture context for closures
    let ctx_print = ctx.clone();
    engine.on_print(move |s| {
        if let Ok(mut g) = ctx_print.lock() {
            g.log(s);
        }
    });

    // Helper macros for locking
    let register_ctx = ctx.clone();

    // env.get / env.set via free functions
    {
        let c = register_ctx.clone();
        engine.register_fn("env_get", move |key: &str| -> String {
            c.lock().map(|g| g.get_var(key)).unwrap_or_default()
        });
    }
    {
        let c = register_ctx.clone();
        engine.register_fn("env_set", move |key: &str, value: &str| {
            if let Ok(mut g) = c.lock() {
                g.set_var(key, value);
            }
        });
    }

    {
        let c = register_ctx.clone();
        engine.register_fn("fs_read", move |path: &str| -> String {
            match c.lock() {
                Ok(g) => g.fs_read(path).unwrap_or_else(|e| {
                    // also push error
                    e
                }),
                Err(_) => String::new(),
            }
        });
    }
    {
        let c = register_ctx.clone();
        engine.register_fn("fs_write", move |path: &str, content: &str| {
            if let Ok(g) = c.lock() {
                let _ = g.fs_write(path, content);
            }
        });
    }
    {
        let c = register_ctx.clone();
        engine.register_fn("fs_append", move |path: &str, content: &str| {
            if let Ok(g) = c.lock() {
                let _ = g.fs_append(path, content);
            }
        });
    }

    {
        let c = register_ctx.clone();
        engine.register_fn("assert_status", move |expected: i64| {
            if let Ok(mut g) = c.lock() {
                g.assert_status(expected);
            }
        });
    }
    {
        let c = register_ctx.clone();
        engine.register_fn("assert_duration_lt", move |max_ms: i64| {
            if let Ok(mut g) = c.lock() {
                g.assert_duration_lt(max_ms);
            }
        });
    }
    {
        let c = register_ctx.clone();
        engine.register_fn("assert_body_field", move |path: &str, expected: &str| {
            if let Ok(mut g) = c.lock() {
                g.assert_body_field(path, expected);
            }
        });
    }

    engine.register_fn("tools_uuid", tools_uuid);
    engine.register_fn("tools_timestamp", tools_timestamp);
    engine.register_fn("tools_date_format", tools_date_format);
    engine.register_fn("tools_base64_encode", tools_base64_encode);
    engine.register_fn("tools_base64_decode", |s: &str| {
        tools_base64_decode(s).unwrap_or_default()
    });
    engine.register_fn("tools_md5", tools_md5);
    engine.register_fn("tools_sha256", tools_sha256);
    engine.register_fn("tools_json_parse", |s: &str| -> String {
        match tools_json_parse(s) {
            Ok(v) => tools_json_stringify(&v),
            Err(e) => format!("error:{e}"),
        }
    });
    engine.register_fn("tools_json_stringify", |s: &str| -> String {
        match tools_json_parse(s) {
            Ok(v) => tools_json_stringify(&v),
            Err(_) => tools_json_stringify(&serde_json::Value::String(s.to_string())),
        }
    });

    // Build scope with req, res, env helpers, fs, tools maps as Dynamic maps/objects
    let mut scope = Scope::new();

    // Snapshot request into map for mutation tracking via callback at end
    let req_map = request_to_map(&ctx.lock().unwrap().request);
    scope.push("req", req_map);

    if let Some(ref res) = ctx.lock().unwrap().response {
        let mut m = Map::new();
        m.insert("status".into(), Dynamic::from_int(res.status as i64));
        m.insert("body".into(), Dynamic::from(res.body.clone()));
        m.insert("duration_ms".into(), Dynamic::from_int(res.duration_ms as i64));
        let mut headers = rhai::Array::new();
        for h in &res.headers {
            let mut hm = Map::new();
            hm.insert("key".into(), Dynamic::from(h.key.clone()));
            hm.insert("value".into(), Dynamic::from(h.value.clone()));
            headers.push(Dynamic::from(hm));
        }
        m.insert("headers".into(), Dynamic::from(headers));
        scope.push("res", m);
    } else {
        scope.push("res", Map::new());
    }

    // Convenience aliases matching spec-ish API via prelude
    // Transform env.set/get, fs.*, tools.* into registered free functions.
    let adapted = adapt_rhai_api(code);
    let full = format!(
        r#"
        // tools helpers available as functions
        {adapted}
        "#
    );

    // Also push helper object using native maps for env/fs/tools with Fn pointers - hard in rhai.
    // Support both styles:
    // env_set("k","v") and after adapt: env.set -> env_set

    engine
        .run_with_scope(&mut scope, &full)
        .map_err(|e| format!("Rhai error: {e}"))?;

    // Read back req mutations
    if let Some(req_dyn) = scope.get_value::<Map>("req") {
        if let Ok(mut g) = ctx.lock() {
            g.request = map_to_request(&req_dyn, &g.request);
        }
    }

    Ok(())
}

fn adapt_rhai_api(code: &str) -> String {
    // Replace env.set( -> env_set(  and env.get( -> env_get(
    // Replace fs.read/write/append similarly
    // Replace tools.uuid() etc.
    let mut s = code.to_string();
    s = s.replace("env.set(", "env_set(");
    s = s.replace("env.get(", "env_get(");
    s = s.replace("fs.read(", "fs_read(");
    s = s.replace("fs.write(", "fs_write(");
    s = s.replace("fs.append(", "fs_append(");
    s = s.replace("tools.uuid()", "tools_uuid()");
    s = s.replace("tools.timestamp()", "tools_timestamp()");
    s = s.replace("tools.date_format(", "tools_date_format(");
    s = s.replace("tools.base64_encode(", "tools_base64_encode(");
    s = s.replace("tools.base64_decode(", "tools_base64_decode(");
    s = s.replace("tools.md5(", "tools_md5(");
    s = s.replace("tools.sha256(", "tools_sha256(");
    s = s.replace("tools.json_parse(", "tools_json_parse(");
    s = s.replace("tools.json_stringify(", "tools_json_stringify(");
    s = s.replace("console.log(", "print(");
    let _ = prelude_unused();
    s
}

fn prelude_unused() {}

fn request_to_map(req: &crate::models::MutableRequest) -> Map {
    let mut m = Map::new();
    m.insert("method".into(), Dynamic::from(req.method.clone()));
    m.insert("url".into(), Dynamic::from(req.url.clone()));
    m.insert("body_type".into(), Dynamic::from(req.body.body_type.clone()));
    m.insert("body".into(), Dynamic::from(req.body.content.clone()));
    let mut headers = rhai::Array::new();
    for h in &req.headers {
        let mut hm = Map::new();
        hm.insert("key".into(), Dynamic::from(h.key.clone()));
        hm.insert("value".into(), Dynamic::from(h.value.clone()));
        hm.insert("enabled".into(), Dynamic::from_bool(h.enabled));
        headers.push(Dynamic::from(hm));
    }
    m.insert("headers".into(), Dynamic::from(headers));
    let mut query = rhai::Array::new();
    for q in &req.query {
        let mut qm = Map::new();
        qm.insert("key".into(), Dynamic::from(q.key.clone()));
        qm.insert("value".into(), Dynamic::from(q.value.clone()));
        qm.insert("enabled".into(), Dynamic::from_bool(q.enabled));
        query.push(Dynamic::from(qm));
    }
    m.insert("query".into(), Dynamic::from(query));
    m
}

fn map_to_request(m: &Map, base: &crate::models::MutableRequest) -> crate::models::MutableRequest {
    let mut req = base.clone();
    if let Some(v) = m.get("method") {
        if let Ok(s) = v.clone().into_string() {
            req.method = s;
        }
    }
    if let Some(v) = m.get("url") {
        if let Ok(s) = v.clone().into_string() {
            req.url = s;
        }
    }
    if let Some(v) = m.get("body") {
        if let Ok(s) = v.clone().into_string() {
            req.body.content = s;
        }
    }
    if let Some(v) = m.get("body_type") {
        if let Ok(s) = v.clone().into_string() {
            req.body.body_type = s;
        }
    }
    if let Some(v) = m.get("headers") {
        if let Some(arr) = v.clone().try_cast::<rhai::Array>() {
            req.headers = arr
                .iter()
                .filter_map(|d| {
                    let hm = d.clone().try_cast::<Map>()?;
                    Some(KeyValue {
                        key: hm.get("key")?.clone().into_string().ok()?,
                        value: hm
                            .get("value")
                            .and_then(|x| x.clone().into_string().ok())
                            .unwrap_or_default(),
                        enabled: hm
                            .get("enabled")
                            .and_then(|x| x.clone().try_cast::<bool>())
                            .unwrap_or(true),
                    })
                })
                .collect();
        }
    }
    if let Some(v) = m.get("query") {
        if let Some(arr) = v.clone().try_cast::<rhai::Array>() {
            req.query = arr
                .iter()
                .filter_map(|d| {
                    let hm = d.clone().try_cast::<Map>()?;
                    Some(KeyValue {
                        key: hm.get("key")?.clone().into_string().ok()?,
                        value: hm
                            .get("value")
                            .and_then(|x| x.clone().into_string().ok())
                            .unwrap_or_default(),
                        enabled: hm
                            .get("enabled")
                            .and_then(|x| x.clone().try_cast::<bool>())
                            .unwrap_or(true),
                    })
                })
                .collect();
        }
    }
    // Allow body as object with type/content
    if let Some(v) = m.get("body") {
        if let Some(bm) = v.clone().try_cast::<Map>() {
            if let Some(t) = bm.get("type").and_then(|x| x.clone().into_string().ok()) {
                req.body = RequestBody {
                    body_type: t,
                    content: bm
                        .get("content")
                        .and_then(|x| x.clone().into_string().ok())
                        .unwrap_or_default(),
                };
            }
        }
    }
    req
}
