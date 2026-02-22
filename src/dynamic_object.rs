use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DynamicObject {
    pub fields: BTreeMap<String, Value>,
}

impl DynamicObject {
    pub fn get(
        &self,
        path: &str,
    ) -> Option<&Value> {
        self.fields
            .get(path)
            .or_else(|| {
                self.fields
                    .iter()
                    .find(|(encoded_path, _)| crate::path::decode_path(encoded_path) == path)
                    .map(|(_, value)| value)
            })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::Value;

    use super::DynamicObject;

    #[test]
    fn get_reads_plain_path() {
        let mut fields = BTreeMap::new();
        fields.insert("metadata.name".to_string(), Value::String("worker-a".to_string()));
        let object = DynamicObject { fields };
        assert_eq!(
            object.get("metadata.name"),
            Some(&Value::String("worker-a".to_string()))
        );
    }

    #[test]
    fn get_reads_encoded_path_via_raw_query_path() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "metadata.annotations.kubectl%2Ekubernetes%2Eio/restartedAt".to_string(),
            Value::String("2026-02-22T10:00:00Z".to_string()),
        );
        let object = DynamicObject { fields };
        assert_eq!(
            object.get("metadata.annotations.kubectl.kubernetes.io/restartedAt"),
            Some(&Value::String("2026-02-22T10:00:00Z".to_string()))
        );
    }
}
