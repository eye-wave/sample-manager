pub use tinyjson::JsonValue;

pub fn get_str<'a>(obj: &'a JsonValue, key: &str) -> Option<&'a str> {
    match obj {
        JsonValue::Object(map) => match map.get(key)? {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

pub fn get_f64(obj: &JsonValue, key: &str) -> Option<f64> {
    match obj {
        JsonValue::Object(map) => match map.get(key)? {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        },
        _ => None,
    }
}

pub fn get_bool(obj: &JsonValue, key: &str) -> Option<bool> {
    match obj {
        JsonValue::Object(map) => match map.get(key)? {
            JsonValue::Boolean(b) => Some(*b),
            _ => None,
        },
        _ => None,
    }
}

pub fn get_array<'a>(obj: &'a JsonValue, key: &str) -> Option<&'a Vec<JsonValue>> {
    match obj {
        JsonValue::Object(map) => match map.get(key)? {
            JsonValue::Array(a) => Some(a),
            _ => None,
        },
        _ => None,
    }
}

pub fn get_obj<'a>(
    obj: &'a JsonValue,
    key: &str,
) -> Option<&'a std::collections::HashMap<String, JsonValue>> {
    match obj {
        JsonValue::Object(map) => match map.get(key)? {
            JsonValue::Object(o) => Some(o),
            _ => None,
        },
        _ => None,
    }
}

pub fn parse(s: &str) -> Option<JsonValue> {
    s.parse().ok()
}

pub fn stringify(val: &JsonValue) -> String {
    val.stringify().unwrap_or_else(|_| "null".into())
}
