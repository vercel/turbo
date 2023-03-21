use std::{
    borrow::Cow,
    collections::HashMap,
    future::Future,
    mem::take,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use futures::join;
use indexmap::IndexSet;
use owo_colors::OwoColorize;
use serde::{de::DeserializeOwned, Serialize};
use tokio::{
    io::{
        stderr, stdout, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
        BufReader, Stderr, Stdout,
    },
    net::{TcpListener, TcpStream},
    process::{Child, ChildStderr, ChildStdout, Command},
    select,
    sync::{OwnedSemaphorePermit, Semaphore},
    time::{sleep, timeout},
};
use turbo_tasks_fs::FileSystemPathVc;
use turbopack_ecmascript::magic_identifier::decode_identifiers;

use crate::{source_map::apply_source_mapping, AssetsForSourceMappingVc};

enum NodeJsPoolProcess {
    Spawned(SpawnedNodeJsPoolProcess),
    Running(RunningNodeJsPoolProcess),
}

struct SpawnedNodeJsPoolProcess {
    child: Child,
    listener: TcpListener,
    assets_for_source_mapping: AssetsForSourceMappingVc,
    assets_root: FileSystemPathVc,
    project_dir: FileSystemPathVc,
    shared_stdout: SharedOutputSet,
    shared_stderr: SharedOutputSet,
    debug: bool,
}

struct RunningNodeJsPoolProcess {
    child: Option<Child>,
    connection: TcpStream,
    assets_for_source_mapping: AssetsForSourceMappingVc,
    assets_root: FileSystemPathVc,
    project_dir: FileSystemPathVc,
    stdout_handler: OutputStreamHandler<ChildStdout, Stdout>,
    stderr_handler: OutputStreamHandler<ChildStderr, Stderr>,
}

impl RunningNodeJsPoolProcess {
    pub async fn apply_source_mapping<'a>(
        &self,
        text: &'a str,
        ansi_colors: bool,
    ) -> Result<Cow<'a, str>> {
        let text = decode_identifiers(text, |content| {
            if ansi_colors {
                format!("{{{}}}", content).italic().to_string()
            } else {
                format!("{{{}}}", content)
            }
        });
        match text {
            Cow::Borrowed(text) => {
                apply_source_mapping(
                    text,
                    self.assets_for_source_mapping,
                    self.assets_root,
                    self.project_dir,
                    ansi_colors,
                )
                .await
            }
            Cow::Owned(ref text) => {
                let cow = apply_source_mapping(
                    text,
                    self.assets_for_source_mapping,
                    self.assets_root,
                    self.project_dir,
                    ansi_colors,
                )
                .await?;
                Ok(Cow::Owned(cow.into_owned()))
            }
        }
    }
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, PartialEq, Eq, Hash)]
struct OutputEntry {
    data: Arc<[u8]>,
    stack_trace: Option<Arc<[u8]>>,
}

type SharedOutputSet = Arc<Mutex<IndexSet<(OutputEntry, u32)>>>;

static GLOBAL_OUTPUT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static MARKER: &[u8] = b"TURBOPACK_OUTPUT_";
static MARKER_STR: &str = "TURBOPACK_OUTPUT_";

