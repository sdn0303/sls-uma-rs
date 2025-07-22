use once_cell::sync::Lazy;
use regex::Regex;

// RFC 5322 compliant email regex pattern with practical TLD length requirement (2+ chars)
pub static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)^(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9][a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$"#
    ).unwrap()
});

// Human name regex pattern supporting English, Japanese, and mixed patterns
pub static USERNAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[\p{L}][\p{L}'\.\-]*(?:\s+[\p{L}][\p{L}'\.\-]*){0,2}$").unwrap());

// Additional validation for name length (1-50 characters)
fn is_valid_username_length(name: &str) -> bool {
    let len = name.chars().count();
    (1..=50).contains(&len) && !name.trim().is_empty()
}

// Additional validation to ensure proper name formatting
fn is_well_formatted_username(name: &str) -> bool {
    let trimmed = name.trim();

    // Check if trimmed name is different from original (leading/trailing spaces)
    if trimmed != name {
        return false;
    }

    // Check for consecutive spaces
    if name.contains("  ") {
        return false;
    }

    // Check for consecutive punctuation in English names
    if name.contains("--") || name.contains("''") || name.contains("..") {
        return false;
    }

    // Split by spaces to check individual name parts
    let parts: Vec<&str> = name.split_whitespace().collect();

    // Check if it's a valid pattern (1-3 parts for English, 1-2 for Japanese)
    if parts.is_empty() || parts.len() > 3 {
        return false;
    }

    // Allow mixing of English and Japanese characters
    // No additional restrictions on script mixing

    // For any names with punctuation, check each part doesn't start/end inappropriately
    for part in &parts {
        if part.starts_with(['-', '\'', '.']) || part.ends_with(['-', '\'']) {
            return false;
        }
    }

    true
}

