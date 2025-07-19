use std::time::Duration;

/// Centralized configuration for all Lambda functions
pub struct LambdaConfig {
    /// Cache TTL for user info and permissions
    pub cache_ttl: Duration,
    /// Cache TTL for hash calculations (longer due to computational cost)
    pub hash_cache_ttl: Duration,
    /// Cache TTL for secrets (longer due to AWS API calls)
    pub secrets_cache_ttl: Duration,
    /// Maximum capacity for all caches
    pub cache_max_capacity: u64,
    /// Maximum capacity for organization users cache (smaller due to list size)
    pub org_users_cache_max_capacity: u64,
    /// Maximum capacity for secrets cache (smaller due to limited secrets)
    pub secrets_cache_max_capacity: u64,
}

impl Default for LambdaConfig {
    fn default() -> Self {
        Self {
            cache_ttl: Duration::from_secs(1800),         // 30 minutes
            hash_cache_ttl: Duration::from_secs(3600),    // 1 hour
            secrets_cache_ttl: Duration::from_secs(3600), // 1 hour
            cache_max_capacity: 1000,
            org_users_cache_max_capacity: 100,
            secrets_cache_max_capacity: 10,
        }
    }
}

impl LambdaConfig {
    /// Create a new configuration with custom settings
    pub fn new(
        cache_ttl: Duration,
        hash_cache_ttl: Duration,
        secrets_cache_ttl: Duration,
        cache_max_capacity: u64,
        org_users_cache_max_capacity: u64,
        secrets_cache_max_capacity: u64,
    ) -> Self {
        Self {
            cache_ttl,
            hash_cache_ttl,
            secrets_cache_ttl,
            cache_max_capacity,
            org_users_cache_max_capacity,
            secrets_cache_max_capacity,
        }
    }

    /// Get configuration from environment variables
    pub fn from_env() -> Self {
        let cache_ttl_secs = std::env::var("CACHE_TTL_SECS")
            .unwrap_or_else(|_| "1800".to_string())
            .parse::<u64>()
            .unwrap_or(1800);

        let hash_cache_ttl_secs = std::env::var("HASH_CACHE_TTL_SECS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse::<u64>()
            .unwrap_or(3600);

        let secrets_cache_ttl_secs = std::env::var("SECRETS_CACHE_TTL_SECS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse::<u64>()
            .unwrap_or(3600);

        Self {
            cache_ttl: Duration::from_secs(cache_ttl_secs),
            hash_cache_ttl: Duration::from_secs(hash_cache_ttl_secs),
            secrets_cache_ttl: Duration::from_secs(secrets_cache_ttl_secs),
            cache_max_capacity: std::env::var("CACHE_MAX_CAPACITY")
                .unwrap_or_else(|_| "1000".to_string())
                .parse::<u64>()
                .unwrap_or(1000),
            org_users_cache_max_capacity: std::env::var("ORG_USERS_CACHE_MAX_CAPACITY")
                .unwrap_or_else(|_| "100".to_string())
                .parse::<u64>()
                .unwrap_or(100),
            secrets_cache_max_capacity: std::env::var("SECRETS_CACHE_MAX_CAPACITY")
                .unwrap_or_else(|_| "10".to_string())
                .parse::<u64>()
                .unwrap_or(10),
        }
    }
}

/// Global configuration instance
pub fn get_config() -> &'static LambdaConfig {
    static CONFIG: once_cell::sync::Lazy<LambdaConfig> =
        once_cell::sync::Lazy::new(LambdaConfig::from_env);
    &CONFIG
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = LambdaConfig::default();

        assert_eq!(config.cache_ttl, Duration::from_secs(1800));
        assert_eq!(config.hash_cache_ttl, Duration::from_secs(3600));
        assert_eq!(config.secrets_cache_ttl, Duration::from_secs(3600));
        assert_eq!(config.cache_max_capacity, 1000);
        assert_eq!(config.org_users_cache_max_capacity, 100);
        assert_eq!(config.secrets_cache_max_capacity, 10);
    }

    #[test]
    fn test_new_config() {
        let config = LambdaConfig::new(
            Duration::from_secs(900),
            Duration::from_secs(1800),
            Duration::from_secs(2700),
            500,
            50,
            5,
        );

        assert_eq!(config.cache_ttl, Duration::from_secs(900));
        assert_eq!(config.hash_cache_ttl, Duration::from_secs(1800));
        assert_eq!(config.secrets_cache_ttl, Duration::from_secs(2700));
        assert_eq!(config.cache_max_capacity, 500);
        assert_eq!(config.org_users_cache_max_capacity, 50);
        assert_eq!(config.secrets_cache_max_capacity, 5);
    }

