mod ledger;
mod schema;

use ledger::{
    AgentInput, AgentPatch, get_meta_usize, ledger_add, ledger_audit, ledger_update, parse_bool,
    set_meta,
};
use schema::open_db;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const DYNAMIC_MARKER: &str = "--- DYNAMIC TASK CONTEXT ---";
const DYNAMIC_FIELDS: &[&str] = &[
    "ROLE",
    "TASK_BRIEF_PATH",
    "REPORT_PATH",
    "AGENT_LEDGER_PATH",
    "BASE_COMMIT",
    "REVIEW_PACKAGE_PATH",
    "FINDINGS_PATH",
    "HARNESS_COMMAND",
    "ALLOWED_WRITE_PATHS",
];
const READ_ONLY_ROLES: &[&str] = &["discussion", "explorer", "reviewer"];
const WRITE_ROLES: &[&str] = &["worker", "fixer"];
const ALL_ROLES: &[&str] = &["discussion", "explorer", "worker", "reviewer", "fixer"];
const STABLE_PREFIX: &str = r#"Use the cached-subagent-harness skill for this dispatch.

Stable operating rules:
- Follow harness-first validation. Work is not complete without reported tests.
- Keep information dense. Read large artifacts from paths; do not ask for pasted diffs or logs.
- Preserve complete-development quality. Do not skip required behavior, tests, error handling, integration, or docs by calling the work an MVP.
- Maintain the PSOC loop: Problem, Scenarios, Options, Chosen Plan.
- If new evidence invalidates PSOC, return LOOP_REQUIRED with the earliest invalid section.
- Use stable role behavior. Do not spawn nested subagents unless explicitly instructed.
- Require ledger state. A planned ledger row, budget, and report path must exist before spawn.
- Keep lifecycle closed. After reporting, the controller must wait, consume the report, then close or mark a final exception with final_reason.
- Close superseded agents. Temporary replacement agents expire when the original agent is resumed or the task is cancelled.
- Follow the report contract. Reports must cover PSOC, files, tests, risks, degraded mode, and final audit evidence.
- Respect ALLOWED_WRITE_PATHS. Read-only roles must treat it as none; writing roles must stay inside it.
- Treat control-plane files and agent-management rules as read-only unless explicitly granted.
- Reconcile unknown UI agents through one /agent snapshot only when they affect budget, cleanup, or correctness.
- Write the full report to REPORT_PATH and return only status, commits, tests, risks, and report path.
"#;

#[derive(Default)]
struct ParsedArgs {
    positionals: Vec<String>,
    flags: HashMap<String, Vec<String>>,
}

#[derive(Default)]
struct RenderOptions {
    role: String,
    brief: Option<String>,
    report: String,
    ledger: Option<String>,
    base_commit: Option<String>,
    review_package: Option<String>,
    findings: Option<String>,
    contexts: Vec<String>,
    harness_command: Option<String>,
    allowed_write_paths: Vec<String>,
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, String> {
    let mut parsed = ParsedArgs::default();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if let Some(name) = arg.strip_prefix("--") {
            if index + 1 >= args.len() || args[index + 1].starts_with("--") {
                return Err(format!("missing value for --{name}"));
            }
            parsed
                .flags
                .entry(name.to_string())
                .or_default()
                .push(args[index + 1].clone());
            index += 2;
        } else {
            parsed.positionals.push(arg.clone());
            index += 1;
        }
    }
    Ok(parsed)
}

fn flag_one(parsed: &ParsedArgs, name: &str) -> Option<String> {
    parsed
        .flags
        .get(name)
        .and_then(|values| values.last())
        .cloned()
}

fn flag_many(parsed: &ParsedArgs, name: &str) -> Vec<String> {
    parsed.flags.get(name).cloned().unwrap_or_default()
}

fn required_flag(parsed: &ParsedArgs, name: &str) -> Result<String, String> {
    flag_one(parsed, name).ok_or_else(|| format!("missing required --{name}"))
}

fn is_known_role(role: &str) -> bool {
    ALL_ROLES.contains(&role)
}

fn is_read_only_role(role: &str) -> bool {
    READ_ONLY_ROLES.contains(&role)
}

fn is_write_role(role: &str) -> bool {
    WRITE_ROLES.contains(&role)
}

fn validate_role(role: &str) -> Result<(), String> {
    if is_known_role(role) {
        Ok(())
    } else {
        Err(format!("unknown role: {role}"))
    }
}

