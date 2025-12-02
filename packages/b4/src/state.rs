use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
}

// std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "wbraid-messages".to_string());

impl AppState {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}
