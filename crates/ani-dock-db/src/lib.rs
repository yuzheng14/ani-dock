use ani_dock_core::constant::DB_FILE_PATH;
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tokio::fs;

mod input;
pub mod model;
pub mod repository;

pub type CoreAnime = ani_dock_core::Anime;
pub type CoreEpisode = ani_dock_core::Episode;

pub async fn ensure_db_dir_exist() -> Result<(), std::io::Error> {
    let db_parent_dir = DB_FILE_PATH.parent().ok_or(std::io::Error::new(
        std::io::ErrorKind::AddrNotAvailable,
        format!("无法找到 db 目录的上级目录 {}", DB_FILE_PATH.display()),
    ))?;

    fs::create_dir_all(db_parent_dir).await?;

    Ok(())
}

pub async fn get_conn_pool() -> Result<Pool<Sqlite>, sqlx::Error> {
    // to avoid error like
    //
    // ```text
    // 2026-08-05T13:31:47.741289Z ERROR ani_dock_server: 63: 启动服务器发生错误 error=error returned from database: (code: 14) unable to open database file
    // Error: Database(SqliteError { code: 14, message: "unable to open database file" })
    // ```
    ensure_db_dir_exist().await?;

    let conn_option = SqliteConnectOptions::new()
        .create_if_missing(true)
        .filename(DB_FILE_PATH.as_path())
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(conn_option)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}
