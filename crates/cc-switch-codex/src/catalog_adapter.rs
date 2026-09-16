use super::*;

/// Thin adapter to the unmodified CC Switch catalog builders above.
pub fn entry(
    model: &str,
    display: &str,
    context: u64,
    overrides: &Value,
    native: bool,
    official_deepseek: bool,
) -> Value {
    let template: Value = serde_json::from_str(if native {
        include_str!("resources/codex_native_responses_template.json")
    } else {
        include_str!("resources/gpt5_5_template.json")
    })
    .expect("bundled CC template");
    let template = template
        .get("models")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .unwrap_or(&template);
    let strings = |key: &str| {
        overrides[key].as_array().map(|v| {
            v.iter()
                .filter_map(|s| s.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
    };
    let spec = CodexCatalogModelSpec {
        model: model.into(),
        display_name: Some(display.into()),
        context_window: Some(context),
        supports_parallel_tool_calls: overrides["parallelToolCalls"].as_bool(),
        input_modalities: strings("inputModalities"),
        base_instructions: overrides["baseInstructions"].as_str().map(str::to_owned),
        reasoning_levels: strings("reasoningLevels"),
        default_reasoning_level: overrides["defaultReasoningLevel"]
            .as_str()
            .map(str::to_owned),
    };
    let mut entry = codex_catalog_model_entry(
        template,
        &spec,
        0,
        if native {
            CodexCatalogToolProfile::NativeResponses
        } else {
            CodexCatalogToolProfile::ProxyChat
        },
        context,
    );
    if native && official_deepseek {
        let vendor: Value = serde_json::from_str(include_str!(
            "resources/codex_deepseek_catalog_template.json"
        ))
        .expect("bundled vendor catalog");
        if let Some(rows) = vendor["models"].as_array() {
            entry = codex_vendor_catalog_model_entry(rows, &spec, 0);
        }
    }
    if let Some(v) = overrides.get("parallelToolCalls") {
        entry["supports_parallel_tool_calls"] = v.clone();
    }
    entry
}
