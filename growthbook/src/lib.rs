use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

/// Explicit bridge keys. Exported Claude Code flags (for example, tengu_* and
/// cai_*) are intentionally not treated as Codex controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureMapping {
    pub growthbook_key: &'static str,
    pub codex_key: &'static str,
    pub codex_default: bool,
}

pub const FEATURE_MAPPINGS: &[FeatureMapping] = &[
    FeatureMapping { growthbook_key: "codex.shell_tool", codex_key: "shell_tool", codex_default: true },
    FeatureMapping { growthbook_key: "codex.unified_exec", codex_key: "unified_exec", codex_default: true },
    FeatureMapping { growthbook_key: "codex.shell_snapshot", codex_key: "shell_snapshot", codex_default: true },
    FeatureMapping { growthbook_key: "codex.view_image", codex_key: "view_image", codex_default: true },
    FeatureMapping { growthbook_key: "codex.sleep_tool", codex_key: "sleep_tool", codex_default: true },
    FeatureMapping { growthbook_key: "codex.hooks", codex_key: "hooks", codex_default: true },
    FeatureMapping { growthbook_key: "codex.multi_agent", codex_key: "multi_agent", codex_default: true },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureDecision {
    pub growthbook_key: &'static str,
    pub codex_key: &'static str,
    pub enabled: bool,
}

/// Load a GrowthBook export directly from disk. Supports a flat export
/// (key -> native JSON value) and GrowthBook feature envelopes
/// (key -> { "defaultValue": ... }). No SDK, HTTP client, or network is used.
pub fn load_export(path: impl AsRef<Path>) -> Result<BTreeMap<String, Value>> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .with_context(|| format!("could not read local GrowthBook export '{}'", path.display()))?;
    let value: Value = serde_json::from_str(&contents)
        .with_context(|| format!("invalid JSON in '{}'", path.display()))?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("GrowthBook export must be a top-level JSON object"))?;

    Ok(object.iter().map(|(key, value)| (key.clone(), value.clone())).collect())
}

fn boolean_value(value: &Value) -> Option<bool> {
    if let Some(value) = value.as_bool() {
        return Some(value);
    }
    value.get("defaultValue").and_then(Value::as_bool)
}

/// Evaluate only the explicit Codex allowlist. Unknown keys and unrelated
/// Claude Code flags are ignored; non-boolean mapped values fail closed.
pub fn evaluate_codex_flags(
    features: &BTreeMap<String, Value>,
) -> Result<Vec<FeatureDecision>> {
    let mut decisions = Vec::new();
    for mapping in FEATURE_MAPPINGS {
        let Some(value) = features.get(mapping.growthbook_key) else {
            continue;
        };
        let enabled = boolean_value(value).ok_or_else(|| {
            anyhow::anyhow!(
                "mapped feature '{}' must be a boolean or contain boolean defaultValue",
                mapping.growthbook_key
            )
        })?;
        decisions.push(FeatureDecision {
            growthbook_key: mapping.growthbook_key,
            codex_key: mapping.codex_key,
            enabled,
        });
    }
    Ok(decisions)
}

/// Build argv without shell interpolation. Only compile-time allowlisted Codex
/// keys can be emitted by the evaluator.
pub fn build_codex_args(
    original_args: &[OsString],
    decisions: &[FeatureDecision],
) -> Vec<OsString> {
    let mut args = Vec::with_capacity(decisions.len() * 2 + original_args.len());
    for decision in decisions {
        args.push(OsString::from(if decision.enabled { "--enable" } else { "--disable" }));
        args.push(OsString::from(decision.codex_key));
    }
    args.extend(original_args.iter().cloned());
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn feature_map(value: Value) -> BTreeMap<String, Value> {
        value.as_object().unwrap().iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    #[test]
    fn reads_flat_native_boolean_values_and_ignores_claude_flags() {
        let features = feature_map(json!({
            "codex.shell_tool": false,
            "tengu-agents-md-mod": true,
            "cai_proud_planet": false,
            "tengu-off-switch": { "activated": false }
        }));
        let decisions = evaluate_codex_flags(&features).unwrap();
        assert_eq!(decisions, vec![FeatureDecision {
            growthbook_key: "codex.shell_tool",
            codex_key: "shell_tool",
            enabled: false,
        }]);
    }

    #[test]
    fn reads_growthbook_default_value_envelope() {
        let features = feature_map(json!({
            "codex.hooks": { "defaultValue": true }
        }));
        let decisions = evaluate_codex_flags(&features).unwrap();
        assert_eq!(decisions[0].codex_key, "hooks");
        assert!(decisions[0].enabled);
    }

    #[test]
    fn missing_keys_leave_codex_defaults_untouched() {
        let features = feature_map(json!({ "tengu-agents-md-mod": true }));
        assert!(evaluate_codex_flags(&features).unwrap().is_empty());
    }

    #[test]
    fn rejects_non_boolean_mapped_values() {
        let features = feature_map(json!({ "codex.shell_tool": "true" }));
        assert!(evaluate_codex_flags(&features).is_err());
    }

    #[test]
    fn builds_arguments_without_shell_interpolation() {
        let decisions = vec![FeatureDecision {
            growthbook_key: "codex.shell_tool",
            codex_key: "shell_tool",
            enabled: false,
        }];
        let original = vec![OsString::from("-p"), OsString::from("inspect this repo")];
        assert_eq!(build_codex_args(&original, &decisions), vec![
            OsString::from("--disable"),
            OsString::from("shell_tool"),
            OsString::from("-p"),
            OsString::from("inspect this repo"),
        ]);
    }
}
