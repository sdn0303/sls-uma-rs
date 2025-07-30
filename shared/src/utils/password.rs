use passwords::PasswordGenerator;

const PASSWORD_LENGTH: usize = 24;

pub fn generate_password() -> Result<String, &'static str> {
    PasswordGenerator::new()
        .length(PASSWORD_LENGTH)
        .numbers(true)
        .lowercase_letters(true)
        .uppercase_letters(true)
        .symbols(true)
        .spaces(true)
        .exclude_similar_characters(true)
        .strict(true)
        .generate_one()
}
