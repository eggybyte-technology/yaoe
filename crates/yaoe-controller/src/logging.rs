//! Controller-level event logging helpers.

use yaoe_home::{LogLevel, log_event as home_log_event};

pub(crate) fn info(scope: &str, event: &str, fields: &[(&str, String)]) {
    event_log(LogLevel::Info, scope, event, fields);
}

pub(crate) fn ok(scope: &str, event: &str, fields: &[(&str, String)]) {
    event_log(LogLevel::Ok, scope, event, fields);
}

pub(crate) fn warn(scope: &str, event: &str, fields: &[(&str, String)]) {
    event_log(LogLevel::Warn, scope, event, fields);
}

pub(crate) fn progress(message: impl AsRef<str>) {
    let detail = one_line(message.as_ref());
    let (scope, event) = progress_scope_and_event(&detail);
    info(&scope, &event, &[]);
}

pub(crate) fn record_event(command: &str, stage: &str, pairs: &[(&str, String)]) {
    let scope = match (command, stage.contains(':')) {
        ("status" | "health" | "apply", _) => format!("{command}:{stage}"),
        (_, true) => format!("{command}.{stage}"),
        _ => format!("{command}.{stage}"),
    };
    info(&scope, "recorded result", pairs);
}

pub(crate) fn tail_lines(text: &str, limit: usize) -> String {
    let mut lines = text.lines().rev().take(limit).collect::<Vec<_>>();
    lines.reverse();
    lines.join("\n")
}

pub(crate) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn progress_scope_and_event(message: &str) -> (String, String) {
    let Some((scope_part, event_part)) = message.split_once(':') else {
        let mut parts = message.splitn(2, char::is_whitespace);
        let scope = parts.next().unwrap_or("yaoe").to_ascii_lowercase();
        let event = parts.next().unwrap_or("event");
        return (normalize_scope(&scope), normalize_event(event));
    };
    let mut scope = normalize_scope(scope_part);
    let mut event = event_part.trim();
    if let Some((target, rest)) = event.split_once(':') {
        scope = format!("{scope}:{}", target.trim());
        event = rest.trim();
    }
    (scope, normalize_event(event))
}

fn normalize_scope(scope: &str) -> String {
    scope
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(".")
        .to_ascii_lowercase()
}

fn normalize_event(event: &str) -> String {
    event.trim().trim_end_matches('.').to_ascii_lowercase()
}

fn event_log(level: LogLevel, scope: &str, event: &str, fields: &[(&str, String)]) {
    home_log_event(level, scope, event, fields);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_events_use_command_scopes() {
        assert_eq!(
            progress_scope_and_event("publish config:linux-amd64: render JSON"),
            (
                "publish.config:linux-amd64".to_string(),
                "render json".to_string()
            )
        );
        assert_eq!(
            progress_scope_and_event("apply:hk: resolving service state"),
            (
                "apply:hk".to_string(),
                "resolving service state".to_string()
            )
        );
        assert_eq!(
            progress_scope_and_event("publish runtime: ensuring fixed Gitee release"),
            (
                "publish.runtime".to_string(),
                "ensuring fixed gitee release".to_string()
            )
        );
    }
}
