//! Status bar for displaying current directory and git branch
//!
//! Renders a status bar at the bottom of the terminal with:
//! - Current working directory
//! - Git branch (if in a git repository)
//! - Error messages feedback
//! - Toast for user notifications for transient errors

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Error severity level
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorLevel {
    /// Transient error that should disappear after a timeout
    Transient,
    /// Persistent error that should stay visible until dismissed
    Persistent,
}

/// Error message with metadata
#[derive(Debug, Clone)]
pub struct StatusError {
    pub message: String,
    pub level: ErrorLevel,
    pub timestamp: Instant,
}

impl StatusError {
    pub fn new(message: String, level: ErrorLevel) -> Self {
        Self {
            message,
            level,
            timestamp: Instant::now(),
        }
    }

    pub fn transient(message: &str) -> Self {
        Self::new(message.to_string(), ErrorLevel::Transient)
    }

    pub fn persistent(message: &str) -> Self {
        Self::new(message.to_string(), ErrorLevel::Persistent)
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.level == ErrorLevel::Transient && self.timestamp.elapsed() > timeout
    }
}

/// Toast notification for transient errors
#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub timestamp: Instant,
}

impl Toast {
    pub fn new(message: String) -> Self {
        Self {
            message,
            timestamp: Instant::now(),
        }
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.timestamp.elapsed() > timeout
    }
}

/// Status bar information
#[derive(Debug, Clone)]
pub struct StatusBar {
    /// Current working directory
    pub current_dir: String,
    /// Git branch (None if not in a git repo)
    pub git_branch: Option<String>,
    /// Whether the status bar is visible
    pub visible: bool,
    /// Current error message (if any)
    pub error: Option<StatusError>,
    /// Toast notifications (transient errors)
    pub toasts: Vec<Toast>,
    /// Timeout for transient errors and toasts
    pub toast_timeout: Duration,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new() -> Self {
        Self {
            current_dir: String::new(),
            git_branch: None,
            visible: true,
            error: None,
            toasts: Vec::new(),
            toast_timeout: Duration::from_secs(5),
        }
    }

    /// Update the status bar with the current directory
    pub fn update(&mut self, dir: &str) {
        self.current_dir = dir.to_string();
        self.git_branch = Self::get_git_branch(dir);
    }

    /// Set a transient error that will auto-dismiss
    pub fn set_error(&mut self, message: &str) {
        self.error = Some(StatusError::transient(message));
    }

    /// Set a persistent error that stays until manually dismissed
    pub fn set_persistent_error(&mut self, message: &str) {
        self.error = Some(StatusError::persistent(message));
    }

    /// Clear the current error
    #[allow(dead_code)]
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Check if there's an error to display
    #[allow(dead_code)]
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the error message if present
    #[allow(dead_code)]
    pub fn get_error(&self) -> Option<&str> {
        self.error.as_ref().map(|e| e.message.as_str())
    }

    /// Add a toast notification
    pub fn add_toast(&mut self, message: &str) {
        self.toasts.push(Toast::new(message.to_string()));
    }

    /// Clear all toasts
    #[allow(dead_code)]
    pub fn clear_toasts(&mut self) {
        self.toasts.clear();
    }

    /// Clean up expired transient errors and toasts
    pub fn cleanup_expired(&mut self) {
        // Clean up expired error
        if let Some(ref error) = self.error {
            if error.is_expired(self.toast_timeout) {
                self.error = None;
            }
        }

        // Clean up expired toasts
        self.toasts.retain(|t| !t.is_expired(self.toast_timeout));
    }

    /// Set the toast timeout duration
    #[allow(dead_code)]
    pub fn set_toast_timeout(&mut self, duration: Duration) {
        self.toast_timeout = duration;
    }

    /// Get toast messages to display
    #[allow(dead_code)]
    pub fn get_toasts(&self) -> Vec<&str> {
        self.toasts.iter().map(|t| t.message.as_str()).collect()
    }

    /// Get the git branch for a directory
    fn get_git_branch(dir: &str) -> Option<String> {
        let path = Path::new(dir);

        // Try to get git branch using git command
        let output = Command::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(path)
            .output()
            .ok()?;

        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !branch.is_empty() && branch != "HEAD" {
                return Some(branch);
            }
        }

        None
    }

    /// Toggle status bar visibility
    #[allow(dead_code)]
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if status bar is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_status_bar_creation() {
        let status_bar = StatusBar::new();
        assert!(status_bar.current_dir.is_empty());
        assert!(status_bar.git_branch.is_none());
        assert!(status_bar.visible);
    }

    #[test]
    fn test_status_bar_toggle() {
        let mut status_bar = StatusBar::new();
        assert!(status_bar.is_visible());

        status_bar.toggle();
        assert!(!status_bar.is_visible());

        status_bar.toggle();
        assert!(status_bar.is_visible());
    }

    #[test]
    fn test_status_bar_update() {
        let mut status_bar = StatusBar::new();
        let current_dir = env::current_dir().unwrap();
        let dir_str = current_dir.to_string_lossy();

        status_bar.update(&dir_str);
        assert_eq!(status_bar.current_dir, dir_str);
    }

    #[test]
    fn test_git_branch_in_repo() {
        // This test assumes we're running in a git repository
        let current_dir = env::current_dir().unwrap();
        let dir_str = current_dir.to_string_lossy();

        let branch = StatusBar::get_git_branch(&dir_str);
        // In a git repo, we should get a branch name
        // (unless in detached HEAD state)
        if let Some(branch_name) = branch {
            assert!(!branch_name.is_empty());
        }
    }
}
