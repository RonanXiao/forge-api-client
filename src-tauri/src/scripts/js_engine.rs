use super::{
    tools_base64_decode, tools_base64_encode, tools_date_format, tools_json_parse,
    tools_json_stringify, tools_md5, tools_sha256, tools_timestamp, tools_uuid, ScriptContext,
};
use boa_engine::native_function::NativeFunction;
use boa_engine::value::JsValue;
use boa_engine::{Context as BoaContext, JsArgs, JsString, Source};
use std::sync::{Arc, Mutex, OnceLock};

fn js_str(s: impl AsRef<str>) -> JsValue {
    JsValue::from(JsString::from(s.as_ref()))
}

/// Thread-local style host context pointer for Boa native fns (Copy closures).
static HOST_CTX: OnceLock<Mutex<Option<Arc<Mutex<ScriptContext>>>>> = OnceLock::new();

fn host_slot() -> &'static Mutex<Option<Arc<Mutex<ScriptContext>>>> {
    HOST_CTX.get_or_init(|| Mutex::new(None))
}

fn with_host<T>(f: impl FnOnce(&mut ScriptContext) -> T) -> Option<T> {
    let guard = host_slot().lock().ok()?;
    let arc = guard.as_ref()?;
    let mut g = arc.lock().ok()?;
    Some(f(&mut g))
}

/// Run a JavaScript script against the shared context using Boa.
pub fn run(code: &str, ctx: &Arc<Mutex<ScriptContext>>, is_pre: bool) -> Result<(), String> {
    {
        let mut slot = host_slot().lock().map_err(|e| e.to_string())?;
        *slot = Some(ctx.clone());
    }

    let result = run_inner(code, ctx, is_pre);

    if let Ok(mut slot) = host_slot().lock() {
        *slot = None;
    }
    result
}

