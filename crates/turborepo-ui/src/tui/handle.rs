use std::sync::{Arc, Mutex};

use tokio::sync::{mpsc, oneshot};

use super::{
    event::{CacheResult, OutputLogs, PaneSize},
    Event, TaskResult,
};

/// Struct for sending app events to TUI rendering
#[derive(Debug, Clone)]
pub struct AppSender {
    primary: mpsc::UnboundedSender<Event>,
}

/// Struct for receiving app events
pub struct AppReceiver {
    primary: mpsc::UnboundedReceiver<Event>,
}

/// Struct for sending events related to a specific task
#[derive(Debug, Clone)]
pub struct TuiTask {
    name: String,
    handle: AppSender,
    logs: Arc<Mutex<Vec<u8>>>,
}

impl AppSender {
    /// Create a new channel for sending app events.
    ///
    /// AppSender is meant to be held by the actual task runner
    /// AppReceiver should be passed to `crate::tui::run_app`
    pub fn new() -> (Self, AppReceiver) {
        let (primary_tx, primary_rx) = mpsc::unbounded_channel();
        (
            Self {
                primary: primary_tx,
            },
            AppReceiver {
                primary: primary_rx,
            },
        )
    }

    /// Construct a sender configured for a specific task
    pub fn task(&self, task: String) -> TuiTask {
        TuiTask {
            name: task,
            handle: self.clone(),
            logs: Default::default(),
        }
    }

    /// Stop rendering TUI and restore terminal to default configuration
    pub async fn stop(&self) {
        let (callback_tx, callback_rx) = oneshot::channel();
        // Send stop event, if receiver has dropped ignore error as
        // it'll be a no-op.
        self.primary.send(Event::Stop(callback_tx)).ok();
        // Wait for callback to be sent or the channel closed.
        callback_rx.await.ok();
    }

    /// Update the list of tasks displayed in the TUI
    pub fn update_tasks(&self, tasks: Vec<String>) -> Result<(), mpsc::error::SendError<Event>> {
        self.primary.send(Event::UpdateTasks { tasks })
    }

    /// Restart the list of tasks displayed in the TUI
    pub fn restart_tasks(&self, tasks: Vec<String>) -> Result<(), mpsc::error::SendError<Event>> {
        self.primary.send(Event::RestartTasks { tasks })
    }

    /// Fetches the size of the terminal pane
    pub async fn pane_size(&self) -> Option<PaneSize> {
        let (callback_tx, callback_rx) = oneshot::channel();
        // Send query, if no receiver to handle the request return None
        self.primary.send(Event::PaneSizeQuery(callback_tx)).ok()?;
        // Wait for callback to be sent
        callback_rx.await.ok()
    }
}

impl AppReceiver {
    /// Receive an event, producing a tick event if no events are received by
    /// the deadline.
    pub async fn recv(&mut self) -> Option<Event> {
        self.primary.recv().await
    }
}

impl TuiTask {
    /// Access the underlying AppSender
    pub fn as_app(&self) -> &AppSender {
        &self.handle
    }

    /// Mark the task as started
    pub fn start(&self, output_logs: OutputLogs) {
        self.handle
            .primary
            .send(Event::StartTask {
                task: self.name.clone(),
                output_logs,
            })
            .ok();
    }

    /// Mark the task as finished
    pub fn succeeded(&self, is_cache_hit: bool) -> Vec<u8> {
        if is_cache_hit {
            self.finish(TaskResult::CacheHit)
        } else {
            self.finish(TaskResult::Success)
        }
    }

    /// Mark the task as finished
    pub fn failed(&self) -> Vec<u8> {
        self.finish(TaskResult::Failure)
    }

    fn finish(&self, result: TaskResult) -> Vec<u8> {
        self.handle
            .primary
            .send(Event::EndTask {
                task: self.name.clone(),
                result,
            })
            .ok();
        self.logs.lock().expect("logs lock poisoned").clone()
    }

    pub fn set_stdin(&self, stdin: Box<dyn std::io::Write + Send>) {
        self.handle
            .primary
            .send(Event::SetStdin {
                task: self.name.clone(),
                stdin,
            })
            .ok();
    }

    pub fn status(&self, status: &str, result: CacheResult) {
        // Since this will be rendered via ratatui we any ANSI escape codes will not be
        // handled.
        // TODO: prevent the status from having ANSI codes in this scenario
        let status = console::strip_ansi_codes(status).into_owned();
        self.handle
            .primary
            .send(Event::Status {
                task: self.name.clone(),
                status,
                result,
            })
            .ok();
    }
}

impl std::io::Write for TuiTask {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let task = self.name.clone();
        {
            self.logs
                .lock()
                .expect("log lock poisoned")
                .extend_from_slice(buf);
        }
        self.handle
            .primary
            .send(Event::TaskOutput {
                task,
                output: buf.to_vec(),
            })
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "receiver dropped"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
