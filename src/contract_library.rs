// Persistent Rholang contract templates

/// Persistent contract template library
///
/// Provides a complete RGB20 contract template for deployment with insertSigned.
/// The template includes all methods (issue, transfer, balanceOf, totalSupply, metadata).
pub struct RholangContractLibrary;

impl RholangContractLibrary {
    /// Get the complete RGB20 persistent contract template (Pattern B)
    ///
    /// This is a full contract with all methods that should be deployed ONCE
    /// using insertSigned to get a persistent registry URI.
    ///
    /// Required variables: TICKER, NAME, TOTAL_SUPPLY, PRECISION
    ///
    /// Token Model:
    /// - Fixed supply set at deployment (never changes)
    /// - Issue allocates tokens from unallocated supply pool
    /// - Transfer moves tokens between addresses
    /// - All amounts must be positive (validated in contract)
    ///
    /// Methods in contract:
    /// - issue: Allocate tokens from unallocated supply
    /// - transfer: Transfer tokens between addresses
    /// - balanceOf: Query balance (returns 0 for new addresses)
    /// - totalSupply: Query total supply (fixed)
    /// - unallocatedSupply: Query remaining unallocated supply
    /// - getMetadata: Query contract metadata
    pub fn rho20_contract() -> &'static str {
        include_str!("templates/rho20_contract.rho")
    }

    /// Substitute variables in template
    ///
    /// Replaces {{KEY}} placeholders with provided values.
    pub fn substitute(template: &str, vars: &[(&str, &str)]) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }

    /// Validate that all template variables have been substituted
    ///
    /// Returns Ok(()) if no {{...}} placeholders remain,
    /// otherwise returns list of unsubstituted variables.
    pub fn validate(template: &str, vars: &[(&str, &str)]) -> Result<(), Vec<String>> {
        let result = Self::substitute(template, vars);
        let remaining = Self::extract_unsubstituted(&result);

        if remaining.is_empty() {
            Ok(())
        } else {
            Err(remaining)
        }
    }

    /// Extract list of unsubstituted variables from text
    fn extract_unsubstituted(text: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'{') {
                chars.next();
                let mut var_name = String::new();

                while let Some(c) = chars.next() {
                    if c == '}' && chars.peek() == Some(&'}') {
                        chars.next();
                        vars.push(var_name);
                        break;
                    }
                    var_name.push(c);
                }
            }
        }

        vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_basic() {
        let template = "Hello {{NAME}}!";
        let vars = [("NAME", "World")];
        let result = RholangContractLibrary::substitute(template, &vars);
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_substitute_multiple() {
        let template = "{{A}} + {{B}} = {{C}}";
        let vars = [("A", "1"), ("B", "2"), ("C", "3")];
        let result = RholangContractLibrary::substitute(template, &vars);
        assert_eq!(result, "1 + 2 = 3");
    }

    #[test]
    fn test_substitute_contract_variables() {
        let contract = RholangContractLibrary::rho20_contract();
        let vars = [
            ("TICKER", "TEST"),
            ("NAME", "Test Token"),
            ("TOTAL_SUPPLY", "1000000"),
            ("PRECISION", "8"),
        ];

        let result = RholangContractLibrary::substitute(contract, &vars);

        assert!(result.contains("TEST"));
        assert!(result.contains("Test Token"));
        assert!(result.contains("1000000"));
        assert!(result.contains("8"));
        assert!(!result.contains("{{TICKER}}"));
        assert!(!result.contains("{{NAME}}"));
        assert!(!result.contains("{{TOTAL_SUPPLY}}"));
        assert!(!result.contains("{{PRECISION}}"));
    }

    #[test]
    fn test_validate_success() {
        let template = "Hello {{NAME}}!";
        let vars = [("NAME", "Alice")];
        let result = RholangContractLibrary::validate(template, &vars);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_failure() {
        let template = "Hello {{NAME}}, welcome to {{CITY}}!";
        let vars = [("NAME", "Alice")];
        let result = RholangContractLibrary::validate(template, &vars);
        assert!(result.is_err());
        let missing = result.unwrap_err();
        assert_eq!(missing, vec!["CITY".to_string()]);
    }

    #[test]
    fn test_validate_contract_missing_variables() {
        let contract = RholangContractLibrary::rho20_contract();
        let vars = [("TICKER", "TEST"), ("NAME", "Test Token")];

        let result = RholangContractLibrary::validate(contract, &vars);
        assert!(result.is_err());
        let missing = result.unwrap_err();
        assert!(missing.contains(&"TOTAL_SUPPLY".to_string()));
        assert!(missing.contains(&"PRECISION".to_string()));
    }

    #[test]
    fn test_validate_contract_all_variables() {
        let contract = RholangContractLibrary::rho20_contract();
        let vars = [
            ("TICKER", "TEST"),
            ("NAME", "Test Token"),
            ("TOTAL_SUPPLY", "21000000"),
            ("PRECISION", "8"),
            ("PUBLIC_KEY", "04abcd1234..."),
            ("DEPLOYER_PUBLIC_KEY", "04efgh5678..."),
            ("SIGNATURE", "3045022100..."),
            ("URI", "rho:id:test123"),
            ("VERSION", "1"),
        ];

        let result = RholangContractLibrary::validate(contract, &vars);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_unsubstituted() {
        let text = "Hello {{NAME}}, you have {{AMOUNT}} {{CURRENCY}}.";
        let vars = RholangContractLibrary::extract_unsubstituted(text);
        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&"NAME".to_string()));
        assert!(vars.contains(&"AMOUNT".to_string()));
        assert!(vars.contains(&"CURRENCY".to_string()));
    }
}
