pub fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}
