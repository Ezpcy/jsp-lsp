use anyhow::{anyhow, Result};
use log::error;
use std::io::Result as IoResult;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, ChildStdout, Command},
};

pub fn check_esentials() -> Result<(&'static str, &'static str, &'static str)> {
    let jar_file =
        "./jdt-language-server/plugins/org.eclipse.equinox.launcher_1.6.400.v20210924-0641.jar";
    match std::fs::exists(jar_file) {
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
        "./jdt-language-server/config_win"
    } else if cfg!(target_os = "macos") {
        "./jdt-language-server/config_macos"
    } else if cfg!(target_os = "linux") {
        "./jdt-language-server/config_linux"
    } else {
        panic!("OS not supported.")
    };

    match std::fs::exists(config_path) {
        Ok(k) => {
            if !k {
                panic!("The config folder for the jdt-language-sever wasn't found")
            }
        }
        Err(_) => {
            panic!("Couldn't acces the jdt-language-server config folder.")
        }
    }
    let lombok = "./lombok.jar";

    match std::fs::exists(lombok) {
        Ok(k) => {
            if !k {
                panic!("The file \'lombok.jar\' wasn't found.")
            }
        }
        Err(_) => panic!("Couldn't acces the \'lombok.jar\' file."),
    }

    Ok((jar_file, config_path, lombok))
}

#[derive(Debug)]
pub struct JavaLspConnection {
    stdin: tokio::sync::Mutex<ChildStdin>,
    stdout: tokio::sync::Mutex<BufReader<ChildStdout>>,
}

impl JavaLspConnection {
    pub async fn new(workspace_path: &str) -> Result<Self> {
        let (jar_file, config_path, lombok_jar) = match check_esentials() {
            Ok(k) => k,
            Err(e) => {
                panic!(
                    "Somehting wen wrong when checking for dependencies: {}",
                    e.to_string()
                )
            }
        };
        let mut child = Command::new("java")
            .args([
                "--Declipse.application=org.eclipse.jdt.ls.core.id1",
                "-Dosgi.bundles.defaultStartLevel=4",
                "-Dosgi.bundles.defaultStartLevel=4 -Declipse.product=org.eclipse.jdt.ls.core.product",
                "-Dosgi.checkConfiguration=true",
                format!("-Dosgi.sharedConfiguration.area={}/{}", config_path, "config.ini").as_str(),
                "-Dosgi.sharedConfiguration.area.readOnly=true",
                "-Dosgi.configuration.cascaded=true",
                "-Xms1G",
                "--add-modules=ALL-SYSTEM",
                "--add-opens",
                "java.base/java.util=ALL-UNNAMED",
                format!("-javaagent:{}", lombok_jar).as_str(),
                "-jar",
                jar_file,
                "-configuration",
                config_path,
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
