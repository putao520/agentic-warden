use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::sync::Arc;

use crate::mcp_routing::decision::DecisionEngine;

use super::{prompts::build_schema_correction_prompt, schema_validator::SchemaValidator};

/// Result of schema correction with applied fixes.
#[derive(Debug, Clone)]
pub struct SchemaCorrectionResult {
    pub schema: Value,
    pub iterations: usize,
    pub applied_fixes: Vec<String>,
    pub warnings: Vec<String>,
}

/// Attempts to iteratively correct invalid workflow input schemas.
pub struct SchemaCorrector;

impl SchemaCorrector {
    /// Validate and, if needed, correct the provided schema using hints from the workflow JS code.
    pub fn correct(js_code: &str, schema: Value) -> Result<SchemaCorrectionResult> {
        let initial_validation = SchemaValidator::validate(&schema);
        if initial_validation.is_valid {
            return Ok(SchemaCorrectionResult {
                schema,
                iterations: 1,
                applied_fixes: Vec::new(),
                warnings: initial_validation.warnings,
            });
        }

        let mut applied_fixes = Vec::new();
        let mut candidate = Self::normalize_root(schema, &mut applied_fixes);
        let inferred_fields = Self::infer_fields_from_js(js_code);
        Self::merge_inferred_fields(&mut candidate, &inferred_fields, &mut applied_fixes);

        let second_pass = SchemaValidator::validate(&candidate);
        if second_pass.is_valid {
            return Ok(SchemaCorrectionResult {
                schema: candidate,
                iterations: 2,
                applied_fixes,
                warnings: second_pass.warnings,
            });
        }

        // Final fallback: rebuild schema purely from inferred fields.
        let fallback = Self::build_fallback_schema(&inferred_fields);
        let final_validation = SchemaValidator::validate(&fallback);
        if final_validation.is_valid {
            return Ok(SchemaCorrectionResult {
                schema: fallback,
                iterations: 3,
                applied_fixes,
                warnings: final_validation.warnings,
            });
        }

        let prompt = build_schema_correction_prompt(js_code, &fallback);
        Err(anyhow!(
            "Schema validation failed after correction attempts: {}. Suggested prompt:\n{}",
            final_validation.errors.join("; "),
            prompt
        ))
    }

    fn normalize_root(schema: Value, applied_fixes: &mut Vec<String>) -> Value {
        let mut root = match schema {
            Value::Object(map) => map,
            _ => {
                applied_fixes
                    .push("Reset schema root to object because previous root was invalid".into());
                Map::new()
            }
        };

        let root_type = root
            .get("type")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        if root_type.as_deref() != Some("object") {
            applied_fixes.push("Enforced root type to 'object'".into());
            root.insert("type".into(), Value::String("object".into()));
        }

        let properties = match root.remove("properties") {
            Some(Value::Object(map)) => map,
            Some(_) => {
                applied_fixes.push("Rebuilt 'properties' as an object map".into());
                Map::new()
            }
            None => {
                applied_fixes.push("Added empty 'properties' to schema".into());
                Map::new()
            }
        };

        let required = match root.remove("required") {
            Some(Value::Array(values)) => values,
            _ => Vec::new(),
        };

        root.insert("properties".into(), Value::Object(properties));
        root.insert("required".into(), Value::Array(required));

        Value::Object(root)
    }

    fn infer_fields_from_js(code: &str) -> Vec<String> {
        static INPUT_FIELD: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"input\.([A-Za-z_][A-Za-z0-9_]*)").unwrap());

        let mut fields = HashSet::new();
        for capture in INPUT_FIELD.captures_iter(code) {
            if let Some(name) = capture.get(1) {
                fields.insert(name.as_str().to_string());
            }
        }
        let mut ordered: Vec<String> = fields.into_iter().collect();
        ordered.sort();
        ordered
    }

    fn merge_inferred_fields(
        schema: &mut Value,
        fields: &[String],
        applied_fixes: &mut Vec<String>,
    ) {
        let Some(root) = schema.as_object_mut() else {
            return;
        };
        if !root
            .get("properties")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            root.insert("properties".into(), Value::Object(Map::new()));
        }

        let Some(properties) = root.get_mut("properties").and_then(Value::as_object_mut) else {
            return;
        };

        let mut required = Vec::new();
        for field in fields {
            let entry = properties.entry(field.clone()).or_insert_with(|| {
                json!({
                    "type": "string",
                    "description": format!("Inferred from workflow code: input.{field}")
                })
            });
            if entry.get("type").is_none() {
                if let Some(obj) = entry.as_object_mut() {
                    obj.insert("type".into(), Value::String("string".into()));
                }
            }
            required.push(Value::String(field.clone()));
        }

        if !required.is_empty() {
            applied_fixes.push(format!(
                "Aligned 'required' fields with workflow usage: {}",
                fields.join(", ")
            ));
            root.insert("required".into(), Value::Array(required));
        }
    }

    fn build_fallback_schema(fields: &[String]) -> Value {
        let mut properties = Map::new();
        for field in fields {
            properties.insert(
                field.clone(),
                json!({"type": "string", "description": format!("Auto-generated for {field}")}),
            );
        }
        let required: Vec<Value> = fields.iter().cloned().map(Value::String).collect();

        json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }
}

