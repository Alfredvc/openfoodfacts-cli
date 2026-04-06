use serde_json::Value;
use std::io::IsTerminal;

pub struct Output {
    force_compact: bool,
    fields: Vec<String>,
}

impl Output {
    pub fn new(force_compact: bool, fields: Vec<String>) -> Self {
        Self {
            force_compact,
            fields,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn output_with_fields(fields: &[&str]) -> Output {
        Output::new(true, fields.iter().map(|s| s.to_string()).collect())
    }

    fn output_all_fields() -> Output {
        Output::new(true, vec![])
    }

    #[test]
    fn filter_fields_empty_returns_value_unchanged() {
        let out = output_all_fields();
        let v = json!({"a": 1, "b": 2});
        assert_eq!(out.filter_fields(v.clone()), v);
    }

    #[test]
    fn filter_fields_on_object() {
        let out = output_with_fields(&["product_name", "brands"]);
        let v = json!({"product_name": "Nutella", "brands": "Ferrero", "nutriscore_grade": "e"});
        let result = out.filter_fields(v);
        assert_eq!(
            result,
            json!({"product_name": "Nutella", "brands": "Ferrero"})
        );
    }

    #[test]
    fn filter_fields_on_flat_array() {
        let out = output_with_fields(&["code"]);
        let v = json!([{"code": "123", "name": "A"}, {"code": "456", "name": "B"}]);
        let result = out.filter_fields(v);
        assert_eq!(result, json!([{"code": "123"}, {"code": "456"}]));
    }

    #[test]
    fn filter_fields_preserves_pagination_envelope() {
        let out = output_with_fields(&["code", "product_name"]);
        let v = json!({
            "count": 100,
            "page": 1,
            "page_count": 5,
            "page_size": 20,
            "skip": 0,
            "products": [
                {"code": "123", "product_name": "A", "brands": "X"},
                {"code": "456", "product_name": "B", "brands": "Y"}
            ]
        });
        let result = out.filter_fields(v);
        // Envelope keys preserved
        assert_eq!(result["count"], 100);
        assert_eq!(result["page"], 1);
        assert_eq!(result["page_count"], 5);
        assert_eq!(result["page_size"], 20);
        assert_eq!(result["skip"], 0);
        // Items filtered
        assert_eq!(result["products"].as_array().unwrap().len(), 2);
        assert_eq!(
            result["products"][0],
            json!({"code": "123", "product_name": "A"})
        );
        assert_eq!(
            result["products"][1],
            json!({"code": "456", "product_name": "B"})
        );
    }

    #[test]
    fn filter_fields_nonexistent_field_returns_empty_object() {
        let out = output_with_fields(&["nonexistent"]);
        let v = json!({"a": 1, "b": 2});
        let result = out.filter_fields(v);
        assert_eq!(result, json!({}));
    }

    #[test]
    fn filter_fields_empty_products_array() {
        let out = output_with_fields(&["code", "product_name"]);
        let v = json!({"count": 0, "page": 1, "page_count": 0, "page_size": 20, "skip": 0, "products": []});
        let result = out.filter_fields(v);
        assert_eq!(result["count"], 0);
        assert_eq!(result["products"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn filter_fields_non_object_passthrough() {
        let out = output_with_fields(&["x"]);
        assert_eq!(out.filter_fields(json!(42)), json!(42));
        assert_eq!(out.filter_fields(json!("hello")), json!("hello"));
        assert_eq!(out.filter_fields(json!(null)), json!(null));
    }
}
