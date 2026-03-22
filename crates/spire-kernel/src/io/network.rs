use std::cmp::min;
use std::io::Read;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{SpireError, SpireResult};

/// Rate-limit and retry configuration for outbound PDG REST requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkThrottleConfig {
    /// Target steady-state request rate in requests per second.
    pub requests_per_second: u32,
    /// Maximum burst capacity in whole requests.
    pub burst_capacity: u32,
    /// Maximum number of retries after the initial attempt.
    pub max_retries: u32,
    /// Initial retry backoff in milliseconds.
    pub base_backoff_ms: u64,
    /// Per-request timeout in milliseconds.
    pub request_timeout_ms: u64,
    /// Maximum allowed in-flight queue depth.
    pub max_queue_depth: usize,
}

impl Default for NetworkThrottleConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 5,
            burst_capacity: 2,
            max_retries: 3,
            base_backoff_ms: 250,
            request_timeout_ms: 2_500,
            max_queue_depth: 16,
        }
    }
}

#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: u32) -> Self {
        Self {
            tokens: capacity as f64,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self, cfg: &NetworkThrottleConfig) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let refill = elapsed * f64::from(cfg.requests_per_second.max(1));
        self.tokens = (self.tokens + refill).min(f64::from(cfg.burst_capacity.max(1)));
        self.last_refill = now;
    }

    fn consume_or_wait(&mut self, cfg: &NetworkThrottleConfig) -> Duration {
        self.refill(cfg);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            return Duration::from_millis(0);
        }

        let missing = 1.0 - self.tokens;
        let per_second = f64::from(cfg.requests_per_second.max(1));
        let wait = Duration::from_secs_f64(missing / per_second);
        self.tokens = 0.0;
        wait
    }
}

/// Runtime diagnostics for throttled network activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDiagnostics {
    /// Total completed request attempts (including retries).
    pub total_requests: u64,
    /// Number of attempts delayed by token-bucket throttling.
    pub throttled_requests: u64,
    /// Number of server rate-limit responses (`429`).
    pub responses_429: u64,
    /// Number of service-unavailable responses (`503`).
    pub responses_503: u64,
    /// Total retry attempts triggered by transient failures.
    pub retries: u64,
    /// Current in-flight queue depth.
    pub queue_depth: usize,
    /// Maximum observed in-flight queue depth.
    pub queue_depth_peak: usize,
    /// Most recent HTTP status code, if available.
    pub last_status_code: Option<u16>,
    /// Most recent backoff duration in milliseconds.
    pub last_backoff_ms: Option<u64>,
    /// Most recent error string, if any.
    pub last_error: Option<String>,
}

#[derive(Debug, Default)]
struct NetworkDiagnosticsState {
    total_requests: AtomicU64,
    throttled_requests: AtomicU64,
    responses_429: AtomicU64,
    responses_503: AtomicU64,
    retries: AtomicU64,
    queue_depth: AtomicUsize,
    queue_depth_peak: AtomicUsize,
    last_status_code: Mutex<Option<u16>>,
    last_backoff_ms: Mutex<Option<u64>>,
    last_error: Mutex<Option<String>>,
}

impl NetworkDiagnosticsState {
    fn snapshot(&self) -> NetworkDiagnostics {
        let last_status_code = self.last_status_code.lock().ok().and_then(|guard| *guard);
        let last_backoff_ms = self.last_backoff_ms.lock().ok().and_then(|guard| *guard);
        let last_error = self
            .last_error
            .lock()
            .ok()
            .and_then(|guard| guard.clone());

        NetworkDiagnostics {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            throttled_requests: self.throttled_requests.load(Ordering::Relaxed),
            responses_429: self.responses_429.load(Ordering::Relaxed),
            responses_503: self.responses_503.load(Ordering::Relaxed),
            retries: self.retries.load(Ordering::Relaxed),
            queue_depth: self.queue_depth.load(Ordering::Relaxed),
            queue_depth_peak: self.queue_depth_peak.load(Ordering::Relaxed),
            last_status_code,
            last_backoff_ms,
            last_error,
        }
    }

    fn set_last_error(&self, message: impl Into<String>) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = Some(message.into());
        }
    }
}

/// HTTP response handling mode for optional entity lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseMode {
    /// Expect a successful JSON payload.
    Required,
    /// Treat HTTP 404 as a miss (`Ok(None)`).
    Optional404,
}

/// Strictly throttled blocking HTTP client with retry and telemetry.
#[derive(Debug)]
pub struct ThrottledHttpClient {
    cfg: NetworkThrottleConfig,
    bucket: Mutex<TokenBucket>,
    agent: ureq::Agent,
    diagnostics: Arc<NetworkDiagnosticsState>,
}

impl ThrottledHttpClient {
    /// Construct a new throttled client.
    pub fn new(cfg: NetworkThrottleConfig) -> Self {
        let timeout = Duration::from_millis(cfg.request_timeout_ms.max(1));
        let agent = ureq::AgentBuilder::new().timeout(timeout).build();
        Self {
            bucket: Mutex::new(TokenBucket::new(cfg.burst_capacity.max(1))),
            cfg,
            agent,
            diagnostics: Arc::new(NetworkDiagnosticsState::default()),
        }
    }

