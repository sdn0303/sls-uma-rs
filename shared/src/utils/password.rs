use passwords::PasswordGenerator;

pub fn generate_password() -> Result<String, &'static str> {
    let pg = PasswordGenerator::new()
        .length(8)
        .numbers(true)
        .lowercase_letters(true)
        .uppercase_letters(true)
        .symbols(true)
        .spaces(true)
        .exclude_similar_characters(true)
        .strict(true);
    pg.generate_one()
}