struct OutputStreamHandler<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    stream: BufReader<R>,
    shared: SharedOutputSet,
    assets_for_source_mapping: AssetsForSourceMappingVc,
    root: FileSystemPathVc,
    project_dir: FileSystemPathVc,
    final_stream: W,
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> OutputStreamHandler<R, W> {
    /// Pipes the `stream` from `final_stream`, but uses `shared` to deduplicate
    /// lines that has beem emitted by other [OutputStreamHandler] instances
    /// with the same `shared` before.
    /// Returns when one operation is done.
    pub async fn handle_operation(&mut self) -> Result<()> {
        let Self {
            stream,
            shared,
            assets_for_source_mapping,
            root,
            project_dir,
            final_stream,
        } = self;

        macro_rules! write_final {
            ($($args:tt)+) => {
                {
                    let _lock = GLOBAL_OUTPUT_LOCK.lock().await;
                    if final_stream.write($($args)+).await.is_err() {
                        println!("Error writing to final stream");
                        // Whatever happened with stdout/stderr, we can't write to it anymore.
                        break;
                    }
                }
            };
        }
        macro_rules! write_source_mapped_final {
            ($bytes:expr) => {
                if let Ok(text) = std::str::from_utf8($bytes) {
                    let text = decode_identifiers(text, |content| {
                        format!("{{{}}}", content).italic().to_string()
                    });
                    match apply_source_mapping(text.as_ref(), *assets_for_source_mapping, *root,*project_dir, true).await {
                        Err(e) => {
                            write_final!(format!("Error applying source mapping: {e}\n").as_bytes());
                            write_final!(text.as_bytes());
                        }
                        Ok(text) => {
                            write_final!(text.as_bytes());
                        }
                    }
                } else {
                    write_final!($bytes);
                }
            };
        }

        let mut buffer = Vec::new();
        let mut own_output = HashMap::new();
        let mut nesting: u32 = 0;
        let mut in_stack = None;
        let mut stack_trace_buffer = Vec::new();
        loop {
            let start = buffer.len();
            match stream.read_until(b'\n', &mut buffer).await {
                Ok(0) => {
                    break;
                }
                Err(err) => {
                    eprintln!("error reading from stream: {}", err);
                    break;
                }
                Ok(_) => {}
            }
            if buffer.len() - start == MARKER.len() + 2
                && &buffer[start..buffer.len() - 2] == MARKER
            {
                // This is new line
                buffer.pop();
                // This is the type
                match buffer.pop() {
                    Some(b'B') => {
                        stack_trace_buffer.clear();
                        buffer.truncate(start);
                        nesting += 1;
                        in_stack = None;
                        continue;
                    }
                    Some(b'E') => {
                        buffer.truncate(start);
                        if let Some(in_stack) = in_stack {
                            if nesting != 0 {
                                stack_trace_buffer = buffer[in_stack..].to_vec();
                            }
                            buffer.truncate(in_stack);
                        }
                        nesting = nesting.saturating_sub(1);
                        in_stack = None;
                        if nesting == 0 {
                            let line = Arc::from(take(&mut buffer).into_boxed_slice());
                            let stack_trace = if stack_trace_buffer.is_empty() {
                                None
                            } else {
                                Some(Arc::from(take(&mut stack_trace_buffer).into_boxed_slice()))
                            };
                            let entry = OutputEntry {
                                data: line,
                                stack_trace,
                            };
                            let occurance_number = *own_output
                                .entry(entry.clone())
                                .and_modify(|c| *c += 1)
                                .or_insert(0);
                            let new_entry = {
                                let mut shared = shared.lock().unwrap();
                                shared.insert((entry.clone(), occurance_number))
                            };
                            if !new_entry {
                                // This line has been printed by another process, so we don't need
                                // to print it again.
                                continue;
                            }
                            write_source_mapped_final!(&entry.data);
                        }
                    }
                    Some(b'S') => {
                        buffer.truncate(start);
                        in_stack = Some(start);
                        continue;
                    }
                    Some(b'D') => {
                        // operation done
                        break;
                    }
                    _ => {}
                }
            }
            if nesting != 0 {
                // When inside of a marked output we want to aggregate until the end marker
                continue;
            }

            write_source_mapped_final!(&buffer);
            buffer.clear();
        }
        Ok(())
    }
}

