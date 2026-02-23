//! ASCII table formatting for MCP list tool results.

use prettytable::{format, Cell, Row, Table};

use super::{ListProvidersResult, ListRolesResult, TaskInfo};

/// Safely truncate a string to at most `max_chars` characters (not bytes),
/// appending "..." if truncated.
fn truncate_str(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
    format!("{truncated}...")
}

/// Format a list of tasks as an ASCII table.
pub fn format_tasks_table(tasks: &[TaskInfo]) -> String {
    if tasks.is_empty() {
        return "No tasks found.".to_string();
    }

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

    table.add_row(Row::new(vec![
        Cell::new("TASK_ID"),
        Cell::new("PID"),
        Cell::new("STATUS"),
        Cell::new("STARTED_AT"),
        Cell::new("COMPLETED_AT"),
    ]));

    for t in tasks {
        let task_id = t
            .task_id
            .as_deref()
            .map(|id| if id.len() > 10 { &id[..10] } else { id })
            .unwrap_or("-");
        let status = format!("{:?}", t.status).to_lowercase();
        let started = t.started_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let completed = t
            .completed_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".to_string());

        table.add_row(Row::new(vec![
            Cell::new(task_id),
            Cell::new(&t.pid.to_string()),
            Cell::new(&status),
            Cell::new(&started),
            Cell::new(&completed),
        ]));
    }

    table.to_string()
}

/// Format list_roles result as an ASCII table.
pub fn format_roles_table(result: &ListRolesResult) -> String {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

    table.add_row(Row::new(vec![
        Cell::new("NAME"),
        Cell::new("TYPE"),
        Cell::new("DESCRIPTION"),
    ]));

    for name in &result.builtin_roles {
        table.add_row(Row::new(vec![
            Cell::new(name),
            Cell::new("builtin"),
            Cell::new("-"),
        ]));
    }

    for role in &result.user_roles {
        let desc = truncate_str(&role.description, 60);
        table.add_row(Row::new(vec![
            Cell::new(&role.name),
            Cell::new("user"),
            Cell::new(&desc),
        ]));
    }

    if result.builtin_roles.is_empty() && result.user_roles.is_empty() {
        return "No roles found.".to_string();
    }

    table.to_string()
}

/// Format list_providers result as an ASCII table.
pub fn format_providers_table(result: &ListProvidersResult) -> String {
    if result.providers.is_empty() {
        return "No providers configured.".to_string();
    }

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

    table.add_row(Row::new(vec![
        Cell::new("NAME"),
        Cell::new("ENABLED"),
        Cell::new("DEFAULT"),
        Cell::new("SCENARIO"),
        Cell::new("COMPATIBLE_WITH"),
    ]));

    for p in &result.providers {
        let is_default = if p.name == result.default_provider {
            "✓"
        } else {
            ""
        };
        let enabled = if p.enabled { "✓" } else { "✗" };
        let scenario = p.scenario.as_deref().unwrap_or("-");
        let compat = p
            .compatible_with
            .as_ref()
            .map(|v| v.join(", "))
            .unwrap_or_else(|| "-".to_string());

        table.add_row(Row::new(vec![
            Cell::new(&p.name),
            Cell::new(enabled),
            Cell::new(is_default),
            Cell::new(scenario),
            Cell::new(&compat),
        ]));
    }

    table.to_string()
}
