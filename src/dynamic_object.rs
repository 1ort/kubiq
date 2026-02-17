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
        self.fields.get(path)
    }
}
