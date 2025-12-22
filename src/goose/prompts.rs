use crate::models::{Incident, FixGenerationRequest};

/// Migration-specific prompt templates for Goose agent
pub struct PromptBuilder;

impl PromptBuilder {
    /// Generate system prompt for migration context
    pub fn system_prompt() -> String {
        r#"You are a code migration assistant specializing in fixing deprecated API usage and migration issues.

Your primary responsibilities:
1. Analyze code incidents and understand migration requirements
2. Generate precise fix suggestions with minimal changes
3. Provide clear explanations for each change
4. Ensure backward compatibility where possible
5. Never make file modifications without explicit user approval

Guidelines:
- Focus on the specific incidents provided
- Use migration guides and documentation when available
- Prefer minimal, targeted changes over large refactoring
- Always explain the reasoning behind each change
- Request user approval before any file modifications"#.to_string()
    }

    /// Generate user prompt for fix generation request
    pub fn fix_generation_prompt(request: &FixGenerationRequest) -> String {
        let mut prompt = format!(
            "Please analyze and provide fixes for the following code incidents in workspace: {}\n\n",
            request.workspace_path
        );

        prompt.push_str("Incidents to fix:\n");
        for (index, incident) in request.incidents.iter().enumerate() {
            prompt.push_str(&format!(
                "{}. File: {} (line {})\n",
                index + 1,
                incident.file_path,
                incident.line_number
            ));
            prompt.push_str(&format!("   Rule: {}\n", incident.rule_id));
            prompt.push_str(&format!("   Severity: {:?}\n", incident.severity));
            prompt.push_str(&format!("   Description: {}\n", incident.description));
            prompt.push_str(&format!("   Details: {}\n", incident.message));

            if !incident.metadata.is_empty() {
                prompt.push_str(&format!("   Metadata: {:?}\n", incident.metadata));
            }
            prompt.push('\n');
        }

        if let Some(migration_context) = &request.migration_context {
            prompt.push_str("Migration context:\n");
            prompt.push_str(&format!("{}\n\n", migration_context));
        }

        prompt.push_str("Please provide specific fix suggestions for each incident, including:");
        prompt.push_str("\n1. Exact code changes required");
        prompt.push_str("\n2. Explanation of why the change is needed");
        prompt.push_str("\n3. Any potential side effects or considerations");
        prompt.push_str("\n4. Testing recommendations");

        prompt
    }

    /// Generate prompt for specific incident analysis
    pub fn incident_analysis_prompt(incident: &Incident, file_content: &str) -> String {
        format!(
            r#"Analyze this specific code incident and provide a targeted fix:

File: {}
Line: {}
Issue: {} ({:?})
Description: {}
Details: {}

Current code context:
```
{}
```

Please provide:
1. Root cause analysis
2. Specific fix for line {}
3. Impact assessment
4. Recommended testing approach

Focus on minimal, precise changes that address the specific issue."#,
            incident.file_path,
            incident.line_number,
            incident.rule_id,
            incident.severity,
            incident.description,
            incident.message,
            file_content,
            incident.line_number
        )
    }

    /// Generate prompt for user interaction/approval
    pub fn approval_prompt(
        file_path: &str,
        original_content: &str,
        proposed_content: &str,
        description: &str,
    ) -> String {
        format!(
            r#"File modification proposal for review:

File: {}
Change description: {}

Original content:
```
{}
```

Proposed content:
```
{}
```

Do you approve this change? This will modify the file as shown above."#,
            file_path, description, original_content, proposed_content
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AiSession, Severity};

    #[test]
    fn test_system_prompt() {
        let prompt = PromptBuilder::system_prompt();
        assert!(prompt.contains("migration assistant"));
        assert!(prompt.contains("user approval"));
    }

    #[test]
    fn test_fix_generation_prompt() {
        let incident = Incident::new(
            "deprecated-api".to_string(),
            "src/main.rs".to_string(),
            42,
            Severity::Warning,
            "Deprecated API usage".to_string(),
            "old_method() is deprecated".to_string(),
            "deprecated".to_string(),
        );

        let session = AiSession::new(
            "/tmp/test".to_string(),
            Some("test".to_string()),
        );

        let request = FixGenerationRequest::new(
            session.id,
            vec![incident],
            "/tmp/test".to_string(),
        );

        let prompt = PromptBuilder::fix_generation_prompt(&request);
        assert!(prompt.contains("/tmp/test"));
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("line 42"));
        assert!(prompt.contains("deprecated-api"));
    }

    #[test]
    fn test_incident_analysis_prompt() {
        let incident = Incident::new(
            "deprecated-api".to_string(),
            "src/main.rs".to_string(),
            42,
            Severity::Warning,
            "Deprecated API usage".to_string(),
            "old_method() is deprecated".to_string(),
            "deprecated".to_string(),
        );

        let prompt = PromptBuilder::incident_analysis_prompt(&incident, "fn main() {\n    old_method();\n}");
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("Line: 42"));
        assert!(prompt.contains("old_method()"));
    }

    #[test]
    fn test_approval_prompt() {
        let prompt = PromptBuilder::approval_prompt(
            "src/main.rs",
            "old_method()",
            "new_method()",
            "Replace deprecated API call",
        );
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("old_method()"));
        assert!(prompt.contains("new_method()"));
        assert!(prompt.contains("Replace deprecated API call"));
    }
}