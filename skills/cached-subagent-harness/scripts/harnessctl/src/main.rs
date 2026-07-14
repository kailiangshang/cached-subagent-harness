mod accounting;
mod bundle;
mod dashboard;
mod domain;
mod hosts;
mod routing;
mod sessions;
mod status;
mod store;

use domain::{
    ActivityInput, DispatchRequest, Language, Operation, Profile, SessionInput, SessionSignature,
    SessionStatus, TaskInput, TemplateValues, UsageInput,
};
use routing::route;
use sessions::decide;
use status::{build_status, render_json, render_text};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;
use store::Store;

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

fn parse_value<T: FromStr<Err = String>>(parsed: &ParsedArgs, name: &str) -> Result<T, String> {
    required_flag(parsed, name)?.parse()
}

fn parse_bool_value(parsed: &ParsedArgs, name: &str, default: bool) -> Result<bool, String> {
    match flag_one(parsed, name).as_deref() {
        None => Ok(default),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(value) => Err(format!("invalid --{name}: {value}; expected true or false")),
    }
}

fn parse_u64_value(parsed: &ParsedArgs, name: &str) -> Result<u64, String> {
    let value = required_flag(parsed, name)?;
    let parsed_value = value
        .parse::<u64>()
        .map_err(|error| format!("invalid --{name}: {error}"))?;
    if parsed_value.to_string() != value {
        return Err(format!("invalid noncanonical --{name}: {value}"));
    }
    Ok(parsed_value)
}

fn open_store(parsed: &ParsedArgs) -> Result<Store, String> {
    Store::open(Path::new(&required_flag(parsed, "db")?))
}

fn cmd_init(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let mut store = open_store(&parsed)?;
    store.create_run(
        &required_flag(&parsed, "run")?,
        &required_flag(&parsed, "goal")?,
        &required_flag(&parsed, "repo-root")?,
        &required_flag(&parsed, "report")?,
    )?;
    println!("OK: run initialized");
    Ok(())
}

fn cmd_task(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let operation = parsed
        .positionals
        .first()
        .ok_or_else(|| "task requires add or update".to_string())?;
    let mut store = open_store(&parsed)?;
    match operation.as_str() {
        "add" => store.add_task(&TaskInput {
            task_id: required_flag(&parsed, "task")?,
            run_id: required_flag(&parsed, "run")?,
            package_key: required_flag(&parsed, "package")?,
            title: required_flag(&parsed, "title")?,
            sequence: parse_u64_value(&parsed, "sequence")?,
            role: parse_value(&parsed, "role")?,
            complexity: parse_value(&parsed, "complexity")?,
            risk: parse_value(&parsed, "risk")?,
            uncertainty: parse_value(&parsed, "uncertainty")?,
            write_scope: flag_many(&parsed, "write-scope"),
            scope_hash: required_flag(&parsed, "scope-hash")?,
            repo_revision: required_flag(&parsed, "revision")?,
            review_boundary: flag_one(&parsed, "review-boundary"),
            required_profile: parse_value(&parsed, "profile")?,
        }),
        "update" => store.update_task(
            &required_flag(&parsed, "task")?,
            parse_value(&parsed, "status")?,
            flag_one(&parsed, "next-action").as_deref(),
        ),
        _ => Err(format!("unknown task operation: {operation}")),
    }?;
    println!("OK: task {operation}");
    Ok(())
}

fn cmd_session(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let operation = parsed
        .positionals
        .first()
        .ok_or_else(|| "session requires record, accept-followup, release, or close".to_string())?;
    let mut store = open_store(&parsed)?;
    let session_id = required_flag(&parsed, "session")?;
    match operation.as_str() {
        "record" => store.add_session(&SessionInput {
            session_id,
            run_id: required_flag(&parsed, "run")?,
            host: required_flag(&parsed, "host")?,
            handle: flag_one(&parsed, "handle"),
            role: parse_value(&parsed, "role")?,
            profile: parse_value(&parsed, "profile")?,
            requested_model: flag_one(&parsed, "requested-model"),
            actual_model: flag_one(&parsed, "actual-model"),
            routing_status: parse_value(&parsed, "routing-status")?,
            package_key: required_flag(&parsed, "package")?,
            scope_hash: required_flag(&parsed, "scope-hash")?,
            repo_revision: required_flag(&parsed, "revision")?,
            review_boundary: flag_one(&parsed, "review-boundary"),
            status: parse_value(&parsed, "status")?,
            current_task_id: flag_one(&parsed, "task"),
        }),
        "accept-followup" => {
            sessions::accept_followup(&mut store, &session_id, &required_flag(&parsed, "task")?)
        }
        "release" => sessions::release_verified(
            &mut store,
            &session_id,
            &required_flag(&parsed, "task")?,
            &required_flag(&parsed, "revision")?,
        ),
        "close" => store.update_session(
            &session_id,
            flag_one(&parsed, "status")
                .map(|value| value.parse())
                .transpose()?
                .unwrap_or(SessionStatus::Closed),
            flag_one(&parsed, "task").as_deref(),
            flag_one(&parsed, "reason").as_deref(),
        ),
        _ => Err(format!("unknown session operation: {operation}")),
    }?;
    println!("OK: session {operation}");
    Ok(())
}