impl NodeJsPoolProcess {
    async fn new(
        cwd: &Path,
        env: &HashMap<String, String>,
        entrypoint: &Path,
        assets_for_source_mapping: AssetsForSourceMappingVc,
        assets_root: FileSystemPathVc,
        project_dir: FileSystemPathVc,
        shared_stdout: SharedOutputSet,
        shared_stderr: SharedOutputSet,
        debug: bool,
    ) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("binding to a port")?;
        let port = listener.local_addr().context("getting port")?.port();
        let mut cmd = Command::new("node");
        cmd.current_dir(cwd);
        if debug {
            cmd.arg("--inspect-brk");
        }
        cmd.arg(entrypoint);
        cmd.arg(port.to_string());
        cmd.env_clear();
        cmd.env(
            "PATH",
            std::env::var("PATH").expect("the PATH environment variable should always be set"),
        );
        #[cfg(target_family = "windows")]
        cmd.env(
            "SystemRoot",
            std::env::var("SystemRoot")
                .expect("the SystemRoot environment variable should always be set"),
        );
        cmd.envs(env);
        cmd.stderr(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.kill_on_drop(true);

        let child = cmd.spawn().context("spawning node pooled process")?;

        Ok(Self::Spawned(SpawnedNodeJsPoolProcess {
            listener,
            child,
            debug,
            assets_for_source_mapping,
            assets_root,
            project_dir,
            shared_stdout,
            shared_stderr,
        }))
    }

    async fn run(self) -> Result<RunningNodeJsPoolProcess> {
        Ok(match self {
            NodeJsPoolProcess::Spawned(SpawnedNodeJsPoolProcess {
                mut child,
                listener,
                assets_for_source_mapping,
                assets_root,
                project_dir,
                shared_stdout,
                shared_stderr,
                debug,
            }) => {
                let timeout = if debug {
                    Duration::MAX
                } else {
                    CONNECT_TIMEOUT
                };

                async fn get_output(child: &mut Child) -> Result<(String, String)> {
                    let mut stdout = Vec::new();
                    let mut stderr = Vec::new();
                    child
                        .stdout
                        .take()
                        .unwrap()
                        .read_to_end(&mut stdout)
                        .await?;
                    child
                        .stderr
                        .take()
                        .unwrap()
                        .read_to_end(&mut stderr)
                        .await?;
                    fn clean(buffer: Vec<u8>) -> Result<String> {
                        Ok(String::from_utf8(buffer)?
                            .lines()
                            .filter(|line| {
                                line.len() != MARKER_STR.len() + 1 && !line.starts_with(MARKER_STR)
                            })
                            .collect::<Vec<_>>()
                            .join("\n"))
                    }
                    Ok((clean(stdout)?, clean(stderr)?))
                }

                let (connection, _) = select! {
                    connection = listener.accept() => connection.context("accepting connection")?,
                    status = child.wait() => {
                        match status {
                            Ok(status) => {
                                let (stdout, stderr) = get_output(&mut child).await?;
                                println!("node process exited before we could connect to it with {status}Process output:\n{stdout}\nProcess error output:\n{stderr}");
                                bail!("node process exited before we could connect to it with {status}\nProcess output:\n{stdout}\nProcess error output:\n{stderr}");
                            }
                            Err(err) => {
                                let _ = child.start_kill();
                                let (stdout, stderr) = get_output(&mut child).await?;
                                println!("node process exited before we could connect to it: {err:?}Process output:\n{stdout}\nProcess error output:\n{stderr}");
                                bail!("node process exited before we could connect to it: {err:?}\nProcess output:\n{stdout}\nProcess error output:\n{stderr}");
                            },
                        }
                    },
                    _ = sleep(timeout) => {
                        let _ = child.start_kill();
                        let (stdout, stderr) = get_output(&mut child).await?;
                        println!("timed out waiting for the Node.js process to connect ({timeout:?})Process output:\n{stdout}\nProcess error output:\n{stderr}");
                        bail!("timed out waiting for the Node.js process to connect ({timeout:?} timeout)\nProcess output:\n{stdout}\nProcess error output:\n{stderr}");
                    },
                };

                let child_stdout = BufReader::new(child.stdout.take().unwrap());
                let child_stderr = BufReader::new(child.stderr.take().unwrap());

                let stdout_handler = OutputStreamHandler {
                    stream: child_stdout,
                    shared: shared_stdout,
                    assets_for_source_mapping: assets_for_source_mapping.clone(),
                    root: assets_root.clone(),
                    project_dir,
                    final_stream: stdout(),
                };
                let stderr_handler = OutputStreamHandler {
                    stream: child_stderr,
                    shared: shared_stderr,
                    assets_for_source_mapping: assets_for_source_mapping.clone(),
                    root: assets_root.clone(),
                    project_dir,
                    final_stream: stderr(),
                };

                RunningNodeJsPoolProcess {
                    child: Some(child),
                    connection,
                    assets_for_source_mapping: assets_for_source_mapping.clone(),
                    assets_root: assets_root.clone(),
                    project_dir,
                    stdout_handler,
                    stderr_handler,
                }
            }
            NodeJsPoolProcess::Running(running) => running,
        })
    }
}

