use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use crate::core_model::json_string;

pub const DEFAULT_CHAT_MAX_TOKENS: usize = 4096;
const DEFAULT_COMPLETION_MAX_TOKENS: usize = 2048;

pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(300);
pub const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_TOTAL_TIMEOUT: Duration = Duration::from_secs(900);
pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;
pub const DEFAULT_RETRY_BACKOFF: Duration = Duration::from_millis(500);

const MAX_RETRY_BACKOFF: Duration = Duration::from_secs(30);
const MAX_CONFIGURABLE_TIMEOUT: Duration = Duration::from_secs(86_400);
const MAX_RESPONSE_BODY_BYTES: usize = 32 * 1024 * 1024;
const MAX_JSON_DEPTH: usize = 64;
const RESPONSE_SNIPPET_CHARS: usize = 400;
const READ_CHUNK_BYTES: usize = 16 * 1024;

/// Transport and sampling knobs for a single model call.
///
/// Defaults stay bit-deterministic (`temperature` 0.0, no `seed`) so repeated
/// runs reproduce, but every field is overridable so an unattended loop can
/// resample an artifact that failed to generate instead of replaying the exact
/// same deterministic failure.
#[derive(Clone, Debug)]
pub struct LlmRequestOptions {
    pub max_tokens: Option<usize>,
    pub temperature: f64,
    pub seed: Option<i64>,
    pub model: Option<String>,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub total_timeout: Duration,
    pub max_attempts: u32,
    pub retry_backoff: Duration,
}

impl Default for LlmRequestOptions {
    fn default() -> Self {
        Self {
            max_tokens: None,
            temperature: 0.0,
            seed: None,
            model: None,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            read_timeout: DEFAULT_READ_TIMEOUT,
            write_timeout: DEFAULT_WRITE_TIMEOUT,
            total_timeout: DEFAULT_TOTAL_TIMEOUT,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            retry_backoff: DEFAULT_RETRY_BACKOFF,
        }
    }
}

impl LlmRequestOptions {
    /// Build options from the process environment so an unattended runner can
    /// retune timeouts and sampling without a rebuild. Every override is
    /// validated; a malformed value is an error rather than a silent fallback.
    pub fn from_env() -> Result<Self, String> {
        let mut options = Self::default();
        if let Some(value) = env_override("AIL_LLM_MAX_TOKENS") {
            options.max_tokens = Some(parse_env_usize("AIL_LLM_MAX_TOKENS", &value)?);
        }
        if let Some(value) = env_override("AIL_LLM_TEMPERATURE") {
            options.temperature = parse_env_f64("AIL_LLM_TEMPERATURE", &value)?;
        }
        if let Some(value) = env_override("AIL_LLM_SEED") {
            options.seed = Some(parse_env_i64("AIL_LLM_SEED", &value)?);
        }
        if let Some(value) = env_override("AIL_LLM_MODEL") {
            options.model = Some(value);
        }
        if let Some(value) = env_override("AIL_LLM_CONNECT_TIMEOUT_SECS") {
            options.connect_timeout = parse_env_secs("AIL_LLM_CONNECT_TIMEOUT_SECS", &value)?;
        }
        if let Some(value) = env_override("AIL_LLM_READ_TIMEOUT_SECS") {
            options.read_timeout = parse_env_secs("AIL_LLM_READ_TIMEOUT_SECS", &value)?;
        }
        if let Some(value) = env_override("AIL_LLM_WRITE_TIMEOUT_SECS") {
            options.write_timeout = parse_env_secs("AIL_LLM_WRITE_TIMEOUT_SECS", &value)?;
        }
        if let Some(value) = env_override("AIL_LLM_TOTAL_TIMEOUT_SECS") {
            options.total_timeout = parse_env_secs("AIL_LLM_TOTAL_TIMEOUT_SECS", &value)?;
        }
        if let Some(value) = env_override("AIL_LLM_MAX_ATTEMPTS") {
            options.max_attempts = parse_env_u32("AIL_LLM_MAX_ATTEMPTS", &value)?;
        }
        if let Some(value) = env_override("AIL_LLM_RETRY_BACKOFF_MS") {
            options.retry_backoff = parse_env_millis("AIL_LLM_RETRY_BACKOFF_MS", &value)?;
        }
        options.validate()?;
        Ok(options)
    }

    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    #[must_use]
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    #[must_use]
    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    #[must_use]
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(format!(
                "llm temperature must be between 0.0 and 2.0, got {}",
                self.temperature
            ));
        }
        if self.max_attempts == 0 {
            return Err("llm max_attempts must be at least 1".to_string());
        }
        for (label, duration) in [
            ("connect_timeout", self.connect_timeout),
            ("read_timeout", self.read_timeout),
            ("write_timeout", self.write_timeout),
            ("total_timeout", self.total_timeout),
        ] {
            if duration.is_zero() {
                return Err(format!("llm {label} must be greater than zero"));
            }
            if duration > MAX_CONFIGURABLE_TIMEOUT {
                return Err(format!(
                    "llm {label} must be at most {}",
                    format_duration(MAX_CONFIGURABLE_TIMEOUT)
                ));
            }
        }
        Ok(())
    }
}

pub fn invoke_llm_text(endpoint: &str, prompt: &str) -> Result<String, String> {
    invoke_llm_text_with_options(endpoint, prompt, &LlmRequestOptions::from_env()?)
}

pub fn invoke_llm_text_with_options(
    endpoint: &str,
    prompt: &str,
    options: &LlmRequestOptions,
) -> Result<String, String> {
    unwrap_artifact(invoke_llm_artifact_response_without_expectation(
        endpoint, prompt, options,
    )?)
}

pub fn invoke_llm_text_for_artifact(
    endpoint: &str,
    prompt: &str,
    expected_artifact_kind: &str,
    expected_profile: &str,
) -> Result<String, String> {
    unwrap_artifact(invoke_llm_artifact_response(
        endpoint,
        prompt,
        expected_artifact_kind,
        expected_profile,
    )?)
}

pub enum LlmArtifactResponse {
    Artifact(String),
    Questions(Vec<String>),
}

pub struct LlmRecordedArtifactResponse {
    pub outcome: LlmArtifactResponse,
    pub request_body: String,
    pub response_body: String,
    pub content_text: String,
    pub content_kind: String,
}

pub fn invoke_llm_artifact_response(
    endpoint: &str,
    prompt: &str,
    expected_artifact_kind: &str,
    expected_profile: &str,
) -> Result<LlmArtifactResponse, String> {
    Ok(invoke_llm_artifact_response_recorded(
        endpoint,
        prompt,
        expected_artifact_kind,
        expected_profile,
    )?
    .outcome)
}

pub fn invoke_llm_artifact_response_recorded(
    endpoint: &str,
    prompt: &str,
    expected_artifact_kind: &str,
    expected_profile: &str,
) -> Result<LlmRecordedArtifactResponse, String> {
    invoke_llm_artifact_response_recorded_with_options(
        endpoint,
        prompt,
        expected_artifact_kind,
        expected_profile,
        &LlmRequestOptions::from_env()?,
    )
}

pub fn invoke_llm_artifact_response_recorded_with_max_tokens(
    endpoint: &str,
    prompt: &str,
    expected_artifact_kind: &str,
    expected_profile: &str,
    max_tokens: usize,
) -> Result<LlmRecordedArtifactResponse, String> {
    invoke_llm_artifact_response_recorded_with_options(
        endpoint,
        prompt,
        expected_artifact_kind,
        expected_profile,
        &LlmRequestOptions::from_env()?.with_max_tokens(max_tokens),
    )
}

