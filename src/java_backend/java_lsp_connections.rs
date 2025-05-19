use anyhow::{Result, anyhow};
use log::error;
use std::io::Result as IoResult;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, ChildStdout, Command},
};

#[derive(Debug)]
pub struct JavaLspConnection {
    stdin: tokio::sync::Mutex<ChildStdin>,
    stdout: tokio::sync::Mutex<BufReader<ChildStdout>>,
}

impl JavaLspConnection {
    pub async fn new(path: String, config_path: String, workspace_path: &str) -> Result<Self> {
        let mut child = Command::new("java")
            .args([
                "--Declipse.application=org.eclipse.jdt.ls.core.id1",
                "-Dosgi.bundles.defaultStartLevel=4",
                "-Dosgi.bundles.defaultStartLevel=4 -Declipse.product=org.eclipse.jdt.ls.core.product",
                "-Dosgi.checkConfiguration=true",
                // TODO 
                // /home/ezpz/.local/share/nvim/mason/share/jdtls/config for the one below to parse
                format!("-Dosgi.sharedConfiguration.area={}", config_path.as_str()).as_str(),
                "-Dosgi.sharedConfiguration.area.readOnly=true",
                "-Dosgi.configuration.cascaded=true",
                "-Xms1G",
                "--add-modules=ALL-SYSTEM",
                "--add-opens",
                "java.base/java.util=ALL-UNNAMED",
                "-javaagent:lombok.jar",
                "-jar",
                path.as_str(),
                "-configuration",
                config_path.as_str(),
                "-data",
                workspace_path,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        Ok(JavaLspConnection {
            stdin: tokio::sync::Mutex::new(child.stdin.take().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::Other, "Failed to open stdin")
            })?),
            stdout: tokio::sync::Mutex::new(BufReader::new(child.stdout.take().ok_or_else(
                || std::io::Error::new(std::io::ErrorKind::Other, "Failed to open stdout"),
            )?)),
        })
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
                return Err(anyhow!("Unexpected EOF while reading headers"));
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

        let len = content_length.ok_or(anyhow!("No Content-Length header found"))?;
        let mut buffer = vec![0; len];
        stdout.read_exact(&mut buffer).await?;
        let message = String::from_utf8(buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(message)
    }
}
