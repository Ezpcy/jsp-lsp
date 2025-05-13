use serde_json::json;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, ChildStdout, Command},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct JavaLspConnection {
    stdin: tokio::sync::Mutex<ChildStdin>,
    stdout: tokio::sync::Mutex<BufReader<ChildStdout>>,
}

impl JavaLspConnection {
    pub async fn new(path: &str, config_path: &str, workspace_path: &str) -> Self {
        let mut child = Command::new("java")
            .args([
                "--jar",
                path,
                "--configuration",
                config_path,
                "--data",
                workspace_path,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn Java LSP");

        JavaLspConnection {
            stdin: tokio::sync::Mutex::new(
                child
                    .stdin
                    .take()
                    .expect("Error accessing Java LSP process"),
            ),
            stdout: tokio::sync::Mutex::new(BufReader::new(
                child
                    .stdout
                    .take()
                    .expect("Error accessing Java LSP process"),
            )),
        }
    }

    pub async fn send_message(&self, msg: &str) -> Result<()> {
        let mut stdin = self.stdin.lock().await;
        let json = serde_json::to_string(msg)?;
        let content = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
        stdin.write_all(content.as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    pub async fn read_message(&self) -> Result<String> {
        let mut stdout = self.stdout.lock().await;
        let mut content_length = None;

        loop {
            let mut line = String::new();
            let bytes_read = stdout.read_line(&mut line).await?;
            if bytes_read == 0 {
                return Err("Unexpected EOF while reading headers".into());
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if line.starts_with("Content-Length:") {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    content_length = Some(parts[1].trim().parse::<usize>()?);
                }
            }
        }

        let len = content_length.ok_or("No Content-Length header found")?;
        let mut buffer = vec![0; len];
        stdout.(&mut buffer).await?;
        let json = String::from_utf8(buffer)?;
        Ok(json)
    }
}