fn cmd_decide(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let run_id = required_flag(&parsed, "run")?;
    let task_id = required_flag(&parsed, "task")?;
    let mut store = open_store(&parsed)?;
    let task = store.task(&run_id, &task_id)?;
    let demand = domain::RouteDemand {
        complexity: task.complexity,
        risk: task.risk,
        role: task.role,
        uncertainty: task.uncertainty,
    };
    let manual = flag_one(&parsed, "profile")
        .map(|value| value.parse::<Profile>())
        .transpose()?;
    let route_decision = route(&demand, manual);
    let request = DispatchRequest {
        run_id,
        task_id,
        signature: SessionSignature {
            host: required_flag(&parsed, "host")?,
            role: task.role,
            profile: route_decision.profile,
            package_key: task.package_key,
            scope_hash: task.scope_hash,
            repo_revision: task.repo_revision,
            review_boundary: task.review_boundary,
        },
        trivial: parse_bool_value(&parsed, "trivial", false)?,
        isolation_required: parse_bool_value(&parsed, "isolation-required", false)?,
        related_ready_count: usize::try_from(parse_u64_value(&parsed, "related-ready")?)
            .map_err(|_| "--related-ready is too large".to_string())?,
        delegation_value_exceeds_cost: parse_bool_value(
            &parsed,
            "delegation-value-exceeds-cost",
            true,
        )?,
        host_supports_followup: parse_bool_value(&parsed, "host-supports-followup", true)?,
    };
    let dispatch = decide(&mut store, &request)?;
    store.append_activity(&ActivityInput {
        run_id: request.run_id,
        task_id: Some(request.task_id),
        session_id: dispatch.session_id.clone(),
        kind: "route".into(),
        summary: format!("{} -> {:?}", route_decision.profile, dispatch.action),
    })?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "route": route_decision,
            "dispatch": dispatch
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn optional_tokens(parsed: &ParsedArgs, name: &str) -> Result<Option<u64>, String> {
    flag_one(parsed, name)
        .map(|value| {
            let parsed_value = value
                .parse::<u64>()
                .map_err(|error| format!("invalid --{name}: {error}"))?;
            if parsed_value.to_string() != value {
                return Err(format!("invalid noncanonical --{name}: {value}"));
            }
            Ok(parsed_value)
        })
        .transpose()
}

fn cmd_usage(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    if parsed.positionals.first().map(String::as_str) != Some("add") {
        return Err("usage requires add".into());
    }
    let mut store = open_store(&parsed)?;
    store.record_usage(&UsageInput {
        usage_id: required_flag(&parsed, "usage")?,
        run_id: required_flag(&parsed, "run")?,
        task_id: flag_one(&parsed, "task"),
        session_id: flag_one(&parsed, "session"),
        phase: parse_value(&parsed, "phase")?,
        input_tokens: optional_tokens(&parsed, "input")?,
        output_tokens: optional_tokens(&parsed, "output")?,
        reasoning_tokens: optional_tokens(&parsed, "reasoning")?,
        cache_read_tokens: optional_tokens(&parsed, "cache-read")?,
        cache_write_tokens: optional_tokens(&parsed, "cache-write")?,
        source: required_flag(&parsed, "source")?,
        quality: parse_value(&parsed, "quality")?,
    })?;
    println!("OK: usage recorded");
    Ok(())
}

fn cmd_status(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let store = open_store(&parsed)?;
    let view = build_status(&store, &required_flag(&parsed, "run")?)?;
    if parse_bool_value(&parsed, "json", false)? {
        println!("{}", render_json(&view)?);
    } else {
        let language = flag_one(&parsed, "lang")
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or(Language::ZhCn);
        print!("{}", render_text(&view, language));
    }
    Ok(())
}

fn cmd_watch(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let interval = flag_one(&parsed, "interval-ms")
        .unwrap_or_else(|| "1500".into())
        .parse::<u64>()
        .map_err(|error| format!("invalid --interval-ms: {error}"))?;
    let iterations = flag_one(&parsed, "iterations")
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|error| format!("invalid --iterations: {error}"))?;
    let language = flag_one(&parsed, "lang")
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(Language::ZhCn);
    let store = open_store(&parsed)?;
    let run_id = required_flag(&parsed, "run")?;
    let mut count = 0_u64;
    loop {
        print!(
            "\x1b[2J\x1b[H{}",
            render_text(&build_status(&store, &run_id)?, language)
        );
        count += 1;
        if iterations.is_some_and(|limit| count >= limit) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(interval));
    }
}