pub fn invoke_llm_artifact_response_recorded_with_options(
    endpoint: &str,
    prompt: &str,
    expected_artifact_kind: &str,
    expected_profile: &str,
    options: &LlmRequestOptions,
) -> Result<LlmRecordedArtifactResponse, String> {
    let expectation = PromptEnvelopeExpectation {
        artifact_kind: expected_artifact_kind,
        expected_profile,
    };
    let completion = invoke_completion(endpoint, prompt, options)?;
    let text = sanitize_model_text(&completion.content);
    let (outcome, content_kind) = artifact_response_from_text(&text, Some(&expectation))?;
    Ok(LlmRecordedArtifactResponse {
        outcome,
        request_body: completion.request_body,
        response_body: completion.response_body,
        content_text: text,
        content_kind,
    })
}

fn unwrap_artifact(response: LlmArtifactResponse) -> Result<String, String> {
    match response {
        LlmArtifactResponse::Artifact(artifact_text) => Ok(artifact_text),
        LlmArtifactResponse::Questions(questions) => Err(format!(
            "model returned blocking questions:\n- {}",
            questions.join("\n- ")
        )),
    }
}

fn artifact_response_from_text(
    text: &str,
    expectation: Option<&PromptEnvelopeExpectation<'_>>,
) -> Result<(LlmArtifactResponse, String), String> {
    match extract_prompt_envelope(text, expectation) {
        Some(PromptEnvelope::Artifact(artifact_text)) => Ok((
            LlmArtifactResponse::Artifact(artifact_text),
            "prompt-envelope-artifact".to_string(),
        )),
        Some(PromptEnvelope::Questions(questions)) => Ok((
            LlmArtifactResponse::Questions(questions),
            "prompt-envelope-questions".to_string(),
        )),
        Some(PromptEnvelope::Invalid(message)) => Err(message),
        None => Ok((
            LlmArtifactResponse::Artifact(text.to_string()),
            "raw-artifact".to_string(),
        )),
    }
}

fn invoke_llm_artifact_response_without_expectation(
    endpoint: &str,
    prompt: &str,
    options: &LlmRequestOptions,
) -> Result<LlmArtifactResponse, String> {
    let completion = invoke_completion(endpoint, prompt, options)?;
    let text = sanitize_model_text(&completion.content);
    artifact_response_from_text(&text, None).map(|(outcome, _kind)| outcome)
}

struct LlmHttpCompletion {
    request_body: String,
    response_body: String,
    content: String,
}

fn invoke_completion(
    endpoint: &str,
    prompt: &str,
    options: &LlmRequestOptions,
) -> Result<LlmHttpCompletion, String> {
    options.validate()?;
    let parsed = parse_http_endpoint(endpoint)?;
    let chat = is_chat_completion_path(&parsed.path);
    let max_tokens = options.max_tokens.unwrap_or(if chat {
        DEFAULT_CHAT_MAX_TOKENS
    } else {
        DEFAULT_COMPLETION_MAX_TOKENS
    });
    let body = if chat {
        chat_completion_body(prompt, max_tokens, options)
    } else {
        text_completion_body(prompt, max_tokens, options)
    };
    let request = http_request(&parsed, &body);
    let mut attempt = 0;
    loop {
        attempt += 1;
        match attempt_completion(&parsed, &request, chat, options) {
            Ok((response_body, content)) => {
                return Ok(LlmHttpCompletion {
                    request_body: body,
                    response_body,
                    content,
                });
            }
            Err(failure) => {
                if failure.retryable && attempt < options.max_attempts {
                    std::thread::sleep(retry_delay(options, attempt));
                    continue;
                }
                return Err(failure.into_message(&parsed, attempt));
            }
        }
    }
}

fn retry_delay(options: &LlmRequestOptions, attempt: u32) -> Duration {
    let factor = 1u32 << attempt.saturating_sub(1).min(16);
    options
        .retry_backoff
        .checked_mul(factor)
        .unwrap_or(MAX_RETRY_BACKOFF)
        .min(MAX_RETRY_BACKOFF)
}

fn attempt_completion(
    endpoint: &HttpEndpoint,
    request: &str,
    chat: bool,
    options: &LlmRequestOptions,
) -> Result<(String, String), LlmFailure> {
    let deadline = Instant::now() + options.total_timeout;
    let mut stream = connect_stream(endpoint, options)?;
    let write_context = format!(
        "failed to send request to {} (write timeout {})",
        endpoint.authority(),
        format_duration(options.write_timeout)
    );
    stream
        .set_write_timeout(Some(options.write_timeout))
        .map_err(|error| LlmFailure::io("write", &write_context, &error))?;
    stream
        .write_all(request.as_bytes())
        .and_then(|()| stream.flush())
        .map_err(|error| LlmFailure::io("write", &write_context, &error))?;
    let read_context = format!(
        "failed to read response from {} (read timeout {}, total budget {})",
        endpoint.authority(),
        format_duration(options.read_timeout),
        format_duration(options.total_timeout)
    );
    let response = read_http_response(&mut stream, options.read_timeout, deadline)
        .map_err(|error| LlmFailure::io("read", &read_context, &error))?;
    if response.status != 200 {
        return Err(http_status_failure(&response));
    }
    let content = completion_content(&response.body, chat)?;
    Ok((response.body, content))
}

fn connect_stream(
    endpoint: &HttpEndpoint,
    options: &LlmRequestOptions,
) -> Result<TcpStream, LlmFailure> {
    let resolve_context = format!("failed to resolve {}", endpoint.authority());
    let addresses = (endpoint.host.as_str(), endpoint.port)
        .to_socket_addrs()
        .map_err(|error| LlmFailure::io("connect", &resolve_context, &error))?;
    let mut last_failure = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, options.connect_timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) => {
                let context = format!(
                    "failed to connect to {address} within {}",
                    format_duration(options.connect_timeout)
                );
                last_failure = Some(LlmFailure::io("connect", &context, &error));
            }
        }
    }
    Err(last_failure.unwrap_or_else(|| {
        LlmFailure::retryable(
            "connect",
            format!("{} resolved to no socket addresses", endpoint.authority()),
        )
    }))
}

fn http_status_failure(response: &HttpResponse) -> LlmFailure {
    let detail = match json_error_message(&response.body) {
        Some(message) => format!(
            "HTTP {} {}: {message}",
            response.status,
            response.reason.trim()
        ),
        None => format!(
            "HTTP {} {}: {}",
            response.status,
            response.reason.trim(),
            response_snippet(&response.body)
        ),
    };
    // 5xx/408/425/429 are transient server-side conditions; every other 4xx is a
    // request defect that will fail identically on every retry.
    let retryable = response.status >= 500 || matches!(response.status, 408 | 425 | 429);
    LlmFailure {
        kind: format!("http-{}", response.status),
        detail,
        retryable,
    }
}

