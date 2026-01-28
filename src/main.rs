mod download;
mod registry;
mod tar;
mod types;

use anyhow::{anyhow, Result};
use std::env;
use std::path::PathBuf;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<()> {
    // 从环境变量获取配置
    let image_ref = env::var("IMAGE_REF")
        .map_err(|_| anyhow!("IMAGE_REF environment variable is required"))?;

    let username = env::var("DOCKER_HUB_USERNAME").ok();
    let password = env::var("DOCKER_HUB_TOKEN").ok();

    let output_dir = PathBuf::from("/mnt/download");
    let tar_output_dir = PathBuf::from("/mnt");

    eprintln!("========================================");
    eprintln!("Docker Image Downloader");
    eprintln!("========================================");
    eprintln!("Image: {}", image_ref);
    eprintln!("Output: {}", output_dir.display());
    eprintln!("========================================");

    // 清理并创建输出目录
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).await?;
    }
    fs::create_dir_all(&output_dir).await?;

    // 下载镜像
    eprintln!("\n📥 Starting download...");
    let downloaded_files = download::download_image(
        &image_ref,
        username,
        password,
        &output_dir,
    ).await?;

    eprintln!("\n✅ Download completed!");
    eprintln!("Downloaded {} files:", downloaded_files.len());
    for file in &downloaded_files {
        eprintln!("  - {}", file);
    }

    // 打包成 tar.gz
    eprintln!("\n📦 Creating tar archive...");
    let tar_filename = format!("{}.tar.gz", tar::sanitize_filename(&image_ref));
    let tar_path = tar_output_dir.join(&tar_filename);

    // 删除旧的 tar 文件（如果存在）
    if tar_path.exists() {
        fs::remove_file(&tar_path).await?;
    }

    tar::create_tar_archive(&output_dir, &tar_path)?;

    // 显示文件大小
    let metadata = fs::metadata(&tar_path).await?;
    let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
    eprintln!("Archive size: {:.2} MB", size_mb);

    eprintln!("\n✅ All done! Archive saved to: {}", tar_path.display());

    // 设置 GitHub Actions 输出
    if let Ok(github_output) = env::var("GITHUB_OUTPUT") {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&github_output) {
            writeln!(file, "archive_path={}", tar_path.display())?;
            writeln!(file, "archive_name={}", tar_filename)?;
            writeln!(file, "archive_size={}", metadata.len())?;
        }
    }

    Ok(())
}
