use crate::config::{ConvertOptions, Mode, Target};
use crate::error::ProviderCompatError;
use serde_json::Value;

pub struct ProviderCompatResult {
    pub errors: Vec<ProviderCompatError>,
}

pub fn check_provider_compat(_schema: &Value, config: &ConvertOptions) -> ProviderCompatResult {
    match config.target {
        Target::OpenaiStrict if config.mode == Mode::Strict => {
            // Placeholder checks — each will be implemented in PR 2/3:
            // - check_root_type(schema)
            // - check_depth_budget(schema, 10)
            // - check_enum_homogeneity(schema)
            // - check_boolean_schemas(schema)
            ProviderCompatResult { errors: vec![] }
        }
        _ => ProviderCompatResult { errors: vec![] },
    }
}
