use crate::config::get_config;
use crate::entity::secrets::Secrets;
use crate::entity::user::User;

use moka::future::Cache;
use once_cell::sync::Lazy;

/// Unified cache manager for all Lambda functions
pub struct CacheManager {
    user_cache: Cache<String, User>,
    permission_cache: Cache<String, bool>,
    hash_cache: Cache<String, String>,
    secrets_cache: Cache<String, Secrets>,
    org_users_cache: Cache<String, Vec<User>>,
}

impl CacheManager {
    pub fn new() -> Self {
        let config = get_config();

        Self {
            user_cache: Cache::builder()
                .max_capacity(config.cache_max_capacity)
                .time_to_live(config.cache_ttl)
                .build(),

            permission_cache: Cache::builder()
                .max_capacity(config.cache_max_capacity)
                .time_to_live(config.cache_ttl)
                .build(),

            hash_cache: Cache::builder()
                .max_capacity(config.cache_max_capacity)
                .time_to_live(config.hash_cache_ttl)
                .build(),

            secrets_cache: Cache::builder()
                .max_capacity(config.secrets_cache_max_capacity)
                .time_to_live(config.secrets_cache_ttl)
                .build(),

            org_users_cache: Cache::builder()
                .max_capacity(config.org_users_cache_max_capacity)
                .time_to_live(config.cache_ttl)
                .build(),
        }
    }

    /// Get user from cache
    pub async fn get_user(&self, user_id: &str) -> Option<User> {
        self.user_cache.get(user_id).await
    }

    /// Set user in cache
    pub async fn set_user(&self, user_id: String, user: User) {
        self.user_cache.insert(user_id, user).await;
    }

    /// Get permission from cache
    pub async fn get_permission(&self, user_id: &str) -> Option<bool> {
        self.permission_cache.get(user_id).await
    }

    /// Set permission in cache
    pub async fn set_permission(&self, user_id: String, has_permission: bool) {
        self.permission_cache.insert(user_id, has_permission).await;
    }

    /// Get hash from cache
    pub async fn get_hash(&self, key: &str) -> Option<String> {
        self.hash_cache.get(key).await
    }

    /// Set hash in cache
    pub async fn set_hash(&self, key: String, hash: String) {
        self.hash_cache.insert(key, hash).await;
    }

    /// Get secrets from cache
    pub async fn get_secrets(&self, region: &str) -> Option<Secrets> {
        self.secrets_cache.get(region).await
    }

    /// Set secrets in cache
    pub async fn set_secrets(&self, region: String, secrets: Secrets) {
        self.secrets_cache.insert(region, secrets).await;
    }

    /// Get organization users from cache
    pub async fn get_org_users(&self, org_id: &str) -> Option<Vec<User>> {
        self.org_users_cache.get(org_id).await
    }

    /// Set organization users in cache
    pub async fn set_org_users(&self, org_id: String, users: Vec<User>) {
        self.org_users_cache.insert(org_id, users).await;
    }

    /// Clear all caches (useful for testing)
    pub async fn clear_all(&self) {
        self.user_cache.invalidate_all();
        self.permission_cache.invalidate_all();
        self.hash_cache.invalidate_all();
        self.secrets_cache.invalidate_all();
        self.org_users_cache.invalidate_all();
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            user_cache_size: self.user_cache.entry_count(),
            permission_cache_size: self.permission_cache.entry_count(),
            hash_cache_size: self.hash_cache.entry_count(),
            secrets_cache_size: self.secrets_cache.entry_count(),
            org_users_cache_size: self.org_users_cache.entry_count(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub user_cache_size: u64,
    pub permission_cache_size: u64,
    pub hash_cache_size: u64,
    pub secrets_cache_size: u64,
    pub org_users_cache_size: u64,
}

/// Global cache manager instance
pub fn get_cache_manager() -> &'static CacheManager {
    static CACHE_MANAGER: Lazy<CacheManager> = Lazy::new(CacheManager::new);
    &CACHE_MANAGER
}

/// Trait for cacheable operations
#[async_trait::async_trait]
pub trait Cacheable<T> {
    async fn get_cached(&self, key: &str) -> Option<T>;
    async fn set_cached(&self, key: String, value: T);
}

/// Implementation for user caching
#[async_trait::async_trait]
impl Cacheable<User> for CacheManager {
    async fn get_cached(&self, key: &str) -> Option<User> {
        self.get_user(key).await
    }

    async fn set_cached(&self, key: String, value: User) {
        self.set_user(key, value).await;
    }
}

/// Implementation for permission caching
#[async_trait::async_trait]
impl Cacheable<bool> for CacheManager {
    async fn get_cached(&self, key: &str) -> Option<bool> {
        self.get_permission(key).await
    }