fn completion_content(body: &str, chat: bool) -> Result<String, LlmFailure> {
    let Some(root) = parse_json_document(body) else {
        return Err(LlmFailure::terminal(
            "protocol",
            format!(
                "response body is not valid JSON: {}",
                response_snippet(body)
            ),
        ));
    };
    if let Some(message) = json_error_message_of(&root) {
        return Err(LlmFailure::terminal(
            "model-error",
            format!("model returned an error envelope instead of a completion: {message}"),
        ));
    }
    let (content, reasoning, finish_reason) = if chat {
        let Some(choice) = root.field("choices").and_then(|choices| choices.entry(0)) else {
            return Err(LlmFailure::terminal(
                "protocol",
                format!(
                    "chat response has no choices[0] entry: {}",
                    response_snippet(body)
                ),
            ));
        };
        let message = choice.field("message");
        let content = message
            .and_then(|message| message.field("content"))
            .and_then(JsonValue::as_str)
            .or_else(|| choice.field("text").and_then(JsonValue::as_str));
        let Some(content) = content else {
            return Err(LlmFailure::terminal(
                "protocol",
                format!(
                    "chat response choices[0].message.content is missing or not a string: {}",
                    response_snippet(body)
                ),
            ));
        };
        let reasoning = message
            .and_then(|message| message.field("reasoning_content"))
            .and_then(JsonValue::as_str)
            .unwrap_or_default();
        let finish_reason = choice
            .field("finish_reason")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        (content, reasoning, finish_reason)
    } else {
        let Some(content) = root.field("content").and_then(JsonValue::as_str) else {
            return Err(LlmFailure::terminal(
                "protocol",
                format!(
                    "completion response has no string content field: {}",
                    response_snippet(body)
                ),
            ));
        };
        let finish_reason = root
            .field("stop_type")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        (content, "", finish_reason)
    };
    if content.trim().is_empty() {
        return Err(LlmFailure::terminal(
            "empty-content",
            empty_content_detail(reasoning, finish_reason),
        ));
    }
    Ok(content.to_string())
}

fn empty_content_detail(reasoning: &str, finish_reason: &str) -> String {
    let mut detail =
        format!("model returned an empty assistant message (finish_reason={finish_reason})");
    if reasoning.trim().is_empty() {
        detail.push_str("; no reasoning_content was returned either, so the generation produced nothing usable. Resample with a different seed or temperature");
    } else {
        detail.push_str(&format!(
            "; the model spent the budget on {} characters of reasoning_content instead, so it ignored chat_template_kwargs.enable_thinking=false. Raise max_tokens or resample with a different seed or temperature",
            reasoning.chars().count()
        ));
    }
    detail
}

fn json_error_message(body: &str) -> Option<String> {
    json_error_message_of(&parse_json_document(body)?)
}

fn json_error_message_of(root: &JsonValue) -> Option<String> {
    let error = root.field("error")?;
    if let Some(message) = error.as_str() {
        return Some(message.to_string());
    }
    let message = error.field("message").and_then(JsonValue::as_str)?;
    let code = error
        .field("type")
        .and_then(JsonValue::as_str)
        .or_else(|| error.field("code").and_then(JsonValue::as_str));
    Some(match code {
        Some(code) => format!("{message} (type={code})"),
        None => message.to_string(),
    })
}

fn response_snippet(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "<empty body>".to_string();
    }
    let mut snippet: String = trimmed
        .chars()
        .take(RESPONSE_SNIPPET_CHARS)
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    if trimmed.chars().nth(RESPONSE_SNIPPET_CHARS).is_some() {
        snippet.push_str("...");
    }
    snippet
}

#[derive(Debug)]
struct LlmFailure {
    kind: String,
    detail: String,
    retryable: bool,
}

impl LlmFailure {
    fn retryable(kind: &str, detail: String) -> Self {
        Self {
            kind: kind.to_string(),
            detail,
            retryable: true,
        }
    }

    fn terminal(kind: &str, detail: String) -> Self {
        Self {
            kind: kind.to_string(),
            detail,
            retryable: false,
        }
    }

    /// Classify an I/O failure. Socket-level problems are transient by nature and
    /// worth retrying; `InvalidData` is raised only by our own HTTP framing
    /// checks, which means the peer speaks something we cannot parse and a retry
    /// would fail identically.
    fn io(kind: &str, context: &str, error: &io::Error) -> Self {
        let timed_out = matches!(
            error.kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        );
        let detail = if timed_out {
            format!("{context}: timed out ({error})")
        } else {
            format!("{context}: {error}")
        };
        Self {
            kind: if timed_out { "timeout" } else { kind }.to_string(),
            detail,
            retryable: error.kind() != io::ErrorKind::InvalidData,
        }
    }

    fn into_message(self, endpoint: &HttpEndpoint, attempts: u32) -> String {
        format!(
            "llm request to {} failed after {} attempt(s) [{}]: {}",
            endpoint.url(),
            attempts,
            self.kind,
            self.detail
        )
    }
}

fn http_request(endpoint: &HttpEndpoint, body: &str) -> String {
    format!(
        concat!(
            "POST {} HTTP/1.1\r\n",
            "Host: {}\r\n",
            "User-Agent: ail/{}\r\n",
            "Accept: application/json\r\n",
            "Content-Type: application/json\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        endpoint.path,
        endpoint.authority(),
        env!("CARGO_PKG_VERSION"),
        body.len(),
        body
    )
}

fn chat_completion_body(prompt: &str, max_tokens: usize, options: &LlmRequestOptions) -> String {
    let (system_prompt, user_prompt) = split_prompt_pack_chat_messages(prompt);
    format!(
        concat!(
            "{{",
            "\"messages\":[",
            "{{\"role\":\"system\",\"content\":{}}},",
            "{{\"role\":\"user\",\"content\":{}}}",
            "],",
            "\"max_tokens\":{},",
            "\"temperature\":{},",
            "\"stream\":false,",
            "\"chat_template_kwargs\":{{\"enable_thinking\":false}},",
            "\"response_format\":{{\"type\":\"json_object\"}}",
            "{}{}",
            "}}"
        ),
        json_string(&system_prompt),
        json_string(&user_prompt),
        max_tokens,
        json_number(options.temperature),
        seed_body_field(options),
        model_body_field(options)
    )
}

fn text_completion_body(prompt: &str, max_tokens: usize, options: &LlmRequestOptions) -> String {
    format!(
        "{{\"prompt\":{},\"n_predict\":{},\"temperature\":{}{}{}}}",
        json_string(prompt),
        max_tokens,
        json_number(options.temperature),
        seed_body_field(options),
        model_body_field(options)
    )
}

fn seed_body_field(options: &LlmRequestOptions) -> String {
    match options.seed {
        Some(seed) => format!(",\"seed\":{seed}"),
        None => String::new(),
    }
}

fn model_body_field(options: &LlmRequestOptions) -> String {
    match options.model.as_deref() {
        Some(model) => format!(",\"model\":{}", json_string(model)),
        None => String::new(),
    }
}

fn json_number(value: f64) -> String {
    let text = format!("{value:?}");
    if text.contains(['.', 'e', 'E']) {
        text
    } else {
        format!("{text}.0")
    }
}

fn split_prompt_pack_chat_messages(prompt: &str) -> (String, String) {
    let default_system_prompt = "You are the AIL toolchain's prompt-pack executor. Follow the provided AIL prompt contract, return only the requested artifact, and never claim the output is trusted until the checker verifies it.";
    let Some(system_start_marker) = prompt.find("\n# Prompt:") else {
        return (default_system_prompt.to_string(), prompt.to_string());
    };
    let system_start = system_start_marker + 1;
    let system_end = prompt[system_start..]
        .find("\nPrefer the prompt-pack JSON envelope")
        .map(|offset| system_start + offset)
        .unwrap_or(prompt.len());
    let system_prompt = prompt[system_start..system_end].trim().to_string();
    if system_prompt.is_empty() {
        return (default_system_prompt.to_string(), prompt.to_string());
    }

    let mut user_prompt = String::new();
    user_prompt.push_str(prompt[..system_start].trim_end());
    let user_tail = prompt[system_end..].trim_start();
    if !user_tail.is_empty() {
        if !user_prompt.is_empty() {
            user_prompt.push_str("\n\n");
        }
        user_prompt.push_str(user_tail);
    }
    (system_prompt, user_prompt)
}

fn is_chat_completion_path(path: &str) -> bool {
    path.ends_with("/chat/completions")
}

fn parse_http_endpoint(endpoint: &str) -> Result<HttpEndpoint, String> {
    let endpoint = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| "only http:// endpoints are supported".to_string())?;
    let (host_port, path) = match endpoint.split_once('/') {
        Some((host_port, "")) => (host_port, "/completion".to_string()),
        Some((host_port, rest)) => (host_port, format!("/{}", rest)),
        None => (endpoint, "/completion".to_string()),
    };
    let (host, port) = match host_port.split_once(':') {
        Some((host, port)) => {
            let port = port
                .parse::<u16>()
                .map_err(|error| format!("invalid port in endpoint: {error}"))?;
            (host.to_string(), port)
        }
        None => (host_port.to_string(), 80),
    };
    if host.is_empty() {
        return Err("endpoint is missing a host".to_string());
    }
    Ok(HttpEndpoint { host, port, path })
}