fn validate_write_scope(role: &str, write_scope: &str) -> Result<(), String> {
    validate_role(role)?;
    if is_read_only_role(role) && write_scope != "none" {
        return Err(format!("{role} must use write_scope=none"));
    }
    if is_write_role(role) && write_scope == "none" {
        return Err(format!("{role} requires explicit write_scope"));
    }
    Ok(())
}

fn validate_prompt_contract(
    role: &str,
    brief: Option<&String>,
    findings: Option<&String>,
) -> Result<(), String> {
    validate_role(role)?;
    if role == "worker" && brief.is_none() {
        return Err("worker requires --brief with PSOC/TASK_BRIEF_PATH".to_string());
    }
    if role == "fixer" && findings.is_none() {
        return Err("fixer requires --findings with FINDINGS_PATH".to_string());
    }
    Ok(())
}

fn dynamic_field_name(line: &str) -> Option<&str> {
    let (name, _) = line.split_once('=')?;
    DYNAMIC_FIELDS.contains(&name).then_some(name)
}

fn dynamic_fields(lines: &[&str], marker_index: usize) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    for line in lines.iter().skip(marker_index + 1) {
        if let Some((name, value)) = line.split_once('=')
            && DYNAMIC_FIELDS.contains(&name)
        {
            fields.insert(name.to_string(), value.trim().to_string());
        }
    }
    fields
}

fn check_prompt(text: &str, max_lines: usize) -> Vec<String> {
    let mut errors = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let marker_index = match lines.iter().position(|line| *line == DYNAMIC_MARKER) {
        Some(index) => index,
        None => {
            errors.push(format!("missing dynamic marker: {DYNAMIC_MARKER}"));
            lines.len()
        }
    };
    let fields = dynamic_fields(&lines, marker_index);

    for (index, line) in lines.iter().take(marker_index).enumerate() {
        if dynamic_field_name(line).is_some() {
            errors.push(format!(
                "dynamic field before marker at line {}: {}",
                index + 1,
                line
            ));
        }
    }

    let role = match fields.get("ROLE") {
        Some(role) if role.trim().is_empty() => {
            errors.push("missing ROLE dynamic field".to_string());
            None
        }
        Some(role) => {
            if let Err(error) = validate_role(role) {
                errors.push(error);
                None
            } else {
                Some(role.as_str())
            }
        }
        None => {
            errors.push("missing ROLE dynamic field".to_string());
            None
        }
    };

    if let Some(role) = role {
        let write_scope = fields.get("ALLOWED_WRITE_PATHS").map(String::as_str);
        if is_write_role(role) && matches!(write_scope, None | Some("none")) {
            errors.push(format!(
                "{role} prompt must include explicit ALLOWED_WRITE_PATHS"
            ));
        }
        if is_read_only_role(role) && write_scope != Some("none") {
            errors.push(format!("{role} prompt must use ALLOWED_WRITE_PATHS=none"));
        }
        if !fields.contains_key("REPORT_PATH") {
            errors.push("missing REPORT_PATH dynamic field".to_string());
        }
        if !fields.contains_key("AGENT_LEDGER_PATH") {
            errors.push("missing AGENT_LEDGER_PATH dynamic field".to_string());
        }
        if role == "worker" && !fields.contains_key("TASK_BRIEF_PATH") {
            errors.push("worker prompt must include TASK_BRIEF_PATH".to_string());
        }
        if role == "fixer" && !fields.contains_key("FINDINGS_PATH") {
            errors.push("fixer prompt must include FINDINGS_PATH".to_string());
        }
    }

    for line in &lines {
        if line.starts_with("diff --git ") {
            errors.push("cache-hostile content found: embedded git diff".to_string());
            break;
        }
    }
    for line in &lines {
        if line.starts_with("@@ ") && line.contains("@@") {
            errors.push("cache-hostile content found: embedded diff hunk".to_string());
            break;
        }
    }
    if text.contains("test session starts") {
        errors.push("cache-hostile content found: pytest session log".to_string());
    }
    if text.contains("Traceback (most recent call last):") {
        errors.push("cache-hostile content found: long traceback".to_string());
    }
    if lines.len() > max_lines {
        errors.push(format!(
            "prompt has {} lines, above limit {}",
            lines.len(),
            max_lines
        ));
    }
    if text.matches("```").count() > 2 {
        errors
            .push("prompt contains multiple fenced blocks; pass bulky content by path".to_string());
    }
    errors
}

