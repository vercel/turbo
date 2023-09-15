//! `process`
//!
//! This module contains the code that is responsible for running the commands
//! that are queued by run. It consists of a set of child processes that are
//! spawned and managed by the manager. The manager is responsible for
//! running these processes to completion, forwarding signals, and closing
//! them when the manager is closed.
//!
//! As of now, the manager will execute futures in a random order, and
//! must be either `wait`ed on or `stop`ped to drive state.

mod child;

use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

pub use child::Command;
use futures::Future;
use tokio::task::JoinSet;
use tracing::{debug, trace};

use self::child::{Child, ChildExit};

/// A process manager that is responsible for spawning and managing child
/// processes. When the manager is Open, new child processes can be spawned
/// using `spawn`. When the manager is Closed, all currently-running children
/// will be closed, and no new children can be spawned.
#[derive(Debug, Clone)]
pub struct ProcessManager(Arc<Mutex<ProcessManagerInner>>);

#[derive(Debug)]
struct ProcessManagerInner {
    is_closing: bool,
    children: Vec<child::Child>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(ProcessManagerInner {
            is_closing: false,
            children: Vec::new(),
        })))
    }
}

impl ProcessManager {
    /// Spawn a new child process to run the given command.
    ///
    /// The handle of the child can be either waited or stopped by the caller,
    /// as well as the entire process manager.
    ///
    /// If spawn returns None, the process manager is closed and the child
    /// process was not spawned. If spawn returns Some(Err), the process
    /// manager is open, but the child process failed to spawn.
    pub fn spawn(
        &self,
        command: child::Command,
        stop_timeout: Duration,
    ) -> Option<io::Result<child::Child>> {
        let mut lock = self.0.lock().unwrap();
        if lock.is_closing {
            return None;
        }
        let child = child::Child::spawn(command, child::ShutdownStyle::Graceful(stop_timeout));
        if let Ok(child) = &child {
            lock.children.push(child.clone());
        }
        Some(child)
    }

    /// Stop the process manager, closing all child processes. On posix
    /// systems this will send a SIGINT, and on windows it will just kill
    /// the process immediately.
    pub async fn stop(&self) {
        self.close(|mut c| async move { c.stop().await }).await
    }

    /// Stop the process manager, waiting for all child processes to exit.
    ///
    /// If you want to set a timeout, use `tokio::time::timeout` and
    /// `Self::stop` if the timeout elapses.
    pub async fn wait(&self) {
        self.close(|mut c| async move { c.wait().await }).await
    }

    /// Close the process manager, running the given callback on each child
    ///
    /// note: this is designed to be called multiple times, ie calling close
    /// with two different strategies will propagate both signals to the child
    /// processes. clearing the task queue and re-enabling spawning are both
    /// idempotent operations
    async fn close<F, C>(&self, callback: F)
    where
        F: Fn(Child) -> C + Sync + Send + Copy + 'static,
        C: Future<Output = Option<ChildExit>> + Sync + Send + 'static,
    {
        let mut set = JoinSet::new();

        {
            let mut lock = self.0.lock().expect("not poisoned");
            lock.is_closing = true;
            for child in lock.children.iter() {
                let child = child.clone();
                set.spawn(async move { callback(child).await });
            }
        }

        debug!("waiting for {} processes to exit", set.len());

        while let Some(out) = set.join_next().await {
            trace!("process exited: {:?}", out);
        }

        {
            let mut lock = self.0.lock().expect("not poisoned");

            // just allocate a new vec rather than clearing the old one
            lock.children = vec![];
            lock.is_closing = false;
        }
    }
}

#[cfg(test)]
mod test {

    use futures::{stream::FuturesUnordered, StreamExt};
    use test_case::test_case;
    use time::Instant;
    use tokio::{join, process::Command, time::sleep};
    use tracing_test::traced_test;

    use super::*;

    fn get_command() -> Command {
        let mut cmd = Command::new("node");
        cmd.arg("./test/scripts/sleep_5_interruptable.js");
        cmd
    }

