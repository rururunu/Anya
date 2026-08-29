use super::language::inject_language_blocks;
use super::slots::inject_context;
use super::*;

use crate::core::chat::limits::{CLIPBOARD_MAX_CHARS, CONTEXT_BLOCKS_TOTAL_MAX_CHARS};
use crate::core::runtime::{ChatMessage, MessageStatus, RequestContext, Role};
use crate::models::settings::{AppLanguage, ReasoningLanguage};

#[test]
fn collaboration_models_are_injected_only_when_configured() {
    let context = RequestContext::default();
    let preferences = PromptPreferences {
        collaboration_models: vec![
            r#"["deepseek","deepseek-v4-pro"]"#.into(),
            "deepseek-v4-flash".into(),
        ],
        ..PromptPreferences::default()
    };
    let request = PromptBuilder::build(PromptBuildInput {
        request_id: "request",
        session_id: "session",
        history: &[],
        context: &context,
        project_rules: None,
        recalled_memories: None,
        preferred_resources: None,
        provider: None,
        preferences: &preferences,
    });
    let collaboration = request
        .messages
        .iter()
        .find(|message| message.id.starts_with("collaboration-models-"));
    assert!(collaboration.is_some_and(|message| {
        message.content.contains("- `deepseek-v4-pro`")
            && message.content.contains("- `deepseek-v4-flash`")
            && !message
                .content
                .contains("- `[\"deepseek\",\"deepseek-v4-pro\"]`")
            && message.content.contains("Routing policy")
            && message.content.contains("exact selected model ID")
            && message.content.contains("Never omit `model`")
            && message
                .content
                .contains("user-selected list defines eligibility only")
            && !message.content.contains("general-purpose candidate")
    }));

    let default_preferences = PromptPreferences::default();
    let without_models = PromptBuilder::build(PromptBuildInput {
        request_id: "request",
        session_id: "session",
        history: &[],
        context: &context,
        project_rules: None,
        recalled_memories: None,
        preferred_resources: None,
        provider: None,
        preferences: &default_preferences,
    });
    assert!(!without_models
        .messages
        .iter()
        .any(|message| message.id.starts_with("collaboration-models-")));
}

#[test]
fn image_mode_policy_is_injected_with_toolbar_values() {
    let context = RequestContext::default();
    let preferences = PromptPreferences {
        image_mode: Some(ImageModePolicy {
            size: "1024x1536".into(),
            quality: "high".into(),
            n: 2,
            style_prompt: "anime illustration".into(),
            has_reference: true,
        }),
        ..PromptPreferences::default()
    };
    let request = PromptBuilder::build(PromptBuildInput {
        request_id: "request",
        session_id: "session",
        history: &[],
        context: &context,
        project_rules: None,
        recalled_memories: None,
        preferred_resources: None,
        provider: None,
        preferences: &preferences,
    });
    let block = request
        .messages
        .iter()
        .find(|message| message.id.starts_with("image-mode-"))
        .expect("image-mode system block");
    assert!(block.content.contains("generate_image"));
    assert!(block.content.contains("1024x1536"));
    assert!(block.content.contains("high"));
    assert!(block.content.contains("2"));
    assert!(block.content.contains("anime illustration"));
    assert!(block.content.contains("Mandatory visual style"));
    assert!(block.content.contains("image-to-image"));
    assert!(block.content.contains("A caption without a tool result is a lie"));
}

#[test]
fn minimal_coding_is_injected_only_when_enabled() {
    let context = RequestContext::default();
    let enabled = PromptPreferences {
        minimal_coding: true,
        ..PromptPreferences::default()
    };
    let request = PromptBuilder::build(PromptBuildInput {
        request_id: "request",
        session_id: "session",
        history: &[],
        context: &context,
        project_rules: None,
        recalled_memories: None,
        preferred_resources: None,
        provider: None,
        preferences: &enabled,
    });
    let block = request
        .messages
        .iter()
        .find(|message| message.id.starts_with("minimal-coding-"));
    assert!(block.is_some_and(|message| {
        message.content.contains("Minimal coding mode")
            && message.content.contains("YAGNI")
            && message.content.contains("Never lazy about")
    }));

    let disabled = PromptBuilder::build(PromptBuildInput {
        request_id: "request",
        session_id: "session",
        history: &[],
        context: &context,
        project_rules: None,
        recalled_memories: None,
        preferred_resources: None,
        provider: None,
        preferences: &PromptPreferences::default(),
    });
    assert!(!disabled
        .messages
        .iter()
        .any(|message| message.id.starts_with("minimal-coding-")));
}

