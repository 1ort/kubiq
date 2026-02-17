use crate::dynamic_object::DynamicObject;

pub fn print_table(objects: &[DynamicObject]) {
    println!("items: {}", objects.len());
}
