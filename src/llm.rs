use std::io::{Read, Write};
use std::net::TcpStream;

use crate::core_model::json_string;

pub fn invoke_llm_text(endpoint: &str, prompt: &str) -> Result<String, String> {
    invoke_llm_text_with_expectation(endpoint, prompt, None)
}

pub fn invoke_llm_text_for_artifact(
    endpoint: &str,
    prompt: &str,
    expected_artifact_kind: &str,
    expected_profile: &str,
) -> Result<String, String> {
    match invoke_llm_artifact_response(endpoint, prompt, expected_artifact_kind, expected_profile)?
    {
        LlmArtifactResponse::Artifact(artifact_text) => Ok(artifact_text),
        LlmArtifactResponse::Questions(questions) => Err(format!(
            "model returned blocking questions:\n- {}",
            questions.join("\n- ")
        )),
    }
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
    let expectation = PromptEnvelopeExpectation {
        artifact_kind: expected_artifact_kind,
        expected_profile,
    };
    invoke_llm_artifact_response_with_expectation(endpoint, prompt, &expectation)
}

pub fn invoke_llm_artifact_response_recorded(
    endpoint: &str,
    prompt: &str,
    expected_artifact_kind: &str,
    expected_profile: &str,
) -> Result<LlmRecordedArtifactResponse, String> {
    let expectation = PromptEnvelopeExpectation {
        artifact_kind: expected_artifact_kind,
        expected_profile,
    };
    invoke_llm_artifact_response_recorded_with_expectation(endpoint, prompt, &expectation)
}

fn invoke_llm_text_with_expectation(
    endpoint: &str,
    prompt: &str,
    expectation: Option<&PromptEnvelopeExpectation<'_>>,
) -> Result<String, String> {
    let response = match expectation {
        Some(expectation) => {
            invoke_llm_artifact_response_with_expectation(endpoint, prompt, expectation)?
        }
        None => invoke_llm_artifact_response_without_expectation(endpoint, prompt)?,
    };
    match response {
        LlmArtifactResponse::Artifact(artifact_text) => Ok(artifact_text),
        LlmArtifactResponse::Questions(questions) => Err(format!(
            "model returned blocking questions:\n- {}",
            questions.join("\n- ")
        )),
    }
}

fn invoke_llm_artifact_response_with_expectation(
    endpoint: &str,
    prompt: &str,
    expectation: &PromptEnvelopeExpectation<'_>,
) -> Result<LlmArtifactResponse, String> {
    Ok(
        invoke_llm_artifact_response_recorded_with_expectation(endpoint, prompt, expectation)?
            .outcome,
    )
}

fn invoke_llm_artifact_response_recorded_with_expectation(
    endpoint: &str,
    prompt: &str,
    expectation: &PromptEnvelopeExpectation<'_>,
) -> Result<LlmRecordedArtifactResponse, String> {
    let completion = invoke_completion(endpoint, prompt)?;
    let text = sanitize_model_text(&completion.content);
    let (outcome, content_kind) = artifact_response_from_text(&text, Some(expectation))?;
    Ok(LlmRecordedArtifactResponse {
        outcome,
        request_body: completion.request_body,
        response_body: completion.response_body,
        content_text: text,
        content_kind,
    })
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
) -> Result<LlmArtifactResponse, String> {
    let completion = invoke_completion(endpoint, prompt)?;
    let text = sanitize_model_text(&completion.content);
    artifact_response_from_text(&text, None).map(|(outcome, _kind)| outcome)
}

struct LlmHttpCompletion {
    request_body: String,
    response_body: String,
    content: String,
}

fn invoke_completion(endpoint: &str, prompt: &str) -> Result<LlmHttpCompletion, String> {
    let parsed = parse_http_endpoint(endpoint)?;
    let body = if is_chat_completion_path(&parsed.path) {
        format!(
            "{{\"messages\":[{{\"role\":\"user\",\"content\":{}}}],\"max_tokens\":4096,\"temperature\":0.0,\"chat_template_kwargs\":{{\"enable_thinking\":false}}}}",
            json_string(prompt)
        )
    } else {
        format!(
            "{{\"prompt\":{},\"n_predict\":2048,\"temperature\":0.0}}",
            json_string(prompt)
        )
    };
    let mut stream = TcpStream::connect((parsed.host.as_str(), parsed.port)).map_err(|error| {
        format!(
            "failed to connect to {}:{}: {error}",
            parsed.host, parsed.port
        )
    })?;
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        parsed.path,
        parsed.host,
        body.len(),
        body
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to send request: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read response: {error}"))?;
    let (status_line, response_body) = response
        .split_once("\r\n")
        .ok_or_else(|| "invalid HTTP response".to_string())?;
    if !status_line.contains("200") {
        return Err(format!("model endpoint returned {status_line}"));
    }
    let response_body = response_body
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .ok_or_else(|| "invalid HTTP response body".to_string())?;
    let content = extract_json_string_field(response_body, "content")
        .ok_or_else(|| "model response missing content field".to_string())?;
    Ok(LlmHttpCompletion {
        request_body: body,
        response_body: response_body.to_string(),
        content,
    })
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
    Ok(HttpEndpoint { host, port, path })
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
    let Some(mut start) = json_field_value_start(text, field) else {
        return Vec::new();
    };
    let bytes = text.as_bytes();
    if bytes.get(start) != Some(&b'[') {
        return Vec::new();
    }
    start += 1;
    let mut chars = text[start..].chars().peekable();
    let mut values = Vec::new();
    loop {
        while chars
            .peek()
            .is_some_and(|ch| ch.is_ascii_whitespace() || *ch == ',')
        {
            chars.next();
        }
        match chars.peek().copied() {
            Some(']') | None => break,
            Some('"') => {
                let Some(value) = parse_json_string(&mut chars) else {
                    break;
                };
                values.push(value);
            }
            Some(_) => break,
        }
    }
    values
}

fn extract_json_string_field(text: &str, field: &str) -> Option<String> {
    let start = json_field_value_start(text, field)?;
    let mut chars = text[start..].chars().peekable();
    parse_json_string(&mut chars)
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

fn parse_json_string<I>(chars: &mut std::iter::Peekable<I>) -> Option<String>
where
    I: Iterator<Item = char>,
{
    if chars.next()? != '"' {
        return None;
    }
    let mut output = String::new();
    let mut escaped = false;
    for ch in chars.by_ref() {
        if escaped {
            output.push(match ch {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '"' => '"',
                other => other,
            });
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return Some(output),
            other => output.push(other),
        }
    }
    None
}

struct HttpEndpoint {
    host: String,
    port: u16,
    path: String,
}
