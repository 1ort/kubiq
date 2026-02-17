use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DynamicObject {
    pub fields: BTreeMap<String, String>,
}

impl DynamicObject {
    pub fn get(
        &self,
        path: &str,
    ) -> Option<&str> {
        self.fields.get(path).map(String::as_str)
    }
}
