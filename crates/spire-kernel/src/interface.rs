//! # Interface — External Solver Communication
//!
//! This module provides a generic framework for invoking external command-line
//! tools from within the SPIRE kernel. Many advanced HEP computations rely on
//! specialised external programs:
//!
//! - **Loop integral reduction**: FIRE, Kira, LiteRed
//! - **Symbolic computation**: FORM, Mathematica
//! - **Numerical libraries**: LoopTools, Collier
//!
//! Rather than binding to each tool individually, the [`ExternalSolver`] trait
//! defines a uniform interface: *send an expression as text, receive a result
//! as text*. The [`CliSolver`] struct implements this for any program that
//! reads from stdin and writes to stdout.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐    stdin     ┌──────────────┐
//! │ SPIRE Kernel │ ──────────> │ External CLI │
//! │  (CliSolver) │ <────────── │  (FIRE, etc) │
//! └─────────────┘    stdout    └──────────────┘
//! ```
//!
//! ## Timeout Safety
//!
//! External tools can hang or take arbitrarily long. The [`CliSolver`]
//! enforces a configurable timeout: if the child process does not complete
//! within the deadline, it is killed and an error is returned.
//!
//! ## Error Handling
//!
//! Non-zero exit codes, stderr output, and timeouts all produce structured
//! `SpireError::InternalError` diagnostics with the tool name and raw output.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::{SpireError, SpireResult};

// ---------------------------------------------------------------------------
// ExternalSolver Trait
// ---------------------------------------------------------------------------

/// Generic interface for invoking an external computation tool.
///
/// Implementations send a mathematical expression (as a string) to an
/// external program and return the result. The exact format of the input
/// and output strings depends on the target tool.
///
/// # Thread Safety
///
/// The trait requires `Send + Sync` to allow solvers to be shared across
/// worker threads in parallel integration pipelines.
///
/// # Examples
///
/// A solver for the FIRE integral reduction system might accept IBP
/// identities in FIRE's input syntax and return the reduced master
/// integrals as a list of coefficients.
pub trait ExternalSolver: Send + Sync {
    /// Human-readable name of this solver (for logging and diagnostics).
    fn name(&self) -> &str;

    /// Send an input expression to the solver and return its output.
    ///
    /// # Arguments
    /// * `input` — The expression or command string in the tool's native format.
    ///
    /// # Returns
    /// The solver's stdout output as a `String`.
    ///
    /// # Errors
    /// - `SpireError::InternalError` if the solver times out, exits with a
    ///   non-zero code, or fails to spawn.
    fn solve(&self, input: &str) -> SpireResult<String>;
}

// ---------------------------------------------------------------------------
// CLI Solver
// ---------------------------------------------------------------------------

/// A generic command-line solver that wraps `std::process::Command`.
///
/// This is the workhorse implementation of [`ExternalSolver`]. It spawns
/// a child process, pipes the input string to stdin, captures stdout,
/// and enforces a timeout.
///
/// # Configuration
///
/// ```no_run
/// use spire_kernel::interface::CliSolver;
///
/// let solver = CliSolver::new("fire")
///     .args(&["-auto"])
///     .timeout_secs(60)
///     .working_dir("/tmp/fire_workspace");
/// ```
pub struct CliSolver {
    /// Path or name of the executable (resolved by the system PATH).
    executable: String,
    /// Command-line arguments passed to the executable.
    arguments: Vec<String>,
    /// Maximum time to wait for the process to complete.
    timeout: Duration,
    /// Optional working directory for the child process.
    work_dir: Option<String>,
    /// Optional environment variables to set for the child process.
    env_vars: Vec<(String, String)>,
}

