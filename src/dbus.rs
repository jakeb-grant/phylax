use std::{collections::HashMap, path::Path, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::UnixStream,
    process,
    sync::mpsc,
};
use zbus::{interface, zvariant::Value};

use crate::{
    authority::{Identity, PolkitError, Result},
    config::SystemConfig,
    events::{AuthenticationAgentEvent, AuthenticationUserEvent},
};

enum AuthResult {
    Success,
    Retry(String),
}

#[derive(Debug)]
pub struct AuthenticationAgent {
    config: SystemConfig,
    sender: mpsc::Sender<AuthenticationAgentEvent>,
    receiver: mpsc::Receiver<AuthenticationUserEvent>,
}

impl AuthenticationAgent {
    pub fn new(
        sender: mpsc::Sender<AuthenticationAgentEvent>,
        receiver: mpsc::Receiver<AuthenticationUserEvent>,
        config: SystemConfig,
    ) -> Self {
        Self {
            sender,
            receiver,
            config,
        }
    }

    async fn authenticate_via_socket(
        &self,
        socket_path: &str,
        user: &str,
        cookie: &str,
        password: &str,
    ) -> std::result::Result<AuthResult, Box<dyn std::error::Error + Send + Sync>> {
        let stream = UnixStream::connect(socket_path).await?;
        let (reader, mut writer) = stream.into_split();

        // Socket protocol: send username first, then cookie
        writer.write_all(user.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.write_all(cookie.as_bytes()).await?;
        writer.write_all(b"\n").await?;

        self.handle_pam_protocol(reader, writer, password).await
    }

    async fn authenticate_via_spawn(
        &self,
        user: &str,
        cookie: &str,
        password: &str,
    ) -> std::result::Result<AuthResult, Box<dyn std::error::Error + Send + Sync>> {
        let mut child = process::Command::new(self.config.get_helper_path())
            .arg(user)
            .env("LC_ALL", "C")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().ok_or("Child did not have stdin")?;
        let stdout = child.stdout.take().ok_or("Child did not have stdout")?;

        // Process protocol: just send cookie (username is passed as arg)
        let mut writer = stdin;
        writer.write_all(cookie.as_bytes()).await?;
        writer.write_all(b"\n").await?;

        self.handle_pam_protocol(stdout, writer, password).await
    }

    async fn handle_pam_protocol<R, W>(
        &self,
        reader: R,
        mut writer: W,
        password: &str,
    ) -> std::result::Result<AuthResult, Box<dyn std::error::Error + Send + Sync>>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut last_info: Option<String> = None;
        let buf_reader = BufReader::new(reader);
        let mut lines = buf_reader.lines();

        while let Some(line) = lines.next_line().await? {
            tracing::debug!("helper stdout: {}", line);

            if let Some(sliced) = line.strip_prefix("PAM_PROMPT_ECHO_OFF") {
                tracing::debug!("received request from helper: '{}'", sliced);
                if sliced.trim() == "Password:" {
                    tracing::debug!("sending password to helper");
                    writer.write_all(password.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                }
            } else if let Some(info) = line.strip_prefix("PAM_TEXT_INFO") {
                let msg = info.trim().to_string();
                tracing::debug!("helper replied with info: {}", msg);
                last_info = Some(msg);
            } else if line.starts_with("FAILURE") {
                let retry_msg = last_info.unwrap_or_else(|| {
                    "Authentication failed. Please try again.".to_string()
                });
                return Ok(AuthResult::Retry(retry_msg));
            } else if line.starts_with("SUCCESS") {
                return Ok(AuthResult::Success);
            }
        }

        writer.flush().await?;
        Ok(AuthResult::Retry("Authentication timed out.".to_string()))
    }
}

#[interface(name = "org.freedesktop.PolicyKit1.AuthenticationAgent")]
impl AuthenticationAgent {
    async fn cancel_authentication(&self, cookie: &str) {
        tracing::debug!("Recieved request to cancel authentication for {}", cookie);
        self.sender
            .send(AuthenticationAgentEvent::Canceled {
                cookie: cookie.to_owned(),
            })
            .await
            .unwrap();
    }

    async fn begin_authentication(
        &mut self,
        action_id: &str,
        message: &str,
        icon_name: &str,
        details: HashMap<String, String>,
        cookie: &str,
        identities: Vec<Identity<'_>>,
    ) -> Result<()> {
        tracing::info!("recieved request to authenticate");
        tracing::debug!(action_id = action_id, message = message, icon_name = icon_name, details = ?details, cookie = cookie, identities = ?identities);

        let mut names: Vec<String> = Vec::new();
        for identity in identities.iter() {
            let details = identity.get_details();
            if identity.get_kind() == "unix-user" {
                let Value::U32(uid) = details["uid"] else {
                    continue;
                };
                if let Ok(Some(u)) = etc_passwd::Passwd::from_uid(uid) {
                    if let Ok(n) = u.name.into_string() {
                        names.push(n);
                    }
                }
            }
        }

        self.sender
            .send(AuthenticationAgentEvent::Started {
                cookie: cookie.to_string(),
                message: message.to_string(),
                names,
            })
            .await
            .map_err(|_| PolkitError::Failed("Failed to send data.".to_string()))?;

        loop {
            match &self.receiver.recv().await.ok_or_else(|| {
                PolkitError::Failed("Failed to recieve data. channel closed".to_string())
            })? {
                AuthenticationUserEvent::Canceled { cookie: c } => {
                    if c == cookie {
                        return Err(PolkitError::Cancelled(
                            "User cancelled the authentication.".to_string(),
                        ));
                    }
                }
                AuthenticationUserEvent::ProvidedPassword {
                    cookie: c,
                    username: user,
                    password: pw,
                } => {
                    if c == cookie {
                        let socket_path = self.config.get_socket_path();
                        let auth_result = if Path::new(socket_path).exists() {
                            tracing::debug!("using socket-based authentication at {}", socket_path);
                            self.authenticate_via_socket(socket_path, user, cookie, pw).await
                        } else {
                            tracing::debug!("socket not found, falling back to direct spawn");
                            self.authenticate_via_spawn(user, cookie, pw).await
                        };

                        match auth_result {
                            Ok(AuthResult::Success) => {
                                tracing::debug!("helper replied with success.");
                                self.sender
                                    .send(AuthenticationAgentEvent::AuthorizationSucceeded {
                                        cookie: cookie.to_string(),
                                    })
                                    .await
                                    .unwrap();
                                return Ok(());
                            }
                            Ok(AuthResult::Retry(msg)) => {
                                tracing::debug!("helper replied with failure.");
                                self.sender
                                    .send(AuthenticationAgentEvent::AuthorizationRetry {
                                        cookie: cookie.to_string(),
                                        retry_message: Some(msg),
                                    })
                                    .await
                                    .unwrap();
                                continue;
                            }
                            Err(e) => {
                                tracing::error!("authentication error: {:?}", e);
                                self.sender
                                    .send(AuthenticationAgentEvent::AuthorizationRetry {
                                        cookie: cookie.to_string(),
                                        retry_message: Some("Authentication error. Please try again.".to_string()),
                                    })
                                    .await
                                    .unwrap();
                                continue;
                            }
                        }
                    }
                }
            }
        }
    }
}