    #[test]
    fn test_from_env_with_defaults() {
        // Clear environment variables to test defaults
        let env_vars = [
            "CACHE_TTL_SECS",
            "HASH_CACHE_TTL_SECS",
            "SECRETS_CACHE_TTL_SECS",
            "CACHE_MAX_CAPACITY",
            "ORG_USERS_CACHE_MAX_CAPACITY",
            "SECRETS_CACHE_MAX_CAPACITY",
        ];

        for var in &env_vars {
            env::remove_var(var);
        }

        let config = LambdaConfig::from_env();

        assert_eq!(config.cache_ttl, Duration::from_secs(1800));
        assert_eq!(config.hash_cache_ttl, Duration::from_secs(3600));
        assert_eq!(config.secrets_cache_ttl, Duration::from_secs(3600));
        assert_eq!(config.cache_max_capacity, 1000);
        assert_eq!(config.org_users_cache_max_capacity, 100);
        assert_eq!(config.secrets_cache_max_capacity, 10);
    }

    #[test]
    fn test_from_env_with_custom_values() {
        // Set custom environment variables
        env::set_var("CACHE_TTL_SECS", "900");
        env::set_var("HASH_CACHE_TTL_SECS", "1800");
        env::set_var("SECRETS_CACHE_TTL_SECS", "2700");
        env::set_var("CACHE_MAX_CAPACITY", "500");
        env::set_var("ORG_USERS_CACHE_MAX_CAPACITY", "50");
        env::set_var("SECRETS_CACHE_MAX_CAPACITY", "5");

        let config = LambdaConfig::from_env();

        assert_eq!(config.cache_ttl, Duration::from_secs(900));
        assert_eq!(config.hash_cache_ttl, Duration::from_secs(1800));
        assert_eq!(config.secrets_cache_ttl, Duration::from_secs(2700));
        assert_eq!(config.cache_max_capacity, 500);
        assert_eq!(config.org_users_cache_max_capacity, 50);
        assert_eq!(config.secrets_cache_max_capacity, 5);

        // Clean up environment variables
        env::remove_var("CACHE_TTL_SECS");
        env::remove_var("HASH_CACHE_TTL_SECS");
        env::remove_var("SECRETS_CACHE_TTL_SECS");
        env::remove_var("CACHE_MAX_CAPACITY");
        env::remove_var("ORG_USERS_CACHE_MAX_CAPACITY");
        env::remove_var("SECRETS_CACHE_MAX_CAPACITY");
    }

    #[test]
    fn test_from_env_with_invalid_values() {
        // Set invalid environment variables (should fallback to defaults)
        env::set_var("CACHE_TTL_SECS", "invalid");
        env::set_var("HASH_CACHE_TTL_SECS", "not_a_number");
        env::set_var("CACHE_MAX_CAPACITY", "not_numeric");

        let config = LambdaConfig::from_env();

        // Should use defaults when parsing fails
        assert_eq!(config.cache_ttl, Duration::from_secs(1800));
        assert_eq!(config.hash_cache_ttl, Duration::from_secs(3600));
        assert_eq!(config.cache_max_capacity, 1000);

        // Clean up environment variables
        env::remove_var("CACHE_TTL_SECS");
        env::remove_var("HASH_CACHE_TTL_SECS");
        env::remove_var("CACHE_MAX_CAPACITY");
    }

    #[test]
    fn test_get_config() {
        // Test that get_config returns a valid configuration
        let config = get_config();

        // Should have valid durations
        assert!(config.cache_ttl.as_secs() > 0);
        assert!(config.hash_cache_ttl.as_secs() > 0);
        assert!(config.secrets_cache_ttl.as_secs() > 0);

        // Should have valid capacities
        assert!(config.cache_max_capacity > 0);
        assert!(config.org_users_cache_max_capacity > 0);
        assert!(config.secrets_cache_max_capacity > 0);
    }

    #[test]
    fn test_config_consistency() {
        let config = LambdaConfig::default();

        // Hash cache should typically have longer TTL than regular cache
        assert!(config.hash_cache_ttl >= config.cache_ttl);

        // Secrets cache should typically have longer TTL than regular cache
        assert!(config.secrets_cache_ttl >= config.cache_ttl);

        // Organization users cache should be smaller than main cache
        assert!(config.org_users_cache_max_capacity <= config.cache_max_capacity);

        // Secrets cache should be smallest
        assert!(config.secrets_cache_max_capacity <= config.org_users_cache_max_capacity);
    }
}
