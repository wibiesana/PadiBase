use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::info;

use super::auth::{generate_token_key, hash_password};
use super::db::{init_db_with_url, json_get_i64, json_get_str, DbPool, Val};
use super::realtime::RealtimeHub;
use super::schema::{generate_id, Admin};

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub realtime: RealtimeHub,
    pub data_dir: PathBuf,
    pub storage_dir: PathBuf,
}

impl AppState {
    pub async fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        Self::new_with_db_url(data_dir, None).await
    }

    pub async fn new_with_db_url<P: AsRef<Path>>(
        data_dir: P,
        db_url: Option<&str>,
    ) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let storage_dir = data_dir.join("storage");
        if !storage_dir.exists() {
            std::fs::create_dir_all(&storage_dir)?;
        }

        let db = init_db_with_url(&data_dir, db_url).await?;
        let realtime = RealtimeHub::new();

        Ok(Self {
            db,
            realtime,
            data_dir,
            storage_dir,
        })
    }

    pub async fn create_admin(&self, email: &str, password: &str) -> Result<Admin> {
        let hash = hash_password(password)?;
        let id = generate_id();
        let token_key = generate_token_key();
        let now = chrono::Utc::now().to_rfc3339();

        let admin = Admin {
            id: id.clone(),
            email: email.to_string(),
            password_hash: hash.clone(),
            token_key: token_key.clone(),
            created: now.clone(),
            updated: now.clone(),
        };

        let sql = if self.db.is_mysql() {
            r#"
            INSERT INTO `_admins` (`id`, `email`, `passwordHash`, `tokenKey`, `created`, `updated`)
            VALUES (?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                `passwordHash` = VALUES(`passwordHash`),
                `tokenKey` = VALUES(`tokenKey`),
                `updated` = VALUES(`updated`)
            "#
        } else {
            r#"
            INSERT INTO `_admins` (`id`, `email`, `passwordHash`, `tokenKey`, `created`, `updated`)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(`email`) DO UPDATE SET
                `passwordHash` = excluded.`passwordHash`,
                `tokenKey` = excluded.`tokenKey`,
                `updated` = excluded.`updated`
            "#
        };

        self.db
            .execute(
                sql,
                &[
                    Val::from(&id),
                    Val::from(email),
                    Val::from(&hash),
                    Val::from(&token_key),
                    Val::from(&now),
                    Val::from(&now),
                ],
            )
            .await?;

        info!("Admin created/updated: {}", email);
        Ok(admin)
    }

    pub async fn get_admin_by_email(&self, email: &str) -> Result<Option<Admin>> {
        let row = self
            .db
            .fetch_optional(
                "SELECT `id`, `email`, `passwordHash`, `tokenKey`, `created`, `updated` FROM `_admins` WHERE `email` = ?",
                &[Val::from(email)],
            )
            .await?;

        Ok(row.map(|r| Admin {
            id: json_get_str(&r, "id"),
            email: json_get_str(&r, "email"),
            password_hash: json_get_str(&r, "passwordHash"),
            token_key: json_get_str(&r, "tokenKey"),
            created: json_get_str(&r, "created"),
            updated: json_get_str(&r, "updated"),
        }))
    }

    pub async fn has_admins(&self) -> Result<bool> {
        let row = self
            .db
            .fetch_one("SELECT COUNT(*) as `total` FROM `_admins`", &[])
            .await?;
        let count = json_get_i64(&row, "total");
        Ok(count > 0)
    }
}