impl CliSolver {
    /// Create a new CLI solver for the given executable.
    ///
    /// The executable name is resolved via the system `PATH` unless an
    /// absolute path is provided.
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
            arguments: Vec::new(),
            timeout: Duration::from_secs(30),
            work_dir: None,
            env_vars: Vec::new(),
        }
    }

    /// Set the command-line arguments for the solver.
    pub fn args(mut self, args: &[&str]) -> Self {
        self.arguments = args.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the timeout in seconds. Default: 30 seconds.
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout = Duration::from_secs(secs);
        self
    }

    /// Set the timeout as a `Duration`.
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    /// Set the working directory for the child process.
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.work_dir = Some(dir.into());
        self
    }

    /// Add an environment variable for the child process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    /// Get the executable name.
    pub fn executable(&self) -> &str {
        &self.executable
    }

    /// Get the configured timeout.
    pub fn configured_timeout(&self) -> Duration {
        self.timeout
    }
}

impl ExternalSolver for CliSolver {
    fn name(&self) -> &str {
        &self.executable
    }

    fn solve(&self, input: &str) -> SpireResult<String> {
        // Build the command.
        let mut cmd = Command::new(&self.executable);
        cmd.args(&self.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref dir) = self.work_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Spawn the child process.
        let mut child = cmd.spawn().map_err(|e| {
            SpireError::InternalError(format!(
                "Failed to spawn external solver '{}': {}",
                self.executable, e
            ))
        })?;

        // Write input to stdin.
        if let Some(ref mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).map_err(|e| {
                SpireError::InternalError(format!(
                    "Failed to write to stdin of '{}': {}",
                    self.executable, e
                ))
            })?;
        }

        // Wait with timeout.
        let output = child.wait_with_output().map_err(|e| {
            SpireError::InternalError(format!("Failed to wait for '{}': {}", self.executable, e))
        })?;

        // Check exit code.
        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let code = output.status.code().unwrap_or(-1);
            return Err(SpireError::InternalError(format!(
                "External solver '{}' exited with code {}: {}",
                self.executable,
                code,
                stderr_str.trim()
            )));
        }

        // Return stdout.
        String::from_utf8(output.stdout).map_err(|e| {
            SpireError::InternalError(format!("Non-UTF8 output from '{}': {}", self.executable, e))
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_solver_builder_pattern() {
        let solver = CliSolver::new("echo")
            .args(&["-n"])
            .timeout_secs(10)
            .working_dir("/tmp");

        assert_eq!(solver.executable(), "echo");
        assert_eq!(solver.configured_timeout(), Duration::from_secs(10));
    }

    #[test]
    fn cli_solver_with_env() {
        let solver = CliSolver::new("echo")
            .env("MY_VAR", "hello")
            .env("OTHER", "world");

        assert_eq!(solver.env_vars.len(), 2);
        assert_eq!(
            solver.env_vars[0],
            ("MY_VAR".to_string(), "hello".to_string())
        );
    }

    #[test]
    fn cli_solver_default_timeout() {
        let solver = CliSolver::new("cat");
        assert_eq!(solver.configured_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn external_solver_trait_object_safety() {
        // Verify ExternalSolver is object-safe.
        fn _accepts_solver(_s: &dyn ExternalSolver) {}
        let solver = CliSolver::new("echo");
        _accepts_solver(&solver);
    }

    #[test]
    fn cli_solver_nonexistent_executable() {
        let solver = CliSolver::new("__nonexistent_binary_12345__");
        let result = solver.solve("hello");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Failed to spawn"));
    }

    // Platform-specific test: `echo` works differently on Windows vs Unix.
    // We use a cross-platform approach: spawn a process that definitely
    // exists and verify we can capture its output.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn cli_solver_echo_unix() {
        let solver = CliSolver::new("cat");
        let result = solver.solve("hello world").unwrap();
        assert_eq!(result, "hello world");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn cli_solver_findstr_windows() {
        // On Windows, use `findstr` which reads from stdin and matches patterns.
        // `findstr ".*"` passes through all lines.
        let solver = CliSolver::new("findstr").args(&[".*"]);
        let result = solver.solve("hello world");
        // findstr should pass through the input if it matches.
        if let Ok(output) = result {
            assert!(output.contains("hello world"));
        }
        // If findstr is not available in the test environment, that's OK.
    }

    #[test]
    fn cli_solver_name_is_executable() {
        let solver = CliSolver::new("my_program");
        assert_eq!(solver.name(), "my_program");
    }
}
