//! apifuzz-core — генератор тестовых запросов из OpenAPI-спецификации.
//! Поддерживается подмножество OpenAPI 3 в JSON. Без внешних зависимостей.
//! Тесты строятся только по объявленным схемам: никаких реальных ключей и обходов авторизации.

use std::collections::BTreeMap;
use std::ffi::{c_char, CStr, CString};

pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Str(String),
    Num(f64),
    Bool(bool),
    Obj(BTreeMap<String, Json>),
    Arr(Vec<Json>),
    Null,
}

fn skip_ws(bytes: &[u8], at: &mut usize) {
    while *at < bytes.len() && bytes[*at].is_ascii_whitespace() {
        *at += 1;
    }
}

fn parse_string(bytes: &[u8], at: &mut usize) -> Option<String> {
    if bytes.get(*at) != Some(&b'"') {
        return None;
    }
    *at += 1;
    let mut out = String::new();
    while *at < bytes.len() {
        let ch = bytes[*at];
        *at += 1;
        match ch {
            b'"' => return Some(out),
            b'\\' => {
                let next = *bytes.get(*at)?;
                *at += 1;
                match next {
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'u' => {
                        if *at + 4 > bytes.len() {
                            return None;
                        }
                        let hex = std::str::from_utf8(&bytes[*at..*at + 4]).ok()?;
                        let code = u32::from_str_radix(hex, 16).ok()?;
                        *at += 4;
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    }
                    _ => out.push(next as char),
                }
            }
            _ => out.push(ch as char),
        }
    }
    None
}

fn parse_value(bytes: &[u8], at: &mut usize) -> Option<Json> {
    skip_ws(bytes, at);
    match bytes.get(*at)? {
        b'"' => parse_string(bytes, at).map(Json::Str),
        b'{' => {
            *at += 1;
            let mut map = BTreeMap::new();
            loop {
                skip_ws(bytes, at);
                if bytes.get(*at) == Some(&b'}') {
                    *at += 1;
                    return Some(Json::Obj(map));
                }
                let key = parse_string(bytes, at)?;
                skip_ws(bytes, at);
                if bytes.get(*at) != Some(&b':') {
                    return None;
                }
                *at += 1;
                map.insert(key, parse_value(bytes, at)?);
                skip_ws(bytes, at);
                match bytes.get(*at) {
                    Some(b',') => *at += 1,
                    Some(b'}') => {
                        *at += 1;
                        return Some(Json::Obj(map));
                    }
                    _ => return None,
                }
            }
        }
        b'[' => {
            *at += 1;
            let mut items = Vec::new();
            loop {
                skip_ws(bytes, at);
                if bytes.get(*at) == Some(&b']') {
                    *at += 1;
                    return Some(Json::Arr(items));
                }
                items.push(parse_value(bytes, at)?);
                skip_ws(bytes, at);
                match bytes.get(*at) {
                    Some(b',') => *at += 1,
                    Some(b']') => {
                        *at += 1;
                        return Some(Json::Arr(items));
                    }
                    _ => return None,
                }
            }
        }
        b't' if bytes[*at..].starts_with(b"true") => {
            *at += 4;
            Some(Json::Bool(true))
        }
        b'f' if bytes[*at..].starts_with(b"false") => {
            *at += 5;
            Some(Json::Bool(false))
        }
        b'n' if bytes[*at..].starts_with(b"null") => {
            *at += 4;
            Some(Json::Null)
        }
        b'-' | b'0'..=b'9' => {
            let start = *at;
            while *at < bytes.len()
                && (bytes[*at].is_ascii_digit()
                    || matches!(bytes[*at], b'-' | b'+' | b'.' | b'e' | b'E'))
            {
                *at += 1;
            }
            std::str::from_utf8(&bytes[start..*at])
                .ok()?
                .parse::<f64>()
                .ok()
                .map(Json::Num)
        }
        _ => None,
    }
}

fn parse_json(input: &str) -> Option<Json> {
    let bytes = input.as_bytes();
    let mut at = 0usize;
    let value = parse_value(bytes, &mut at)?;
    skip_ws(bytes, &mut at);
    if at == bytes.len() {
        Some(value)
    } else {
        None
    }
}

pub fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 2);
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn jstr(input: &str) -> String {
    format!("\"{}\"", json_escape(input))
}

