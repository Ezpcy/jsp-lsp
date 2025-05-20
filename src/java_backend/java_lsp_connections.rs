use anyhow::{anyhow, Result};
use log::error;
use serde_json::json;
use std::io::{stdin, stdout, Result as IoResult};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, ChildStdout, Command},
};
use tower_lsp::{
    lsp_types::{MessageType, Url},
    Client,
};

pub enum JavaLspMethod {
    DidOpen,
    DidChange,
}

impl JavaLspMethod {
    fn value(&self) -> &str {
        match *self {
            JavaLspMethod::DidOpen => "textDocument/didOpen",
            JavaLspMethod::DidChange => "textDocument/didChange",
        }
    }
}

pub fn check_esentials() -> Result<(String, String, String)> {
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let jar_file = exe_dir.join(
        "jdt-language-server/plugins/org.eclipse.equinox.launcher_1.6.400.v20210924-0641.jar",
    );
    match std::fs::exists(&jar_file) {
        Ok(k) => {
            if !k {
                panic!("Jdtls launcher jar file not found.")
            }
        }
        Err(_) => {
            panic!("Couldn't acces the jdt-language-server folder.")
        }
    }

    let config_path = if cfg!(target_os = "windows") {
        exe_dir.join("./jdt-language-server/config_win")
    } else if cfg!(target_os = "macos") {
        exe_dir.join("./jdt-language-server/config_macos")
    } else if cfg!(target_os = "linux") {
        exe_dir.join("./jdt-language-server/config_linux")
    } else {
        panic!("OS not supported.")
    };

    match std::fs::exists(&config_path) {
        Ok(k) => {
            if !k {
                panic!("The config folder for the jdt-language-sever wasn't found")
            }
        }
        Err(_) => {
            panic!("Couldn't acces the jdt-language-server config folder.")
        }
    }
    let lombok = exe_dir.join("lombok.jar");

    match std::fs::exists(&lombok) {
        Ok(k) => {
            if !k {
                panic!("The file \'lombok.jar\' wasn't found.")
            }
        }
        Err(_) => panic!("Couldn't acces the \'lombok.jar\' file."),
    }

    let jar_file_str = String::from(jar_file.to_str().unwrap());
    let config_path_str = String::from(config_path.to_str().unwrap());
    let lombok_str = String::from(lombok.to_str().unwrap());

    Ok((jar_file_str, config_path_str, lombok_str))
}

#[derive(Debug)]
pub struct JavaLspConnection {
    stdin: tokio::sync::Mutex<ChildStdin>,
    stdout: tokio::sync::Mutex<BufReader<ChildStdout>>,
}

impl JavaLspConnection {
    pub async fn new(client: &Client, workspace_path: &str) -> Result<Self> {
        let (jar_file, config_path, lombok_jar) = match check_esentials() {
            Ok(k) => k,
            Err(e) => {
                panic!(
                    "Somehting went wrong when checking for dependencies: {}",
                    e.to_string()
                )
            }
        };
        let mut child = Command::new("java")
            .args([
                "-Declipse.application=org.eclipse.jdt.ls.core.id1",
                "-Dosgi.bundles.defaultStartLevel=4",
                "-Declipse.product=org.eclipse.jdt.ls.core.product",
                "-Dosgi.checkConfiguration=true",
                &format!("-Dosgi.sharedConfiguration.area={}", config_path),
                "-Dosgi.sharedConfiguration.area.readOnly=true",
                "-Dosgi.configuration.cascaded=true",
                "-Xms1G",
                "--add-modules=ALL-SYSTEM",
                "--add-opens",
                "java.base/java.util=ALL-UNNAMED",
                &format!("-javaagent:{}", lombok_jar),
                "-jar",
                &jar_file,
                "-configuration",
                &config_path,
                "-data",
                &workspace_path,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());

        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": null,
                "capabilities": {}
            }
        });
        let init_str = serde_json::to_string(&init)?;

        let header = format!("Content-Length: {}\r\n\r\n", init_str.len());
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(init_str.as_bytes()).await?;
        stdin.flush().await?;

        let mut content_length = None;

        loop {
            let mut line = String::new();
            stdout.read_line(&mut line).await?;
            if line.trim().is_empty() {
                break;
            };
            if line.starts_with("Content-Length:") {
                let len = line["Content-Length:".len()..].trim().parse::<usize>()?;
                content_length = Some(len);
            }
        }
        let len = content_length.expect("No Content-Length found");
        let mut buf = vec![0; len];
        stdout.read_exact(&mut buf).await?;
        client
            .log_message(MessageType::INFO, String::from_utf8_lossy(&buf))
            .await;

        let init_notif = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        let notif_str = serde_json::to_string(&init_notif)?;
        let header = format!("Content-Length: {}\r\n\r\n", notif_str.len());
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(notif_str.as_bytes()).await?;
        stdin.flush().await?;

        Ok(JavaLspConnection {
            stdin: tokio::sync::Mutex::new(stdin),
            stdout: tokio::sync::Mutex::new(stdout),
        })
    }

    pub async fn send_message(&self, msg: &str, uri: &Url, method: JavaLspMethod) -> Result<()> {
        let mut stdin = self.stdin.lock().await;
        let json = json!({
            "jsonrpc": "2.0",
            "method": method.value(),
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "java",
                "version": 1,
                "text": msg,
            }
        }
        });
        let msg = serde_json::to_string(&json)?;
        let header = format!("Content-Length: {}\r\n\r\n", msg.len());
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(msg.as_bytes()).await?;
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
