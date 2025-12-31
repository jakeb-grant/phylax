use authority::{AuthorityProxy, Subject};
use dbus::AuthenticationAgent;
use eyre::{Result, WrapErr, ensure};
use relm4::RelmApp;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use tokio::sync::mpsc::channel;
use tracing::level_filters::LevelFilter;
use zbus::zvariant::Value;

use zbus::conn;

use crate::config::SystemConfig;
use crate::events::{AuthenticationAgentEvent, AuthenticationUserEvent};
use crate::ui::App;

mod authority;
mod config;
mod constants;
mod dbus;
mod events;
mod ui;

fn setup_tracing() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy()
                .add_directive("[start_object_server]=debug".parse()?),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing()?;

    let demo_mode = env::var("PHYLAX_DEMO").is_ok();

    let config_path = std::env::var("XDG_CONFIG_HOME")
        .or(std::env::var("HOME").map(|e| e + "/.config"))
        .context("Could not resolve configuration path")?;
    let css_path = format!("{config_path}/phylax/style.css");
    let path = Path::new(&css_path);

    let (agent_sender, agent_receiver) = channel::<AuthenticationAgentEvent>(32);
    let (user_sender, user_receiver) = channel::<AuthenticationUserEvent>(32);

    // Must persist for app lifetime to keep D-Bus agent alive
    let _connection: Option<zbus::Connection>;

    if demo_mode {
        tracing::info!("Running in demo mode");
        _connection = None;
        agent_sender
            .send(AuthenticationAgentEvent::Started {
                cookie: "demo".to_string(),
                message: "Demo application wants to do something requiring authentication."
                    .to_string(),
                names: vec!["user".to_string(), "root".to_string()],
            })
            .await?;
    } else {
        let config: SystemConfig = SystemConfig::from_file()?;

        ensure!(
            Path::new(config.get_helper_path()).exists(),
            "Authentication helper located at {} does not exist.",
            config.get_helper_path()
        );
        tracing::info!(
            "using authentication helper located at {}",
            config.get_helper_path()
        );

        let locale = gtk4::glib::language_names()[0].as_str().to_string();
        tracing::info!("Registering authentication agent with locale: {}", locale);
        let subject_kind = "unix-session".to_string();

        let subject_details = HashMap::from([(
            "session-id".to_string(),
            Value::new(
                std::env::var("XDG_SESSION_ID")
                    .context("Could not get XDG session id, make sure that it is set and try again.")?,
            ),
        )]);
        let subject = Subject::new(subject_kind, subject_details);

        let agent = AuthenticationAgent::new(agent_sender, user_receiver, config.clone());
        _connection = Some(
            conn::Builder::system()?
                .serve_at(constants::SELF_OBJECT_PATH, agent)?
                .build()
                .await?,
        );

        let proxy = AuthorityProxy::new(_connection.as_ref().unwrap()).await?;
        proxy
            .register_authentication_agent(&subject, &locale, constants::SELF_OBJECT_PATH)
            .await?;

        tracing::info!("Registered as authentication provider.");
    }

    let app = RelmApp::new("io.github.jakebgrant.phylax");
    if path.is_file() {
        tracing::info!("loading css stylesheet from {}", css_path);
        relm4::set_global_css_from_file(path)
            .context("Could not load CSS stylesheet for some reason")?;
    }
    app.run_async::<App>((user_sender, agent_receiver, path.to_path_buf()));

    Ok(())
}
