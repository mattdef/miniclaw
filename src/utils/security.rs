//! Security utilities for miniclaw
//!
//! Provides security-related functionality including:
//! - Whitelist checking for Telegram user IDs
//! - Exec command blacklist validation

use std::collections::HashSet;

/// Special value that represents "allow all users" wildcard
const ALLOW_ALL_WILDCARD: i64 = -1;

/// Whitelist checker for Telegram user IDs.
///
/// Implements secure-by-default behavior: empty whitelist = no one allowed
#[derive(Debug, Clone)]
pub struct WhitelistChecker {
    allowed_users: HashSet<i64>,
}

impl WhitelistChecker {
    /// Create a new WhitelistChecker with the given allowed user IDs.
    ///
    /// # Arguments
    /// * `allowed_users` - Vector of allowed Telegram user IDs
    pub fn new(allowed_users: Vec<i64>) -> Self {
        let has_wildcard = allowed_users.contains(&ALLOW_ALL_WILDCARD);

        if has_wildcard {
            tracing::warn!("WARNING: Allow-all mode enabled - all users allowed!");
        } else if allowed_users.is_empty() {
            tracing::warn!("Whitelist empty, no users allowed (secure by default)");
        } else {
            tracing::info!(
                "Whitelist initialized with {} allowed user(s)",
                allowed_users.len()
            );
        }

        Self {
            allowed_users: allowed_users.into_iter().collect(),
        }
    }

    /// Check if a user ID is allowed.
    ///
    /// Implements secure-by-default: empty whitelist = no one allowed
    ///
    /// # Arguments
    /// * `user_id` - The Telegram user ID to check
    ///
    /// # Returns
    /// `true` if user is allowed, `false` otherwise
    pub fn is_allowed(&self, user_id: i64) -> bool {
        // Check for allow-all wildcard first
        if self.allowed_users.contains(&ALLOW_ALL_WILDCARD) {
            return true;
        }

        if self.allowed_users.is_empty() {
            // Secure by default: empty whitelist = no one allowed (NFR-S5)
            return false;
        }

        self.allowed_users.contains(&user_id)
    }

    /// Add a user to the whitelist.
    ///
    /// # Arguments
    /// * `user_id` - The Telegram user ID to add (must be positive)
    ///
    /// # Returns
    /// `Ok(())` if user was added, `Err` if user_id is invalid (zero or negative)
    pub fn add_user(&mut self, user_id: i64) -> Result<(), String> {
        if user_id <= 0 {
            return Err(format!(
                "Invalid user ID: {}. User IDs must be positive integers.",
                user_id
            ));
        }
        self.allowed_users.insert(user_id);
        Ok(())
    }

    /// Get the number of whitelisted users.
    pub fn len(&self) -> usize {
        self.allowed_users.len()
    }

    /// Check if whitelist is empty.
    pub fn is_empty(&self) -> bool {
        self.allowed_users.is_empty()
    }
}

/// Blacklisted commands that cannot be executed via exec tool (NFR-S4).
const BLACKLISTED_COMMANDS: &[&str] = &[
    "rm", "sudo", "dd", "mkfs", "shutdown", "reboot", "passwd", "visudo",
];

/// Check if a command is allowed to be executed.
///
/// # Arguments
/// * `command` - The command to check
///
/// # Returns
/// `true` if command is allowed, `false` if blacklisted
pub fn is_command_allowed(command: &str) -> bool {
    let command_lower = command.to_lowercase();
    !BLACKLISTED_COMMANDS.contains(&command_lower.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitelist_allows_configured_users() {
        let checker = WhitelistChecker::new(vec![123_456_789, 987_654_321]);
        assert!(checker.is_allowed(123_456_789));
        assert!(checker.is_allowed(987_654_321));
    }

    #[test]
    fn test_whitelist_rejects_unknown_users() {
        let checker = WhitelistChecker::new(vec![123_456_789]);
        assert!(!checker.is_allowed(999_999_999));
    }

    #[test]
    fn test_empty_whitelist_rejects_all() {
        let checker = WhitelistChecker::new(vec![]);
        assert!(!checker.is_allowed(123_456_789));
        assert!(!checker.is_allowed(987_654_321));
        assert!(checker.is_empty());
    }

    #[test]
    fn test_add_user_to_whitelist() {
        let mut checker = WhitelistChecker::new(vec![]);
        assert!(checker.add_user(123_456_789).is_ok());
        assert!(checker.is_allowed(123_456_789));
        assert_eq!(checker.len(), 1);
    }

    #[test]
    fn test_add_duplicate_user_idempotent() {
        let mut checker = WhitelistChecker::new(vec![123_456_789]);
        assert!(checker.add_user(123_456_789).is_ok()); // Already exists
        assert_eq!(checker.len(), 1);
    }

    #[test]
    fn test_add_user_rejects_invalid_ids() {
        let mut checker = WhitelistChecker::new(vec![]);

        // Zero should be rejected
        assert!(checker.add_user(0).is_err());
        assert_eq!(checker.len(), 0);

        // Negative should be rejected
        assert!(checker.add_user(-123).is_err());
        assert_eq!(checker.len(), 0);

        // Positive should be accepted
        assert!(checker.add_user(123).is_ok());
        assert_eq!(checker.len(), 1);
    }

    #[test]
    fn test_command_allowed() {
        assert!(is_command_allowed("ls"));
        assert!(is_command_allowed("cat"));
        assert!(is_command_allowed("echo"));
    }

    #[test]
    fn test_command_blacklisted() {
        assert!(!is_command_allowed("rm"));
        assert!(!is_command_allowed("sudo"));
        assert!(!is_command_allowed("dd"));
        assert!(!is_command_allowed("shutdown"));
    }

    #[test]
    fn test_command_blacklisted_case_insensitive() {
        assert!(!is_command_allowed("RM"));
        assert!(!is_command_allowed("Sudo"));
        assert!(!is_command_allowed("DD"));
    }

    #[test]
    fn test_wildcard_allows_all_users() {
        let checker = WhitelistChecker::new(vec![ALLOW_ALL_WILDCARD]);
        assert!(checker.is_allowed(123));
        assert!(checker.is_allowed(456));
        assert!(checker.is_allowed(-999)); // Even invalid IDs are allowed in allow-all mode
        assert!(checker.is_allowed(0));
    }

    #[test]
    fn test_wildcard_with_other_users() {
        // Wildcard should work even with other user IDs
        let checker = WhitelistChecker::new(vec![ALLOW_ALL_WILDCARD, 123456789]);
        assert!(checker.is_allowed(123456789));
        assert!(checker.is_allowed(999)); // Any user is allowed
        assert!(checker.is_allowed(0));
    }

    #[test]
    fn test_empty_whitelist_still_rejects_all() {
        // Empty whitelist should still reject all (secure-by-default preserved)
        let checker = WhitelistChecker::new(vec![]);
        assert!(!checker.is_allowed(123));
        assert!(!checker.is_allowed(456));
    }
}