fn role_rules(role: &str) -> &'static [&'static str] {
    match role {
        "discussion" => &[
            "Read only. Do not edit files, commit, or mutate skills.",
            "Discuss product, architecture, or process questions and write conclusions to REPORT_PATH if requested.",
            "If an edit looks necessary, return the proposed worker brief instead of changing files.",
        ],
        "explorer" => &[
            "Read only. Do not edit files or commit.",
            "Investigate only the requested scope and write findings to REPORT_PATH.",
            "Return status plus the report path only.",
        ],
        "worker" => &[
            "You are the only writer for this gate.",
            "Use TDD for behavior changes, run focused tests, commit completed work.",
            "If PSOC becomes invalid, stop that path and report LOOP_REQUIRED.",
        ],
        "reviewer" => &[
            "Read only. Review the brief, report, and review package.",
            "Lead with findings ordered by severity.",
            "Do not run broad rediscovery unless a provided artifact is missing.",
        ],
        "fixer" => &[
            "Fix only the provided Critical/Important findings.",
            "Run covering tests, append results to the existing report, and commit.",
            "Do not broaden scope while fixing.",
        ],
        _ => &[],
    }
}

fn abs_path(value: &str) -> String {
    let path = Path::new(value);
    if path.is_absolute() {
        value.to_string()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| Path::new(".").to_path_buf())
            .join(path)
            .display()
            .to_string()
    }
}

fn render_prompt_full(options: &RenderOptions) -> Result<String, String> {
    validate_role(&options.role)?;
    validate_prompt_contract(
        &options.role,
        options.brief.as_ref(),
        options.findings.as_ref(),
    )?;
    let write_scope = if options.allowed_write_paths.is_empty() {
        "none".to_string()
    } else {
        options.allowed_write_paths.join(",")
    };
    validate_write_scope(&options.role, &write_scope)?;

    let mut lines = vec![STABLE_PREFIX.trim_end().to_string(), String::new()];
    lines.push(format!("Role: {}", options.role));
    for rule in role_rules(&options.role) {
        lines.push(format!("- {rule}"));
    }
    if !options.contexts.is_empty() {
        lines.push(String::new());
        lines.push("Stable context files to read if needed:".to_string());
        for context in &options.contexts {
            lines.push(format!("- {}", abs_path(context)));
        }
    }
    lines.push(String::new());
    lines.push(DYNAMIC_MARKER.to_string());
    lines.push(format!("ROLE={}", options.role));
    if let Some(brief) = &options.brief {
        lines.push(format!("TASK_BRIEF_PATH={}", abs_path(brief)));
    }
    lines.push(format!("REPORT_PATH={}", abs_path(&options.report)));
    lines.push(format!(
        "AGENT_LEDGER_PATH={}",
        options
            .ledger
            .as_ref()
            .map(|value| abs_path(value))
            .unwrap_or_else(|| abs_path(&options.report))
    ));
    if let Some(base_commit) = &options.base_commit {
        lines.push(format!("BASE_COMMIT={base_commit}"));
    }
    if let Some(review_package) = &options.review_package {
        lines.push(format!("REVIEW_PACKAGE_PATH={}", abs_path(review_package)));
    }
    if let Some(findings) = &options.findings {
        lines.push(format!("FINDINGS_PATH={}", abs_path(findings)));
    }
    lines.push(format!(
        "HARNESS_COMMAND={}",
        options
            .harness_command
            .as_deref()
            .unwrap_or(".venv/bin/python scripts/feedback_agent_harness.py")
    ));
    lines.push(format!("ALLOWED_WRITE_PATHS={write_scope}"));
    Ok(format!("{}\n", lines.join("\n")))
}

fn cmd_render_prompt(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let role = required_flag(&parsed, "role")?;
    let report = required_flag(&parsed, "report")?;
    let options = RenderOptions {
        role,
        brief: flag_one(&parsed, "brief"),
        report,
        ledger: flag_one(&parsed, "ledger"),
        base_commit: flag_one(&parsed, "base-commit"),
        review_package: flag_one(&parsed, "review-package"),
        findings: flag_one(&parsed, "findings"),
        contexts: flag_many(&parsed, "context"),
        harness_command: flag_one(&parsed, "harness-command"),
        allowed_write_paths: flag_many(&parsed, "allowed-write-paths"),
    };
    print!("{}", render_prompt_full(&options)?);
    Ok(())
}

fn cmd_check_prompt(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let file = required_flag(&parsed, "file")?;
    let max_lines = flag_one(&parsed, "max-lines")
        .unwrap_or_else(|| "120".to_string())
        .parse::<usize>()
        .map_err(|error| format!("invalid --max-lines: {error}"))?;
    let text = fs::read_to_string(&file).map_err(|error| error.to_string())?;
    let errors = check_prompt(&text, max_lines);
    if errors.is_empty() {
        println!("OK: dispatch prompt is cache-friendly");
        Ok(())
    } else {
        for error in errors {
            eprintln!("FAIL: {error}");
        }
        Err("dispatch prompt check failed".to_string())
    }
}