#[test]
fn injects_reasoning_language_for_chinese_user_text() {
    let prefs = PromptPreferences {
        app_language: AppLanguage::EnUs,
        reasoning_language: ReasoningLanguage::Auto,
        ..PromptPreferences::default()
    };
    let content = inject_language_blocks("帮我解释这段代码", &prefs);
    assert!(content.starts_with("<reasoning-language>"));
    assert!(content.contains("<response-language>"));
    assert!(content.contains("必须使用简体中文"));
    assert!(content.ends_with("帮我解释这段代码"));
}

#[test]
fn auto_response_follows_chinese_message_even_when_ui_is_english() {
    let prefs = PromptPreferences {
        app_language: AppLanguage::EnUs,
        reasoning_language: ReasoningLanguage::Auto,
        ..PromptPreferences::default()
    };
    let content = inject_language_blocks("把这个 bug 修一下", &prefs);
    assert!(content.contains("必须使用简体中文"));
    assert!(!content.contains("use English for user-facing replies"));
}

#[test]
fn auto_response_follows_english_message_even_when_ui_is_chinese() {
    let prefs = PromptPreferences {
        app_language: AppLanguage::ZhCn,
        reasoning_language: ReasoningLanguage::Auto,
        ..PromptPreferences::default()
    };
    let content = inject_language_blocks("Please explain this function", &prefs);
    assert!(content.contains("use English for user-facing replies"));
    assert!(!content.contains("必须使用简体中文"));
}

#[test]
fn auto_reasoning_skips_english_only_prompt() {
    let prefs = PromptPreferences {
        app_language: AppLanguage::EnUs,
        reasoning_language: ReasoningLanguage::Auto,
        ..PromptPreferences::default()
    };
    let content = inject_language_blocks("Explain this snippet", &prefs);
    assert!(!content.contains("<reasoning-language>"));
    assert!(content.starts_with("<response-language>"));
}

#[test]
fn workspace_context_identifies_the_exact_active_directory() {
    let context = RequestContext {
        workspace: Some(crate::core::runtime::request::WorkspaceContext {
            name: "Customer App".to_string(),
            root: r"C:\projects\customer-app".to_string(),
        }),
        active_window: Some("Peek - source code".to_string()),
        ..RequestContext::default()
    };
    let mut messages = Vec::new();

    inject_context(&mut messages, "session-1", &context);

    assert_eq!(messages.len(), 1);
    let content = &messages[0].content;
    assert!(content.contains(
        "[Current Workspace]\nName: Customer App\nRoot Directory: C:\\projects\\customer-app"
    ));
    assert!(content.contains("Do not infer another project"));
    assert!(content.contains("MCP filesystem allow-lists"));
}

#[test]
fn injects_environment_context_into_agent_prompt() {
    let context = RequestContext {
        git_status: Some("## main\n M src/main.rs".to_string()),
        last_shell_execution: Some(
            "Command: cargo test\nWorking Directory: C:\\work\nResult:\nexit_code: 0".to_string(),
        ),
        ..RequestContext::default()
    };
    let mut messages = Vec::new();

    inject_context(&mut messages, "session-environment", &context);

    assert_eq!(messages.len(), 1);
    assert!(messages[0].content.contains("[Git Status]\n## main"));
    assert!(messages[0]
        .content
        .contains("[Last Agent Shell Execution]\nCommand: cargo test"));
}

