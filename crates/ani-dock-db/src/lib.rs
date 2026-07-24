use ani_dock_core::constant::DB_FILE_PATH;
use tokio::fs;

pub mod error;
pub async fn ensure_db_file_exist() -> Result<(), std::io::Error> {
    let db_parent_dir = DB_FILE_PATH.parent().ok_or(std::io::Error::new(
        std::io::ErrorKind::AddrNotAvailable,
        format!("无法找到 db 目录的上级目录 {}", DB_FILE_PATH.display()),
    ))?;

    fs::create_dir_all(db_parent_dir).await?;

    todo!();

    Ok(())
}
