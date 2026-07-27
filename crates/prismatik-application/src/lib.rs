//! # prismatik-prismatik-application
//!
//! Layer 4 — Application and shell
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — application composition contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use app::{AppConfig, AppError, DefaultPrismatikApp, PrismatikApp};
pub use profile::{AppProfile, CloudApp, DesktopApp, EnterpriseApp};
pub use task_graph::{PipelineTask, TaskGraph, TaskId, TaskKind, Trigger};
pub use wiring::{AppWiring, AppWiringError};

/// Application root contracts.
pub mod app {
    use crate::task_graph::{PipelineTask, TaskGraph, TaskKind, Trigger};
    use crate::wiring::{AppWiring, AppWiringError};

    /// Application configuration.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct AppConfig {
        /// Profile identifier.
        pub profile_id: String,
        /// Optional host bind for API/server components.
        pub bind_address: Option<String>,
    }

    /// Application error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct AppError {
        /// Error message.
        pub message: String,
    }

    impl From<AppWiringError> for AppError {
        fn from(value: AppWiringError) -> Self {
            Self {
                message: value.message,
            }
        }
    }

    /// Composition root contract.
    pub trait PrismatikApp: Send + Sync {
        /// Start app services.
        fn start(&self, config: &AppConfig) -> Result<TaskGraph, AppError>;
    }

    /// Default in-process application bootstrapper.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct DefaultPrismatikApp;

    impl PrismatikApp for DefaultPrismatikApp {
        fn start(&self, config: &AppConfig) -> Result<TaskGraph, AppError> {
            if config.profile_id.trim().is_empty() {
                return Err(AppError {
                    message: "profile_id cannot be empty".to_owned(),
                });
            }

            let mut wiring = AppWiring::new();
            wiring.register_task(PipelineTask {
                id: "ingest-bootstrap".to_owned(),
                kind: TaskKind::Ingest,
                trigger: Trigger::Schedule,
            })?;
            wiring.register_task(PipelineTask {
                id: "research-bootstrap".to_owned(),
                kind: TaskKind::Research,
                trigger: Trigger::Event,
            })?;
            wiring.register_task(PipelineTask {
                id: "execution-bootstrap".to_owned(),
                kind: TaskKind::Execute,
                trigger: Trigger::Manual,
            })?;

            wiring.build_task_graph().map_err(AppError::from)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn default_app_starts_with_bootstrap_tasks() {
            let app = DefaultPrismatikApp;
            let graph = app
                .start(&AppConfig {
                    profile_id: "desktop".to_owned(),
                    bind_address: None,
                })
                .expect("app should start");
            assert_eq!(graph.tasks.len(), 3);
        }
    }
}

/// Task-graph contracts.
pub mod task_graph {
    /// Stable task id.
    pub type TaskId = String;

    /// Task trigger source.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Trigger {
        /// Manual trigger.
        Manual,
        /// Scheduled trigger.
        Schedule,
        /// Event-driven trigger.
        Event,
    }

    /// Task kind.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum TaskKind {
        /// Ingest task.
        Ingest,
        /// Research task.
        Research,
        /// Execution task.
        Execute,
    }

    /// Pipeline task.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PipelineTask {
        /// Task id.
        pub id: TaskId,
        /// Task kind.
        pub kind: TaskKind,
        /// Trigger.
        pub trigger: Trigger,
    }

    /// In-memory task graph.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct TaskGraph {
        /// Registered tasks.
        pub tasks: Vec<PipelineTask>,
    }
}

/// Profile marker contracts.
pub mod profile {
    /// Application profile.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum AppProfile {
        /// Desktop profile.
        Desktop,
        /// Cloud profile.
        Cloud,
        /// Enterprise profile.
        Enterprise,
    }

    /// Desktop app marker.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct DesktopApp;
    /// Cloud app marker.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct CloudApp;
    /// Enterprise app marker.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct EnterpriseApp;
}

