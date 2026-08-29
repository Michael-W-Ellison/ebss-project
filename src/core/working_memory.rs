// src/core/working_memory.rs
//! Working memory system for short-term task management and active information.
//!
//! Working memory is the small amount of information that can be held in mind
//! and used in the execution of cognitive tasks. It includes:
//! - Current goals and tasks
//! - Active attention focus
//! - Temporary information storage
//! - Task switching and prioritization

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Priority level for tasks in working memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl TaskPriority {
    /// Get numeric value for priority
    pub fn value(&self) -> u8 {
        match self {
            TaskPriority::Low => 1,
            TaskPriority::Normal => 2,
            TaskPriority::High => 3,
            TaskPriority::Critical => 4,
        }
    }
}

/// Status of a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Active,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// A task in working memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingTask {
    pub id: Uuid,
    pub description: String,
    pub priority: TaskPriority,
    pub status: TaskStatus,

    /// When this task was created
    pub created: u64,

    /// When this task started (if active)
    pub started: Option<u64>,

    /// Expected duration (if known)
    pub duration_estimate: Option<u64>,

    /// Motivation level (0.0 to 1.0)
    pub motivation: f32,

    /// Associated goal ID (if part of larger goal)
    pub goal_id: Option<Uuid>,

    /// Required resources
    pub required_resources: Vec<String>,

    /// Location where task should be performed
    pub location: Option<(i32, i32, i32)>,

    /// Other agents involved
    pub collaborators: Vec<Uuid>,
}

impl WorkingTask {
    pub fn new(description: String, priority: TaskPriority, created: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            priority,
            status: TaskStatus::Pending,
            created,
            started: None,
            duration_estimate: None,
            motivation: 0.5,
            goal_id: None,
            required_resources: Vec::new(),
            location: None,
            collaborators: Vec::new(),
        }
    }

    /// Start this task
    pub fn start(&mut self, current_time: u64) {
        self.status = TaskStatus::Active;
        self.started = Some(current_time);
    }

    /// Complete this task
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
    }


    /// Pause this task
    pub fn pause(&mut self) {
        self.status = TaskStatus::Paused;
    }

    /// Resume this task
    pub fn resume(&mut self) {
        self.status = TaskStatus::Active;
    }


    /// Is this task overdue?
    pub fn is_overdue(&self, current_time: u64) -> bool {
        if let (Some(started), Some(duration)) = (self.started, self.duration_estimate) {
            let elapsed = current_time.saturating_sub(started);
            elapsed > duration
        } else {
            false
        }
    }

    /// Calculate task score (for prioritization)
    pub fn score(&self, current_time: u64) -> f32 {
        let mut score = self.priority.value() as f32;

        // Add urgency based on age
        let age = current_time.saturating_sub(self.created);
        score += (age as f32 / 1000.0).min(3.0);

        // Add motivation factor
        score *= 1.0 + self.motivation;

        // Critical priority gets huge boost
        if self.priority == TaskPriority::Critical {
            score *= 2.0;
        }

        // Overdue tasks get priority
        if self.is_overdue(current_time) {
            score *= 1.5;
        }

        score
    }
}

/// Temporary information stored in working memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporaryInfo {
    pub key: String,
    pub value: String,
    pub expires: u64, // When this info expires
}

/// Working memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemory {
    /// Current active tasks (limited capacity)
    tasks: Vec<WorkingTask>,

    /// Maximum number of simultaneous tasks
    max_tasks: usize,

    /// Temporary information storage
    temp_info: Vec<TemporaryInfo>,

    /// Current focus (what the agent is actively doing)
    current_focus: Option<Uuid>, // Task ID

    /// Current time
    current_time: u64,
}

impl WorkingMemory {
    pub fn new(max_tasks: usize) -> Self {
        Self {
            tasks: Vec::new(),
            max_tasks,
            temp_info: Vec::new(),
            current_focus: None,
            current_time: 0,
        }
    }

    /// Add a new task
    pub fn add_task(&mut self, mut task: WorkingTask) -> Result<Uuid, String> {
        // Remove lowest priority task if at capacity
        if self.tasks.len() >= self.max_tasks {
            // Find lowest priority pending task
            if let Some(lowest_idx) = self.tasks
                .iter()
                .enumerate()
                .filter(|(_, t)| t.status == TaskStatus::Pending)
                .min_by_key(|(_, t)| t.score(self.current_time) as i32)
                .map(|(i, _)| i)
            {
                self.tasks.remove(lowest_idx);
            } else {
                return Err("Working memory full - all tasks active".to_string());
            }
        }

        let task_id = task.id;
        task.created = self.current_time;
        self.tasks.push(task);
        Ok(task_id)
    }