#[test]
fn injects_ide_context_into_agent_prompt() {
    use crate::core::context::models::{CursorPosition, IDEContext};
    use std::path::PathBuf;

    let context = RequestContext {
        ide_context: Some(IDEContext {
            ide: "vscode".to_string(),
            active_file: Some(PathBuf::from(r"C:\project\src\main.rs")),
            workspace: Some(PathBuf::from(r"C:\project")),
            language: Some("rust".to_string()),
            selection: Some("fn main() {}".to_string()),
            cursor: Some(CursorPosition {
                line: 15,
                column: 5,
            }),
        }),
        ..RequestContext::default()
    };
    let mut messages = Vec::new();

    inject_context(&mut messages, "session-ide", &context);

    let content = &messages[0].content;
    assert!(content.contains("[IDE Context]"));
    assert!(content.contains("IDE:\nVSCode"));
    assert!(content.contains("Language:\nrust"));
    assert!(content.contains("Line 15, Column 5"));
    assert!(content.contains("Selection:\nfn main() {}"));
}

#[test]
fn workspace_context_precedes_history_and_current_user() {
    let message = |id: &str, role: Role, content: &str| ChatMessage {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        role,
        content: content.to_string(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 1,
        estimated_tokens: None,
    };
    let history = vec![
        message("old-user", Role::User, "old question"),
        message("old-assistant", Role::Assistant, "old answer"),
        message("current-user", Role::User, "new question"),
    ];
    let context = RequestContext {
        workspace: Some(crate::core::runtime::request::WorkspaceContext {
            name: "Peek".to_string(),
            root: r"D:\Code\Peek".to_string(),
        }),
        ..RequestContext::default()
    };

    let preferences = PromptPreferences::default();
    let request = PromptBuilder::build(PromptBuildInput {
        request_id: "request-1",
        session_id: "session-1",
        history: &history,
        context: &context,
        project_rules: None,
        recalled_memories: None,
        preferred_resources: None,
        provider: None,
        preferences: &preferences,
    });

    assert!(request.messages[0].id.starts_with("system-"));
    assert!(request.messages[1].id.starts_with("context-"));
    assert_eq!(request.messages[2].id, "old-user");
    assert_eq!(request.messages.last().unwrap().id, "current-user");
}

#[test]
fn prompt_keeps_empty_assistant_tool_calls_and_tool_results() {
    use crate::core::runtime::ToolCallPayload;

    let message = |id: &str, role: Role, content: &str| ChatMessage {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        role,
        content: content.to_string(),
        reasoning: None,
        work_timeline: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 1,
        estimated_tokens: None,
    };
    let mut assistant = message("a-tools", Role::Assistant, "");
    assistant.tool_calls = Some(vec![ToolCallPayload {
        id: "call-1".into(),
        name: "read_file".into(),
        arguments: r#"{"path":"a.rs"}"#.into(),
        thought_signature: None,
    }]);
    let mut tool = message("t1", Role::Tool, "file contents");
    tool.tool_call_id = Some("call-1".into());
    let history = vec![
        message("old-user", Role::User, "read the file"),
        assistant,
        tool,
        message("current-user", Role::User, "now edit it"),
    ];
    let request = PromptBuilder::build(PromptBuildInput {
        request_id: "request-1",
        session_id: "session-1",
        history: &history,
        context: &RequestContext::default(),
        project_rules: None,
        recalled_memories: None,
        preferred_resources: None,
        provider: None,
        preferences: &PromptPreferences::default(),
    });
    let ids: Vec<_> = request.messages.iter().map(|m| m.id.as_str()).collect();
    assert!(ids.contains(&"a-tools"));
    assert!(ids.contains(&"t1"));
}

#[test]
fn recalled_memories_are_injected_as_untrusted_system_context() {
    let context = RequestContext::default();
    let preferences = PromptPreferences::default();
    let request = PromptBuilder::build(PromptBuildInput {
        request_id: "request-1",
        session_id: "session-1",
        history: &[],
        context: &context,
        project_rules: None,
        recalled_memories: Some("<relevant-memories>\nUses pnpm\n</relevant-memories>"),
        preferred_resources: None,
        provider: None,
        preferences: &preferences,
    });

    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.messages[1].role, Role::System);
    assert!(request.messages[1].content.contains("<relevant-memories>"));
    assert!(request.messages[1].id.starts_with("memories-"));
    assert!(request.messages[1].content.contains("Uses pnpm"));
}