struct HttpEndpoint {
    host: String,
    port: u16,
    path: String,
}

impl HttpEndpoint {
    fn authority(&self) -> String {
        if self.port == 80 {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    fn url(&self) -> String {
        format!("http://{}{}", self.authority(), self.path)
    }
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    reason: String,
    body: String,
}

/// A byte source whose read timeout can be re-armed before every syscall, so a
/// long generation is bounded per read *and* by an overall request budget.
trait TimedRead {
    fn arm_read_timeout(&mut self, timeout: Duration) -> io::Result<()>;
    fn read_available(&mut self, buffer: &mut [u8]) -> io::Result<usize>;
}

impl TimedRead for TcpStream {
    fn arm_read_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        self.set_read_timeout(Some(timeout))
    }

    fn read_available(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.read(buffer)
    }
}

struct ResponseReader<'a, R: TimedRead> {
    source: &'a mut R,
    buffer: Vec<u8>,
    cursor: usize,
    eof: bool,
    read_timeout: Duration,
    deadline: Instant,
}

impl<'a, R: TimedRead> ResponseReader<'a, R> {
    fn new(source: &'a mut R, read_timeout: Duration, deadline: Instant) -> Self {
        Self {
            source,
            buffer: Vec::new(),
            cursor: 0,
            eof: false,
            read_timeout,
            deadline,
        }
    }

    fn fill(&mut self) -> io::Result<usize> {
        if self.eof {
            return Ok(0);
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "total request budget exhausted",
            ));
        }
        self.source
            .arm_read_timeout(self.read_timeout.min(remaining))?;
        let mut chunk = [0u8; READ_CHUNK_BYTES];
        let read = self.source.read_available(&mut chunk)?;
        if read == 0 {
            self.eof = true;
            return Ok(0);
        }
        if self.buffer.len() + read > MAX_RESPONSE_BODY_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("response exceeded the {MAX_RESPONSE_BODY_BYTES} byte cap"),
            ));
        }
        self.buffer.extend_from_slice(&chunk[..read]);
        Ok(read)
    }

    fn read_line(&mut self) -> io::Result<Vec<u8>> {
        let mut search = self.cursor;
        loop {
            if let Some(offset) = find_crlf(&self.buffer[search..]) {
                let end = search + offset;
                let line = self.buffer[self.cursor..end].to_vec();
                self.cursor = end + 2;
                return Ok(line);
            }
            search = self.buffer.len().saturating_sub(1).max(self.cursor);
            if self.fill()? == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "connection closed before the HTTP line terminator",
                ));
            }
        }
    }

    fn read_exact_bytes(&mut self, count: usize) -> io::Result<Vec<u8>> {
        while self.buffer.len() - self.cursor < count {
            if self.fill()? == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!(
                        "connection closed after {} of {count} announced body bytes",
                        self.buffer.len() - self.cursor
                    ),
                ));
            }
        }
        let end = self.cursor + count;
        let bytes = self.buffer[self.cursor..end].to_vec();
        self.cursor = end;
        Ok(bytes)
    }

    fn read_to_end(&mut self) -> io::Result<Vec<u8>> {
        while self.fill()? > 0 {}
        let bytes = self.buffer[self.cursor..].to_vec();
        self.cursor = self.buffer.len();
        Ok(bytes)
    }
}

fn find_crlf(bytes: &[u8]) -> Option<usize> {
    bytes.windows(2).position(|window| window == b"\r\n")
}

fn read_http_response<R: TimedRead>(
    source: &mut R,
    read_timeout: Duration,
    deadline: Instant,
) -> io::Result<HttpResponse> {
    let mut reader = ResponseReader::new(source, read_timeout, deadline);
    let status_line = reader.read_line()?;
    let (status, reason) = parse_status_line(&String::from_utf8_lossy(&status_line))?;
    let mut headers = Vec::new();
    loop {
        let line = reader.read_line()?;
        if line.is_empty() {
            break;
        }
        let line = String::from_utf8_lossy(&line);
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    let body = if is_chunked(&headers) {
        read_chunked_body(&mut reader)?
    } else if let Some(length) = header_value(&headers, "content-length")
        .and_then(|value| value.trim().parse::<usize>().ok())
    {
        reader.read_exact_bytes(length)?
    } else {
        reader.read_to_end()?
    };
    Ok(HttpResponse {
        status,
        reason,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

fn parse_status_line(line: &str) -> io::Result<(u16, String)> {
    let line = line.trim();
    let mut parts = line.splitn(3, ' ');
    let version = parts.next().unwrap_or_default();
    if !version.starts_with("HTTP/") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected an HTTP status line, got {line:?}"),
        ));
    }
    let status = parts
        .next()
        .unwrap_or_default()
        .parse::<u16>()
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid HTTP status code in {line:?}: {error}"),
            )
        })?;
    Ok((status, parts.next().unwrap_or_default().trim().to_string()))
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(header, _)| header == name)
        .map(|(_, value)| value.as_str())
}

fn is_chunked(headers: &[(String, String)]) -> bool {
    header_value(headers, "transfer-encoding").is_some_and(|value| {
        value
            .split(',')
            .any(|token| token.trim().eq_ignore_ascii_case("chunked"))
    })
}

fn read_chunked_body<R: TimedRead>(reader: &mut ResponseReader<'_, R>) -> io::Result<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let header = reader.read_line()?;
        let header = String::from_utf8_lossy(&header);
        // A chunk header is `<hex-size>[;chunk-extension]`.
        let size_text = header.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid chunked transfer-encoding size {size_text:?}: {error}"),
            )
        })?;
        if size == 0 {
            // Consume any trailer section up to its terminating blank line.
            while !reader.read_line()?.is_empty() {}
            return Ok(body);
        }
        if body.len() + size > MAX_RESPONSE_BODY_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("chunked response exceeded the {MAX_RESPONSE_BODY_BYTES} byte cap"),
            ));
        }
        body.extend_from_slice(&reader.read_exact_bytes(size)?);
        let terminator = reader.read_line()?;
        if !terminator.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "chunked transfer-encoding chunk was not terminated by CRLF",
            ));
        }
    }
}

