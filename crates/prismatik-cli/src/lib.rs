//! # prismatik-prismatik-cli
//!
//! Layer 4 — Application and shell
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — CLI command contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

use prismatik_application::{AppConfig, DefaultPrismatikApp, PrismatikApp};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

pub use public_verify::{PublicTreeHead, PublicVerifyService, VerifyReport};

pub mod public_verify;

/// CLI service failure.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// File access failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Verification input was invalid.
    #[error("verification error: {0}")]
    Verify(String),
}

/// Verify a manifest file at a caller-supplied instant.
pub fn verify_manifest_file(
    path: &str,
    transitional: bool,
    verified_at: time::OffsetDateTime,
) -> Result<prismatik_manifest::VerificationReport, CliError> {
    let json = std::fs::read_to_string(path)?;
    let verifier = prismatik_manifest::StandaloneVerifier { transitional };
    verifier
        .verify_json(&json, None, None, verified_at)
        .map_err(|error| CliError::Verify(error.to_string()))
}

/// CLI subcommand definitions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Verify research bundle.
    Verify {
        /// Bundle path.
        bundle_path: String,
    },
    /// Inspect manifest.
    ManifestInspect {
        /// Manifest id.
        manifest_id: String,
    },
    /// Verify audit chain.
    AuditVerify,
    /// Generate calendar artifact.
    CalendarGenerate,
    /// Start in-process application bootstrap.
    Start {
        /// Profile name (e.g. `desktop`, `cloud`, `enterprise`).
        profile: String,
    },
    /// Run a minimal local application server.
    Serve {
        /// Host/IP bind.
        host: String,
        /// TCP port bind.
        port: u16,
    },
}

/// Parse argv-like arguments into a command.
pub fn parse_args(args: &[String]) -> Option<Command> {
    match args {
        [_, cmd, path] if cmd == "verify" => Some(Command::Verify {
            bundle_path: path.clone(),
        }),
        [_, cmd, sub, id] if cmd == "manifest" && sub == "inspect" => {
            Some(Command::ManifestInspect {
                manifest_id: id.clone(),
            })
        },
        [_, cmd, sub] if cmd == "audit" && sub == "verify" => Some(Command::AuditVerify),
        [_, cmd, sub] if cmd == "calendar" && sub == "generate" => Some(Command::CalendarGenerate),
        [_, cmd] if cmd == "start" => Some(Command::Start {
            profile: "desktop".to_owned(),
        }),
        [_, cmd, profile] if cmd == "start" => Some(Command::Start {
            profile: profile.clone(),
        }),
        [_, cmd] if cmd == "serve" => Some(Command::Serve {
            host: "127.0.0.1".to_owned(),
            port: 8787,
        }),
        [_, cmd, host, port] if cmd == "serve" => {
            let Ok(port) = port.parse::<u16>() else {
                return None;
            };
            Some(Command::Serve {
                host: host.clone(),
                port,
            })
        },
        _ => None,
    }
}

/// Execute command from argv-like input and return process-style exit code.
pub fn execute(args: &[String]) -> i32 {
    let Some(command) = parse_args(args) else {
        eprintln!(
            "usage: prismatik <verify|manifest inspect|audit verify|calendar generate|start|serve>"
        );
        return 2;
    };

    match command {
        Command::Verify { bundle_path } => {
            println!("verify requested for bundle: {bundle_path}");
            0
        },
        Command::ManifestInspect { manifest_id } => {
            println!("manifest inspect requested for id: {manifest_id}");
            0
        },
        Command::AuditVerify => {
            println!("audit verification requested");
            0
        },
        Command::CalendarGenerate => {
            println!("calendar generate requested");
            0
        },
        Command::Start { profile } => run_start(profile),
        Command::Serve { host, port } => run_serve(host, port),
    }
}

fn run_start(profile: String) -> i32 {
    let app = DefaultPrismatikApp;
    match app.start(&AppConfig {
        profile_id: profile,
        bind_address: None,
    }) {
        Ok(graph) => {
            println!("prismatik bootstrap started");
            println!("task_count={}", graph.tasks.len());
            for task in graph.tasks {
                println!(
                    "task={} kind={:?} trigger={:?}",
                    task.id, task.kind, task.trigger
                );
            }
            0
        },
        Err(error) => {
            eprintln!("startup failed: {}", error.message);
            1
        },
    }
}

fn run_serve(host: String, port: u16) -> i32 {
    let bind = format!("{host}:{port}");
    let app = DefaultPrismatikApp;
    let Ok(graph) = app.start(&AppConfig {
        profile_id: "desktop".to_owned(),
        bind_address: Some(bind.clone()),
    }) else {
        eprintln!("startup failed before serve");
        return 1;
    };

    println!(
        "prismatik server bootstrapped with {} tasks",
        graph.tasks.len()
    );
    let listener = match TcpListener::bind(&bind) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("failed to bind {bind}: {error}");
            return 1;
        },
    };
    println!("listening on http://{bind}");

    loop {
        let (mut stream, _) = match listener.accept() {
            Ok(pair) => pair,
            Err(error) => {
                eprintln!("accept error: {error}");
                continue;
            },
        };
        if let Err(error) = respond_once(&mut stream, graph.tasks.len()) {
            eprintln!("request handling error: {error}");
        }
    }
}

fn respond_once(stream: &mut TcpStream, task_count: usize) -> std::io::Result<()> {
    let mut buffer = [0_u8; 1024];
    let bytes = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes]);
    let first_line = request.lines().next().unwrap_or_default();
    let (status_line, body) = if first_line.starts_with("GET /health ") {
        ("HTTP/1.1 200 OK", "{\"ok\":true}")
    } else if first_line.starts_with("GET /meta ") {
        (
            "HTTP/1.1 200 OK",
            if task_count >= 3 {
                "{\"status\":\"ready\"}"
            } else {
                "{\"status\":\"degraded\"}"
            },
        )
    } else {
        ("HTTP/1.1 404 Not Found", "{\"error\":\"not_found\"}")
    };

    let response = format!(
        "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_start_defaults_to_desktop() {
        let args = vec!["prismatik".to_owned(), "start".to_owned()];
        assert_eq!(
            parse_args(&args),
            Some(Command::Start {
                profile: "desktop".to_owned()
            })
        );
    }

    #[test]
    fn parse_serve_defaults() {
        let args = vec!["prismatik".to_owned(), "serve".to_owned()];
        assert_eq!(
            parse_args(&args),
            Some(Command::Serve {
                host: "127.0.0.1".to_owned(),
                port: 8787,
            })
        );
    }
}
