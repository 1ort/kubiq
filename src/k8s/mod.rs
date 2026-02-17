use crate::dynamic_object::DynamicObject;

pub fn list(resource: &str) -> Result<Vec<DynamicObject>, String> {
    if resource.trim().is_empty() {
        return Err("resource name is empty".to_string());
    }

    Ok(Vec::new())
}