/// Dependency wiring namespace.
pub mod wiring {
    use crate::task_graph::{PipelineTask, TaskGraph, TaskKind, Trigger};
    use std::collections::BTreeSet;

    /// In-memory wiring registry.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct AppWiring {
        tasks: Vec<PipelineTask>,
    }

    /// Wiring validation failure.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct AppWiringError {
        /// Error message.
        pub message: String,
    }

    impl AppWiring {
        /// Create empty wiring.
        pub fn new() -> Self {
            Self::default()
        }

        /// Register a pipeline task.
        pub fn register_task(&mut self, task: PipelineTask) -> Result<(), AppWiringError> {
            if self.tasks.iter().any(|existing| existing.id == task.id) {
                return Err(AppWiringError {
                    message: format!("duplicate task id '{}'", task.id),
                });
            }
            self.tasks.push(task);
            Ok(())
        }

        /// Validate wiring invariants.
        pub fn validate(&self) -> Result<(), AppWiringError> {
            if self.tasks.is_empty() {
                return Err(AppWiringError {
                    message: "no tasks registered".to_owned(),
                });
            }
            let mut seen = BTreeSet::new();
            for task in &self.tasks {
                if !seen.insert(task.id.clone()) {
                    return Err(AppWiringError {
                        message: format!("duplicate task id '{}'", task.id),
                    });
                }
            }
            Ok(())
        }

        /// Build a deterministic `TaskGraph` from currently registered tasks.
        pub fn build_task_graph(&self) -> Result<TaskGraph, AppWiringError> {
            self.validate()?;
            let mut tasks = self.tasks.clone();
            tasks.sort_by(|left, right| {
                trigger_rank(left.trigger)
                    .cmp(&trigger_rank(right.trigger))
                    .then_with(|| task_kind_rank(left.kind).cmp(&task_kind_rank(right.kind)))
                    .then_with(|| left.id.cmp(&right.id))
            });
            Ok(TaskGraph { tasks })
        }
    }

    fn trigger_rank(trigger: Trigger) -> u8 {
        match trigger {
            Trigger::Event => 0,
            Trigger::Schedule => 1,
            Trigger::Manual => 2,
        }
    }

    fn task_kind_rank(kind: TaskKind) -> u8 {
        match kind {
            TaskKind::Ingest => 0,
            TaskKind::Research => 1,
            TaskKind::Execute => 2,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::task_graph::{PipelineTask, TaskKind, Trigger};

        #[test]
        fn rejects_duplicate_task_ids() {
            let mut wiring = AppWiring::new();
            wiring
                .register_task(PipelineTask {
                    id: "task-1".to_owned(),
                    kind: TaskKind::Ingest,
                    trigger: Trigger::Manual,
                })
                .expect("first register");
            let error = wiring
                .register_task(PipelineTask {
                    id: "task-1".to_owned(),
                    kind: TaskKind::Execute,
                    trigger: Trigger::Event,
                })
                .expect_err("duplicate id should fail");
            assert!(error.message.contains("duplicate task id"));
        }

        #[test]
        fn builds_task_graph_in_deterministic_order() {
            let mut wiring = AppWiring::new();
            wiring
                .register_task(PipelineTask {
                    id: "b".to_owned(),
                    kind: TaskKind::Execute,
                    trigger: Trigger::Manual,
                })
                .expect("register");
            wiring
                .register_task(PipelineTask {
                    id: "a".to_owned(),
                    kind: TaskKind::Ingest,
                    trigger: Trigger::Event,
                })
                .expect("register");
            wiring
                .register_task(PipelineTask {
                    id: "c".to_owned(),
                    kind: TaskKind::Research,
                    trigger: Trigger::Schedule,
                })
                .expect("register");

            let graph = wiring.build_task_graph().expect("graph");
            let ids: Vec<&str> = graph.tasks.iter().map(|task| task.id.as_str()).collect();
            assert_eq!(ids, vec!["a", "c", "b"]);
        }
    }
}
