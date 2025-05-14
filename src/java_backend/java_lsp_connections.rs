use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, ChildStdout, Command},
};
use log::{error};
use std::io::Result as IoResult;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct JavaLspConnection {
    stdin: tokio::sync::Mutex<ChildStdin>,
    stdout: tokio::sync::Mutex<BufReader<ChildStdout>>,
}

impl JavaLspConnection {
    pub async fn new(path: String, config_path: String, workspace_path: &str) -> Self {
        let mut child = Command::new("java")
            .args([
                "--jar",
                path.as_str(),
                "--configuration",
                config_path.as_str(),
                "--data",
                workspace_path,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|_| error!("Failed to spawn Java LSP")).unwrap();

        JavaLspConnection {
            stdin: tokio::sync::Mutex::new(
                child
                    .stdin
                    .take()
                    .ok_or_else(|| error!("Error accessing Java LSP process"))
                    .unwrap()
            ),
            stdout: tokio::sync::Mutex::new(BufReader::new(
                child
                    .stdout
                    .take()
                    .ok_or_else(|| error!("Error accessing Java LSP process"))
                    .unwrap()
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
        stdout.read_exact(&mut buffer).await?;
        let message = String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(message)
    }
}
