use anyhow::Result;
use bollard::container::{LogOutput, LogsOptions};
use futures::{Stream, StreamExt};
use std::pin::Pin;

use crate::runtime::{LogLine, LogStream};

use super::DockerRuntime;

pub async fn logs(
    runtime: &DockerRuntime,
    container_id: &str,
) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>> {
    let options = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        follow: false,            // Don't follow - just fetch existing logs
        timestamps: true,         // Include Docker timestamps
        tail: "1000".to_string(), // Get last 1000 lines
        ..Default::default()
    };

    let stream = runtime.client.logs(container_id, Some(options));

    let mapped = stream.filter_map(|result| async move {
        match result {
            Ok(output) => {
                let (stream_type, message) = match output {
                    LogOutput::StdOut { message } => (LogStream::Stdout, message),
                    LogOutput::StdErr { message } => (LogStream::Stderr, message),
                    _ => return None,
                };
                let message_str = String::from_utf8_lossy(&message).to_string();
                // Parse Docker timestamp from the beginning of the message
                // Format: "2024-01-01T00:00:00.000000000Z message"
                let (timestamp, msg) =
                    if message_str.len() > 30 && message_str.chars().nth(4) == Some('-') {
                        // Has timestamp prefix
                        let parts: Vec<&str> = message_str.splitn(2, ' ').collect();
                        if parts.len() == 2 {
                            (parts[0].to_string(), parts[1].to_string())
                        } else {
                            (chrono::Utc::now().to_rfc3339(), message_str)
                        }
                    } else {
                        (chrono::Utc::now().to_rfc3339(), message_str)
                    };
                Some(LogLine {
                    timestamp,
                    message: msg.trim_end().to_string(),
                    stream: stream_type,
                })
            }
            Err(e) => {
                tracing::warn!("Error reading container log: {}", e);
                None
            }
        }
    });

    Ok(Box::pin(mapped))
}

pub async fn logs_stream(
    runtime: &DockerRuntime,
    container_id: &str,
) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>> {
    let options = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        follow: true, // Follow logs in real-time
        timestamps: true,
        tail: "100".to_string(), // Get last 100 lines then continue streaming
        ..Default::default()
    };

    let stream = runtime.client.logs(container_id, Some(options));

    let mapped = stream.filter_map(|result| async move {
        match result {
            Ok(output) => {
                let (stream_type, message) = match output {
                    LogOutput::StdOut { message } => (LogStream::Stdout, message),
                    LogOutput::StdErr { message } => (LogStream::Stderr, message),
                    _ => return None,
                };
                let message_str = String::from_utf8_lossy(&message).to_string();
                // Parse Docker timestamp from the beginning of the message
                let (timestamp, msg) =
                    if message_str.len() > 30 && message_str.chars().nth(4) == Some('-') {
                        let parts: Vec<&str> = message_str.splitn(2, ' ').collect();
                        if parts.len() == 2 {
                            (parts[0].to_string(), parts[1].to_string())
                        } else {
                            (chrono::Utc::now().to_rfc3339(), message_str)
                        }
                    } else {
                        (chrono::Utc::now().to_rfc3339(), message_str)
                    };
                Some(LogLine {
                    timestamp,
                    message: msg.trim_end().to_string(),
                    stream: stream_type,
                })
            }
            Err(e) => {
                tracing::warn!("Error reading container log: {}", e);
                None
            }
        }
    });

    Ok(Box::pin(mapped))
}