    #[tokio::test]
    async fn test_basic() {
        let manager = ProcessManager::new();
        manager.spawn(get_command(), Duration::from_secs(2));
        manager.stop().await;
    }

    #[tokio::test]
    async fn test_multiple() {
        let manager = ProcessManager::new();

        manager.spawn(get_command(), Duration::from_secs(2));
        manager.spawn(get_command(), Duration::from_secs(2));
        manager.spawn(get_command(), Duration::from_secs(2));

        sleep(Duration::from_millis(100)).await;

        manager.stop().await;
    }

    #[tokio::test]
    async fn test_closed() {
        let manager = ProcessManager::new();
        manager.spawn(get_command(), Duration::from_secs(2));
        manager.stop().await;

        manager.spawn(get_command(), Duration::from_secs(2));

        sleep(Duration::from_millis(100)).await;

        manager.stop().await;
    }

    #[tokio::test]
    async fn test_exit_code() {
        let manager = ProcessManager::new();
        let mut child = manager
            .spawn(get_command(), Duration::from_secs(2))
            .unwrap()
            .unwrap();

        sleep(Duration::from_millis(100)).await;

        let code = child.wait().await;
        assert_eq!(code, Some(ChildExit::Finished(Some(0))));

        manager.stop().await;
    }

    #[tokio::test]
    #[traced_test]
    async fn test_message_after_stop() {
        let manager = ProcessManager::new();
        let mut child = manager
            .spawn(get_command(), Duration::from_secs(2))
            .unwrap()
            .unwrap();

        sleep(Duration::from_millis(100)).await;

        let exit = child.wait().await;
        assert_eq!(exit, Some(ChildExit::Finished(Some(0))));

        manager.stop().await;

        // this is idempotent, so calling it after the manager is stopped is ok
        child.kill().await;

        let code = child.wait().await;
        assert_eq!(code, None);
    }

    #[tokio::test]
    async fn test_reuse_manager() {
        let manager = ProcessManager::new();
        manager.spawn(get_command(), Duration::from_secs(2));

        sleep(Duration::from_millis(100)).await;

        manager.stop().await;

        assert!(manager.0.lock().unwrap().children.is_empty());

        // idempotent
        manager.stop().await;
    }

    #[test_case("stop", if cfg!(windows) {ChildExit::Killed} else {ChildExit::Finished(None)})] // windows doesn't support graceful stop
    #[test_case("wait", ChildExit::Finished(Some(0)))]
    #[tokio::test]
    async fn test_stop_multiple_tasks_shared(strat: &str, expected: ChildExit) {
        let manager = ProcessManager::new();
        let tasks = FuturesUnordered::new();

        for _ in 0..10 {
            let manager = manager.clone();
            tasks.push(tokio::spawn(async move {
                manager
                    .spawn(get_command(), Duration::from_secs(1))
                    .unwrap()
                    .unwrap()
                    .wait()
                    .await
            }));
        }

        // wait for tasks to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        match strat {
            "stop" => manager.stop().await,
            "wait" => manager.wait().await,
            _ => panic!("unknown strat"),
        }

        // tasks return proper exit code
        assert!(
            tasks.all(|v| async { v.unwrap() == Some(expected) }).await,
            "not all tasks returned the correct code: {:?}",
            expected
        );
    }

    #[tokio::test]
    async fn test_wait_multiple_tasks() {
        let manager = ProcessManager::new();

        manager.spawn(get_command(), Duration::from_secs(1));

        // let the task start
        tokio::time::sleep(Duration::from_millis(50)).await;

        let start_time = Instant::now();

        // we support 'close escalation'; someone can call
        // stop even if others are waiting
        let _ = join! {
            manager.wait(),
            manager.wait(),
            manager.stop(),
        };

        let finish_time = Instant::now();

        assert!((finish_time - start_time).lt(&Duration::from_secs(2)));
    }
}
