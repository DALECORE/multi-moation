use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use growthbook_rust::client::{GrowthBookClient, GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::Value;

/// GrowthBook keys are owned by this project and are intentionally namespaced.
/// Only stable, public Codex settings are mapped. Internal OpenAI rollout keys,
/// authentication gates, managed-policy controls, and experimental features
/// are deliberately excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureMapping {
    pub growthbook_key: &'static str,
    pub codex_key: &'static str,
    pub codex_default: bool,
}

pub const FEATURE_MAPPINGS: &[FeatureMapping] = &[
    FeatureMapping {
        growthbook_key: "codex.shell_tool",
        codex_key: "shell_tool",
        codex_default: true,
    },
    FeatureMapping {
        growthbook_key: "codex.unified_exec",
        codex_key: "unified_exec",
        codex_default: true,
    },
    FeatureMapping {
        growthbook_key: "codex.shell_snapshot",
        codex_key: "shell_snapshot",
        codex_default: true,
    },
    FeatureMapping {
        growthbook_key: "codex.view_image",
        codex_key: "view_image",
        codex_default: true,
    },
    FeatureMapping {
        growthbook_key: "codex.sleep_tool",
        codex_key: "sleep_tool",
        codex_default: true,
    },
    FeatureMapping {
        growthbook_key: "codex.hooks",
        codex_key: "hooks",
        codex_default: true,
    },
    FeatureMapping {
        growthbook_key: "codex.multi_agent",
        codex_key: "multi_agent",
        codex_default: true,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureDecision {
    pub growthbook_key: &'static str,
    pub codex_key: &'static str,
    pub enabled: bool,
}

/// Connects to GrowthBook and fetches a fresh feature payload before returning.
/// Failure is surfaced; callers must not silently launch Codex with stale remote
/// decisions when GrowthBook was explicitly configured as the control plane.
pub async fn load_growthbook_client() -> Result<GrowthBookClient> {
    let sdk_key = env::var("GB_SDK_KEY")
        .context("GB_SDK_KEY is required (use a GrowthBook SDK client key)")?;
    if sdk_key.trim().is_empty() {
        bail!("GB_SDK_KEY must not be empty");
    }

    let api_url = env::var("GB_URL")
        .unwrap_or_else(|_| "https://cdn.growthbook.io".to_string());

    let mut builder = GrowthBookClientBuilder::new()
        .api_url(api_url)
        .client_key(sdk_key)
        .ttl(Duration::from_secs(0))
        .auto_refresh(false);

    if let Ok(decryption_key) = env::var("GB_DECRYPTION_KEY") {
        if !decryption_key.trim().is_empty() {
            builder = builder.decryption_key(decryption_key);
        }
    }

    // Attributes are opt-in. The launcher does not invent or collect a user ID.
    if let Ok(raw_attributes) = env::var("GB_ATTRIBUTES_JSON") {
        let value: Value = serde_json::from_str(&raw_attributes)
            .context("GB_ATTRIBUTES_JSON must contain a JSON object")?;
        if !value.is_object() {
            bail!("GB_ATTRIBUTES_JSON must contain a JSON object");
        }

        let attributes: HashMap<String, GrowthBookAttribute> =
            GrowthBookAttribute::from(value)
                .map_err(|error| anyhow!("invalid GrowthBook attributes: {error}"))?
                .into_iter()
                .map(|attribute| (attribute.key.clone(), attribute))
                .collect();
        builder = builder.attributes(attributes);
    }

    builder
        .build()
        .await
        .map_err(|error| anyhow!("GrowthBook feature fetch failed: {error}"))
}

/// Evaluates only the allowlisted mappings above. Missing GrowthBook keys are
/// ignored, preserving the existing Codex config/default for that feature.
/// Non-boolean values are rejected rather than being coerced.
pub fn evaluate_codex_flags(client: &GrowthBookClient) -> Result<Vec<FeatureDecision>> {
    let mut decisions = Vec::new();

    for mapping in FEATURE_MAPPINGS {
        let result = client.feature_result(mapping.growthbook_key, None);
        if result.source == "unknownFeature" {
            continue;
        }

        let enabled = result.value_as::<bool>().map_err(|error| {
            anyhow!(
                "GrowthBook feature '{}' must resolve to a boolean: {error}",
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

/// Builds argv without a shell. GrowthBook values can only select a Codex key
/// from FEATURE_MAPPINGS, so they cannot inject arbitrary command-line options.
pub fn build_codex_args(
    original_args: &[OsString],
    decisions: &[FeatureDecision],
) -> Vec<OsString> {
    let mut args = Vec::with_capacity(decisions.len() * 2 + original_args.len());

    for decision in decisions {
        args.push(OsString::from(if decision.enabled {
            "--enable"
        } else {
            "--disable"
        }));
        args.push(OsString::from(decision.codex_key));
    }

    args.extend(original_args.iter().cloned());
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn client_with(features: Value) -> GrowthBookClient {
        GrowthBookClientBuilder::new()
            .features_json(features)
            .expect("valid test feature payload")
            .build()
            .await
            .expect("manual feature client should build without network")
    }

    #[tokio::test]
    async fn evaluates_only_allowlisted_codex_features() {
        let client = client_with(json!({
            "codex.shell_tool": { "defaultValue": false },
            "codex.unified_exec": { "defaultValue": true },
            "codex.private_internal_flag": { "defaultValue": true }
        }))
        .await;

        let decisions = evaluate_codex_flags(&client).expect("boolean features");
        assert_eq!(
            decisions,
            vec![
                FeatureDecision {
                    growthbook_key: "codex.shell_tool",
                    codex_key: "shell_tool",
                    enabled: false
                },
                FeatureDecision {
                    growthbook_key: "codex.unified_exec",
                    codex_key: "unified_exec",
                    enabled: true
                }
            ]
        );
    }

    #[tokio::test]
    async fn missing_growthbook_keys_leave_codex_defaults_untouched() {
        let client = client_with(json!({
            "codex.view_image": { "defaultValue": false }
        }))
        .await;

        let decisions = evaluate_codex_flags(&client).expect("valid feature set");
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].codex_key, "view_image");
        assert!(!decisions[0].enabled);
    }

    #[tokio::test]
    async fn rejects_non_boolean_growthbook_values() {
        let client = client_with(json!({
            "codex.shell_tool": { "defaultValue": "true" }
        }))
        .await;

        let error = evaluate_codex_flags(&client).expect_err("string is not a boolean");
        assert!(error.to_string().contains("must resolve to a boolean"));
    }

    #[test]
    fn builds_codex_argv_without_shell_interpolation() {
        let decisions = vec![
            FeatureDecision {
                growthbook_key: "codex.shell_tool",
                codex_key: "shell_tool",
                enabled: true,
            },
            FeatureDecision {
                growthbook_key: "codex.unified_exec",
                codex_key: "unified_exec",
                enabled: false,
            },
        ];
        let original = vec![OsString::from("-p"), OsString::from("inspect this repo")];

        assert_eq!(
            build_codex_args(&original, &decisions),
            vec![
                OsString::from("--enable"),
                OsString::from("shell_tool"),
                OsString::from("--disable"),
                OsString::from("unified_exec"),
                OsString::from("-p"),
                OsString::from("inspect this repo")
            ]
        );
    }
}
