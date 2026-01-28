use crate::registry::{parse_image_ref, RegistryClient};
use crate::types::ManifestResponse;
use anyhow::{anyhow, Result};
use std::path::Path;
use tokio::fs;

/// 下载完整的 Docker 镜像
pub async fn download_image(
    image_ref: &str,
    username: Option<String>,
    password: Option<String>,
    output_dir: &Path,
) -> Result<Vec<String>> {
    // 1. 解析镜像引用
    eprintln!("Parsing image reference: {}", image_ref);
    let image = parse_image_ref(image_ref)?;

    eprintln!("Registry: {}", image.registry);
    eprintln!("Repository: {}", image.repository);
    eprintln!("Reference: {}", image.reference);

    // 2. 创建 Registry 客户端并认证
    let scope = format!("repository:{}:pull", image.repository);
    let mut client = RegistryClient::new(image.registry.clone(), username, password)?;

    eprintln!("Authenticating...");
    client.authenticate(&image.repository, &scope).await?;

    // 3. 获取 Manifest
    eprintln!("Fetching manifest...");
    let (content_type, manifest_json) =
        client.fetch_manifest(&image.repository, &image.reference).await?;

    // 4. 判断是否为 Manifest List（多架构）
    let manifest = if content_type.contains("manifest.list") || content_type.contains("index.v1") {
        eprintln!("Manifest List detected, selecting linux/amd64...");
        let digest = select_manifest_for_platform(&manifest_json, "linux", "amd64", None).await?;

        // 使用选中的 digest 重新请求完整的 manifest
        eprintln!("Fetching specific manifest for linux/amd64...");
        let (_, specific_manifest) = client.fetch_manifest(&image.repository, &digest).await?;
        specific_manifest
    } else {
        manifest_json
    };

    // 5. 解析 Manifest
    let manifest: ManifestResponse = serde_json::from_value(manifest.clone())?;

    // 6. 创建输出目录
    let blobs_dir = output_dir.join("blobs");
    fs::create_dir_all(&blobs_dir).await?;

    let mut downloaded_files = Vec::new();

    // 7. 下载 Config
    eprintln!("Downloading config: {}", manifest.config.digest);
    let config_path = blobs_dir.join(&manifest.config.digest);
    client
        .download_blob(&image.repository, &manifest.config.digest, &config_path)
        .await?;
    downloaded_files.push(config_path.to_string_lossy().to_string());

    // 8. 下载所有 Layers
    eprintln!("Downloading {} layers...", manifest.layers.len());
    for (idx, layer) in manifest.layers.iter().enumerate() {
        eprintln!(
            "Layer {}/{}: {} ({} bytes)",
            idx + 1,
            manifest.layers.len(),
            layer.digest,
            layer.size
        );

        let layer_path = blobs_dir.join(&layer.digest);
        client
            .download_blob(&image.repository, &layer.digest, &layer_path)
            .await?;

        downloaded_files.push(layer_path.to_string_lossy().to_string());
    }

    eprintln!("All layers downloaded successfully!");

    // 9. 保存 manifest
    let manifest_path = output_dir.join("manifest.json");
    let manifest_content = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, manifest_content).await?;
    downloaded_files.push(manifest_path.to_string_lossy().to_string());

    Ok(downloaded_files)
}

/// 从 Manifest List 中选择指定平台的 Manifest Digest
async fn select_manifest_for_platform(
    manifest_list: &serde_json::Value,
    os: &str,
    arch: &str,
    variant: Option<&str>,
) -> Result<String> {
    let manifests = manifest_list
        .get("manifests")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("Invalid manifest list"))?;

    // 查找匹配的 manifest
    for manifest in manifests {
        if let Some(platform) = manifest.get("platform") {
            let platform_os = platform.get("os").and_then(|v| v.as_str());
            let platform_arch = platform.get("architecture").and_then(|v| v.as_str());
            let platform_variant = platform.get("variant").and_then(|v| v.as_str());

            if platform_os == Some(os) && platform_arch == Some(arch) {
                if variant.is_none() || platform_variant == variant {
                    // 获取 digest
                    let digest = manifest
                        .get("digest")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow!("Missing digest"))?;

                    eprintln!("Selected manifest digest: {}", digest);
                    return Ok(digest.to_string());
                }
            }
        }
    }

    Err(anyhow!(
        "No manifest found for platform: {}/{}{:?}",
        os,
        arch,
        variant
    ))
}
