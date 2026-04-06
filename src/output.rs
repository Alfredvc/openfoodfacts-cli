use serde_json::Value;
use std::io::IsTerminal;

pub struct Output {
    force_compact: bool,
    fields: Vec<String>,
}

impl Output {
    pub fn new(force_compact: bool, fields: Vec<String>) -> Self {
        Self { force_compact, fields }
    }

    pub fn print(&self, value: &Value) {
        let filtered = self.filter_fields(value.clone());
        let s = if self.force_compact || !std::io::stdout().is_terminal() {
            serde_json::to_string(&filtered).unwrap()
        } else {
            serde_json::to_string_pretty(&filtered).unwrap()
        };
        println!("{s}");
    }

    pub fn filter_fields(&self, value: Value) -> Value {
        if self.fields.is_empty() {
            return value;
        }
        match value {
            Value::Object(ref map) if map.contains_key("products") => {
                let mut new_map = map.clone();
                if let Some(Value::Array(products)) = new_map.get("products").cloned() {
                    let filtered: Vec<Value> = products
                        .into_iter()
                        .map(|p| self.filter_object(&p))
                        .collect();
                    new_map.insert("products".to_string(), Value::Array(filtered));
                }
                Value::Object(new_map)
            }
            Value::Array(arr) => {
                Value::Array(arr.into_iter().map(|v| self.filter_object(&v)).collect())
            }
            Value::Object(_) => self.filter_object(&value),
            other => other,
        }
    }

    fn filter_object(&self, value: &Value) -> Value {
        if let Value::Object(map) = value {
            let filtered = map
                .iter()
                .filter(|(k, _)| self.fields.iter().any(|f| f == k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Value::Object(filtered)
        } else {
            value.clone()
        }
    }
}
