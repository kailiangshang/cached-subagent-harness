use crate::domain::{HostTemplate, Operation, Profile, TemplateValues};
use std::collections::BTreeMap;

const BUNDLED_TEMPLATES: &str = include_str!("../../../references/host-templates.json");

pub(crate) fn bundled_templates() -> Result<BTreeMap<String, HostTemplate>, String> {
    serde_json::from_str(BUNDLED_TEMPLATES).map_err(|error| error.to_string())
}

pub(crate) fn render_command(
    template: &HostTemplate,
    operation: Operation,
    values: &TemplateValues,
    profile: Profile,
) -> Result<Vec<String>, String> {
    let base = match operation {
        Operation::Spawn => Some(&template.spawn_command),
        Operation::Followup => template.followup_command.as_ref(),
        Operation::Close => template.close_command.as_ref(),
    }
    .ok_or_else(|| format!("{} does not support {operation:?}", template.name))?;
    if base.is_empty() || base[0].trim().is_empty() {
        return Err("host command requires a nonempty executable".into());
    }
    let mut parts = base.clone();
    if let Some(profile_parts) = template.profile_arguments.get(&profile) {
        parts.extend(profile_parts.iter().cloned());
    }
    parts
        .into_iter()
        .map(|part| substitute(&part, values))
        .collect()
}

fn substitute(part: &str, values: &TemplateValues) -> Result<String, String> {
    let mut rendered = part.to_string();
    for (placeholder, value) in [
        ("{prompt}", values.prompt.as_deref()),
        ("{session}", values.session.as_deref()),
        ("{model}", values.model.as_deref()),
    ] {
        if rendered.contains(placeholder) {
            let value = value.ok_or_else(|| {
                format!("missing {} value", &placeholder[1..placeholder.len() - 1])
            })?;
            rendered = rendered.replace(placeholder, value);
        }
    }
    if rendered.contains('{') || rendered.contains('}') {
        return Err(format!(
            "unknown placeholder in command argument: {rendered}"
        ));
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::{bundled_templates, render_command};
    use crate::domain::{Operation, Profile, TemplateValues};

    #[test]
    fn bundled_templates_cover_primary_hosts_and_render_argument_arrays() {
        let templates = bundled_templates().unwrap();
        for host in ["codex", "claude", "opencode"] {
            assert!(templates.contains_key(host), "missing {host}");
        }
        let command = render_command(
            &templates["codex"],
            Operation::Spawn,
            &TemplateValues {
                prompt: Some("do work; never shell this".into()),
                session: None,
                model: Some("gpt-5-mini".into()),
            },
            Profile::Light,
        )
        .unwrap();
        assert_eq!(command[0], "codex");
        assert!(command.iter().any(|arg| arg == "do work; never shell this"));
        assert!(!command.iter().any(|arg| arg == "sh" || arg == "bash"));
    }

    #[test]
    fn rendering_rejects_missing_values_and_unsupported_operations() {
        let templates = bundled_templates().unwrap();
        let missing = render_command(
            &templates["claude"],
            Operation::Followup,
            &TemplateValues {
                prompt: Some("next".into()),
                session: None,
                model: None,
            },
            Profile::Standard,
        )
        .unwrap_err();
        assert!(missing.contains("session"));
        assert!(
            render_command(
                &templates["opencode"],
                Operation::Close,
                &TemplateValues {
                    prompt: None,
                    session: Some("s1".into()),
                    model: None
                },
                Profile::Standard,
            )
            .is_err()
        );
    }
}