    /// Remove a task
    pub fn remove_task(&mut self, task_id: Uuid) -> Option<WorkingTask> {
        if let Some(idx) = self.tasks.iter().position(|t| t.id == task_id) {
            Some(self.tasks.remove(idx))
        } else {
            None
        }
    }

    /// Get task by ID
    pub fn get_task(&self, task_id: Uuid) -> Option<&WorkingTask> {
        self.tasks.iter().find(|t| t.id == task_id)
    }

    /// Get mutable task by ID
    pub fn get_task_mut(&mut self, task_id: Uuid) -> Option<&mut WorkingTask> {
        self.tasks.iter_mut().find(|t| t.id == task_id)
    }

    /// Set current focus
    pub fn set_focus(&mut self, task_id: Option<Uuid>) {
        self.current_focus = task_id;

        // Mark focused task as active
        if let Some(id) = task_id {
            let current_time = self.current_time;
            if let Some(task) = self.get_task_mut(id) {
                if task.status == TaskStatus::Pending {
                    task.start(current_time);
                }
            }
        }
    }

    /// Get current focus
    pub fn get_focus(&self) -> Option<&WorkingTask> {
        self.current_focus.and_then(|id| self.get_task(id))
    }

    /// Get highest priority pending task
    pub fn next_task(&self) -> Option<&WorkingTask> {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .max_by(|a, b| {
                a.score(self.current_time)
                    .partial_cmp(&b.score(self.current_time))
                    .unwrap()
            })
    }

    /// Get all tasks with a specific status
    pub fn tasks_with_status(&self, status: TaskStatus) -> Vec<&WorkingTask> {
        self.tasks.iter().filter(|t| t.status == status).collect()
    }

    /// Get all pending tasks sorted by priority
    pub fn pending_tasks(&self) -> Vec<&WorkingTask> {
        let mut tasks: Vec<&WorkingTask> = self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .collect();

        tasks.sort_by(|a, b| {
            b.score(self.current_time)
                .partial_cmp(&a.score(self.current_time))
                .unwrap()
        });

        tasks
    }

    /// Store temporary information
    pub fn store_temp(&mut self, key: String, value: String, duration: u64) {
        let expires = self.current_time + duration;
        self.temp_info.push(TemporaryInfo { key, value, expires });
    }

    /// Retrieve temporary information
    pub fn retrieve_temp(&self, key: &str) -> Option<String> {
        self.temp_info
            .iter()
            .find(|info| info.key == key && info.expires > self.current_time)
            .map(|info| info.value.clone())
    }

    /// Clear expired temporary information
    pub fn clear_expired(&mut self) {
        self.temp_info.retain(|info| info.expires > self.current_time);
    }

    /// Complete current task and switch to next
    pub fn complete_current(&mut self) -> Option<Uuid> {
        if let Some(task_id) = self.current_focus {
            if let Some(task) = self.get_task_mut(task_id) {
                task.complete();
            }
        }

        // Switch to next task
        let next = self.next_task().map(|t| t.id);
        self.set_focus(next);
        next
    }

    /// Tick the working memory
    pub fn tick(&mut self, current_time: u64) {
        self.current_time = current_time;
        self.clear_expired();

        // Auto-complete very old completed/failed tasks
        self.tasks.retain(|t| {
            let age = current_time.saturating_sub(t.created);
            !(matches!(t.status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled) && age > 1000)
        });
    }

    /// Get working memory statistics
    pub fn stats(&self) -> WorkingMemoryStats {
        let mut pending = 0;
        let mut active = 0;
        let mut paused = 0;
        let mut completed = 0;

        for task in &self.tasks {
            match task.status {
                TaskStatus::Pending => pending += 1,
                TaskStatus::Active => active += 1,
                TaskStatus::Paused => paused += 1,
                TaskStatus::Completed => completed += 1,
                _ => {}
            }
        }

        WorkingMemoryStats {
            total_tasks: self.tasks.len(),
            pending_tasks: pending,
            active_tasks: active,
            paused_tasks: paused,
            completed_tasks: completed,
            has_focus: self.current_focus.is_some(),
            capacity_used: (self.tasks.len() as f32 / self.max_tasks as f32) * 100.0,
        }
    }

    /// Clear all tasks
    pub fn clear(&mut self) {
        self.tasks.clear();
        self.temp_info.clear();
        self.current_focus = None;
    }
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new(7) // Miller's Law: 7 ± 2 items
    }
}