fn sanitize_model_text(text: &str) -> String {
    let text = text.trim();
    let without_think = if let Some(start) = text.find("<think>") {
        if let Some(end) = text.find("</think>") {
            let mut out = String::new();
            out.push_str(text[..start].trim_end());
            out.push_str(text[end + "</think>".len()..].trim_start());
            out
        } else {
            text[..start].trim_end().to_string()
        }
    } else {
        text.to_string()
    };
    let without_fences = strip_code_fences(&without_think);
    without_fences.trim().to_string()
}

fn strip_code_fences(text: &str) -> String {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix("```") {
        let without_language = match rest.split_once('\n') {
            Some((first_line, remainder))
                if first_line.trim().len() <= 8
                    && first_line
                        .trim()
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric()) =>
            {
                remainder
            }
            _ => rest,
        };
        if let Some(end) = without_language.rfind("```") {
            return without_language[..end].trim().to_string();
        }
    }
    text.to_string()
}

enum PromptEnvelope {
    Artifact(String),
    Questions(Vec<String>),
    Invalid(String),
}

struct PromptEnvelopeExpectation<'a> {
    artifact_kind: &'a str,
    expected_profile: &'a str,
}

fn extract_prompt_envelope(
    text: &str,
    expectation: Option<&PromptEnvelopeExpectation<'_>>,
) -> Option<PromptEnvelope> {
    let trimmed = text.trim();
    if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
        return None;
    }
    if !looks_like_prompt_envelope(trimmed) {
        return None;
    }
    if let Some(expectation) = expectation
        && let Some(message) = validate_prompt_envelope_handoff(trimmed, expectation)
    {
        return Some(PromptEnvelope::Invalid(message));
    }
    let artifact_text = extract_json_string_field(trimmed, "artifact_text").unwrap_or_default();
    let questions = extract_json_string_array_field(trimmed, "questions");
    if artifact_text.trim().is_empty() {
        if questions.is_empty() {
            return Some(PromptEnvelope::Invalid(
                "AIL-PROMPT-001 prompt envelope must contain artifact_text or questions"
                    .to_string(),
            ));
        }
        return Some(PromptEnvelope::Questions(questions));
    }
    if !questions.is_empty() {
        return Some(PromptEnvelope::Invalid(
            "AIL-PROMPT-001 prompt envelope cannot contain both artifact_text and questions"
                .to_string(),
        ));
    }
    Some(PromptEnvelope::Artifact(artifact_text.trim().to_string()))
}

fn validate_prompt_envelope_handoff(
    text: &str,
    expectation: &PromptEnvelopeExpectation<'_>,
) -> Option<String> {
    let artifact_kind = extract_json_string_field(text, "artifact_kind").unwrap_or_default();
    if artifact_kind != expectation.artifact_kind {
        return Some(format!(
            "AIL-PROMPT-001 prompt envelope artifact_kind must be {}",
            expectation.artifact_kind
        ));
    }
    if extract_json_bool_field(text, "must_check") != Some(true) {
        return Some(
            "AIL-PROMPT-001 prompt envelope checker_handoff.must_check must be true".to_string(),
        );
    }
    let expected_profile = extract_json_string_field(text, "expected_profile");
    if expected_profile.as_deref() != Some(expectation.expected_profile) {
        return Some(format!(
            "AIL-PROMPT-001 prompt envelope checker_handoff.expected_profile must be {}",
            expectation.expected_profile
        ));
    }
    None
}

fn looks_like_prompt_envelope(text: &str) -> bool {
    field_exists(text, "artifact_kind")
        || field_exists(text, "artifact_text")
        || field_exists(text, "questions")
        || field_exists(text, "checker_handoff")
}

fn field_exists(text: &str, field: &str) -> bool {
    json_field_value_start(text, field).is_some()
}

fn extract_json_string_array_field(text: &str, field: &str) -> Vec<String> {
    let Some(start) = json_field_value_start(text, field) else {
        return Vec::new();
    };
    let bytes = text.as_bytes();
    if bytes.get(start) != Some(&b'[') {
        return Vec::new();
    }
    let mut index = start + 1;
    let mut values = Vec::new();
    loop {
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b',')
        {
            index += 1;
        }
        if bytes.get(index) != Some(&b'"') {
            return values;
        }
        let Some((value, end)) = parse_json_string_at(text, index) else {
            return values;
        };
        values.push(value);
        index = end;
    }
}

fn extract_json_string_field(text: &str, field: &str) -> Option<String> {
    let start = json_field_value_start(text, field)?;
    parse_json_string_at(text, start).map(|(value, _end)| value)
}

fn extract_json_bool_field(text: &str, field: &str) -> Option<bool> {
    let start = json_field_value_start(text, field)?;
    let rest = &text[start..];
    if rest.starts_with("true") {
        return Some(true);
    }
    if rest.starts_with("false") {
        return Some(false);
    }
    None
}

fn json_field_value_start(text: &str, field: &str) -> Option<usize> {
    let needle = format!("\"{field}\"");
    let mut search_start = 0;
    while search_start < text.len() {
        let field_start = search_start + text[search_start..].find(&needle)?;
        let mut index = field_start + needle.len();
        index = skip_json_whitespace(text, index);
        if text.as_bytes().get(index) == Some(&b':') {
            return Some(skip_json_whitespace(text, index + 1));
        }
        search_start = field_start + needle.len();
    }
    None
}

fn skip_json_whitespace(text: &str, mut index: usize) -> usize {
    while text
        .as_bytes()
        .get(index)
        .is_some_and(u8::is_ascii_whitespace)
    {
        index += 1;
    }
    index
}

/// Parse the JSON string literal starting at `start`, returning the decoded
/// value and the index just past the closing quote.
fn parse_json_string_at(text: &str, start: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    if bytes.get(start) != Some(&b'"') {
        return None;
    }
    let mut index = start + 1;
    let mut output = String::new();
    while let Some(byte) = bytes.get(index).copied() {
        match byte {
            b'"' => return Some((output, index + 1)),
            b'\\' => index = decode_json_escape(text, index + 1, &mut output)?,
            _ => {
                let ch = text[index..].chars().next()?;
                output.push(ch);
                index += ch.len_utf8();
            }
        }
    }
    None
}