fn run_inner(code: &str, ctx: &Arc<Mutex<ScriptContext>>, is_pre: bool) -> Result<(), String> {
    let snapshot = {
        let g = ctx.lock().map_err(|e| e.to_string())?;
        serde_json::json!({
            "request": {
                "method": g.request.method,
                "url": g.request.url,
                "headers": g.request.headers,
                "query": g.request.query,
                "body": {
                    "type": g.request.body.body_type,
                    "content": g.request.body.content,
                    "language": g.request.body.language,
                },
            },
            "response": g.response.as_ref().map(|r| serde_json::json!({
                "status": r.status,
                "body": r.body,
                "duration_ms": r.duration_ms,
                "headers": r.headers,
            })),
            "variables": g.variables,
            "isPre": is_pre,
        })
    };
    let state_json = serde_json::to_string(&snapshot).map_err(|e| e.to_string())?;
    let user_code = code;

    let wrapped = format!(
        r#"
const __state = {state_json};
const logs = [];
const assertions = [];
const variables = Object.assign({{}}, __state.variables || {{}});
const req = {{
  method: __state.request.method,
  url: __state.request.url,
  headers: __state.request.headers || [],
  query: __state.request.query || [],
  body: __state.request.body || {{ type: "none", content: "" }},
}};
const res = __state.response ? {{
  status: __state.response.status,
  body: __state.response.body,
  duration_ms: __state.response.duration_ms,
  headers: __state.response.headers || [],
}} : null;
const env = {{
  get: (k) => (variables[k] !== undefined ? String(variables[k]) : ""),
  set: (k, v) => {{ variables[String(k)] = String(v); }},
}};
const console = {{ log: (...a) => logs.push(a.map(String).join(" ")) }};
function print(...a) {{ console.log(...a); }}
const tools = {{
  uuid: () => __host_uuid(),
  timestamp: () => __host_timestamp(),
  date_format: (f) => __host_date_format(String(f)),
  base64_encode: (s) => __host_b64e(String(s)),
  base64_decode: (s) => __host_b64d(String(s)),
  md5: (s) => __host_md5(String(s)),
  sha256: (s) => __host_sha256(String(s)),
  json_parse: (s) => __host_json_parse(String(s)),
  json_stringify: (s) => __host_json_stringify(String(s)),
}};
const fs = {{
  read: (p) => __host_fs_read(String(p)),
  write: (p, c) => __host_fs_write(String(p), String(c)),
  append: (p, c) => __host_fs_append(String(p), String(c)),
}};
function assert_status(expected) {{
  const actual = res ? res.status : -1;
  assertions.push({{ name: "status", passed: Number(actual) === Number(expected), message: "expected status " + expected + ", got " + actual }});
}}
function assert_duration_lt(maxMs) {{
  const actual = res ? res.duration_ms : -1;
  assertions.push({{ name: "duration", passed: actual >= 0 && actual < maxMs, message: "expected duration < " + maxMs + "ms, got " + actual + "ms" }});
}}
function assert_body_field(path, expected) {{
  let cur = null;
  try {{ cur = JSON.parse(res ? res.body : "{{}}"); }} catch (e) {{ cur = null; }}
  for (const p of String(path).split(".").filter(Boolean)) {{ if (cur == null) break; cur = cur[p]; }}
  const actual = (cur === undefined || cur === null) ? null : String(cur);
  assertions.push({{ name: "body." + path, passed: actual === String(expected), message: "expected body field '" + path + "' == '" + expected + "', got '" + actual + "'" }});
}}
{user_code}
JSON.stringify({{ logs: logs, assertions: assertions, variables: variables, request: req }});
"#
    );

    let mut context = BoaContext::default();

    let register = |ctx: &mut BoaContext, name: &str, f: NativeFunction| {
        let _ = ctx.register_global_callable(JsString::from(name), 0, f);
    };

    register(
        &mut context,
        "__host_uuid",
        NativeFunction::from_copy_closure(|_, _, _| Ok(js_str(tools_uuid()))),
    );
    register(
        &mut context,
        "__host_timestamp",
        NativeFunction::from_copy_closure(|_, _, _| Ok(JsValue::from(tools_timestamp() as i32))),
    );
    register(
        &mut context,
        "__host_date_format",
        NativeFunction::from_copy_closure(|_, args, context| {
            let f = args
                .get_or_undefined(0)
                .to_string(context)?
                .to_std_string_escaped();
            Ok(js_str(tools_date_format(&f)))
        }),
    );
    register(
        &mut context,
        "__host_b64e",
        NativeFunction::from_copy_closure(|_, args, context| {
            let s = args
                .get_or_undefined(0)
                .to_string(context)?
                .to_std_string_escaped();
            Ok(js_str(tools_base64_encode(&s)))
        }),
    );
    register(
        &mut context,
        "__host_b64d",
        NativeFunction::from_copy_closure(|_, args, context| {
            let s = args
                .get_or_undefined(0)
                .to_string(context)?
                .to_std_string_escaped();
            Ok(js_str(tools_base64_decode(&s).unwrap_or_default()))
        }),
    );
    register(
        &mut context,
        "__host_md5",
        NativeFunction::from_copy_closure(|_, args, context| {
            let s = args
                .get_or_undefined(0)
                .to_string(context)?
                .to_std_string_escaped();
            Ok(js_str(tools_md5(&s)))
        }),
    );
    register(
        &mut context,
        "__host_sha256",
        NativeFunction::from_copy_closure(|_, args, context| {
            let s = args
                .get_or_undefined(0)
                .to_string(context)?
                .to_std_string_escaped();
            Ok(js_str(tools_sha256(&s)))
        }),
    );
    register(
        &mut context,
        "__host_json_parse",
        NativeFunction::from_copy_closure(|_, args, context| {
            let s = args
                .get_or_undefined(0)
                .to_string(context)?
                .to_std_string_escaped();
            match tools_json_parse(&s) {
                Ok(v) => Ok(js_str(tools_json_stringify(&v))),
                Err(e) => Ok(js_str(format!("error:{e}"))),
            }
        }),
    );
    register(
        &mut context,
        "__host_json_stringify",
        NativeFunction::from_copy_closure(|_, args, context| {
            let s = args
                .get_or_undefined(0)
                .to_string(context)?
                .to_std_string_escaped();
            // Accept raw JSON text or plain text
            let out = match tools_json_parse(&s) {
                Ok(v) => tools_json_stringify(&v),
                Err(_) => tools_json_stringify(&serde_json::Value::String(s)),
            };
            Ok(js_str(out))
        }),
    );
    register(
        &mut context,
        "__host_fs_read",
        NativeFunction::from_copy_closure(|_, args, context| {
            let p = args
                .get_or_undefined(0)
                .to_string(context)?
                .to_std_string_escaped();
            let val = with_host(|g| g.fs_read(&p).unwrap_or_default()).unwrap_or_default();
            Ok(js_str(val))
        }),
    );
    register(
        &mut context,
        "__host_fs_write",
        NativeFunction::from_copy_closure(|_, args, context| {
            let p = args
                .get_or_undefined(0)
                .to_string(context)?
                .to_std_string_escaped();
            let c = args
                .get_or_undefined(1)
                .to_string(context)?
                .to_std_string_escaped();
            let _ = with_host(|g| g.fs_write(&p, &c));
            Ok(JsValue::undefined())
        }),
    );
    register(
        &mut context,
        "__host_fs_append",
        NativeFunction::from_copy_closure(|_, args, context| {
            let p = args
                .get_or_undefined(0)
                .to_string(context)?
                .to_std_string_escaped();
            let c = args
                .get_or_undefined(1)
                .to_string(context)?
                .to_std_string_escaped();
            let _ = with_host(|g| g.fs_append(&p, &c));
            Ok(JsValue::undefined())
        }),
    );

    let value = context
        .eval(Source::from_bytes(wrapped.as_bytes()))
        .map_err(|e| format!("JavaScript error: {e}"))?;
    let json = value
        .to_string(&mut context)
        .map_err(|e| format!("JS result: {e}"))?
        .to_std_string_escaped();

    apply_js_result(ctx, &json)
}