/// Working memory statistics
#[derive(Debug, Clone)]
pub struct WorkingMemoryStats {
    pub total_tasks: usize,
    pub pending_tasks: usize,
    pub active_tasks: usize,
    pub paused_tasks: usize,
    pub completed_tasks: usize,
    pub has_focus: bool,
    pub capacity_used: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = WorkingTask::new(
            "Gather wood".to_string(),
            TaskPriority::Normal,
            0,
        );

        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, TaskPriority::Normal);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = WorkingTask::new(
            "Build shelter".to_string(),
            TaskPriority::High,
            0,
        );

        assert_eq!(task.status, TaskStatus::Pending);

        task.start(10);
        assert_eq!(task.status, TaskStatus::Active);
        assert_eq!(task.started, Some(10));

        task.complete();
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[test]
    fn test_working_memory_add() {
        let mut wm = WorkingMemory::new(5);
        let task = WorkingTask::new("Task 1".to_string(), TaskPriority::Normal, 0);
        let task_id = wm.add_task(task).unwrap();

        assert_eq!(wm.tasks.len(), 1);
        assert!(wm.get_task(task_id).is_some());
    }

    #[test]
    fn test_working_memory_capacity() {
        let mut wm = WorkingMemory::new(3);

        for i in 0..5 {
            let task = WorkingTask::new(
                format!("Task {}", i),
                TaskPriority::Normal,
                i,
            );
            let _ = wm.add_task(task);
        }

        // Should cap at 3, removing lowest priority tasks
        assert_eq!(wm.tasks.len(), 3);
    }

    #[test]
    fn test_focus_management() {
        let mut wm = WorkingMemory::new(5);

        let task = WorkingTask::new("Focus task".to_string(), TaskPriority::High, 0);
        let task_id = wm.add_task(task).unwrap();

        wm.set_focus(Some(task_id));
        assert_eq!(wm.current_focus, Some(task_id));

        let focused = wm.get_focus().unwrap();
        assert_eq!(focused.status, TaskStatus::Active);
    }

    #[test]
    fn test_next_task_priority() {
        let mut wm = WorkingMemory::new(5);

        wm.add_task(WorkingTask::new("Low".to_string(), TaskPriority::Low, 0)).unwrap();
        wm.add_task(WorkingTask::new("High".to_string(), TaskPriority::High, 0)).unwrap();
        wm.add_task(WorkingTask::new("Normal".to_string(), TaskPriority::Normal, 0)).unwrap();

        let next = wm.next_task().unwrap();
        assert_eq!(next.priority, TaskPriority::High);
    }

    #[test]
    fn test_temp_storage() {
        let mut wm = WorkingMemory::new(5);
        wm.current_time = 0;

        wm.store_temp("location".to_string(), "10,20,0".to_string(), 100);

        assert_eq!(wm.retrieve_temp("location"), Some("10,20,0".to_string()));

        // Fast forward past expiry
        wm.current_time = 200;
        wm.clear_expired();

        assert_eq!(wm.retrieve_temp("location"), None);
    }

    #[test]
    fn test_task_score_calculation() {
        let mut task1 = WorkingTask::new("Task 1".to_string(), TaskPriority::High, 0);
        task1.motivation = 0.8;

        let mut task2 = WorkingTask::new("Task 2".to_string(), TaskPriority::Normal, 0);
        task2.motivation = 0.3;

        assert!(task1.score(100) > task2.score(100));
    }

    #[test]
    fn test_complete_current() {
        let mut wm = WorkingMemory::new(5);

        let task1 = WorkingTask::new("Task 1".to_string(), TaskPriority::High, 0);
        let task1_id = wm.add_task(task1).unwrap();

        let task2 = WorkingTask::new("Task 2".to_string(), TaskPriority::Normal, 0);
        wm.add_task(task2).unwrap();

        wm.set_focus(Some(task1_id));
        wm.complete_current();

        // Task 1 should be completed
        let completed_task = wm.get_task(task1_id).unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);

        // Should switch to task 2
        assert!(wm.current_focus.is_some());
        assert_ne!(wm.current_focus, Some(task1_id));
    }

    #[test]
    fn test_overdue_task() {
        let mut task = WorkingTask::new("Quick task".to_string(), TaskPriority::Normal, 0);
        task.duration_estimate = Some(100);
        task.start(0);

        assert!(!task.is_overdue(50));
        assert!(task.is_overdue(150));
    }

    #[test]
    fn test_critical_priority_boost() {
        let critical = WorkingTask::new("Critical".to_string(), TaskPriority::Critical, 0);
        let high = WorkingTask::new("High".to_string(), TaskPriority::High, 0);

        assert!(critical.score(0) > high.score(0) * 1.5);
    }
}