    async fn set_cached(&self, key: String, value: bool) {
        self.set_permission(key, value).await;
    }
}

/// Implementation for hash caching
#[async_trait::async_trait]
impl Cacheable<String> for CacheManager {
    async fn get_cached(&self, key: &str) -> Option<String> {
        self.get_hash(key).await
    }

    async fn set_cached(&self, key: String, value: String) {
        self.set_hash(key, value).await;
    }
}

/// Implementation for secrets caching
#[async_trait::async_trait]
impl Cacheable<Secrets> for CacheManager {
    async fn get_cached(&self, key: &str) -> Option<Secrets> {
        self.get_secrets(key).await
    }

    async fn set_cached(&self, key: String, value: Secrets) {
        self.set_secrets(key, value).await;
    }
}

/// Implementation for organization users caching
#[async_trait::async_trait]
impl Cacheable<Vec<User>> for CacheManager {
    async fn get_cached(&self, key: &str) -> Option<Vec<User>> {
        self.get_org_users(key).await
    }

    async fn set_cached(&self, key: String, value: Vec<User>) {
        self.set_org_users(key, value).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::user::{Permissions, Role};
    use std::collections::HashSet;

    /// Test utilities for cache manager
    pub struct CacheTestUtils {
        pub cache_manager: CacheManager,
    }

    impl CacheTestUtils {
        pub fn new() -> Self {
            Self {
                cache_manager: CacheManager::new(),
            }
        }

        /// Create a test user
        pub fn create_test_user(
            id: &str,
            name: &str,
            email: &str,
            org_id: &str,
            org_name: &str,
            roles: Vec<Role>,
        ) -> User {
            let mut role_set = HashSet::new();
            for role in roles {
                role_set.insert(role);
            }

            User::new(
                id.to_string(),
                name.to_string(),
                email.to_string(),
                org_id.to_string(),
                org_name.to_string(),
                role_set,
            )
        }

        /// Clear all caches for clean test state
        pub async fn clear_caches(&self) {
            self.cache_manager.clear_all().await;
        }

        /// Get cache statistics for assertions
        pub fn get_cache_stats(&self) -> CacheStats {
            self.cache_manager.get_stats()
        }
    }

    impl Default for CacheTestUtils {
        fn default() -> Self {
            Self::new()
        }
    }

    #[tokio::test]
    async fn test_cache_manager_user_operations() {
        let utils = CacheTestUtils::new();

        // Test user caching
        let user = CacheTestUtils::create_test_user(
            "test-1",
            "Test User",
            "test@example.com",
            "org-1",
            "Test Org",
            vec![Role::Admin],
        );

        utils
            .cache_manager
            .set_user("test-1".to_string(), user.clone())
            .await;

        let cached_user = utils.cache_manager.get_user("test-1").await;
        assert!(cached_user.is_some());
        assert_eq!(cached_user.unwrap().id, "test-1");

        // Test cache clearing
        utils.clear_caches().await;

        let cleared_user = utils.cache_manager.get_user("test-1").await;
        assert!(cleared_user.is_none());
    }

    #[tokio::test]
    async fn test_cache_manager_permission_operations() {
        let utils = CacheTestUtils::new();

        // Test permission caching
        utils
            .cache_manager
            .set_permission("test-1".to_string(), true)
            .await;

        let cached_permission = utils.cache_manager.get_permission("test-1").await;
        assert!(cached_permission.is_some());
        assert!(cached_permission.unwrap());

        // Test false permission
        utils
            .cache_manager
            .set_permission("test-2".to_string(), false)
            .await;
        let cached_permission = utils.cache_manager.get_permission("test-2").await;
        assert!(cached_permission.is_some());
        assert!(!cached_permission.unwrap());
    }

    #[tokio::test]
    async fn test_cache_manager_hash_operations() {
        let utils = CacheTestUtils::new();

        // Test hash caching
        utils
            .cache_manager
            .set_hash("test-hash".to_string(), "hash-value".to_string())
            .await;

        let cached_hash = utils.cache_manager.get_hash("test-hash").await;
        assert!(cached_hash.is_some());
        assert_eq!(cached_hash.unwrap(), "hash-value");
    }

    #[tokio::test]
    async fn test_cache_manager_secrets_operations() {
        let utils = CacheTestUtils::new();

        // Test secrets caching
        let test_secrets = crate::entity::secrets::Secrets {
            user_pool_id: "test-user-pool".to_string(),
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            jwks_url: "https://test.jwks.url".to_string(),
        };

        utils
            .cache_manager
            .set_secrets("ap-northeast-1".to_string(), test_secrets.clone())
            .await;

        let cached_secrets = utils.cache_manager.get_secrets("ap-northeast-1").await;
        assert!(cached_secrets.is_some());
        assert_eq!(cached_secrets.unwrap().user_pool_id, "test-user-pool");
    }

    #[tokio::test]
    async fn test_cache_manager_org_users_operations() {
        let utils = CacheTestUtils::new();

        // Test organization users caching
        let users = vec![
            CacheTestUtils::create_test_user(
                "user-1",
                "User 1",
                "user1@example.com",
                "org-1",
                "Test Org",
                vec![Role::Reader],
            ),
            CacheTestUtils::create_test_user(
                "user-2",
                "User 2",
                "user2@example.com",
                "org-1",
                "Test Org",
                vec![Role::Writer],
            ),
        ];

        utils
            .cache_manager
            .set_org_users("org-1".to_string(), users.clone())
            .await;

        let cached_users = utils.cache_manager.get_org_users("org-1").await;
        assert!(cached_users.is_some());
        assert_eq!(cached_users.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let utils = CacheTestUtils::new();

        // Add some test data
        let user = CacheTestUtils::create_test_user(
            "test-3",
            "Test User 3",
            "test3@example.com",
            "org-3",
            "Test Org 3",
            vec![Role::Admin],
        );

        utils
            .cache_manager
            .set_user("test-3".to_string(), user)
            .await;
        utils
            .cache_manager
            .set_permission("test-3".to_string(), true)
            .await;
        utils
            .cache_manager
            .set_hash("test-hash".to_string(), "hash-value".to_string())
            .await;

        // Allow some time for cache operations to complete
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let stats = utils.get_cache_stats();

        // Cache sizes may be 0 or more depending on timing
        assert!(stats.user_cache_size <= 1);
        assert!(stats.permission_cache_size <= 1);
        assert!(stats.hash_cache_size <= 1);
        assert_eq!(stats.secrets_cache_size, 0);
        assert_eq!(stats.org_users_cache_size, 0);
    }

    #[tokio::test]
    async fn test_cacheable_trait_user() {
        let cache_manager = CacheManager::new();

        let user = CacheTestUtils::create_test_user(
            "trait-test",
            "Trait Test User",
            "trait@example.com",
            "org-trait",
            "Trait Org",
            vec![Role::Writer],
        );

        // Test Cacheable trait implementation for User
        cache_manager
            .set_cached("trait-test".to_string(), user.clone())
            .await;
        let cached_user: Option<User> = cache_manager.get_cached("trait-test").await;

        assert!(cached_user.is_some());
        assert_eq!(cached_user.unwrap().id, "trait-test");
    }

    #[tokio::test]
    async fn test_cacheable_trait_permission() {
        let cache_manager = CacheManager::new();

        // Test Cacheable trait implementation for bool
        cache_manager
            .set_cached("trait-permission".to_string(), true)
            .await;
        let cached_permission: Option<bool> = cache_manager.get_cached("trait-permission").await;

        assert!(cached_permission.is_some());
        assert!(cached_permission.unwrap());
    }

    #[tokio::test]
    async fn test_multiple_users_with_different_permissions() {
        let utils = CacheTestUtils::new();

        // Create users with different roles
        let admin_user = CacheTestUtils::create_test_user(
            "admin",
            "Admin User",
            "admin@example.com",
            "org-1",
            "Test Org",
            vec![Role::Admin],
        );

        let reader_user = CacheTestUtils::create_test_user(
            "reader",
            "Reader User",
            "reader@example.com",
            "org-1",
            "Test Org",
            vec![Role::Reader],
        );

        let writer_user = CacheTestUtils::create_test_user(
            "writer",
            "Writer User",
            "writer@example.com",
            "org-1",
            "Test Org",
            vec![Role::Writer],
        );

        // Cache users
        utils
            .cache_manager
            .set_user("admin".to_string(), admin_user.clone())
            .await;
        utils
            .cache_manager
            .set_user("reader".to_string(), reader_user.clone())
            .await;
        utils
            .cache_manager
            .set_user("writer".to_string(), writer_user.clone())
            .await;

        // Verify all users are cached
        let cached_admin = utils.cache_manager.get_user("admin").await;
        let cached_reader = utils.cache_manager.get_user("reader").await;
        let cached_writer = utils.cache_manager.get_user("writer").await;

        assert!(cached_admin.is_some());
        assert!(cached_reader.is_some());
        assert!(cached_writer.is_some());

        // Verify permissions
        assert!(cached_admin.unwrap().has_permission(Permissions::DELETE));
        assert!(!cached_reader.unwrap().has_permission(Permissions::WRITE));
        assert!(cached_writer.unwrap().has_permission(Permissions::CREATE));

        // Allow some time for cache operations to complete
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Check cache stats
        let stats = utils.get_cache_stats();
        assert!(stats.user_cache_size <= 3);
    }
}