pub fn to_json(value: &Json) -> String {
    match value {
        Json::Str(s) => jstr(s),
        Json::Num(n) if n.fract() == 0.0 && n.is_finite() => format!("{}", *n as i64),
        Json::Num(n) => format!("{}", n),
        Json::Bool(b) => b.to_string(),
        Json::Null => "null".into(),
        Json::Arr(items) => format!("[{}]", items.iter().map(to_json).collect::<Vec<_>>().join(",")),
        Json::Obj(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(k, v)| format!("{}:{}", jstr(k), to_json(v)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn as_obj(value: &Json) -> Option<&BTreeMap<String, Json>> {
    match value {
        Json::Obj(map) => Some(map),
        _ => None,
    }
}

fn get<'a>(map: &'a BTreeMap<String, Json>, key: &str) -> Option<&'a Json> {
    map.get(key)
}

fn field_str(map: &BTreeMap<String, Json>, key: &str) -> String {
    match get(map, key) {
        Some(Json::Str(s)) => s.clone(),
        Some(Json::Num(n)) => format!("{}", n),
        _ => String::new(),
    }
}

fn field_array(map: &BTreeMap<String, Json>, key: &str) -> Vec<Json> {
    match get(map, key) {
        Some(Json::Arr(items)) => items.clone(),
        _ => Vec::new(),
    }
}

fn schema_type(schema: &Json) -> String {
    match as_obj(schema) {
        Some(map) => {
            let declared = field_str(map, "type");
            if !declared.is_empty() {
                declared
            } else if get(map, "properties").is_some() {
                "object".into()
            } else if get(map, "items").is_some() {
                "array".into()
            } else {
                "any".into()
            }
        }
        None => "any".into(),
    }
}

fn sample_value(schema: &Json) -> Json {
    let map = match as_obj(schema) {
        Some(map) => map,
        None => return Json::Str("sample".into()),
    };
    let enums = field_array(map, "enum");
    if let Some(first) = enums.first() {
        return first.clone();
    }
    match schema_type(schema).as_str() {
        "string" => Json::Str("sample".into()),
        "integer" => Json::Num(1.0),
        "number" => Json::Num(1.5),
        "boolean" => Json::Bool(true),
        "array" => Json::Arr(vec![]),
        "object" => {
            let mut out = BTreeMap::new();
            if let Some(props) = get(map, "properties").and_then(as_obj) {
                for (key, prop) in props {
                    out.insert(key.clone(), sample_value(prop));
                }
            }
            Json::Obj(out)
        }
        _ => Json::Str("sample".into()),
    }
}

fn invalid_values(schema: &Json) -> Vec<Json> {
    let mut out = vec![Json::Null, Json::Str("bad-value".into())];
    match schema_type(schema).as_str() {
        "string" => {
            out.push(Json::Str(String::new()));
            out.push(Json::Str("x".repeat(300)));
        }
        "integer" | "number" => {
            out.push(Json::Str("not-a-number".into()));
            out.push(Json::Num(-999999.0));
        }
        "boolean" => out.push(Json::Str("not-a-bool".into())),
        "array" => out.push(Json::Str("not-an-array".into())),
        "object" => out.push(Json::Str("not-an-object".into())),
        _ => {}
    }
    out
}

fn param_text(value: &Json) -> String {
    match value {
        Json::Str(s) => s.clone(),
        Json::Num(n) if n.fract() == 0.0 => format!("{}", *n as i64),
        Json::Num(n) => format!("{}", n),
        Json::Bool(b) => b.to_string(),
        Json::Null => String::new(),
        other => to_json(other),
    }
}

fn urlencode(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

fn render_path(template: &str, params: &BTreeMap<String, Json>) -> String {
    let mut out = template.to_string();
    for (name, value) in params {
        out = out.replace(&format!("{{{}}}", name), &urlencode(&param_text(value)));
    }
    out
}

struct Param {
    name: String,
    location: String,
    schema: Json,
}

fn collect_params(op: &BTreeMap<String, Json>) -> Vec<Param> {
    field_array(op, "parameters")
        .iter()
        .filter_map(|item| {
            let map = as_obj(item)?;
            Some(Param {
                name: field_str(map, "name"),
                location: field_str(map, "in"),
                schema: get(map, "schema").cloned().unwrap_or(Json::Null),
            })
        })
        .filter(|p| !p.name.is_empty())
        .collect()
}

fn body_schema(op: &BTreeMap<String, Json>) -> Option<Json> {
    let request = get(op, "requestBody")?;
    let request = as_obj(request)?;
    let content = get(request, "content")?;
    let content = as_obj(content)?;
    let media = get(content, "application/json")?;
    let media = as_obj(media)?;
    get(media, "schema").cloned()
}

fn expected_statuses(op: &BTreeMap<String, Json>) -> Vec<Json> {
    let responses = match get(op, "responses").and_then(as_obj) {
        Some(map) => map,
        None => return vec![Json::Num(200.0)],
    };
    let mut values: Vec<Json> = responses
        .keys()
        .filter_map(|key| key.parse::<f64>().ok())
        .map(Json::Num)
        .collect();
    if values.is_empty() {
        values.push(Json::Num(200.0));
    }
    values
}

type Case = BTreeMap<String, Json>;

fn make_case(
    name: String,
    method: &str,
    path: &str,
    query: String,
    body: String,
    kind: &str,
    expected: &[Json],
) -> Case {
    let mut case = BTreeMap::new();
    case.insert("name".into(), Json::Str(name));
    case.insert("method".into(), Json::Str(method.to_uppercase()));
    case.insert("path".into(), Json::Str(path.into()));
    case.insert("query".into(), Json::Str(query));
    case.insert("body".into(), Json::Str(body));
    case.insert("kind".into(), Json::Str(kind.into()));
    case.insert("expected_status".into(), Json::Arr(expected.to_vec()));
    case
}

fn build_query(params: &[Param], values: &BTreeMap<String, Json>) -> String {
    let mut parts = Vec::new();
    for param in params {
        if param.location == "query" {
            if let Some(value) = values.get(&param.name) {
                parts.push(format!("{}={}", urlencode(&param.name), urlencode(&param_text(value))));
            }
        }
    }
    parts.join("&")
}

fn generate_operation(
    cases: &mut Vec<Case>,
    method: &str,
    template: &str,
    op: &BTreeMap<String, Json>,
) {
    let params = collect_params(op);
    let expected = expected_statuses(op);
    let schema = body_schema(op);

    let mut baseline = BTreeMap::new();
    for param in &params {
        baseline.insert(param.name.clone(), sample_value(&param.schema));
    }
    let baseline_body = match &schema {
        Some(value) => to_json(&sample_value(value)),
        None => String::new(),
    };
    let path = render_path(template, &baseline);
    let query = build_query(&params, &baseline);
    cases.push(make_case(
        format!("{} {} baseline", method.to_uppercase(), template),
        method,
        &path,
        query.clone(),
        baseline_body.clone(),
        "valid",
        &expected,
    ));

    for param in &params {
        for invalid in invalid_values(&param.schema) {
            let mut values = baseline.clone();
            values.insert(param.name.clone(), invalid);
            cases.push(make_case(
                format!("{} {} param {}", method.to_uppercase(), template, param.name),
                method,
                &render_path(template, &values),
                build_query(&params, &values),
                baseline_body.clone(),
                "param_invalid",
                &expected,
            ));
        }
        if let Some(map) = as_obj(&param.schema) {
            for value in field_array(map, "enum") {
                let mut values = baseline.clone();
                values.insert(param.name.clone(), value);
                cases.push(make_case(
                    format!("{} {} enum {}", method.to_uppercase(), template, param.name),
                    method,
                    &render_path(template, &values),
                    build_query(&params, &values),
                    baseline_body.clone(),
                    "enum",
                    &expected,
                ));
            }
        }
    }

    if let Some(schema) = schema {
        if let Some(props) = as_obj(&schema).and_then(|m| get(m, "properties")).and_then(as_obj) {
            let mut base = match as_obj(&sample_value(&schema)) {
                Some(map) => map.clone(),
                None => BTreeMap::new(),
            };
            for (name, prop) in props {
                for invalid in invalid_values(prop) {
                    let mut mutated = base.clone();
                    mutated.insert(name.clone(), invalid);
                    cases.push(make_case(
                        format!("{} {} body {}", method.to_uppercase(), template, name),
                        method,
                        &path,
                        query.clone(),
                        to_json(&Json::Obj(mutated)),
                        "body_invalid",
                        &expected,
                    ));
                }
                if let Some(map) = as_obj(prop) {
                    for value in field_array(map, "enum") {
                        let mut mutated = base.clone();
                        mutated.insert(name.clone(), value);
                        cases.push(make_case(
                            format!("{} {} body enum {}", method.to_uppercase(), template, name),
                            method,
                            &path,
                            query.clone(),
                            to_json(&Json::Obj(mutated)),
                            "enum",
                            &expected,
                        ));
                    }
                }
            }
            if let Some(required) = as_obj(&schema).and_then(|m| get(m, "required")) {
                if let Json::Arr(names) = required {
                    if let Some(Json::Str(name)) = names.first() {
                        base.remove(name);
                        cases.push(make_case(
                            format!("{} {} body missing {}", method.to_uppercase(), template, name),
                            method,
                            &path,
                            query,
                            to_json(&Json::Obj(base)),
                            "body_missing",
                            &expected,
                        ));
                    }
                }
            }
        }
    }
}

/// Возвращает JSON с массивом сгенерированных тестовых случаев.
pub fn generate(spec_input: &str) -> Result<String, String> {
    let Some(spec) = parse_json(spec_input) else {
        return Err("invalid specification JSON".into());
    };
    let paths = match get(as_obj(&spec).ok_or("spec must be an object")?, "paths").and_then(as_obj) {
        Some(map) => map,
        None => return Err("spec has no paths object".into()),
    };

    let mut cases = Vec::new();
    let mut operations = 0usize;
    for (template, item) in paths {
        let Some(item) = as_obj(item) else { continue };
        for method in ["get", "post", "put", "patch", "delete"] {
            if let Some(op) = get(item, method).and_then(as_obj) {
                operations += 1;
                generate_operation(&mut cases, method, template, op);
            }
        }
    }

    let mut out = BTreeMap::new();
    out.insert("version".into(), Json::Str(VERSION.into()));
    out.insert("operations".into(), Json::Num(operations as f64));
    out.insert("cases".into(), Json::Arr(cases.into_iter().map(Json::Obj).collect()));
    Ok(to_json(&Json::Obj(out)))
}

fn read_input(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().ok().map(str::to_string)
}

fn error(message: &str) -> String {
    format!("{{\"error\":\"{}\"}}", json_escape(message))
}

fn to_c(value: String) -> *mut c_char {
    CString::new(value)
        .unwrap_or_else(|_| CString::new("{\"error\":\"invalid string\"}").unwrap())
        .into_raw()
}

#[no_mangle]
pub extern "C" fn apifuzz_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn apifuzz_generate(spec_ptr: *const c_char) -> *mut c_char {
    let result = match read_input(spec_ptr) {
        Some(value) => generate(&value).unwrap_or_else(|e| error(&e)),
        None => error("null input"),
    };
    to_c(result)
}

#[no_mangle]
pub extern "C" fn apifuzz_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> &'static str {
        r#"{
          "openapi":"3.0.3",
          "info":{"title":"demo","version":"1.0.0"},
          "paths":{
            "/users/{id}":{"get":{"parameters":[
              {"name":"id","in":"path","required":true,"schema":{"type":"string"}},
              {"name":"page","in":"query","required":false,"schema":{"type":"integer"}}
            ],"responses":{"200":{},"404":{}}}},
            "/users":{"post":{"requestBody":{"content":{"application/json":{"schema":{
              "type":"object","properties":{
                "name":{"type":"string"},
                "role":{"type":"string","enum":["admin","user"]}
              },"required":["name"]}}}},"responses":{"201":{},"400":{}}}}
          }
        }"#
    }

    fn cases() -> Vec<Case> {
        let raw = generate(spec()).unwrap();
        let value = parse_json(&raw).unwrap();
        let map = as_obj(&value).unwrap();
        match get(map, "cases") {
            Some(Json::Arr(items)) => items.iter().filter_map(as_obj).cloned().collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn generates_baseline_for_each_operation() {
        let cases = cases();
        assert!(cases.iter().any(|c| field_str(c, "method") == "GET" && field_str(c, "kind") == "valid"));
        assert!(cases.iter().any(|c| field_str(c, "method") == "POST" && field_str(c, "kind") == "valid"));
    }

    #[test]
    fn renders_path_and_query() {
        let cases = cases();
        let baseline = cases.iter().find(|c| field_str(c, "method") == "GET" && field_str(c, "kind") == "valid").unwrap();
        assert_eq!(field_str(baseline, "path"), "/users/sample");
        assert_eq!(field_str(baseline, "query"), "page=1");
    }

    #[test]
    fn generates_invalid_and_enum_cases() {
        let cases = cases();
        assert!(cases.iter().any(|c| field_str(c, "kind") == "param_invalid"));
        assert!(cases.iter().any(|c| field_str(c, "kind") == "body_invalid"));
        assert!(cases.iter().any(|c| field_str(c, "kind") == "body_missing"));
        assert!(cases.iter().any(|c| field_str(c, "kind") == "enum"));
    }

    #[test]
    fn invalid_spec_is_error() {
        assert!(generate("not json").is_err());
        assert!(generate("{}").is_err());
    }
}