fn cmd_audit(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let store = open_store(&parsed)?;
    match store.final_audit(&required_flag(&parsed, "run")?) {
        Ok(()) => {
            println!("OK: final audit passed");
            Ok(())
        }
        Err(errors) => Err(errors.join("\n")),
    }
}

fn cmd_bundle(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let store = open_store(&parsed)?;
    let snapshot = store.snapshot(&required_flag(&parsed, "run")?)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&bundle::bundle_ready(&snapshot.tasks))
            .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn cmd_host_command(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let host = required_flag(&parsed, "host")?;
    let custom_templates = flag_one(&parsed, "templates");
    let templates = hosts::load_templates(custom_templates.as_deref().map(Path::new))?;
    let template = templates
        .get(&host)
        .ok_or_else(|| format!("unknown host template: {host}"))?;
    let operation = match required_flag(&parsed, "operation")?.as_str() {
        "spawn" => Operation::Spawn,
        "followup" => Operation::Followup,
        "close" => Operation::Close,
        value => return Err(format!("invalid --operation: {value}")),
    };
    let command = hosts::render_command(
        template,
        operation,
        &TemplateValues {
            prompt: flag_one(&parsed, "prompt"),
            session: flag_one(&parsed, "session"),
            model: flag_one(&parsed, "model"),
        },
        parse_value(&parsed, "profile")?,
    )?;
    println!(
        "{}",
        serde_json::to_string(&command).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn cmd_dashboard(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let db = required_flag(&parsed, "db")?;
    let bind = flag_one(&parsed, "bind")
        .unwrap_or_else(|| "127.0.0.1".into())
        .parse::<std::net::IpAddr>()
        .map_err(|error| format!("invalid --bind: {error}"))?;
    let port = flag_one(&parsed, "port")
        .unwrap_or_else(|| "7347".into())
        .parse::<u16>()
        .map_err(|error| format!("invalid --port: {error}"))?;
    let language = flag_one(&parsed, "lang")
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(Language::ZhCn);
    let address = dashboard::serve(
        Path::new(&db),
        &required_flag(&parsed, "run")?,
        dashboard::DashboardOptions {
            bind,
            port,
            language,
            allow_remote: parse_bool_value(&parsed, "allow-remote", false)?,
        },
    )?;
    println!("Dashboard: http://{address}");
    loop {
        std::thread::park();
    }
}

fn usage() -> &'static str {
    r#"usage:
  harnessctl render-prompt --role ROLE --report PATH [--brief PATH for worker] [--ledger PATH] [--allowed-write-paths PATH]...
  harnessctl check-prompt --file PATH [--max-lines N]
  harnessctl init --db DB --run ID --goal TEXT --repo-root PATH --report PATH
  harnessctl task add|update --db DB ...
  harnessctl decide --db DB --run ID --task ID --host HOST ...
  harnessctl session record|accept-followup|release|close --db DB ...
  harnessctl usage add --db DB --run ID --usage ID --phase PHASE --source SOURCE --quality QUALITY ...
  harnessctl status --db DB --run ID [--json true] [--lang zh-CN|en-US]
  harnessctl watch --db DB --run ID [--interval-ms 1500] [--iterations N]
  harnessctl audit --db DB --run ID
  harnessctl bundle --db DB --run ID
  harnessctl host-command --host HOST --operation spawn|followup|close --profile light|standard|deep [--templates FILE] ...
  harnessctl dashboard --db DB --run ID [--bind 127.0.0.1] [--port 7347] [--lang zh-CN|en-US]
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
        "init" => cmd_init(rest),
        "task" => cmd_task(rest),
        "decide" => cmd_decide(rest),
        "session" => cmd_session(rest),
        "usage" => cmd_usage(rest),
        "status" => cmd_status(rest),
        "watch" => cmd_watch(rest),
        "audit" => cmd_audit(rest),
        "bundle" => cmd_bundle(rest),
        "host-command" => cmd_host_command(rest),
        "dashboard" => cmd_dashboard(rest),
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
