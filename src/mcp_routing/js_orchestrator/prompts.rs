use serde_json::Value;

/// Build a schema correction prompt snippet that pairs workflow JS code with the
/// current JSON schema. This is used when automated correction fails and we need
/// to surface a ready-to-send prompt to an LLM or human reviewer.
pub fn build_schema_correction_prompt(js_code: &str, current_schema: &Value) -> String {
    let schema_pretty =
        serde_json::to_string_pretty(current_schema).unwrap_or_else(|_| "{}".to_string());
    format!(
        r#"You are Agentic-Warden's MCP schema auditor. Fix the workflow input schema so it matches the code signature.

javascript
{js_code}
json
{schema}
"#,
        js_code = js_code.trim(),
        schema = schema_pretty
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_prompt_with_code_and_schema() {
        let prompt = build_schema_correction_prompt(
            "async function workflow() {}",
            &json!({"type": "object"}),
        );
        assert!(prompt.contains("async function workflow"));
        assert!(prompt.contains(r#""type": "object""#));
        assert!(prompt.contains("schema auditor"));
    }
}
