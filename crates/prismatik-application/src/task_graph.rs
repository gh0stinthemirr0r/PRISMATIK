//! Durable task recovery and execution.

use async_trait::async_trait;
use prismatik_storage::{RepositoryError, SqliteBackend, TaskRecord, TaskState};
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;

/// One executable pipeline task kind.
#[async_trait]
pub trait PipelineTask: Send + Sync {
    /// Execute a recovered task record.
    async fn run(&self, task: &TaskRecord) -> Result<(), String>;
}

/// Aggregate result from one recovery pass.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskRunSummary {
    /// Tasks completed successfully.
    pub completed: Vec<String>,
    /// Tasks marked failed, paired with their error.
    pub failed: Vec<(String, String)>,
}

/// Task graph persistence errors.
#[derive(Debug, Error)]
pub enum TaskGraphError {
    /// SQLite repository operation failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Registry that resumes pending/running tasks after a restart.
pub struct TaskGraph<'a> {
    backend: &'a SqliteBackend,
    handlers: BTreeMap<String, Arc<dyn PipelineTask>>,
}

impl std::fmt::Debug for TaskGraph<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TaskGraph")
            .field("handler_kinds", &self.handlers.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl<'a> TaskGraph<'a> {
    /// Create a graph over the operational database.
    pub fn new(backend: &'a SqliteBackend) -> Self {
        Self {
            backend,
            handlers: BTreeMap::new(),
        }
    }

    /// Register a handler for a persisted task kind.
    pub fn register(&mut self, kind: impl Into<String>, handler: Arc<dyn PipelineTask>) {
        self.handlers.insert(kind.into(), handler);
    }

    /// Recover and execute all incomplete tasks, persisting every transition.
    pub async fn run_pending(&self) -> Result<TaskRunSummary, TaskGraphError> {
        let tasks = self.backend.recover_incomplete_tasks()?;
        let mut summary = TaskRunSummary::default();
        for task in tasks {
            self.backend
                .set_task_state(&task.task_id, TaskState::Running, None)?;
            let result = match self.handlers.get(&task.task_kind) {
                Some(handler) => handler.run(&task).await,
                None => Err(format!("no handler registered for {}", task.task_kind)),
            };
            match result {
                Ok(()) => {
                    self.backend
                        .set_task_state(&task.task_id, TaskState::Completed, None)?;
                    summary.completed.push(task.task_id);
                },
                Err(error) => {
                    self.backend
                        .set_task_state(&task.task_id, TaskState::Failed, Some(&error))?;
                    summary.failed.push((task.task_id, error));
                },
            }
        }
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Pass;

    #[async_trait]
    impl PipelineTask for Pass {
        async fn run(&self, _task: &TaskRecord) -> Result<(), String> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn recovered_task_is_completed() {
        let db = SqliteBackend::open_in_memory().unwrap();
        let task = db.insert_task("normalize", "{}", None).unwrap();
        let mut graph = TaskGraph::new(&db);
        graph.register("normalize", Arc::new(Pass));
        let summary = graph.run_pending().await.unwrap();
        assert_eq!(summary.completed, vec![task.task_id]);
        assert!(db.recover_incomplete_tasks().unwrap().is_empty());
    }

    #[tokio::test]
    async fn missing_handler_marks_task_failed() {
        let db = SqliteBackend::open_in_memory().unwrap();
        db.insert_task("unknown", "{}", None).unwrap();
        let summary = TaskGraph::new(&db).run_pending().await.unwrap();
        assert_eq!(summary.failed.len(), 1);
        assert!(db.recover_incomplete_tasks().unwrap().is_empty());
    }
}
