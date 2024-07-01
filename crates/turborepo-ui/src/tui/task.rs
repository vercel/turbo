#![allow(dead_code)]
use std::time::Instant;

use super::event::TaskResult;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Planned;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Running {
    start: Instant,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Finished {
    start: Instant,
    end: Instant,
    result: TaskResult,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Task<S> {
    name: String,
    state: S,
}

pub enum TaskType {
    Planned,
    Running,
    Finished,
}

#[derive(Clone)]
pub struct TasksByStatus {
    pub running: Vec<Task<Running>>,
    pub planned: Vec<Task<Planned>>,
    pub finished: Vec<Task<Finished>>,
}

impl TasksByStatus {
    pub fn all_empty(&self) -> bool {
        self.planned.is_empty() && self.finished.is_empty() && self.running.is_empty()
    }

    pub fn task_names_in_displayed_order(&self) -> Vec<String> {
        let running_names = self
            .running
            .iter()
            .map(|task| task.name().to_string())
            .collect::<Vec<_>>();
        let planned_names = self
            .planned
            .iter()
            .map(|task| task.name().to_string())
            .collect::<Vec<_>>();
        let finished_names = self
            .finished
            .iter()
            .map(|task| task.name().to_string())
            .collect::<Vec<_>>();

        [
            running_names.as_slice(),
            planned_names.as_slice(),
            finished_names.as_slice(),
        ]
        .concat()
    }

    pub fn tasks_started(&self) -> Vec<&str> {
        let (errors, success): (Vec<_>, Vec<_>) = self
            .finished
            .iter()
            .partition(|task| matches!(task.result(), TaskResult::Failure));

        // We return errors last as they most likely have information users want to see
        success
            .into_iter()
            .map(|task| task.name())
            .chain(self.running.iter().map(|task| task.name()))
            .chain(errors.into_iter().map(|task| task.name()))
            .collect()
    }
}

impl<S> Task<S> {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Task<Planned> {
    pub fn new(name: String) -> Task<Planned> {
        Task {
            name,
            state: Planned,
        }
    }

    pub fn start(self) -> Task<Running> {
        Task {
            name: self.name,
            state: Running {
                start: Instant::now(),
            },
        }
    }
}

impl Task<Running> {
    pub fn finish(self, result: TaskResult) -> Task<Finished> {
        let Task {
            name,
            state: Running { start },
        } = self;
        Task {
            name,
            state: Finished {
                start,
                result,
                end: Instant::now(),
            },
        }
    }

    pub fn start(&self) -> Instant {
        self.state.start
    }
}

impl Task<Finished> {
    pub fn start(&self) -> Instant {
        self.state.start
    }

    pub fn end(&self) -> Instant {
        self.state.end
    }

    pub fn result(&self) -> TaskResult {
        self.state.result
    }
}
