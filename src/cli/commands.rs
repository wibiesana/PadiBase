use anyhow::Result;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;

use crate::api::router::create_router;
use crate::core::app::AppState;
use crate::core::env::Env;

#[derive(Parser)]
#[command(
    name = "padiBase",
    author = "padiBase Team",
    version = env!("CARGO_PKG_VERSION"),
    about = "PocketBase-inspired Backend-as-a-Service written in Rust",
    long_about = "High performance BaaS in Rust supporting SQLite, MySQL, MariaDB, and PostgreSQL with Realtime SSE, Dynamic Collections, Auth, and Embedded Admin Dashboard."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Data directory where local SQLite and file storage are stored
    #[arg(short, long, default_value = "./padi_data", global = true)]
    pub dir: PathBuf,

    /// Optional database connection URL (sqlite://..., mysql://..., mariadb://..., postgres://...)
    #[arg(long, env = "DATABASE_URL", global = true)]
    pub db_url: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the padiBase web server
    Serve {
        /// The network interface and port to bind to (defaults to APP_HOST:APP_PORT from .env or 127.0.0.1:8090)
        #[arg(long)]
        http: Option<String>,
    },
    /// Manage admin accounts
    Admin {
        #[command(subcommand)]
        action: AdminCommands,
    },
    /// Print padiBase version
    Version,
}

#[derive(Subcommand)]
pub enum AdminCommands {
    /// Create a new super admin account
    Create {
        /// Admin email address
        email: String,
        /// Admin password
        password: String,
    },
}

pub async fn run_cli() -> Result<()> {
    Env::load(".env");
    let cli = Cli::parse();
    let data_dir = cli.dir;
    let db_url = cli.db_url;

    match cli.command.unwrap_or(Commands::Serve { http: None }) {
        Commands::Serve { http } => {
            let state = AppState::new_with_db_url(&data_dir, db_url.as_deref()).await?;
            let db_driver = state.db.driver_name();
            let app_env = Env::get_or("APP_ENV", "development");
            let app_debug = Env::is_debug();
            let resp_fmt = Env::get_or("RESPONSE_FORMAT", "full");
            let app = create_router(state);

            let bind_str = http.unwrap_or_else(|| {
                let host = Env::get_or("APP_HOST", "127.0.0.1");
                let port = Env::get_or("APP_PORT", "8090");
                format!("{}:{}", host, port)
            });

            let addr: SocketAddr = bind_str.parse()?;
            info!(
                "🌾 padiBase running at http://{} [env: {}, debug: {}, db: {}, format: {}]",
                addr, app_env, app_debug, db_driver, resp_fmt
            );
            info!("👉 Admin Dashboard available at http://{}/_/", addr);
            info!("👉 REST API endpoints available at http://{}/api/", addr);

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, app).await?;
        }
        Commands::Admin { action } => match action {
            AdminCommands::Create { email, password } => {
                let state = AppState::new_with_db_url(&data_dir, db_url.as_deref()).await?;
                state.create_admin(&email, &password).await?;
                println!("Successfully created admin account: {}", email);
            }
        },
        Commands::Version => {
            println!("padiBase v{}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