impl RunningNodeJsPoolProcess {
    async fn recv(&mut self) -> Result<Vec<u8>> {
        let connection = &mut self.connection;
        let recv_future = async move {
            let packet_len = connection
                .read_u32()
                .await
                .context("reading packet length")?
                .try_into()
                .context("storing packet length")?;
            let mut packet_data = vec![0; packet_len];
            connection
                .read_exact(&mut packet_data)
                .await
                .context("reading packet data")?;
            Ok(packet_data)
        };
        let (result, stdout, stderr) = join!(
            recv_future,
            self.stdout_handler.handle_operation(),
            self.stderr_handler.handle_operation(),
        );
        stdout?;
        stderr?;
        result
    }

    async fn send(&mut self, packet_data: Vec<u8>) -> Result<()> {
        self.connection
            .write_u32(
                packet_data
                    .len()
                    .try_into()
                    .context("packet length does not fit into u32")?,
            )
            .await
            .context("writing packet length")?;
        self.connection
            .write_all(&packet_data)
            .await
            .context("writing packet data")?;
        Ok(())
    }
}

/// A pool of Node.js workers operating on [entrypoint] with specific [cwd] and
/// [env].
///
/// The pool will spawn processes when needed and reuses old ones. It will never
/// spawn more then a certain number of concurrent processes. This is specified
/// with the `concurrency` argument in the constructor.
///
/// The worker will *not* use the env of the parent process by default. All env
/// vars need to be provided to make the execution as pure as possible.
#[turbo_tasks::value(into = "new", cell = "new", serialization = "none", eq = "manual")]
pub struct NodeJsPool {
    cwd: PathBuf,
    entrypoint: PathBuf,
    env: HashMap<String, String>,
    pub assets_for_source_mapping: AssetsForSourceMappingVc,
    pub assets_root: FileSystemPathVc,
    pub project_dir: FileSystemPathVc,
    #[turbo_tasks(trace_ignore, debug_ignore)]
    processes: Arc<Mutex<Vec<NodeJsPoolProcess>>>,
    #[turbo_tasks(trace_ignore, debug_ignore)]
    semaphore: Arc<Semaphore>,
    #[turbo_tasks(trace_ignore, debug_ignore)]
    shared_stdout: SharedOutputSet,
    #[turbo_tasks(trace_ignore, debug_ignore)]
    shared_stderr: SharedOutputSet,
    debug: bool,
}

impl NodeJsPool {
    /// * debug: Whether to automatically enable Node's `--inspect-brk` when
    ///   spawning it. Note: automatically overrides concurrency to 1.
    pub(super) fn new(
        cwd: PathBuf,
        entrypoint: PathBuf,
        env: HashMap<String, String>,
        assets_for_source_mapping: AssetsForSourceMappingVc,
        assets_root: FileSystemPathVc,
        project_dir: FileSystemPathVc,
        concurrency: usize,
        debug: bool,
    ) -> Self {
        Self {
            cwd,
            entrypoint,
            env,
            assets_for_source_mapping,
            assets_root,
            project_dir,
            processes: Arc::new(Mutex::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(if debug { 1 } else { concurrency })),
            shared_stdout: Arc::new(Mutex::new(IndexSet::new())),
            shared_stderr: Arc::new(Mutex::new(IndexSet::new())),
            debug,
        }
    }

