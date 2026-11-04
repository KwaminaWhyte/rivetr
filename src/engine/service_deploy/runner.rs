//! Command execution for the service deploy pipeline.
//!
//! Compose services are driven through the `docker` and `git` CLIs (the same
//! way the start/stop handlers do it). Every external command the pipeline runs
//! goes through [`CommandRunner`], so tests swap in a recording mock and check
//! which commands ran, in which order, without a Docker daemon.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

/// Sink for deployment log lines (stored in `service_deployment_logs`).
#[async_trait]
pub trait DeployLog: Send + Sync {
    async fn line(&self, level: &str, message: &str);

    async fn info(&self, message: &str) {
        self.line("info", message).await
    }
    async fn warn(&self, message: &str) {
        self.line("warn", message).await
    }
    async fn error(&self, message: &str) {
        self.line("error", message).await
    }
}

/// One external command.
#[derive(Debug, Clone, Default)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    /// Written to the child's stdin, then stdin is closed.
    pub stdin: Option<String>,
    pub timeout: Option<Duration>,
    /// Forward each output line to the deploy log while the command runs.
    pub stream: bool,
}

impl CommandSpec {
    pub fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.to_string(),
            args: args.iter().map(|a| a.to_string()).collect(),
            ..Default::default()
        }
    }

    pub fn args<I, S>(program: &str, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            program: program.to_string(),
            args: args.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }

    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_string(), value.to_string()));
        self
    }

    pub fn stdin(mut self, input: String) -> Self {
        self.stdin = Some(input);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn streamed(mut self) -> Self {
        self.stream = true;
        self
    }

    /// The command line as one string (for logs and test assertions).
    pub fn display(&self) -> String {
        let mut out = self.program.clone();
        for a in &self.args {
            out.push(' ');
            out.push_str(a);
        }
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct CommandOutput {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn ok(stdout: &str) -> Self {
        Self {
            success: true,
            code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    pub fn failed(code: i32, stderr: &str) -> Self {
        Self {
            success: false,
            code: Some(code),
            stdout: String::new(),
            stderr: stderr.to_string(),
        }
    }

    /// Last few non-empty lines of stderr (or stdout), for error messages.
    pub fn error_tail(&self) -> String {
        let src = if self.stderr.trim().is_empty() {
            &self.stdout
        } else {
            &self.stderr
        };
        let lines: Vec<&str> = src.lines().filter(|l| !l.trim().is_empty()).collect();
        let start = lines.len().saturating_sub(5);
        let tail = lines[start..].join("\n");
        if tail.is_empty() {
            match self.code {
                Some(c) => format!("exited with status {}", c),
                None => "terminated by a signal".to_string(),
            }
        } else {
            tail
        }
    }
}

#[async_trait]
pub trait CommandRunner: Send + Sync {
    /// Run a command to completion. `Err` means it could not be started or
    /// timed out; a non-zero exit is reported through `CommandOutput::success`.
    async fn run(&self, spec: &CommandSpec, log: Option<&dyn DeployLog>) -> Result<CommandOutput>;
}

/// Runs real processes.
#[derive(Debug, Default, Clone, Copy)]
pub struct CliRunner;

#[async_trait]
impl CommandRunner for CliRunner {
    async fn run(&self, spec: &CommandSpec, log: Option<&dyn DeployLog>) -> Result<CommandOutput> {
        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let mut cmd = tokio::process::Command::new(&spec.program);
        cmd.args(&spec.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(if spec.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .kill_on_drop(true);
        if let Some(dir) = &spec.cwd {
            cmd.current_dir(dir);
        }
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to run {}", spec.program))?;

        if let (Some(input), Some(mut stdin)) = (spec.stdin.clone(), child.stdin.take()) {
            stdin.write_all(input.as_bytes()).await.ok();
            drop(stdin);
        }

        let stdout = child.stdout.take().context("no stdout")?;
        let stderr = child.stderr.take().context("no stderr")?;
        let stream = spec.stream;

        let work = async move {
            let mut out_lines = BufReader::new(stdout).lines();
            let mut err_lines = BufReader::new(stderr).lines();
            let mut out = String::new();
            let mut err = String::new();
            let (mut out_done, mut err_done) = (false, false);
            while !(out_done && err_done) {
                tokio::select! {
                    line = out_lines.next_line(), if !out_done => match line {
                        Ok(Some(l)) => {
                            if stream && !l.trim().is_empty() {
                                if let Some(log) = log { log.info(&l).await; }
                            }
                            out.push_str(&l);
                            out.push('\n');
                        }
                        _ => out_done = true,
                    },
                    line = err_lines.next_line(), if !err_done => match line {
                        Ok(Some(l)) => {
                            if stream && !l.trim().is_empty() {
                                if let Some(log) = log { log.info(&l).await; }
                            }
                            err.push_str(&l);
                            err.push('\n');
                        }
                        _ => err_done = true,
                    },
                }
            }
            let status = child.wait().await?;
            Ok::<_, std::io::Error>(CommandOutput {
                success: status.success(),
                code: status.code(),
                stdout: out,
                stderr: err,
            })
        };

        match spec.timeout {
            Some(t) => match tokio::time::timeout(t, work).await {
                Ok(r) => Ok(r?),
                Err(_) => anyhow::bail!("{} timed out after {}s", spec.program, t.as_secs()),
            },
            None => Ok(work.await?),
        }
    }
}

/// Test double: records every command and answers from a list of rules.
#[cfg(test)]
pub mod mock {
    use super::*;
    use parking_lot::Mutex;

    type Responder = Box<dyn Fn(&CommandSpec) -> Option<CommandOutput> + Send + Sync>;

    #[derive(Default)]
    pub struct MockRunner {
        pub calls: Mutex<Vec<CommandSpec>>,
        rules: Mutex<Vec<(String, Responder)>>,
    }

    impl MockRunner {
        pub fn new() -> Self {
            Self::default()
        }

        /// Answer commands whose display string contains `needle`. Later rules win.
        pub fn on(&self, needle: &str, output: CommandOutput) {
            self.rules
                .lock()
                .push((needle.to_string(), Box::new(move |_| Some(output.clone()))));
        }

        pub fn on_fn<F>(&self, needle: &str, f: F)
        where
            F: Fn(&CommandSpec) -> Option<CommandOutput> + Send + Sync + 'static,
        {
            self.rules.lock().push((needle.to_string(), Box::new(f)));
        }

        pub fn commands(&self) -> Vec<String> {
            self.calls.lock().iter().map(|c| c.display()).collect()
        }

        pub fn position(&self, needle: &str) -> Option<usize> {
            self.commands().iter().position(|c| c.contains(needle))
        }

        pub fn ran(&self, needle: &str) -> bool {
            self.position(needle).is_some()
        }
    }

    #[async_trait]
    impl CommandRunner for MockRunner {
        async fn run(
            &self,
            spec: &CommandSpec,
            log: Option<&dyn DeployLog>,
        ) -> Result<CommandOutput> {
            self.calls.lock().push(spec.clone());
            let display = spec.display();
            let answer = {
                let rules = self.rules.lock();
                rules
                    .iter()
                    .rev()
                    .filter(|(needle, _)| display.contains(needle.as_str()))
                    .find_map(|(_, f)| f(spec))
            };
            let out = answer.unwrap_or_else(|| CommandOutput::ok(""));
            if spec.stream {
                if let Some(log) = log {
                    for l in out.stdout.lines().chain(out.stderr.lines()) {
                        log.info(l).await;
                    }
                }
            }
            Ok(out)
        }
    }
}
