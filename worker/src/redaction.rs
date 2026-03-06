use tracing::instrument;

pub fn redact_secret(secret: &str) -> String {
    if secret.len() <= 8 {
        "*".repeat(secret.len())
    } else {
        format!("{}...{}", &secret[..4], &secret[secret.len() - 4..])
    }
}

pub fn redact_if_secret(key: &str, value: &str) -> String {
    let lower_key = key.to_lowercase();
    if lower_key.contains("secret")
        || lower_key.contains("private")
        || lower_key.contains("key")
        || lower_key.contains("password")
        || lower_key.contains("token")
        || lower_key.contains("credential")
    {
        redact_secret(value)
    } else {
        value.to_string()
    }
}

#[instrument(skip_all)]
pub fn log_safe_config(config: &serde_json::Value) -> serde_json::Value {
    match config {
        serde_json::Value::Object(map) => {
            let mut safe_map = serde_json::Map::new();
            for (key, value) in map {
                let safe_key = key.clone();
                let safe_value = match value {
                    serde_json::Value::String(s) => {
                        serde_json::Value::String(redact_if_secret(key, s))
                    }
                    serde_json::Value::Object(_) => log_safe_config(value),
                    serde_json::Value::Array(arr) => {
                        let safe_arr: Vec<serde_json::Value> =
                            arr.iter().map(|v| log_safe_config(v)).collect();
                        serde_json::Value::Array(safe_arr)
                    }
                    _ => value.clone(),
                };
                safe_map.insert(safe_key, safe_value);
            }
            serde_json::Value::Object(safe_map)
        }
        serde_json::Value::Array(arr) => {
            let safe_arr: Vec<serde_json::Value> = arr.iter().map(|v| log_safe_config(v)).collect();
            serde_json::Value::Array(safe_arr)
        }
        _ => config.clone(),
    }
}