fn cmd_ledger_init(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let db = required_flag(&parsed, "db")?;
    let max_concurrent = flag_one(&parsed, "max-concurrent").unwrap_or_else(|| "2".to_string());
    let max_total = flag_one(&parsed, "max-total").unwrap_or_else(|| "4".to_string());
    let mut conn = open_db(&db)?;
    set_meta(&mut conn, "max_concurrent", &max_concurrent)?;
    set_meta(&mut conn, "max_total", &max_total)?;
    println!("OK: ledger initialized at {db}");
    Ok(())
}

fn cmd_ledger_add(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let db = required_flag(&parsed, "db")?;
    let role = required_flag(&parsed, "role")?;
    let write_scope = flag_one(&parsed, "write-scope").unwrap_or_else(|| "none".to_string());
    let input = AgentInput {
        handle: required_flag(&parsed, "handle")?,
        role,
        task: required_flag(&parsed, "task")?,
        status: flag_one(&parsed, "status").unwrap_or_else(|| "planned".to_string()),
        report_path: flag_one(&parsed, "report-path").unwrap_or_default(),
        spawned_at: flag_one(&parsed, "spawned-at").unwrap_or_default(),
        waited: flag_one(&parsed, "waited")
            .map(|value| parse_bool(&value))
            .transpose()?
            .unwrap_or(false),
        closed: flag_one(&parsed, "closed")
            .map(|value| parse_bool(&value))
            .transpose()?
            .unwrap_or(false),
        write_scope,
        token_risk: flag_one(&parsed, "token-risk").unwrap_or_default(),
        final_reason: flag_one(&parsed, "reason").unwrap_or_default(),
        next_action: flag_one(&parsed, "next-action").unwrap_or_default(),
    };
    let mut conn = open_db(&db)?;
    ledger_add(&mut conn, &input)?;
    println!("OK: ledger row added for {}", input.handle);
    Ok(())
}

fn cmd_ledger_update(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let db = required_flag(&parsed, "db")?;
    let handle = required_flag(&parsed, "handle")?;
    let patch = AgentPatch {
        status: flag_one(&parsed, "status"),
        report_path: flag_one(&parsed, "report-path"),
        waited: flag_one(&parsed, "waited")
            .map(|value| parse_bool(&value))
            .transpose()?,
        closed: flag_one(&parsed, "closed")
            .map(|value| parse_bool(&value))
            .transpose()?,
        write_scope: flag_one(&parsed, "write-scope"),
        token_risk: flag_one(&parsed, "token-risk"),
        final_reason: flag_one(&parsed, "reason"),
        next_action: flag_one(&parsed, "next-action"),
    };
    let mut conn = open_db(&db)?;
    ledger_update(&mut conn, &handle, &patch)?;
    println!("OK: ledger row updated for {handle}");
    Ok(())
}

fn cmd_ledger_audit(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let db = required_flag(&parsed, "db")?;
    let conn = open_db(&db)?;
    let mode = flag_one(&parsed, "mode").unwrap_or_else(|| "budget".to_string());
    let max_concurrent = flag_one(&parsed, "max-concurrent")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("invalid --max-concurrent: {error}"))?
        .unwrap_or(get_meta_usize(&conn, "max_concurrent", 2)?);
    let max_total = flag_one(&parsed, "max-total")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("invalid --max-total: {error}"))?
        .unwrap_or(get_meta_usize(&conn, "max_total", 4)?);
    let errors = ledger_audit(&conn, &mode, max_concurrent, max_total)?;
    if errors.is_empty() {
        println!("OK: ledger audit passed");
        Ok(())
    } else {
        for error in errors {
            eprintln!("FAIL: {error}");
        }
        Err("ledger audit failed".to_string())
    }
}

fn usage() -> &'static str {
    r#"usage:
  harnessctl render-prompt --role ROLE --report PATH [--brief PATH for worker] [--ledger PATH] [--allowed-write-paths PATH]...
  harnessctl check-prompt --file PATH [--max-lines N]
  harnessctl ledger-init --db PATH [--max-concurrent N] [--max-total N]
  harnessctl ledger-add --db PATH --handle ID --role ROLE --task TEXT [--status STATUS] [--write-scope SCOPE] [--reason TEXT]
  harnessctl ledger-update --db PATH --handle ID [--status STATUS] [--waited true] [--closed true] [--reason TEXT]
  harnessctl ledger-audit --db PATH [--mode budget|final]
