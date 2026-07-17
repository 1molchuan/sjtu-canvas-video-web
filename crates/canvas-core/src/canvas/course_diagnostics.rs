use std::collections::BTreeSet;

use reqwest::{Response, header::CONTENT_TYPE};
use serde_json::Value;

pub(super) struct RestProbeMetadata {
    status: u16,
    host: String,
    content_type: String,
    redirected: bool,
    declared_length: Option<u64>,
}

impl RestProbeMetadata {
    pub(super) fn from_response(response: &Response) -> Self {
        Self {
            status: response.status().as_u16(),
            host: response.url().host_str().unwrap_or("unknown").to_owned(),
            content_type: response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("unknown")
                .to_owned(),
            redirected: response.status().is_redirection(),
            declared_length: response.content_length(),
        }
    }

    pub(super) fn record(
        &self,
        actual_length: Option<usize>,
        structure: &str,
        course_count: Option<usize>,
    ) {
        tracing::info!(
            operation = "canvas_course_discovery",
            method = "GET",
            host = %self.host,
            path_template = "/api/v1/courses",
            status = self.status,
            content_type = %self.content_type,
            redirected = self.redirected,
            declared_length = ?self.declared_length,
            response_length = ?actual_length,
            response_structure = structure,
            course_count = ?course_count,
            "Canvas course discovery response classified"
        );
    }
}

pub(super) fn summarize_response_structure(body: &[u8]) -> String {
    if looks_like_html(body) {
        return "html_like".to_owned();
    }

    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(object)) => summarize_object_keys(object.keys()),
        Ok(Value::Array(items)) => summarize_array(&items),
        Ok(Value::Null) => "json_null".to_owned(),
        Ok(value) => format!("json_scalar(type={})", value_kind(&value)),
        Err(_) => "unrecognized".to_owned(),
    }
}

fn looks_like_html(body: &[u8]) -> bool {
    let prefix_len = body.len().min(256);
    let prefix = String::from_utf8_lossy(&body[..prefix_len]).to_ascii_lowercase();
    let trimmed = prefix.trim_start();
    trimmed.starts_with("<!doctype html") || trimmed.starts_with("<html")
}

fn summarize_object_keys<'a>(keys: impl Iterator<Item = &'a String>) -> String {
    let safe_keys = keys
        .filter(|key| is_safe_key(key))
        .take(16)
        .cloned()
        .collect::<BTreeSet<_>>();
    let summary = if safe_keys.is_empty() {
        "none".to_owned()
    } else {
        safe_keys.iter().cloned().collect::<Vec<_>>().join(",")
    };
    format!("json_object(keys={summary})")
}

fn summarize_array(items: &[Value]) -> String {
    let mut parts = Vec::new();
    for field in ["id", "name", "course_code", "term", "term.name"] {
        let types = collect_field_types(items, field);
        parts.push(format!("{field}={}", join_or_none(&types)));
    }
    format!("json_array(len={},types={})", items.len(), parts.join(";"))
}

fn collect_field_types(items: &[Value], field: &str) -> BTreeSet<String> {
    items
        .iter()
        .map(|item| field_value(item, field).map_or("missing", value_kind))
        .map(str::to_owned)
        .collect()
}

fn field_value<'a>(item: &'a Value, field: &str) -> Option<&'a Value> {
    if field == "term.name" {
        return item.get("term").and_then(|term| term.get("name"));
    }
    item.get(field)
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn is_safe_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_.-".contains(character))
}

fn join_or_none(values: &BTreeSet<String>) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values.iter().cloned().collect::<Vec<_>>().join("|")
}

#[cfg(test)]
mod tests {
    use super::summarize_response_structure;

    #[test]
    fn response_structure_summary_excludes_json_values() {
        let body = br#"{"courses":"private-course","meta":"private-account"}"#;

        let summary = summarize_response_structure(body);

        assert_eq!(summary, "json_object(keys=courses,meta)");
        assert!(!summary.contains("private-course"));
        assert!(!summary.contains("private-account"));
    }

    #[test]
    fn response_structure_summary_does_not_echo_html() {
        let body = b"<!doctype html><title>private account</title>";

        let summary = summarize_response_structure(body);

        assert_eq!(summary, "html_like");
        assert!(!summary.contains("private account"));
    }

    #[test]
    fn course_array_summary_reports_types_without_values() {
        let body = br#"[
            {"id":123,"name":"private-name","course_code":"private-code","term":null},
            {"id":"456","name":null,"term":{"name":"private-term"}}
        ]"#;

        let summary = summarize_response_structure(body);

        assert!(summary.contains("id=number|string"));
        assert!(summary.contains("name=null|string"));
        assert!(summary.contains("course_code=missing|string"));
        assert!(summary.contains("term=null|object"));
        assert!(summary.contains("term.name=missing|string"));
        assert!(!summary.contains("private"));
    }
}