#[test]
fn project_rules_are_injected_as_system_context() {
    let context = RequestContext::default();
    let preferences = PromptPreferences::default();
    let request = PromptBuilder::build(PromptBuildInput {
        request_id: "request-1",
        session_id: "session-1",
        history: &[],
        context: &context,
        project_rules: Some("<project-rules>\nUse pnpm\n</project-rules>"),
        recalled_memories: None,
        preferred_resources: None,
        provider: None,
        preferences: &preferences,
    });

    assert_eq!(request.messages[1].id, "rules-session-1");
    assert!(request.messages[1].content.contains("Use pnpm"));
}

#[test]
fn clipboard_context_is_hard_capped() {
    let context = RequestContext {
        clipboard: Some("Z".repeat(CLIPBOARD_MAX_CHARS + 500)),
        ..RequestContext::default()
    };
    let mut messages = Vec::new();
    inject_context(&mut messages, "session-1", &context);
    let content = &messages[0].content;
    assert!(content.chars().count() <= CONTEXT_BLOCKS_TOTAL_MAX_CHARS);
    assert!(content.contains('…') || content.contains("[Clipboard]"));
    let clipboard_body = content.split("[Clipboard]\n").nth(1).unwrap_or_default();
    assert!(clipboard_body.chars().count() <= CLIPBOARD_MAX_CHARS + 1);
}

#[test]
fn optional_policies_never_shift_stable_prefix_slots() {
    let context = RequestContext {
        workspace: Some(crate::core::runtime::request::WorkspaceContext {
            name: "Demo".to_string(),
            root: r"C:\demo".to_string(),
        }),
        ..RequestContext::default()
    };
    let rules = Some("<project-rules>\nUse pnpm\n</project-rules>");
    let memories = Some("<relevant-memories>\nUses pnpm\n</relevant-memories>");

    let baseline = PromptBuilder::build(PromptBuildInput {
        request_id: "r",
        session_id: "s",
        history: &[],
        context: &context,
        project_rules: rules,
        recalled_memories: memories,
        preferred_resources: None,
        provider: None,
        preferences: &PromptPreferences::default(),
    });
    let with_optional = PromptBuilder::build(PromptBuildInput {
        request_id: "r",
        session_id: "s",
        history: &[],
        context: &context,
        project_rules: rules,
        recalled_memories: memories,
        preferred_resources: None,
        provider: None,
        preferences: &PromptPreferences {
            collaboration_models: vec!["model-a".into()],
            minimal_coding: true,
            ..PromptPreferences::default()
        },
    });

    // Slots [0]–[3] keep the same ids and relative order regardless of toggles.
    for i in 0..4 {
        assert_eq!(baseline.messages[i].id, with_optional.messages[i].id);
    }
    assert!(baseline.messages[0].id.starts_with("system-"));
    assert_eq!(baseline.messages[1].id, "context-s");
    assert_eq!(baseline.messages[2].id, "rules-s");
    assert_eq!(baseline.messages[3].id, "memories-s");

    // Optional policies only appear after the stable prefix.
    let optional_ids: Vec<_> = with_optional
        .messages
        .iter()
        .skip(4)
        .map(|m| m.id.as_str())
        .collect();
    assert!(optional_ids
        .iter()
        .any(|id| id.starts_with("collaboration-models-")));
    assert!(optional_ids
        .iter()
        .any(|id| id.starts_with("minimal-coding-")));
    assert!(!baseline
        .messages
        .iter()
        .any(|m| m.id.starts_with("collaboration-models-") || m.id.starts_with("minimal-coding-")));
}

#[test]
fn office_context_is_injected_into_prompt() {
    let context = RequestContext {
        office_context: Some(crate::core::office::OfficeContext {
            app: "word".to_string(),
            is_foreground: true,
            document_name: Some("Report.docx".to_string()),
            selected_text: Some("draft paragraph".to_string()),
            ..crate::core::office::OfficeContext::default()
        }),
        ..RequestContext::default()
    };
    let mut messages = Vec::new();
    inject_context(&mut messages, "session-1", &context);
    let content = &messages[0].content;
    assert!(content.contains("[Microsoft Word Context]"));
    assert!(content.contains("Report.docx"));
    assert!(content.contains("word_get_selection"));
}