/// Decode the escape sequence whose body starts at `index` (just past the
/// backslash), pushing the decoded text onto `output` and returning the index
/// just past the sequence.
fn decode_json_escape(text: &str, index: usize, output: &mut String) -> Option<usize> {
    let escape = text.as_bytes().get(index).copied()?;
    let simple = match escape {
        b'n' => Some('\n'),
        b'r' => Some('\r'),
        b't' => Some('\t'),
        b'b' => Some('\u{0008}'),
        b'f' => Some('\u{000c}'),
        b'"' => Some('"'),
        b'\\' => Some('\\'),
        b'/' => Some('/'),
        _ => None,
    };
    if let Some(ch) = simple {
        output.push(ch);
        return Some(index + 1);
    }
    if escape != b'u' {
        // Unknown escape: keep the historical lenient behaviour of emitting the
        // escaped character as-is rather than failing the whole parse.
        let ch = text[index..].chars().next()?;
        output.push(ch);
        return Some(index + ch.len_utf8());
    }
    let (first, mut next) = parse_hex4(text, index + 1)?;
    let code = if (0xd800..0xdc00).contains(&first) {
        match parse_low_surrogate(text, next) {
            Some((low, after_low)) => {
                next = after_low;
                0x1_0000 + ((first - 0xd800) << 10) + (low - 0xdc00)
            }
            None => u32::from(char::REPLACEMENT_CHARACTER),
        }
    } else if (0xdc00..0xe000).contains(&first) {
        u32::from(char::REPLACEMENT_CHARACTER)
    } else {
        first
    };
    output.push(char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER));
    Some(next)
}

/// Read a `\uXXXX` sequence at `index` and return its value when it is a low
/// surrogate, so a high surrogate can be paired into a single scalar value.
fn parse_low_surrogate(text: &str, index: usize) -> Option<(u32, usize)> {
    let bytes = text.as_bytes();
    if bytes.get(index) != Some(&b'\\') || bytes.get(index + 1) != Some(&b'u') {
        return None;
    }
    let (low, next) = parse_hex4(text, index + 2)?;
    if (0xdc00..0xe000).contains(&low) {
        Some((low, next))
    } else {
        None
    }
}

fn parse_hex4(text: &str, index: usize) -> Option<(u32, usize)> {
    let digits = text.get(index..index + 4)?;
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some((value, index + 4))
}