// Complete username validation combining regex, length, and formatting checks
pub fn is_valid_username(name: &str) -> bool {
    USERNAME_REGEX.is_match(name)
        && is_valid_username_length(name)
        && is_well_formatted_username(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_regex_valid_emails() {
        let valid_emails = [
            "test@example.com",
            "user.name@domain.co.jp",
            "user+tag@example.org",
            "first.last@subdomain.example.com",
            "user123@example123.com",
            "test_user@example-domain.net",
            "user%test@domain.info",
            "user@domain-with-dash.com",
            "test123+tag@example.com",
            "a@b.co",
            "very.common@example.com",
            "disposable.style.email.with+symbol@example.com",
            "x@example.com",
            "example@s.example",
            "test@example-one.com",
            "test@example.name",
        ];

        for email in &valid_emails {
            assert!(
                EMAIL_REGEX.is_match(email),
                "Email '{}' should be valid",
                email
            );
        }
    }

    #[test]
    fn test_email_regex_invalid_emails() {
        let invalid_emails = [
            "invalid-email",         // No @ symbol
            "@example.com",          // Missing local part
            "user@",                 // Missing domain
            "user@domain",           // Missing TLD
            "user.domain.com",       // Missing @ symbol
            "user@domain.",          // Domain ends with dot
            "",                      // Empty string
            "user@domain.c",         // TLD too short
            "user name@domain.com",  // Space in local part
            "user..name@domain.com", // Consecutive dots in local part
            ".user@domain.com",      // Local part starts with dot
            "user.@domain.com",      // Local part ends with dot
            "user@domain..com",      // Consecutive dots in domain
            "user@.domain.com",      // Domain starts with dot
            "user@domain-.com",      // Domain label ends with hyphen
            "user@-domain.com",      // Domain label starts with hyphen
            "user@domain.com.",      // Trailing dot after domain
            "user@@domain.com",      // Double @ symbol
            "user@domain@com",       // Multiple @ symbols
            "user@domain,com",       // Comma instead of dot
        ];

        for email in &invalid_emails {
            assert!(
                !EMAIL_REGEX.is_match(email),
                "Email '{}' should be invalid",
                email
            );
        }
    }

    #[test]
    fn test_english_name_patterns() {
        let valid_english_names = [
            // Single names (first name only)
            "John",
            "Mary",
            "Elizabeth",
            "Michael",
            // Two names (first + last)
            "John Smith",
            "Mary Johnson",
            "Elizabeth Brown",
            "Michael Davis",
            // Three names (first + middle + last)
            "John Michael Smith",
            "Mary Jane Watson",
            "Elizabeth Anne Brown",
            "Michael James Davis",
            // Names with common punctuation
            "O'Connor",
            "Mary-Jane",
            "John Jr.",
            "Dr. Smith",
            "Jean-Pierre",
            // Hyphenated last names
            "John Smith-Johnson",
            "Mary Wilson-Davis",
        ];

        for name in &valid_english_names {
            assert!(
                is_valid_username(name),
                "English name '{}' should be valid",
                name
            );
        }
    }

    #[test]
    fn test_japanese_name_patterns() {
        let valid_japanese_names = [
            // Single names (rare but possible)
            "田中",
            "佐藤",
            "鈴木",
            "高橋",
            // Two part names without space (traditional)
            "田中太郎",
            "佐藤花子",
            "鈴木一郎",
            "高橋美香",
            // Two part names with space (modern style)
            "田中 太郎",
            "佐藤 花子",
            "鈴木 一郎",
            "高橋 美香",
            // Hiragana names
            "たなか",
            "さとう",
            "たなか たろう",
            // Katakana names
            "タナカ",
            "サトウ",
            "タナカ タロウ",
            // Mixed scripts (common in Japanese)
            "田中 たろう",
            "佐藤 ハナコ",
        ];

        for name in &valid_japanese_names {
            assert!(
                is_valid_username(name),
                "Japanese name '{}' should be valid",
                name
            );
        }
    }

    #[test]
    fn test_invalid_name_patterns() {
        let invalid_names = [
            // Empty or whitespace
            "",
            " ",
            "  ",
            // Too many parts (more than 3)
            "John Michael James Smith",
            "田中 太郎 三郎 四郎",
            // Too many parts (more than 3)
            // Leading/trailing spaces
            " John Smith",
            "John Smith ",
            " 田中太郎",
            "田中太郎 ",
            // Consecutive spaces
            "John  Smith",
            "田中  太郎",
            // Invalid punctuation patterns
            "-John",
            "John-",
            "'John",
            ".John",
            "John--Smith",
            "John''Smith",
            "John..Smith",
            // Numbers in names (not typical for real names)
            "John123",
            "Smith2",
            "田中1",
            // Special characters not appropriate for names
            "John@Smith",
            "John#Smith",
            "John$Smith",
            "田中@太郎",
        ];

        for name in &invalid_names {
            assert!(
                !is_valid_username(name),
                "Name '{}' should be invalid",
                name
            );
        }
    }

    #[test]
    fn test_name_length_validation() {
        // Test length boundaries
        assert!(is_valid_username_length("A")); // 1 char - valid
        assert!(is_valid_username_length("田")); // 1 Japanese char - valid
        assert!(is_valid_username_length(&"a".repeat(50))); // 50 chars - valid
        assert!(!is_valid_username_length("")); // 0 chars - invalid
        assert!(!is_valid_username_length(&"a".repeat(51))); // 51 chars - invalid
        assert!(!is_valid_username_length("   ")); // Only spaces - invalid
    }

    #[test]
    fn test_name_formatting_validation() {
        // Valid formatting
        assert!(is_well_formatted_username("John Smith"));
        assert!(is_well_formatted_username("田中太郎"));
        assert!(is_well_formatted_username("田中 太郎"));
        assert!(is_well_formatted_username("Jean-Pierre"));
        assert!(is_well_formatted_username("O'Connor"));

        // Invalid formatting
        assert!(!is_well_formatted_username("John  Smith")); // Double space
        assert!(!is_well_formatted_username(" John Smith")); // Leading space
        assert!(!is_well_formatted_username("John Smith ")); // Trailing space
        assert!(!is_well_formatted_username("John--Smith")); // Double hyphen
        assert!(!is_well_formatted_username("John''Smith")); // Double apostrophe
        assert!(!is_well_formatted_username("John..Smith")); // Double period
                                                             // Removed: Mixed scripts are now allowed
        assert!(!is_well_formatted_username("-John")); // Starts with hyphen
        assert!(!is_well_formatted_username("John-")); // Ends with hyphen
    }

    #[test]
    fn test_mixed_language_patterns() {
        let valid_mixed_names = [
            // English-Japanese mixing
            "John 田中",
            "田中 Smith",
            "Mary 花子",
            "太郎 Johnson",
            // Three part mixed names
            "John 田中 Smith",
            "田中 Mary 太郎",
            // Mixed scripts within single part (common in modern Japan)
            "田中John",
            "Maryたなか",
        ];

        for name in &valid_mixed_names {
            assert!(
                is_valid_username(name),
                "Mixed language name '{}' should be valid",
                name
            );
        }
    }

    #[test]
    fn test_edge_cases() {
        // Valid edge cases
        let valid_cases = [
            "A",             // Single letter
            "田",            // Single Japanese character
            "Jo",            // Two letters
            "John Jr.",      // With suffix
            "Mary-Jane",     // Hyphenated first name
            "O'Connor",      // With apostrophe
            "da Silva",      // Lowercase particle (Portuguese style)
            "von Neumann",   // German style
            "田中 太郎",     // Japanese with space
            "たなか たろう", // Hiragana with space
            "タナカ タロウ", // Katakana with space
        ];

        for name in &valid_cases {
            assert!(is_valid_username(name), "Name '{}' should be valid", name);
        }

        // Invalid edge cases
        let invalid_cases = [
            "John Michael James Smith Jr.", // Too many parts (5)
            "田中 太郎 三郎 四郎",          // Too many Japanese parts (4)
            "-John",                        // Starts with hyphen
            "John-",                        // Ends with hyphen
            "'John",                        // Starts with apostrophe
            "Jo--hn",                       // Double hyphen
            "Jo''hn",                       // Double apostrophe
            "Jo..hn",                       // Double period
        ];

        for name in &invalid_cases {
            assert!(
                !is_valid_username(name),
                "Name '{}' should be invalid",
                name
            );
        }
    }

    #[test]
    fn test_regex_compilation() {
        // Test that regex patterns compile without panicking
        let email_regex = Regex::new(
            r#"(?i)^(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9][a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$"#,
        );
        assert!(email_regex.is_ok());

        let username_regex = Regex::new(r"^[\p{L}][\p{L}'\.\-]*(?:\s+[\p{L}][\p{L}'\.\-]*){0,2}$");
        assert!(username_regex.is_ok());
    }

    #[test]
    fn test_email_regex_edge_cases() {
        // Test specific edge cases that should be valid
        let valid_edge_cases = [
            "a@b.co",                    // Minimal valid email
            "test123@example123.com",    // Numbers in domain
            "user+tag+more@example.com", // Multiple plus signs in local part
            "user_name@example.com",     // Underscore in local part
            "user-name@example.com",     // Hyphen in local part
        ];

        for email in &valid_edge_cases {
            assert!(
                EMAIL_REGEX.is_match(email),
                "Email '{}' should be valid",
                email
            );
        }

        // Test specific edge cases that should be invalid
        let invalid_edge_cases = [
            "a@b.c",                 // TLD too short
            "test@domain-.com",      // Hyphen at end of domain part
            "test@-domain.com",      // Hyphen at start of domain part
            ".test@domain.com",      // Local part starts with dot
            "test.@domain.com",      // Local part ends with dot
            "test..test@domain.com", // Consecutive dots in local part
            "test@domain..com",      // Consecutive dots in domain
        ];

        for email in &invalid_edge_cases {
            assert!(
                !EMAIL_REGEX.is_match(email),
                "Email '{}' should be invalid",
                email
            );
        }
    }

    #[test]
    fn test_email_regex_case_insensitive() {
        // Test that email regex is case insensitive
        let test_cases = [
            ("Test@Example.Com", true),
            ("USER@DOMAIN.COM", true),
            ("user@domain.com", true),
            ("User.Name@Example.Org", true),
        ];

        for (email, should_match) in &test_cases {
            assert_eq!(
                EMAIL_REGEX.is_match(email),
                *should_match,
                "Email '{}' case sensitivity test failed",
                email
            );
        }
    }

    #[test]
    fn test_email_regex_performance() {
        // Test with a variety of emails to ensure regex performance is acceptable
        let test_emails = vec!["test@example.com"; 1000];

        let start = std::time::Instant::now();
        for email in &test_emails {
            EMAIL_REGEX.is_match(email);
        }
        let duration = start.elapsed();

        // Should complete within reasonable time (less than 100ms for 1000 validations)
        assert!(
            duration.as_millis() < 100,
            "Email regex performance test failed: took {}ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_username_performance() {
        // Test username validation performance
        let test_names = vec!["John Smith"; 500];
        let test_japanese_names = vec!["田中太郎"; 500];

        let start = std::time::Instant::now();
        for name in &test_names {
            is_valid_username(name);
        }
        for name in &test_japanese_names {
            is_valid_username(name);
        }
        let duration = start.elapsed();

        // Should complete within reasonable time (less than 100ms for 1000 validations)
        assert!(
            duration.as_millis() < 100,
            "Username validation performance test failed: took {}ms",
            duration.as_millis()
        );
    }
}
