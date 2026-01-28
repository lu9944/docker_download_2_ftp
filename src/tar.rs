use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::path::Path;
use tar::Builder;

/// 将目录打包成 tar.gz 文件
pub fn create_tar_archive(input_dir: &Path, output_file: &Path) -> Result<()> {
    eprintln!("Creating tar archive: {}", output_file.display());

    // 创建输出文件
    let output = File::create(output_file)?;
    let gz_encoder = GzEncoder::new(output, Compression::default());
    let mut tar_builder = Builder::new(gz_encoder);

    // 遍历目录并添加所有文件到 tar
    add_dir_to_tar(&mut tar_builder, input_dir, "")?;

    // 完成 tar 构建
    tar_builder.finish()?;

    eprintln!("Tar archive created successfully!");

    Ok(())
}

/// 递归添加目录到 tar
fn add_dir_to_tar<W: std::io::Write>(
    tar_builder: &mut Builder<GzEncoder<W>>,
    dir: &Path,
    prefix: &str,
) -> Result<()> {
    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();

        // 构建归档中的路径
        let tar_path = if prefix.is_empty() {
            name.to_string_lossy().to_string()
        } else {
            format!("{}/{}", prefix, name.to_string_lossy())
        };

        if path.is_dir() {
            // 递归处理子目录
            add_dir_to_tar(tar_builder, &path, &tar_path)?;
        } else {
            // 添加文件到 tar
            eprintln!("Adding to archive: {}", tar_path);
            let mut file = File::open(&path)?;
            tar_builder.append_file(&tar_path, &mut file)?;
        }
    }

    Ok(())
}

/// 从镜像引用生成安全的文件名
pub fn sanitize_filename(image_ref: &str) -> String {
    image_ref
        .replace(':', "_")
        .replace('/', "_")
        .replace('\\', "_")
}