/// The subset of JSON we need from a server response. Strings, arrays and
/// objects carry data; every other scalar is parsed for structural correctness
/// and then discarded.
enum JsonValue {
    Scalar,
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    fn field(&self, name: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(entries) => entries
                .iter()
                .find(|(key, _)| key.as_str() == name)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    fn entry(&self, index: usize) -> Option<&JsonValue> {
        match self {
            JsonValue::Array(items) => items.get(index),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(value) => Some(value.as_str()),
            _ => None,
        }
    }
}

fn parse_json_document(text: &str) -> Option<JsonValue> {
    let (value, end) = parse_json_value(text, 0, 0)?;
    if skip_json_whitespace(text, end) == text.len() {
        Some(value)
    } else {
        None
    }
}

fn parse_json_value(text: &str, index: usize, depth: usize) -> Option<(JsonValue, usize)> {
    if depth > MAX_JSON_DEPTH {
        return None;
    }
    let index = skip_json_whitespace(text, index);
    match text.as_bytes().get(index)? {
        b'"' => {
            parse_json_string_at(text, index).map(|(value, end)| (JsonValue::String(value), end))
        }
        b'[' => parse_json_array(text, index, depth),
        b'{' => parse_json_object(text, index, depth),
        _ => parse_json_scalar(text, index),
    }
}

fn parse_json_array(text: &str, index: usize, depth: usize) -> Option<(JsonValue, usize)> {
    let mut index = skip_json_whitespace(text, index + 1);
    let mut items = Vec::new();
    if text.as_bytes().get(index) == Some(&b']') {
        return Some((JsonValue::Array(items), index + 1));
    }
    loop {
        let (value, end) = parse_json_value(text, index, depth + 1)?;
        items.push(value);
        index = skip_json_whitespace(text, end);
        match text.as_bytes().get(index)? {
            b',' => index = skip_json_whitespace(text, index + 1),
            b']' => return Some((JsonValue::Array(items), index + 1)),
            _ => return None,
        }
    }
}

fn parse_json_object(text: &str, index: usize, depth: usize) -> Option<(JsonValue, usize)> {
    let mut index = skip_json_whitespace(text, index + 1);
    let mut entries = Vec::new();
    if text.as_bytes().get(index) == Some(&b'}') {
        return Some((JsonValue::Object(entries), index + 1));
    }
    loop {
        let (key, end) = parse_json_string_at(text, index)?;
        let mut cursor = skip_json_whitespace(text, end);
        if text.as_bytes().get(cursor) != Some(&b':') {
            return None;
        }
        cursor = skip_json_whitespace(text, cursor + 1);
        let (value, end) = parse_json_value(text, cursor, depth + 1)?;
        entries.push((key, value));
        index = skip_json_whitespace(text, end);
        match text.as_bytes().get(index)? {
            b',' => index = skip_json_whitespace(text, index + 1),
            b'}' => return Some((JsonValue::Object(entries), index + 1)),
            _ => return None,
        }
    }
}

fn parse_json_scalar(text: &str, index: usize) -> Option<(JsonValue, usize)> {
    for literal in ["true", "false", "null"] {
        if text[index..].starts_with(literal) {
            return Some((JsonValue::Scalar, index + literal.len()));
        }
    }
    let bytes = text.as_bytes();
    let mut end = index;
    while end < bytes.len() && matches!(bytes[end], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
    {
        end += 1;
    }
    if end == index {
        return None;
    }
    Some((JsonValue::Scalar, end))
}

fn env_override(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_env_secs(name: &str, value: &str) -> Result<Duration, String> {
    let seconds = value
        .parse::<u64>()
        .map_err(|error| format!("invalid {name}={value:?}: {error}"))?;
    Ok(Duration::from_secs(seconds))
}

fn parse_env_millis(name: &str, value: &str) -> Result<Duration, String> {
    let millis = value
        .parse::<u64>()
        .map_err(|error| format!("invalid {name}={value:?}: {error}"))?;
    Ok(Duration::from_millis(millis))
}

fn parse_env_usize(name: &str, value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid {name}={value:?}: {error}"))
}

fn parse_env_u32(name: &str, value: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {name}={value:?}: {error}"))
}

fn parse_env_i64(name: &str, value: &str) -> Result<i64, String> {
    value
        .parse::<i64>()
        .map_err(|error| format!("invalid {name}={value:?}: {error}"))
}

fn parse_env_f64(name: &str, value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|error| format!("invalid {name}={value:?}: {error}"))
}

fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis.is_multiple_of(1000) {
        format!("{}s", millis / 1000)
    } else {
        format!("{millis}ms")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    impl TimedRead for Cursor<Vec<u8>> {
        fn arm_read_timeout(&mut self, _timeout: Duration) -> io::Result<()> {
            Ok(())
        }

        fn read_available(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            // Hand back a few bytes at a time so the framing code has to stitch
            // partial reads together the way a real socket forces it to.
            let limit = buffer.len().min(7);
            self.read(&mut buffer[..limit])
        }
    }

    fn read_canned(raw: &str) -> io::Result<HttpResponse> {
        let mut source = Cursor::new(raw.as_bytes().to_vec());
        read_http_response(
            &mut source,
            Duration::from_secs(1),
            Instant::now() + Duration::from_secs(60),
        )
    }

    fn decode(literal: &str) -> Option<String> {
        parse_json_string_at(literal, 0).map(|(value, _end)| value)
    }

    /// One raw JSON `backslash-u` escape for `hex`, assembled at runtime so the
    /// escape never appears literally in this file.
    fn esc(hex: &str) -> String {
        format!("\\u{hex}")
    }

    /// Wrap `body` in the quotes that make it a JSON string literal.
    fn quoted(body: &str) -> String {
        format!("\"{body}\"")
    }

    #[test]
    fn decodes_basic_multilingual_plane_unicode_escapes() {
        assert_eq!(quoted(&esc("0041")), "\"\\u0041\"");
        assert_eq!(decode(&quoted(&esc("0041"))).as_deref(), Some("A"));
        assert_eq!(decode(&quoted(&esc("00e9"))).as_deref(), Some("\u{e9}"));
        assert_eq!(decode(&quoted(&esc("2014"))).as_deref(), Some("\u{2014}"));
        assert_eq!(
            decode(&quoted(&format!("a{}b", esc("0041")))).as_deref(),
            Some("aAb")
        );
        assert_eq!(decode(&quoted(&esc("0000"))).as_deref(), Some("\0"));
    }

    #[test]
    fn decodes_surrogate_pairs() {
        let rocket = format!("{}{}", esc("d83d"), esc("de80"));
        assert_eq!(quoted(&rocket), "\"\\ud83d\\ude80\"");
        assert_eq!(decode(&quoted(&rocket)).as_deref(), Some("\u{1f680}"));
        assert_eq!(
            decode(&quoted(&format!("x{rocket}y"))).as_deref(),
            Some("x\u{1f680}y")
        );
        let uppercase = format!("{}{}", esc("D83D"), esc("DE80"));
        assert_eq!(decode(&quoted(&uppercase)).as_deref(), Some("\u{1f680}"));
    }

    #[test]
    fn replaces_lone_surrogates_instead_of_emitting_garbage() {
        assert_eq!(decode(&quoted(&esc("d83d"))).as_deref(), Some("\u{fffd}"));
        assert_eq!(decode(&quoted(&esc("dc00"))).as_deref(), Some("\u{fffd}"));
        assert_eq!(
            decode(&quoted(&format!("{}{}", esc("d83d"), esc("0041")))).as_deref(),
            Some("\u{fffd}A")
        );
        assert_eq!(decode(&quoted(&esc("zzzz"))), None);
    }

    #[test]
    fn decodes_the_remaining_json_escapes() {
        assert_eq!(
            decode(r#""a\nb\tc\\d\"e\/f""#).as_deref(),
            Some("a\nb\tc\\d\"e/f")
        );
        assert_eq!(decode(r#""\b\f\r""#).as_deref(), Some("\u{0008}\u{000c}\r"));
    }

    #[test]
    fn extracts_string_fields_with_unicode_escapes() {
        let body = format!(
            "{{\"artifact_text\":{}}}",
            quoted(&format!("{}{}{}", esc("0041"), esc("d83d"), esc("de80")))
        );
        assert_eq!(
            extract_json_string_field(&body, "artifact_text").as_deref(),
            Some("A\u{1f680}")
        );
    }

    #[test]
    fn extracts_string_arrays_with_unicode_escapes() {
        let body = format!(
            "{{\"questions\":[{}, {}]}}",
            quoted(&format!("{}?", esc("0041"))),
            quoted("b?")
        );
        assert_eq!(
            extract_json_string_array_field(&body, "questions"),
            vec!["A?".to_string(), "b?".to_string()]
        );
    }

    #[test]
    fn reads_content_length_framed_responses() {
        let body = r#"{"choices":[{"message":{"content":"hi"}}]}"#;
        let raw = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let response = read_canned(&raw).expect("content-length response should parse");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, body);
        assert_eq!(completion_content(&response.body, true).unwrap(), "hi");
    }

    #[test]
    fn reads_chunked_responses() {
        // A JSON body split mid-token across two chunks, with a chunk extension
        // on the second, exactly as a reverse proxy in front of llama.cpp frames
        // it. Sizes are computed so the fixture cannot drift out of sync.
        let first = r#"{"choices":[{"message":"#;
        let second = r#"{"content":"ok"}}]}"#;
        let raw = format!(
            concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: application/json\r\n",
                "Transfer-Encoding: chunked\r\n",
                "\r\n",
                "{:x}\r\n{}\r\n",
                "{:x};ext=1\r\n{}\r\n",
                "0\r\n",
                "\r\n"
            ),
            first.len(),
            first,
            second.len(),
            second
        );
        let response = read_canned(&raw).expect("chunked response should parse");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, format!("{first}{second}"));
        assert_eq!(completion_content(&response.body, true).unwrap(), "ok");
    }

    #[test]
    fn reads_chunked_responses_with_trailers() {
        let raw = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Transfer-Encoding: chunked\r\n",
            "\r\n",
            "5\r\nhello\r\n",
            "0\r\n",
            "X-Trailer: done\r\n",
            "\r\n"
        );
        let response = read_canned(raw).expect("trailers should be consumed");
        assert_eq!(response.body, "hello");
    }

    #[test]
    fn rejects_malformed_chunk_headers() {
        let raw = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\nhello\r\n0\r\n\r\n";
        let error = read_canned(raw).expect_err("a non-hex chunk size must fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn reads_connection_close_framed_responses() {
        let raw = "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n{\"content\":\"tail\"}";
        let response = read_canned(raw).expect("eof framing should parse");
        assert_eq!(completion_content(&response.body, false).unwrap(), "tail");
    }

    #[test]
    fn rejects_non_http_status_lines() {
        let error = read_canned("garbage\r\n\r\n").expect_err("non-HTTP preamble must fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_empty_assistant_content_with_reasoning_leakage() {
        let body = r#"{"choices":[{"finish_reason":"length","index":0,"message":{"role":"assistant","content":"","reasoning_content":"thinking hard"}}]}"#;
        let failure = completion_content(body, true).expect_err("empty content must not succeed");
        assert_eq!(failure.kind, "empty-content");
        assert!(!failure.retryable);
        assert!(failure.detail.contains("finish_reason=length"));
        assert!(failure.detail.contains("reasoning_content"));
        assert!(failure.detail.contains("13 characters"));
    }

    #[test]
    fn rejects_empty_assistant_content_without_reasoning() {
        let body = r#"{"choices":[{"finish_reason":"stop","message":{"content":"   "}}]}"#;
        let failure = completion_content(body, true).expect_err("blank content must not succeed");
        assert_eq!(failure.kind, "empty-content");
        assert!(failure.detail.contains("Resample"));
    }

    #[test]
    fn reports_error_envelopes_returned_with_a_200() {
        let body =
            r#"{"error":{"message":"context window exceeded","type":"invalid_request_error"}}"#;
        let failure = completion_content(body, true).expect_err("error envelope must not succeed");
        assert_eq!(failure.kind, "model-error");
        assert!(!failure.retryable);
        assert!(failure.detail.contains("context window exceeded"));
        assert!(failure.detail.contains("invalid_request_error"));
    }

    #[test]
    fn reports_error_message_for_non_200_statuses() {
        let body = r#"{"error":{"message":"no slots available","code":503}}"#;
        let response = HttpResponse {
            status: 503,
            reason: "Service Unavailable".to_string(),
            body: body.to_string(),
        };
        let failure = http_status_failure(&response);
        assert_eq!(failure.kind, "http-503");
        assert!(failure.retryable);
        assert!(failure.detail.contains("no slots available"));
    }

    #[test]
    fn does_not_retry_client_errors() {
        let response = HttpResponse {
            status: 404,
            reason: "Not Found".to_string(),
            body: "nope".to_string(),
        };
        let failure = http_status_failure(&response);
        assert_eq!(failure.kind, "http-404");
        assert!(!failure.retryable);
        assert!(failure.detail.contains("nope"));
        assert!(
            http_status_failure(&HttpResponse {
                status: 429,
                reason: "Too Many Requests".to_string(),
                body: String::new(),
            })
            .retryable
        );
    }

    #[test]
    fn ignores_the_first_content_key_outside_the_assistant_message() {
        // The old scanner matched the first `"content"` key anywhere in the body,
        // which is the prompt echo here rather than the completion.
        let body = r#"{"prompt":{"content":"echoed prompt"},"choices":[{"message":{"content":"real answer"}}]}"#;
        assert_eq!(
            completion_content(body, true).unwrap(),
            "real answer".to_string()
        );
    }

    #[test]
    fn rejects_truncated_json_bodies() {
        let failure = completion_content(r#"{"choices":[{"message":{"content":"oops"#, true)
            .expect_err("truncated JSON must not succeed");
        assert_eq!(failure.kind, "protocol");
        assert!(!failure.retryable);
    }

    #[test]
    fn classifies_io_failures() {
        let timeout = LlmFailure::io(
            "read",
            "ctx",
            &io::Error::new(io::ErrorKind::WouldBlock, "would block"),
        );
        assert_eq!(timeout.kind, "timeout");
        assert!(timeout.retryable);
        let refused = LlmFailure::io(
            "connect",
            "ctx",
            &io::Error::new(io::ErrorKind::ConnectionRefused, "refused"),
        );
        assert_eq!(refused.kind, "connect");
        assert!(refused.retryable);
        let framing = LlmFailure::io(
            "read",
            "ctx",
            &io::Error::new(io::ErrorKind::InvalidData, "bad chunk"),
        );
        assert!(!framing.retryable);
    }

    #[test]
    fn error_messages_name_the_endpoint_kind_and_attempt_count() {
        let endpoint = parse_http_endpoint("http://127.0.0.1:8091/v1/chat/completions").unwrap();
        let message = LlmFailure::retryable("timeout", "connect timed out".to_string())
            .into_message(&endpoint, 3);
        assert_eq!(
            message,
            "llm request to http://127.0.0.1:8091/v1/chat/completions failed after 3 attempt(s) [timeout]: connect timed out"
        );
    }

    #[test]
    fn backs_off_exponentially_up_to_a_cap() {
        let options = LlmRequestOptions::default();
        assert_eq!(retry_delay(&options, 1), Duration::from_millis(500));
        assert_eq!(retry_delay(&options, 2), Duration::from_millis(1000));
        assert_eq!(retry_delay(&options, 3), Duration::from_millis(2000));
        assert_eq!(retry_delay(&options, 40), MAX_RETRY_BACKOFF);
    }

    #[test]
    fn defaults_stay_deterministic_and_bounded() {
        let options = LlmRequestOptions::default();
        options.validate().unwrap();
        assert_eq!(options.temperature, 0.0);
        assert!(options.seed.is_none());
        assert!(options.model.is_none());
        assert_eq!(options.max_attempts, DEFAULT_MAX_ATTEMPTS);
        assert_eq!(options.connect_timeout, DEFAULT_CONNECT_TIMEOUT);
        assert_eq!(options.read_timeout, DEFAULT_READ_TIMEOUT);
        assert_eq!(options.write_timeout, DEFAULT_WRITE_TIMEOUT);
    }

    #[test]
    fn rejects_invalid_option_values() {
        assert!(
            LlmRequestOptions::default()
                .with_temperature(9.0)
                .validate()
                .is_err()
        );
        assert!(
            LlmRequestOptions {
                max_attempts: 0,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            LlmRequestOptions {
                read_timeout: Duration::ZERO,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            LlmRequestOptions {
                total_timeout: Duration::from_secs(90_000),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn chat_body_keeps_the_existing_contract_and_adds_sampling_controls() {
        let body = chat_completion_body("hello", 16, &LlmRequestOptions::default());
        assert!(body.contains(r#""temperature":0.0"#));
        assert!(body.contains(r#""chat_template_kwargs":{"enable_thinking":false}"#));
        assert!(body.contains(r#""response_format":{"type":"json_object"}"#));
        assert!(body.contains(r#""stream":false"#));
        assert!(body.contains(r#""max_tokens":16"#));
        assert!(!body.contains("\"seed\""));
        assert!(!body.contains("\"model\""));
        assert!(parse_json_document(&body).is_some());

        let tuned = chat_completion_body(
            "hello",
            16,
            &LlmRequestOptions::default()
                .with_temperature(0.7)
                .with_seed(42)
                .with_model("ornith-1.0-35b"),
        );
        assert!(tuned.contains(r#""temperature":0.7"#));
        assert!(tuned.contains(r#""seed":42"#));
        assert!(tuned.contains(r#""model":"ornith-1.0-35b""#));
        assert!(parse_json_document(&tuned).is_some());
    }

    #[test]
    fn completion_body_keeps_the_existing_contract_and_adds_sampling_controls() {
        let body = text_completion_body("hello", 8, &LlmRequestOptions::default());
        assert_eq!(
            body,
            r#"{"prompt":"hello","n_predict":8,"temperature":0.0}"#
        );
        let tuned = text_completion_body(
            "hello",
            8,
            &LlmRequestOptions::default().with_seed(7).with_model("m"),
        );
        assert_eq!(
            tuned,
            r#"{"prompt":"hello","n_predict":8,"temperature":0.0,"seed":7,"model":"m"}"#
        );
    }

    #[test]
    fn request_line_preserves_path_detection_and_carries_the_port_in_host() {
        let endpoint =
            parse_http_endpoint("http://192.168.60.171:8091/v1/chat/completions").unwrap();
        assert!(is_chat_completion_path(&endpoint.path));
        let request = http_request(&endpoint, "{}");
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1\r\n"));
        assert!(request.contains("Host: 192.168.60.171:8091\r\n"));
        assert!(request.contains("Content-Length: 2\r\n"));
        assert!(request.contains("Connection: close\r\n"));
    }

    #[test]
    fn root_endpoints_still_shorthand_to_the_completion_path() {
        assert_eq!(
            parse_http_endpoint("http://host:8091").unwrap().path,
            "/completion"
        );
        assert_eq!(
            parse_http_endpoint("http://host:8091/").unwrap().path,
            "/completion"
        );
        assert_eq!(parse_http_endpoint("http://host").unwrap().port, 80);
        assert_eq!(
            parse_http_endpoint("http://host").unwrap().authority(),
            "host"
        );
        assert!(!is_chat_completion_path(
            &parse_http_endpoint("http://host:8091").unwrap().path
        ));
        assert!(parse_http_endpoint("https://host").is_err());
    }

    #[test]
    fn prompt_envelope_handling_is_unchanged() {
        let envelope = r#"{"artifact_kind":"AIL-Spec Canonical","artifact_text":"spec A","checker_handoff":{"must_check":true,"expected_profile":"core"}}"#;
        let expectation = PromptEnvelopeExpectation {
            artifact_kind: "AIL-Spec Canonical",
            expected_profile: "core",
        };
        let (outcome, kind) = artifact_response_from_text(envelope, Some(&expectation)).unwrap();
        assert_eq!(kind, "prompt-envelope-artifact");
        let LlmArtifactResponse::Artifact(text) = outcome else {
            panic!("expected an artifact outcome");
        };
        assert_eq!(text, "spec A");
    }
}
