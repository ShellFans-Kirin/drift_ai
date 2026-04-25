//! Shared streaming helpers used by the OpenAI / Gemini / Ollama / OpenAI-compatible
//! providers. The Anthropic provider in `compaction.rs` predates this module
//! and inlines its own SSE parser; we keep that as-is to avoid disturbing the
//! v0.2-shipped code path.

use crate::compaction::CompactionError;
use futures_util::StreamExt;
use serde_json::Value;

/// Find the next `\n\n` event boundary in a partial buffer.
pub fn find_double_newline(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\n\n")
}

/// Find the next `\n` line boundary in a partial buffer (NDJSON).
pub fn find_newline(buf: &[u8]) -> Option<usize> {
    buf.iter().position(|&b| b == b'\n')
}

/// Drain SSE `data:` payload(s) out of a single event block. Multiple `data:`
/// lines in one event concatenate per the SSE spec; each `data: [DONE]` is
/// surfaced as `None`.
pub fn extract_sse_data(event_block: &str) -> Vec<Option<String>> {
    let mut out = Vec::new();
    for line in event_block.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            let data = rest.trim();
            if data.is_empty() {
                continue;
            }
            if data == "[DONE]" {
                out.push(None);
            } else {
                out.push(Some(data.to_string()));
            }
        }
    }
    out
}

/// Stream a `bytes_stream` response line-by-line as NDJSON. Each non-empty
/// line is parsed as a `serde_json::Value`. Errors during JSON parsing are
/// surfaced as `CompactionError::Stream`.
pub async fn for_each_ndjson<F>(
    resp: reqwest::Response,
    mut on_value: F,
) -> Result<(), CompactionError>
where
    F: FnMut(Value) -> Result<(), CompactionError>,
{
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(4096);

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(CompactionError::TransientNetwork)?;
        buf.extend_from_slice(&bytes);
        while let Some(idx) = find_newline(&buf) {
            let line: Vec<u8> = buf.drain(..=idx).collect();
            // Drop trailing newline.
            let line = &line[..line.len().saturating_sub(1)];
            if line.is_empty() {
                continue;
            }
            let v: Value = serde_json::from_slice(line)
                .map_err(|e| CompactionError::Stream(format!("bad NDJSON: {}", e)))?;
            on_value(v)?;
        }
    }
    // Trailing partial line, if any.
    if !buf.is_empty() {
        let v: Value = serde_json::from_slice(&buf)
            .map_err(|e| CompactionError::Stream(format!("bad NDJSON tail: {}", e)))?;
        on_value(v)?;
    }
    Ok(())
}

/// Stream a `bytes_stream` response as `\n\n`-delimited SSE blocks. Calls
/// `on_data(payload)` for each `data:` payload, and `on_data(None)` for each
/// `[DONE]` sentinel. Caller decides when to stop early via `Err`.
pub async fn for_each_sse_data<F>(
    resp: reqwest::Response,
    mut on_data: F,
) -> Result<(), CompactionError>
where
    F: FnMut(Option<&str>) -> Result<(), CompactionError>,
{
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(8192);

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(CompactionError::TransientNetwork)?;
        buf.extend_from_slice(&bytes);
        while let Some(idx) = find_double_newline(&buf) {
            let block: Vec<u8> = buf.drain(..idx + 2).collect();
            let s = String::from_utf8_lossy(&block);
            for payload in extract_sse_data(&s) {
                on_data(payload.as_deref())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newline_finder() {
        assert_eq!(find_newline(b"abc\ndef"), Some(3));
        assert_eq!(find_newline(b"no newline here"), None);
    }

    #[test]
    fn double_newline_finder() {
        assert_eq!(find_double_newline(b"abc\n\ndef"), Some(3));
        assert_eq!(find_double_newline(b"abc"), None);
    }

    #[test]
    fn extract_sse_done_sentinel() {
        let block = "event: x\ndata: [DONE]\n\n";
        let payloads = extract_sse_data(block);
        assert_eq!(payloads.len(), 1);
        assert!(payloads[0].is_none());
    }

    #[test]
    fn extract_sse_skips_empty_data() {
        let block = "data: \ndata:   {\"a\":1}\n\n";
        let payloads = extract_sse_data(block);
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn extract_sse_multiple_data_lines() {
        let block = "data: {\"a\":1}\ndata: {\"b\":2}\n\n";
        let payloads = extract_sse_data(block);
        assert_eq!(payloads.len(), 2);
    }
}
