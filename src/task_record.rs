use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Running,
    CompletedButUnread,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub started_at: DateTime<Utc>,
    pub log_id: String,
    pub log_path: String,
    #[serde(default)]
    pub manager_pid: Option<u32>,
    #[serde(default)]
    pub cleanup_reason: Option<String>,
    #[serde(default)]
    pub status: TaskStatus,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    // New fields for process tree tracking
    #[serde(default)]
    pub process_chain: Vec<u32>,
    #[serde(default)]
    pub root_parent_pid: Option<u32>,
    #[serde(default)]
    pub process_tree_depth: usize,
}

impl TaskRecord {
    pub fn new(
        started_at: DateTime<Utc>,
        log_id: String,
        log_path: String,
        manager_pid: Option<u32>,
    ) -> Self {
        Self {
            started_at,
            log_id,
            log_path,
            manager_pid,
            cleanup_reason: None,
            status: TaskStatus::Running,
            result: None,
            completed_at: None,
            exit_code: None,
            process_chain: Vec::new(),
            root_parent_pid: None,
            process_tree_depth: 0,
        }
    }

    pub fn with_process_tree(
        mut self,
        process_chain: Vec<u32>,
        root_parent_pid: Option<u32>,
        process_tree_depth: usize,
    ) -> Self {
        self.process_chain = process_chain;
        self.root_parent_pid = root_parent_pid;
        self.process_tree_depth = process_tree_depth;
        self
    }

    pub fn mark_completed(
        mut self,
        result: Option<String>,
        exit_code: Option<i32>,
        completed_at: DateTime<Utc>,
    ) -> Self {
        self.status = TaskStatus::CompletedButUnread;
        self.result = result;
        self.exit_code = exit_code;
        self.completed_at = Some(completed_at);
        self
    }

    pub fn with_cleanup_reason(mut self, reason: &str) -> Self {
        let result = self.result.clone();
        let exit_code = self.exit_code;
        let completed_at = self.completed_at.unwrap_or_else(Utc::now);
        self = self.mark_completed(result, exit_code, completed_at);
        self.cleanup_reason = Some(reason.to_owned());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_task_record_with_process_tree() {
        let base_time = Utc::now();
        let record = TaskRecord::new(
            base_time,
            "1234".to_string(),
            "/tmp/1234.log".to_string(),
            Some(5678),
        );

        let process_chain = vec![5678, 1, 0];
        let root_parent_pid = Some(0);
        let depth = 3;

        let enhanced_record =
            record.with_process_tree(process_chain.clone(), root_parent_pid, depth);

        assert_eq!(enhanced_record.process_chain, process_chain);
        assert_eq!(enhanced_record.root_parent_pid, root_parent_pid);
        assert_eq!(enhanced_record.process_tree_depth, depth);
        assert_eq!(enhanced_record.log_id, "1234");
        assert_eq!(enhanced_record.manager_pid, Some(5678));
    }

    #[test]
    fn test_task_record_serialization_with_process_tree() {
        let base_time = Utc::now();
        let record = TaskRecord::new(
            base_time,
            "1234".to_string(),
            "/tmp/1234.log".to_string(),
            Some(5678),
        )
        .with_process_tree(vec![5678, 1], Some(1), 2);

        // Test that the record can be serialized to JSON
        let json_str = serde_json::to_string(&record).expect("Failed to serialize");

        // Test that it can be deserialized back
        let deserialized: TaskRecord =
            serde_json::from_str(&json_str).expect("Failed to deserialize");

        assert_eq!(deserialized.process_chain, vec![5678, 1]);
        assert_eq!(deserialized.root_parent_pid, Some(1));
        assert_eq!(deserialized.process_tree_depth, 2);
    }

    #[test]
    fn test_task_record_backward_compatibility() {
        // Test that a record without process tree fields can still be deserialized
        let old_record_json = r#"{
            "started_at": "2024-01-01T12:00:00Z",
            "log_id": "1234",
            "log_path": "/tmp/1234.log",
            "manager_pid": 5678,
            "cleanup_reason": null,
            "status": "running",
            "result": null,
            "completed_at": null,
            "exit_code": null
        }"#;

        let deserialized: TaskRecord =
            serde_json::from_str(old_record_json).expect("Failed to deserialize old format");

        assert_eq!(deserialized.process_chain, Vec::<u32>::new());
        assert_eq!(deserialized.root_parent_pid, None);
        assert_eq!(deserialized.process_tree_depth, 0);
        assert_eq!(deserialized.log_id, "1234");
        assert_eq!(deserialized.manager_pid, Some(5678));
    }

    #[test]
    fn test_task_record_mark_completed_preserves_process_tree() {
        let base_time = Utc::now();
        let record = TaskRecord::new(
            base_time,
            "1234".to_string(),
            "/tmp/1234.log".to_string(),
            Some(5678),
        )
        .with_process_tree(vec![5678, 1], Some(1), 2);

        let completed_record =
            record.mark_completed(Some("success".to_string()), Some(0), Utc::now());

        // Process tree fields should be preserved
        assert_eq!(completed_record.process_chain, vec![5678, 1]);
        assert_eq!(completed_record.root_parent_pid, Some(1));
        assert_eq!(completed_record.process_tree_depth, 2);

        // Status should be updated
        assert_eq!(completed_record.status, TaskStatus::CompletedButUnread);
        assert_eq!(completed_record.result, Some("success".to_string()));
        assert_eq!(completed_record.exit_code, Some(0));
    }
}
