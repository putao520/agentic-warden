//! JavaScript Code Validator
//!
//! Multi-layer validation: syntax check + security check + dry-run test.

use anyhow::{anyhow, Result};
use boa_engine::{Context, Source};
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::runtime::{Builder, Handle};

use super::engine::BoaRuntime;

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    fn success() -> Self {
        Self {
            passed: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn failure(errors: Vec<String>) -> Self {
        Self {
            passed: false,
            errors,
            warnings: Vec::new(),
        }
    }
}

/// JavaScript code validator
pub struct JsCodeValidator;

impl JsCodeValidator {
    /// Validate JavaScript code
    ///
    /// Performs three checks:
    /// 1. Syntax check (using Boa parser)
    /// 2. Security check (dangerous patterns)
    /// 3. Dry-run test (optional, with mock data)
    pub fn validate(code: &str) -> Result<ValidationResult> {
        // Step 1: Syntax check
        if let Err(e) = Self::check_syntax(code) {
            return Ok(ValidationResult::failure(vec![format!(
                "Syntax error: {}",
                e
            )]));
        }

        // Step 2: Security check
        if let Err(e) = Self::check_security(code) {
            return Ok(ValidationResult::failure(vec![format!(
                "Security violation: {}",
                e
            )]));
        }

        // Step 3: Dry-run test with mock MCP functions
        if let Err(e) = Self::perform_dry_run(code) {
            return Ok(ValidationResult::failure(vec![format!(
                "Dry-run failed: {}",
                e
            )]));
        }

        Ok(ValidationResult::success())
    }

    /// Check JavaScript syntax using Boa parser
    fn check_syntax(code: &str) -> Result<()> {
        let mut context = Context::default();
        // Try to eval the code - syntax errors will be caught
        // Note: This doesn't execute the code in a meaningful way,
        // just validates the syntax
        let _ = context
            .eval(Source::from_bytes(code))
            .map_err(|e| anyhow!("Syntax error: {}", e))?;

        Ok(())
    }

    /// Check for dangerous JavaScript patterns
    fn check_security(code: &str) -> Result<()> {
        static DANGEROUS_PATTERNS: Lazy<Vec<(&str, Regex)>> = Lazy::new(|| {
            vec![
                ("eval usage", Regex::new(r"\beval\s*\(").unwrap()),
                (
                    "Function constructor",
                    Regex::new(r"\bFunction\s*\(").unwrap(),
                ),
                ("__proto__ manipulation", Regex::new(r"__proto__").unwrap()),
                (
                    "constructor access",
                    Regex::new(r"\.constructor\s*\(").unwrap(),
                ),
                ("require usage", Regex::new(r"\brequire\s*\(").unwrap()),
                ("import usage", Regex::new(r"\bimport\s+").unwrap()),
            ]
        });

        let mut violations = Vec::new();

        for (name, pattern) in DANGEROUS_PATTERNS.iter() {
            if pattern.is_match(code) {
                violations.push(name.to_string());
            }
        }

        if !violations.is_empty() {
            return Err(anyhow!(
                "Dangerous patterns detected: {}",
                violations.join(", ")
            ));
        }

        Ok(())
    }

    fn perform_dry_run(code: &str) -> Result<()> {
        if Handle::try_current().is_ok() {
            let owned = code.to_owned();
            std::thread::spawn(move || -> Result<()> {
                Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| anyhow!("Failed to build tokio runtime for dry-run: {e}"))?
                    .block_on(Self::dry_run_async(owned))
            })
            .join()
            .map_err(|_| anyhow!("Dry-run thread panicked"))??;
            return Ok(());
        }

        let owned = code.to_owned();
        Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| anyhow!("Failed to build tokio runtime for dry-run: {e}"))?
            .block_on(Self::dry_run_async(owned))
    }

    async fn dry_run_async(code: String) -> Result<()> {
        let runtime = BoaRuntime::new()?;
        let context_code = code.clone();
        runtime
            .with_context(move |ctx| {
                Self::inject_mock_functions(ctx, &context_code)?;
                Ok(())
            })
            .await?;

        // Replace workflow() call with workflow(mockInput) for dry-run
        let validation_code = Self::add_mock_input_to_workflow(&code);
        runtime.execute(&validation_code).await.map(|_| ())
    }

    /// Replace workflow() call with workflow({mock data}) for validation
    /// Generated workflow functions expect an input parameter
    fn add_mock_input_to_workflow(code: &str) -> String {
        static WORKFLOW_CALL: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"workflow\s*\(\s*\)\s*;").unwrap());

        let mock_input = r#"{
            repo_url: "https://github.com/test/repo",
            repo_path: "/tmp/test",
            format: "json",
            path: "/tmp/file.txt",
            content: "mock content"
        }"#;

        WORKFLOW_CALL
            .replace(code, &format!("workflow({});", mock_input))
            .to_string()
    }

    fn inject_mock_functions(context: &mut Context, _code: &str) -> Result<()> {
        let script = r#"
            globalThis.mcp = {
                async call(server, tool, args) {
                    return {
                        mock: true,
                        server: server || "unknown",
                        tool: tool || "unknown",
                        args: args || {}
                    };
                }
            };
        "#;
        context
            .eval(Source::from_bytes(script.as_bytes()))
            .map_err(|e| anyhow!("Failed to inject mock MCP object: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_code() {
        let code = r#"
            async function workflow(input) {
                const result = await mcp.call("fs", "git_status", input);
                return result;
            }
        "#;

        let result = JsCodeValidator::validate(code);
        assert!(result.is_ok());
        assert!(result.unwrap().passed);
    }

    #[test]
    fn test_syntax_error() {
        let code = "function broken( {";

        let result = JsCodeValidator::validate(code);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(!validation.passed);
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_eval_detected() {
        let code = r#"
            function bad() {
                eval("console.log('danger')");
            }
        "#;

        let result = JsCodeValidator::validate(code);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(!validation.passed);
        assert!(validation.errors[0].contains("eval"));
    }

    #[test]
    fn test_function_constructor_detected() {
        let code = r#"
            const fn = new Function('return 42');
        "#;

        let result = JsCodeValidator::validate(code);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(!validation.passed);
    }
}