/// Iterative schema fixer with LLM correction loop.
pub struct IterativeSchemaFixer {
    decision_engine: Arc<DecisionEngine>,
    max_iterations: usize,
}

impl IterativeSchemaFixer {
    pub fn new(decision_engine: Arc<DecisionEngine>) -> Self {
        Self {
            decision_engine,
            max_iterations: 3,
        }
    }

    /// Fix schema using iterative validation loop:
    /// 1. Auto-fix -> Validate
    /// 2. If fails -> LLM correct + Auto-fix -> Validate
    /// 3. Loop until valid or max iterations
    pub async fn fix_schema_with_retry(
        &self,
        tool_name: &str,
        description: &str,
        js_code: &str,
        initial_schema: Value,
    ) -> Result<Value> {
        let mut current_schema = initial_schema;

        for iteration in 0..self.max_iterations {
            eprintln!(
                "🔄 Schema correction iteration {}/{}",
                iteration + 1,
                self.max_iterations
            );

            let corrected = SchemaCorrector::correct(js_code, current_schema.clone())?;
            let validation = SchemaValidator::validate(&corrected.schema);

            if validation.is_valid {
                eprintln!(
                    "✅ Schema validation passed after {} iteration(s)",
                    iteration + 1
                );
                if !validation.warnings.is_empty() {
                    eprintln!(
                        "⚠️  Schema warnings after correction: {:?}",
                        validation.warnings
                    );
                }
                return Ok(corrected.schema);
            }

            eprintln!("⚠️  Validation errors: {:?}", validation.errors);

            let llm_corrected = match self
                .llm_correct_schema(
                    tool_name,
                    description,
                    js_code,
                    &corrected.schema,
                    &validation.errors,
                )
                .await
            {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("⚠️  LLM schema correction failed: {}", err);
                    current_schema = corrected.schema;
                    continue;
                }
            };

            let combined = SchemaCorrector::correct(js_code, llm_corrected)?;
            let revalidation = SchemaValidator::validate(&combined.schema);

            if revalidation.is_valid {
                eprintln!("✅ Schema validation passed after LLM correction");
                if !revalidation.warnings.is_empty() {
                    eprintln!(
                        "⚠️  Schema warnings after correction: {:?}",
                        revalidation.warnings
                    );
                }
                return Ok(combined.schema);
            }

            eprintln!(
                "⚠️  Validation errors after LLM correction: {:?}",
                revalidation.errors
            );
            current_schema = combined.schema;
        }

        Err(anyhow!(
            "Failed to fix schema after {} iterations. Last errors: {:?}",
            self.max_iterations,
            SchemaValidator::validate(&current_schema).errors
        ))
    }

    async fn llm_correct_schema(
        &self,
        tool_name: &str,
        description: &str,
        js_code: &str,
        current_schema: &Value,
        validation_errors: &[String],
    ) -> Result<Value> {
        let mut prompt = build_schema_correction_prompt(js_code, current_schema);

        if !tool_name.trim().is_empty() || !description.trim().is_empty() {
            prompt.push_str("\nWorkflow context:\n");
            if !tool_name.trim().is_empty() {
                prompt.push_str(&format!("Name: {}\n", tool_name.trim()));
            }
            if !description.trim().is_empty() {
                prompt.push_str(&format!("Description: {}\n", description.trim()));
            }
        }

        if !validation_errors.is_empty() {
            prompt.push_str("\nValidation errors:\n");
            for error in validation_errors {
                prompt.push_str("- ");
                prompt.push_str(error);
                prompt.push('\n');
            }
        }

        let response = self.call_llm_for_schema_correction(&prompt).await?;

        serde_json::from_str::<Value>(&response)
            .map_err(|e| anyhow!("LLM returned invalid JSON schema: {}", e))
    }

    async fn call_llm_for_schema_correction(&self, prompt: &str) -> Result<String> {
        let system_prompt =
            "You are Agentic-Warden's schema corrector. Return ONLY the corrected JSON schema.";
        self.decision_engine
            .chat_completion(system_prompt, prompt)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrects_invalid_schema_using_inferred_fields() {
        let js_code = r#"
        async function workflow(input) {
            await mcp.call("fs", "read_file", { path: input.path });
            return input.path;
        }"#;

        let schema = json!("invalid");
        let result = SchemaCorrector::correct(js_code, schema).expect("schema corrected");
        assert_eq!(result.iterations, 2);
        assert!(result
            .schema
            .get("properties")
            .and_then(Value::as_object)
            .unwrap()
            .contains_key("path"));
    }

    #[test]
    fn produces_valid_schema_when_no_fields_inferred() {
        let js_code = "async function workflow() { return true; }";
        let schema = json!(null);

        let result = SchemaCorrector::correct(js_code, schema).expect("schema corrected");
        assert!(result.schema.get("type").is_some());
        assert!(SchemaValidator::validate(&result.schema).is_valid);
    }
}