    /// Fetch and deserialize JSON from `url`.
    pub fn get_json<T>(&self, url: &str) -> SpireResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.get_json_with_mode(url, ResponseMode::Required)
    }

    /// Fetch and deserialize JSON from `url`, optionally allowing HTTP 404 misses.
    pub fn get_json_with_mode<T>(&self, url: &str, mode: ResponseMode) -> SpireResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        let queue_depth = self
            .diagnostics
            .queue_depth
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);

        self.bump_peak(queue_depth);

        if queue_depth > self.cfg.max_queue_depth {
            self.diagnostics.queue_depth.fetch_sub(1, Ordering::Relaxed);
            let message = format!(
                "PDG REST queue depth {} exceeded configured limit {}",
                queue_depth, self.cfg.max_queue_depth
            );
            self.diagnostics.set_last_error(message.clone());
            return Err(SpireError::InternalError(message));
        }

        let result = self.run_request(url, mode);
        self.diagnostics.queue_depth.fetch_sub(1, Ordering::Relaxed);
        result
    }

    /// Return an immutable snapshot of network diagnostics.
    pub fn diagnostics(&self) -> NetworkDiagnostics {
        self.diagnostics.snapshot()
    }

    fn run_request<T>(&self, url: &str, mode: ResponseMode) -> SpireResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut attempt: u32 = 0;

        loop {
            let wait_for = {
                let mut guard = self.bucket.lock().map_err(|_| {
                    SpireError::InternalError("Failed to acquire network throttle lock".to_string())
                })?;
                guard.consume_or_wait(&self.cfg)
            };

            if !wait_for.is_zero() {
                self.diagnostics
                    .throttled_requests
                    .fetch_add(1, Ordering::Relaxed);
                thread::sleep(wait_for);
            }

            self.diagnostics
                .total_requests
                .fetch_add(1, Ordering::Relaxed);

            match self.agent.get(url).call() {
                Ok(response) => {
                    if let Ok(mut guard) = self.diagnostics.last_status_code.lock() {
                        *guard = Some(response.status());
                    }
                    if let Ok(mut guard) = self.diagnostics.last_error.lock() {
                        *guard = None;
                    }

                    let mut body = String::new();
                    response
                        .into_reader()
                        .read_to_string(&mut body)
                        .map_err(|err| {
                            SpireError::DataParseError(format!(
                                "Failed to read PDG REST response body: {}",
                                err
                            ))
                        })?;

                    let decoded: T = serde_json::from_str(&body).map_err(|err| {
                        SpireError::DataParseError(format!(
                            "Failed to decode PDG REST response JSON: {}",
                            err
                        ))
                    })?;
                    return Ok(Some(decoded));
                }
                Err(ureq::Error::Status(code, response)) => {
                    if let Ok(mut guard) = self.diagnostics.last_status_code.lock() {
                        *guard = Some(code);
                    }

                    if mode == ResponseMode::Optional404 && code == 404 {
                        return Ok(None);
                    }

                    if code == 429 {
                        self.diagnostics.responses_429.fetch_add(1, Ordering::Relaxed);
                    } else if code == 503 {
                        self.diagnostics.responses_503.fetch_add(1, Ordering::Relaxed);
                    }

                    let should_retry = (code == 429 || code == 503) && attempt < self.cfg.max_retries;
                    if should_retry {
                        let backoff = self.compute_backoff(attempt);
                        self.diagnostics.retries.fetch_add(1, Ordering::Relaxed);
                        if let Ok(mut guard) = self.diagnostics.last_backoff_ms.lock() {
                            *guard = Some(backoff.as_millis() as u64);
                        }
                        attempt += 1;
                        thread::sleep(backoff);
                        continue;
                    }

                    let status_text = response.status_text().to_string();
                    let message = format!(
                        "PDG REST request failed with HTTP {} {}",
                        code, status_text
                    );
                    self.diagnostics.set_last_error(message.clone());
                    return Err(SpireError::DatabaseError(message));
                }
                Err(ureq::Error::Transport(err)) => {
                    let is_timeout = err.to_string().to_ascii_lowercase().contains("timed out");
                    let should_retry = attempt < self.cfg.max_retries;
                    if should_retry {
                        let backoff = self.compute_backoff(attempt);
                        self.diagnostics.retries.fetch_add(1, Ordering::Relaxed);
                        if let Ok(mut guard) = self.diagnostics.last_backoff_ms.lock() {
                            *guard = Some(backoff.as_millis() as u64);
                        }
                        attempt += 1;
                        thread::sleep(backoff);
                        continue;
                    }

                    let message = if is_timeout {
                        format!(
                            "PDG REST API timeout after {} ms",
                            self.cfg.request_timeout_ms
                        )
                    } else {
                        format!("PDG REST transport error: {}", err)
                    };
                    self.diagnostics.set_last_error(message.clone());
                    return Err(SpireError::InternalError(message));
                }
            }
        }
    }

    fn compute_backoff(&self, attempt: u32) -> Duration {
        let exponent = min(attempt, 6);
        let multiplier = 2u64.saturating_pow(exponent);
        Duration::from_millis(self.cfg.base_backoff_ms.saturating_mul(multiplier))
    }

    fn bump_peak(&self, candidate: usize) {
        let mut current = self.diagnostics.queue_depth_peak.load(Ordering::Relaxed);
        while candidate > current {
            match self.diagnostics.queue_depth_peak.compare_exchange_weak(
                current,
                candidate,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
    }
}