fn apply_js_result(ctx: &Arc<Mutex<ScriptContext>>, json: &str) -> Result<(), String> {
    let parsed: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("JS result parse: {e} — {json}"))?;
    let mut g = ctx.lock().map_err(|e| e.to_string())?;
    if let Some(logs) = parsed.get("logs").and_then(|l| l.as_array()) {
        for l in logs {
            if let Some(s) = l.as_str() {
                g.log(s);
            }
        }
    }
    if let Some(assertions) = parsed.get("assertions").and_then(|a| a.as_array()) {
        for a in assertions {
            g.assertions.push(crate::models::AssertionResult {
                name: a
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("assert")
                    .to_string(),
                passed: a.get("passed").and_then(|x| x.as_bool()).unwrap_or(false),
                message: a
                    .get("message")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }
    if let Some(vars) = parsed.get("variables").and_then(|v| v.as_object()) {
        for (k, v) in vars {
            g.set_var(k, &value_to_string(v));
        }
    }
    if let Some(req) = parsed.get("request") {
        if let Some(m) = req.get("method").and_then(|x| x.as_str()) {
            g.request.method = m.to_string();
        }
        if let Some(u) = req.get("url").and_then(|x| x.as_str()) {
            g.request.url = u.to_string();
        }
        if let Some(body) = req.get("body") {
            if let Some(t) = body.get("type").and_then(|x| x.as_str()) {
                g.request.body.body_type = t.to_string();
            }
            if let Some(c) = body.get("content").and_then(|x| x.as_str()) {
                g.request.body.content = c.to_string();
            }
            if let Some(l) = body.get("language").and_then(|x| x.as_str()) {
                g.request.body.language = Some(l.to_string());
            } else if body.get("language").map(|x| x.is_null()).unwrap_or(false) {
                g.request.body.language = None;
            }
            g.request.body = g.request.body.clone().normalize();
        }
        if let Some(headers) = req.get("headers").and_then(|h| h.as_array()) {
            g.request.headers = headers
                .iter()
                .filter_map(|h| {
                    Some(crate::models::KeyValue {
                        key: h.get("key")?.as_str()?.to_string(),
                        value: h
                            .get("value")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        enabled: h.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                    })
                })
                .collect();
        }
        if let Some(query) = req.get("query").and_then(|q| q.as_array()) {
            g.request.query = query
                .iter()
                .filter_map(|q| {
                    Some(crate::models::KeyValue {
                        key: q.get("key")?.as_str()?.to_string(),
                        value: q
                            .get("value")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        enabled: q.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                    })
                })
                .collect();
        }
    }
    Ok(())
}

fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
