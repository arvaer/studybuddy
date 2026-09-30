use sqlx::PgPool;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub pool:        PgPool,
    pub jwt_secret:  String,
    pub uploads_dir: PathBuf,
}