    async fn acquire_process(&self) -> Result<(NodeJsPoolProcess, OwnedSemaphorePermit)> {
        let permit = self.semaphore.clone().acquire_owned().await?;

        let popped = {
            let mut processes = self.processes.lock().unwrap();
            processes.pop()
        };
        let process = match popped {
            Some(process) => process,
            None => NodeJsPoolProcess::new(
                self.cwd.as_path(),
                &self.env,
                self.entrypoint.as_path(),
                self.assets_for_source_mapping,
                self.assets_root,
                self.project_dir,
                self.shared_stdout.clone(),
                self.shared_stderr.clone(),
                self.debug,
            )
            .await
            .context("creating new process")?,
        };
        Ok((process, permit))
    }

    pub async fn operation(&self) -> Result<NodeJsOperation> {
        let (process, permit) = self.acquire_process().await?;

        Ok(NodeJsOperation {
            process: Some(process.run().await?),
            permit,
            processes: self.processes.clone(),
            allow_process_reuse: true,
        })
    }
}

pub struct NodeJsOperation {
    process: Option<RunningNodeJsPoolProcess>,
    // This is used for drop
    #[allow(dead_code)]
    permit: OwnedSemaphorePermit,
    processes: Arc<Mutex<Vec<NodeJsPoolProcess>>>,
    allow_process_reuse: bool,
}

impl NodeJsOperation {
    async fn with_process<'a, F: Future<Output = Result<T>> + Send + 'a, T>(
        &'a mut self,
        f: impl FnOnce(&'a mut RunningNodeJsPoolProcess) -> F,
    ) -> Result<T> {
        let process = self
            .process
            .as_mut()
            .context("Node.js operation already finished")?;

        if !self.allow_process_reuse {
            bail!("Node.js process is no longer usable");
        }

        let result = f(process).await;
        if result.is_err() {
            self.allow_process_reuse = false;
        }
        result
    }

    pub async fn recv<M>(&mut self) -> Result<M>
    where
        M: DeserializeOwned,
    {
        let message = self
            .with_process(|process| async move {
                timeout(Duration::from_secs(30), process.recv())
                    .await
                    .context("timeout while receiving message from process")?
                    .context("failed to receive message")
            })
            .await?;
        serde_json::from_slice(&message).context("failed to deserialize message")
    }

    pub async fn send<M>(&mut self, message: M) -> Result<()>
    where
        M: Serialize,
    {
        let message = serde_json::to_vec(&message).context("failed to serialize message")?;
        self.with_process(|process| async move {
            timeout(Duration::from_secs(30), process.send(message))
                .await
                .context("timeout while sending message")?
                .context("failed to send message")?;
            Ok(())
        })
        .await
    }

    pub async fn wait_or_kill(mut self) -> Result<ExitStatus> {
        let mut process = self
            .process
            .take()
            .context("Node.js operation already finished")?;

        let mut child = process
            .child
            .take()
            .context("Node.js operation already finished")?;

        // Ignore error since we are not sure if the process is still alive
        let _ = child.start_kill();
        let status = timeout(Duration::from_secs(30), child.wait())
            .await
            .context("timeout while waiting for process end")?
            .context("waiting for process end")?;

        Ok(status)
    }

    pub fn disallow_reuse(&mut self) {
        self.allow_process_reuse = false;
    }

    pub async fn apply_source_mapping<'a>(
        &self,
        text: &'a str,
        ansi_colors: bool,
    ) -> Result<Cow<'a, str>> {
        if let Some(process) = self.process.as_ref() {
            process.apply_source_mapping(text, ansi_colors).await
        } else {
            Ok(Cow::Borrowed(text))
        }
    }
}

impl Drop for NodeJsOperation {
    fn drop(&mut self) {
        if self.allow_process_reuse {
            if let Some(process) = self.process.take() {
                self.processes
                    .lock()
                    .unwrap()
                    .push(NodeJsPoolProcess::Running(process));
            }
        }
    }
}
