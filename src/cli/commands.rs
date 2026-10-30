use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::api::router::create_router;
use crate::core::api_collection::{build_all_collections_json, build_single_collection_json};
use crate::core::app::AppState;
use crate::core::db::{get_collection_by_name_or_id, list_collections};
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
    /// Export REST Client API Collection (Postman v2.1.0 JSON format compatible with Postman, Insomnia, Thunder Client, Bruno, and VS Code REST Client)
    #[command(name = "api-collection", alias = "export-api-collection")]
    ApiCollection {
        /// Specific collection name to export (e.g. users). If omitted, exports all collections.
        #[arg(short, long)]
        collection: Option<String>,

        /// Destination directory to save generated JSON collection files
        #[arg(short, long, default_value = "./api_collection")]
        output: PathBuf,

        /// Base URL variable value in the collection
        #[arg(long, default_value = "http://127.0.0.1:8090")]
        base_url: String,

        /// Overwrite existing files without asking
        #[arg(short, long)]
        force: bool,
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

async fn export_api_collection_cli(
    state: &AppState,
    collection_name: Option<&str>,
    output_dir: &Path,
    base_url: &str,
    force: bool,
) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    println!("\x1b[32m🌾 PadiBase REST Client API Collection Exporter\x1b[0m");
    println!("{}", "=".repeat(60));

    if let Some(col_name) = collection_name {
        let col = get_collection_by_name_or_id(&state.db, col_name)
            .await?
            .ok_or_else(|| anyhow!("Collection '{}' not found in database.", col_name))?;

        let file_path = output_dir.join(format!("{}_collection.json", col.name));
        if file_path.exists() && !force {
            println!(
                "\x1b[33mℹ API Collection for '{}' already exists at {:?}. Skipping (use --force to overwrite).\x1b[0m",
                col.name, file_path
            );
            return Ok(());
        }

        let json_val = build_single_collection_json(&col, base_url);
        let formatted = serde_json::to_string_pretty(&json_val)?;
        fs::write(&file_path, formatted)?;

        println!("\x1b[32m✓ Single Collection exported:\x1b[0m {:?}", file_path);
    } else {
        let cols = list_collections(&state.db).await?;

        // 1. Export master full collection
        let full_path = output_dir.join("padi_base_api_collection.json");
        if !full_path.exists() || force {
            let full_json = build_all_collections_json(&cols, base_url);
            let formatted = serde_json::to_string_pretty(&full_json)?;
            fs::write(&full_path, formatted)?;
            println!("\x1b[32m✓ Full System API Collection saved:\x1b[0m {:?}", full_path);
        } else {
            println!(
                "\x1b[33mℹ Full API Collection already exists at {:?}. Skipping (use --force to overwrite).\x1b[0m",
                full_path
            );
        }

        // 2. Export individual collection files (just like Padi Rust)
        let mut count = 0;
        for col in &cols {
            let single_path = output_dir.join(format!("{}_collection.json", col.name));
            if !single_path.exists() || force {
                let single_json = build_single_collection_json(col, base_url);
                let formatted = serde_json::to_string_pretty(&single_json)?;
                fs::write(&single_path, formatted)?;
                println!("\x1b[32m✓ Collection '{}' saved:\x1b[0m {:?}", col.name, single_path);
                count += 1;
            } else {
                println!(
                    "\x1b[33mℹ Collection '{}' already exists at {:?}. Skipping.\x1b[0m",
                    col.name, single_path
                );
            }
        }

        println!("{}", "=".repeat(60));
        println!(
            "\x1b[32m🎉 Export completed! {} collection file(s) generated in {:?}\x1b[0m",
            count + 1,
            output_dir
        );
        println!(
            "\x1b[36m💡 You can import these files directly into Postman, Insomnia, Thunder Client, Bruno, or VS Code REST Client.\x1b[0m\n"
        );
    }

    Ok(())
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
            info!("👉 Admin Dashboard available at http://{}/dashboard/", addr);
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
        Commands::ApiCollection {
            collection,
            output,
            base_url,
            force,
        } => {
            let state = AppState::new_with_db_url(&data_dir, db_url.as_deref()).await?;
            export_api_collection_cli(&state, collection.as_deref(), &output, &base_url, force).await?;
        }
        Commands::Version => {
            println!("padiBase v{}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
