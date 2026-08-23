use std::process::Stdio;

use thiserror::Error;
use tokio::process::Command;

pub struct FFmpeg;

#[derive(Debug, Error)]
pub enum FFmpegError {
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ffmpeg not in path")]
    FFmpegNotExist,
    #[error("{0}")]
    Command(String),
}

type FFmpegResult<T = ()> = Result<T, FFmpegError>;

impl FFmpeg {
    pub async fn exist() -> FFmpegResult<bool> {
        let status = Command::new("ffmpeg")
            .kill_on_drop(true)
            .arg("-h")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;

        Ok(status.success())
    }

    pub async fn merge_m3u8(
        pl_path: impl Into<String>,
        output_path: impl Into<String>,
    ) -> FFmpegResult {
        // TODO drop ffmpeg stdout and stderr
        let output = Command::new("ffmpeg")
            .kill_on_drop(true)
            .arg("-allowed_extensions")
            .arg("ALL")
            .arg("-i")
            .arg(pl_path.into())
            .arg("-c")
            .arg("copy")
            .arg(output_path.into())
            .arg("-y")
            .output()
            .await?;

        if !output.status.success() {
            return Err(FFmpegError::Command(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ffmpeg_exists() -> FFmpegResult {
        assert!(FFmpeg::exist().await?);

        Ok(())
    }
}
