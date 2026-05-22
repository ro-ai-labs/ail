use std::io::{Read, Write};
use std::net::TcpStream;

use crate::core_model::json_string;
use crate::parse_rsl_text;
use crate::render_rif_document;
use crate::rif_model::RifDocument;

pub fn llm_round_trip(document: &RifDocument, endpoint: &str) -> Result<(String, String), String> {
    let canonical = render_rif_document(document);
    let prompt = build_round_trip_prompt(&canonical);
    let response = invoke_completion(endpoint, &prompt)?;
    let candidate_rsl = sanitize_model_text(&response);
    let candidate_document = parse_rsl_text(&candidate_rsl)?;
    let round_tripped = render_rif_document(&candidate_document);
    if round_tripped != canonical {
        return Err("LLM rewrite did not round-trip back to the same canonical RIF".to_string());
    }
    Ok((candidate_rsl, round_tripped))
}

fn build_round_trip_prompt(canonical_rif: &str) -> String {
    format!(
        concat!(
            "Rewrite the canonical RIF below into controlled EIGL RSL.\n",
            "Preserve meaning exactly and output only parseable RSL text.\n",
            "Do not output descriptive Markdown headings like ## Domain Model. Do not use bullets. Do not use code fences. Do not include reasoning.\n",
            "Keep the same application facts, intent names, sections, steps, endpoint routes, triggers, and guarantees.\n",
            "Use this exact RSL style when possible:\n\n",
            "app ExampleApp\n\n",
            "things:\n",
            "  A Customer has an email address.\n",
            "  An Order can be Draft, Confirmed, Cancelled.\n\n",
            "intent Describe Domain:\n",
            "  subject:\n",
            "    order: Order\n\n",
            "The output must parse back to the same canonical RIF.\n\n",
            "CANONICAL RIF:\n",
            "<<<RIF\n",
            "{}\n",
            "RIF\n",
            ">>>\n"
        ),
        canonical_rif
    )
}

fn invoke_completion(endpoint: &str, prompt: &str) -> Result<String, String> {
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
    extract_json_string_field(response_body, "content")
        .ok_or_else(|| "model response missing content field".to_string())
}

fn is_chat_completion_path(path: &str) -> bool {
    path.ends_with("/chat/completions")
}

fn parse_http_endpoint(endpoint: &str) -> Result<HttpEndpoint, String> {
    let endpoint = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| "only http:// endpoints are supported".to_string())?;
    let (host_port, path) = match endpoint.split_once('/') {
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

fn extract_json_string_field(text: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":\"");
    let start = text.find(&needle)? + needle.len();
    let mut output = String::new();
    let mut escaped = false;
    for ch in text[start..].chars() {
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
