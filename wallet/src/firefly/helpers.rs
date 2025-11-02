/// Helper functions for Rholang/F1r3fly integration

/// Convert a Rholang expression (from explore-deploy) to plain JSON
/// This recursively unwraps ExprMap, ExprString, ExprInt, ExprBool, etc.
pub fn convert_rholang_to_json(value: &serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Handle ExprMap
    if let Some(expr_map) = value.get("ExprMap").and_then(|v| v.get("data")) {
        let mut result = serde_json::Map::new();
        if let Some(map_obj) = expr_map.as_object() {
            for (key, val) in map_obj {
                result.insert(key.clone(), convert_rholang_to_json(val)?);
            }
        }
        return Ok(serde_json::Value::Object(result));
    }
    
    // Handle ExprString
    if let Some(expr_str) = value.get("ExprString").and_then(|v| v.get("data")) {
        return Ok(expr_str.clone());
    }
    
    // Handle ExprInt
    if let Some(expr_int) = value.get("ExprInt").and_then(|v| v.get("data")) {
        return Ok(expr_int.clone());
    }
    
    // Handle ExprBool
    if let Some(expr_bool) = value.get("ExprBool").and_then(|v| v.get("data")) {
        return Ok(expr_bool.clone());
    }
    
    // Handle arrays
    if let Some(arr) = value.as_array() {
        let mut result = Vec::new();
        for item in arr {
            result.push(convert_rholang_to_json(item)?);
        }
        return Ok(serde_json::Value::Array(result));
    }
    
    // If we can't convert, return as-is
    Ok(value.clone())
}

