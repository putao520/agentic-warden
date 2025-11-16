use serde_json::{Map, Value};

/// Validation outcome for a generated JSON schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl SchemaValidationResult {
    fn from(errors: Vec<String>, warnings: Vec<String>) -> Self {
        Self {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

/// Lightweight JSON schema validator tailored for workflow input schemas.
pub struct SchemaValidator;

impl SchemaValidator {
    /// Validate that the schema has a proper object root and well-formed properties/required fields.
    pub fn validate(schema: &Value) -> SchemaValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        let Some(root_map) = schema.as_object() else {
            errors.push("Root schema must be a JSON object".into());
            return SchemaValidationResult::from(errors, warnings);
        };

        Self::validate_root_type(root_map, &mut errors);
        let properties = Self::validate_properties(root_map, &mut errors, &mut warnings);
        Self::validate_required(root_map, &properties, &mut errors, &mut warnings);

        if properties.is_empty() {
            warnings.push("Schema has no input properties defined".into());
        }

        SchemaValidationResult::from(errors, warnings)
    }

    fn validate_root_type(root: &Map<String, Value>, errors: &mut Vec<String>) {
        match root.get("type") {
            Some(Value::String(kind)) if kind == "object" => {}
            Some(Value::String(other)) => {
                errors.push(format!("Schema root must be type=object, found '{other}'"))
            }
            Some(_) => errors.push("Schema root type must be a string literal".into()),
            None => errors.push("Schema root missing 'type' field".into()),
        }
    }

    fn validate_properties(
        root: &Map<String, Value>,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) -> Map<String, Value> {
        let Some(raw) = root.get("properties") else {
            warnings.push("Schema missing 'properties'; defaulting to empty".into());
            return Map::new();
        };

        let Value::Object(map) = raw else {
            errors.push("Schema 'properties' must be an object map".into());
            return Map::new();
        };

        let mut validated = Map::new();
        for (name, value) in map {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                warnings.push("Encountered property with empty name; skipping".into());
                continue;
            }
            match value {
                Value::Object(prop_obj) => {
                    Self::validate_property_type(trimmed, prop_obj, errors, warnings);
                    validated.insert(trimmed.to_string(), Value::Object(prop_obj.clone()));
                }
                _ => errors.push(format!(
                    "Schema property '{}' must be an object with at least a 'type' field",
                    trimmed
                )),
            }
        }

        validated
    }

    fn validate_property_type(
        name: &str,
        prop: &Map<String, Value>,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        match prop.get("type") {
            Some(Value::String(kind)) => {
                let allowed = ["string", "number", "boolean", "object", "array", "integer"];
                if !allowed.contains(&kind.as_str()) {
                    errors.push(format!(
                        "Property '{}' has unsupported type '{}'",
                        name, kind
                    ));
                }
            }
            Some(_) => errors.push(format!("Property '{}' type must be a string literal", name)),
            None => warnings.push(format!(
                "Property '{}' missing type; defaulting to string during correction",
                name
            )),
        }
    }

    fn validate_required(
        root: &Map<String, Value>,
        properties: &Map<String, Value>,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        let Some(raw_required) = root.get("required") else {
            return;
        };

        let Value::Array(entries) = raw_required else {
            errors.push("Schema 'required' must be an array of property names".into());
            return;
        };

        for entry in entries {
            match entry {
                Value::String(name) => {
                    if !properties.contains_key(name) {
                        warnings.push(format!(
                            "Required field '{}' not present in properties",
                            name
                        ));
                    }
                }
                _ => errors.push("Entries in 'required' must be strings".into()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_minimal_object_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        });

        let result = SchemaValidator::validate(&schema);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn detects_invalid_root_and_properties() {
        let schema = json!({
            "type": "array",
            "properties": "invalid",
            "required": [1, 2]
        });

        let result = SchemaValidator::validate(&schema);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("type=object")));
        assert!(result.errors.iter().any(|e| e.contains("properties")));
        assert!(result.errors.iter().any(|e| e.contains("required")));
    }

    #[test]
    fn warns_on_missing_type_and_required_mismatch() {
        let schema = json!({
            "type": "object",
            "properties": {
                "repo": {}
            },
            "required": ["missing"]
        });

        let result = SchemaValidator::validate(&schema);
        assert!(result.is_valid);
        assert!(result.warnings.iter().any(|w| w.contains("missing type")));
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("not present in properties")));
    }
}
