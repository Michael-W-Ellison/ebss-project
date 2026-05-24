// src/bevy_gui/resources/notifications.rs
//! Notification/toast message resource.

use bevy::prelude::*;

/// Notification severity/type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
}

/// A notification/toast message
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub notification_type: NotificationType,
    pub created_at: f64,
    pub duration: f64,
}

impl Notification {
    pub fn new(message: impl Into<String>, notification_type: NotificationType, current_time: f64) -> Self {
        Self {
            message: message.into(),
            notification_type,
            created_at: current_time,
            duration: 3.0,
        }
    }

    pub fn info(message: impl Into<String>, current_time: f64) -> Self {
        Self::new(message, NotificationType::Info, current_time)
    }

    pub fn success(message: impl Into<String>, current_time: f64) -> Self {
        Self::new(message, NotificationType::Success, current_time)
    }

    pub fn warning(message: impl Into<String>, current_time: f64) -> Self {
        Self::new(message, NotificationType::Warning, current_time)
    }

    pub fn error(message: impl Into<String>, current_time: f64) -> Self {
        Self::new(message, NotificationType::Error, current_time)
    }

    pub fn is_expired(&self, current_time: f64) -> bool {
        current_time > self.created_at + self.duration
    }

    pub fn remaining_time(&self, current_time: f64) -> f64 {
        (self.created_at + self.duration - current_time).max(0.0)
    }
}

/// Queue of active notifications
#[derive(Resource, Default)]
pub struct NotificationQueue {
    pub notifications: Vec<Notification>,
}

impl NotificationQueue {
    pub fn push(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    pub fn notify(&mut self, message: impl Into<String>, notification_type: NotificationType, current_time: f64) {
        self.push(Notification::new(message, notification_type, current_time));
    }

    pub fn info(&mut self, message: impl Into<String>, current_time: f64) {
        self.push(Notification::info(message, current_time));
    }

    pub fn success(&mut self, message: impl Into<String>, current_time: f64) {
        self.push(Notification::success(message, current_time));
    }

    pub fn warning(&mut self, message: impl Into<String>, current_time: f64) {
        self.push(Notification::warning(message, current_time));
    }

    pub fn error(&mut self, message: impl Into<String>, current_time: f64) {
        self.push(Notification::error(message, current_time));
    }

    pub fn cleanup_expired(&mut self, current_time: f64) {
        self.notifications.retain(|n| !n.is_expired(current_time));
    }

    pub fn clear(&mut self) {
        self.notifications.clear();
    }
}