"#
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        return Err(usage().to_string());
    };
    match command.as_str() {
        "render-prompt" => cmd_render_prompt(rest),
        "check-prompt" => cmd_check_prompt(rest),
        "ledger-init" => cmd_ledger_init(rest),
        "ledger-add" => cmd_ledger_add(rest),
        "ledger-update" => cmd_ledger_update(rest),
        "ledger-audit" => cmd_ledger_audit(rest),
        "help" | "--help" | "-h" => {
            print!("{}", usage());
            Ok(())
        }
        _ => Err(format!("unknown command: {command}\n{}", usage())),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_prompt_rejects_worker_without_write_scope() {
        let prompt = "\
Stable text

--- DYNAMIC TASK CONTEXT ---
ROLE=worker
REPORT_PATH=/tmp/report.md
AGENT_LEDGER_PATH=/tmp/harness.db
";
        let errors = check_prompt(prompt, 120);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("explicit ALLOWED_WRITE_PATHS")),
            "{errors:?}"
        );
    }

    #[test]
    fn check_prompt_rejects_missing_role() {
        let prompt = "\
Stable text

--- DYNAMIC TASK CONTEXT ---
REPORT_PATH=/tmp/report.md
AGENT_LEDGER_PATH=/tmp/harness.db
ALLOWED_WRITE_PATHS=issue_feedback_agent/tests
";
        let errors = check_prompt(prompt, 120);
        assert!(
            errors.iter().any(|error| error.contains("missing ROLE")),
            "{errors:?}"
        );
    }

    #[test]
    fn check_prompt_rejects_unknown_role() {
        let prompt = "\
Stable text

--- DYNAMIC TASK CONTEXT ---
ROLE=bogus
REPORT_PATH=/tmp/report.md
AGENT_LEDGER_PATH=/tmp/harness.db
ALLOWED_WRITE_PATHS=issue_feedback_agent/tests
";
        let errors = check_prompt(prompt, 120);
        assert!(
            errors.iter().any(|error| error.contains("unknown role")),
            "{errors:?}"
        );
    }

    #[test]
    fn check_prompt_rejects_worker_without_brief() {
        let prompt = "\
Stable text

--- DYNAMIC TASK CONTEXT ---
ROLE=worker
REPORT_PATH=/tmp/report.md
AGENT_LEDGER_PATH=/tmp/harness.db
ALLOWED_WRITE_PATHS=issue_feedback_agent/tests
";
        let errors = check_prompt(prompt, 120);
        assert!(
            errors.iter().any(|error| error.contains("TASK_BRIEF_PATH")),
            "{errors:?}"
        );
    }

    #[test]
    fn render_prompt_rejects_worker_without_brief() {
        let error = render_prompt_full(&RenderOptions {
            role: "worker".to_string(),
            report: "/tmp/report.md".to_string(),
            allowed_write_paths: vec!["issue_feedback_agent/tests".to_string()],
            ..RenderOptions::default()
        })
        .expect_err("worker without a brief must fail");
        assert!(error.contains("--brief"), "{error}");
    }

    #[test]
    fn check_prompt_rejects_dynamic_field_before_marker() {
        let prompt = "\
ROLE=worker
--- DYNAMIC TASK CONTEXT ---
ROLE=worker
REPORT_PATH=/tmp/report.md
AGENT_LEDGER_PATH=/tmp/harness.db
ALLOWED_WRITE_PATHS=issue_feedback_agent/tests
";
        let errors = check_prompt(prompt, 120);
        assert!(
            errors.iter().any(|error| error.contains("before marker")),
            "{errors:?}"
        );
    }

    #[test]
    fn render_prompt_places_dynamic_values_after_marker() {
        let prompt = render_prompt_full(&RenderOptions {
            role: "worker".to_string(),
            brief: Some("/tmp/brief.md".to_string()),
            report: "/tmp/report.md".to_string(),
            allowed_write_paths: vec![String::from("issue_feedback_agent/services")],
            ..RenderOptions::default()
        })
        .unwrap();
        let marker = prompt.find(DYNAMIC_MARKER).unwrap();
        let role_pos = prompt.find("ROLE=worker").unwrap();
        let brief_pos = prompt.find("TASK_BRIEF_PATH=").unwrap();
        let write_pos = prompt.find("ALLOWED_WRITE_PATHS=").unwrap();
        assert!(role_pos > marker);
        assert!(brief_pos > marker);
        assert!(write_pos > marker);
    }
}
