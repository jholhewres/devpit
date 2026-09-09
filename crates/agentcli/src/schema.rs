//! Whether an answer is the shape the step asked for.
//!
//! Apart from running the turn because it is a different question: one is
//! about a process, this is about a string.

/// Whether the answer satisfies the schema the step declared.
///
/// Deliberately shallow: required keys and their types, not a full JSON Schema
/// engine. A step's schema is written next to the step, and the failure that
/// matters is "the agent answered prose where the card expected fields".
pub fn validates(answer: &str, schema: &str) -> Result<(), String> {
    let answer: serde_json::Value =
        serde_json::from_str(answer).map_err(|_| "the answer is not JSON".to_owned())?;
    let schema: serde_json::Value =
        serde_json::from_str(schema).map_err(|_| "the schema is not JSON".to_owned())?;

    let Some(required) = schema.get("required").and_then(|r| r.as_array()) else {
        return Ok(());
    };
    let properties = schema.get("properties");

    for key in required.iter().filter_map(|k| k.as_str()) {
        let Some(value) = answer.get(key) else {
            return Err(format!("the answer has no `{key}`"));
        };
        let expected = properties
            .and_then(|p| p.get(key))
            .and_then(|p| p.get("type"))
            .and_then(|t| t.as_str());
        let matches = match expected {
            Some("string") => value.is_string(),
            Some("number") => value.is_number(),
            Some("boolean") => value.is_boolean(),
            Some("array") => value.is_array(),
            Some("object") => value.is_object(),
            _ => true,
        };
        if !matches {
            return Err(format!(
                "`{key}` should be {} and is {value}",
                expected.unwrap_or("something else")
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
