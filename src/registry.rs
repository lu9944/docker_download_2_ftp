use crate::types::*;
use anyhow::{anyhow, Result};
use base64::Engine;
use reqwest::{header, Client, StatusCode};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::fs::File;
use futures_util::stream::StreamExt;

/// Docker Registry 客户端
pub struct RegistryClient {
    client: Client,
    registry: String,
    username: Option<String>,
    password: Option<String>,
    token: Option<String>,
}

impl RegistryClient {
    /// 创建新的 Registry 客户端
    pub fn new(
        registry: String,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(1800)) // 30分钟超时
            .build()?;

        Ok(Self {
            client,
            registry,
            username,
            password,
            token: None,
        })
    }

    /// 获取认证 Token
    pub async fn authenticate(&mut self, repository: &str, scope: &str) -> Result<String> {
        // 注意：PAT token 不能直接用作 Registry API 的 Bearer token
        // 它需要通过 Docker Hub 的认证服务来获取 Registry token
        // 所以我们继续下面的正常流程，使用 PAT token 作为 Basic Auth 凭证

        // 触发认证挑战
        let url = format!("https://{}/v2/", self.registry);
        let resp = self.client.get(&url).send().await?;

        if resp.status() != StatusCode::UNAUTHORIZED {
            // 无需认证
            return Ok(String::new());
        }

        // 解析 WWW-Authenticate 头
        let auth_header = resp
            .headers()
            .get(header::WWW_AUTHENTICATE)
            .ok_or_else(|| anyhow!("Missing WWW-Authenticate header"))?
            .to_str()?;

        let auth_config = parse_www_authenticate(auth_header)?;

        // 构建请求 URL
        let mut token_url = format!("{}?service={}", auth_config.realm, auth_config.service.unwrap_or_default());

        // 使用传入的 scope 参数
        if !scope.is_empty() {
            token_url.push_str(&format!("&scope={}", scope));
        }

        // 发送认证请求
        let mut req = self.client.get(&token_url);

        // 添加 Basic Auth（如果有用户名密码）
        // 注意：如果没有用户名密码，Docker Hub 会返回匿名 token（用于公开镜像）
        //
        // Docker Hub Access Token 的正确使用方式：
        // - 使用 Access Token 时，用户名应该为空，密码是 token
        // - 检测 token 的特征（通常很长，>50 字符）
        if let Some(password) = &self.password {
            let credentials = if let Some(username) = &self.username {
                // 如果密码看起来像 Access Token（很长），忽略用户名
                if password.len() > 50 {
                    format!(":{}", password)  // 空用户名 + token 作为密码
                } else {
                    format!("{}:{}", username, password)  // 用户名 + 密码
                }
            } else {
                // 没有用户名，只有密码（可能是 token）
                format!(":{}", password)
            };
            let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
            req = req.header(header::AUTHORIZATION, format!("Basic {}", encoded));
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get token: {} - {}", status, error_text));
        }

        let token_resp: TokenResponse = resp.json().await?;

        let token = token_resp.access_token.unwrap_or(token_resp.token);
        self.token = Some(token.clone());

        Ok(token)
    }

    /// 获取 Manifest
    pub async fn fetch_manifest(
        &self,
        repository: &str,
        reference: &str,
    ) -> Result<(String, serde_json::Value)> {
        let url = format!(
            "https://{}/v2/{}/manifests/{}",
            self.registry, repository, reference
        );

        let accept = "application/vnd.docker.distribution.manifest.v2+json, \
                     application/vnd.docker.distribution.manifest.list.v2+json, \
                     application/vnd.oci.image.manifest.v1+json, \
                     application/vnd.oci.image.index.v1+json";

        let mut req = self.client.get(&url).header(header::ACCEPT, accept);

        if let Some(token) = &self.token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to fetch manifest: {} - {}", status, error_text));
        }

        let content_type = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = resp.text().await?;
        let json: serde_json::Value = serde_json::from_str(&body)?;

        Ok((content_type, json))
    }

    /// 下载 Blob（支持断点续传）
    pub async fn download_blob(
        &self,
        repository: &str,
        digest: &str,
        output_path: &std::path::Path,
    ) -> Result<()> {
        let url = format!(
            "https://{}/v2/{}/blobs/{}",
            self.registry, repository, digest
        );

        // 检查是否有部分下载的文件
        let start_byte = if output_path.exists() {
            let metadata = tokio::fs::metadata(output_path).await?;
            metadata.len()
        } else {
            0
        };

        let mut req = self.client.get(&url);

        if let Some(token) = &self.token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        // 断点续传
        if start_byte > 0 {
            req = req.header(header::RANGE, format!("bytes={}-", start_byte));
        }

        // 禁止自动解压缩，保持原始数据
        req = req.header(header::ACCEPT_ENCODING, "identity");

        let resp = req.send().await?;
        let status = resp.status();

        match status {
            StatusCode::OK | StatusCode::PARTIAL_CONTENT => {
                let mut file = if start_byte > 0 && status == StatusCode::PARTIAL_CONTENT {
                    // 追加模式
                    File::options().append(true).open(output_path).await?
                } else {
                    // 新建或覆盖
                    File::create(output_path).await?
                };

                let mut stream = resp.bytes_stream();
                let mut total_bytes = start_byte;

                while let Some(chunk_result) = stream.next().await {
                    let chunk = chunk_result?;
                    file.write_all(&chunk).await?;
                    total_bytes += chunk.len() as u64;

                    // 每 10% 显示一次进度
                    if total_bytes % (10 * 1024 * 1024) == 0 {
                        eprintln!("Downloaded: {} MB", total_bytes / (1024 * 1024));
                    }
                }

                file.flush().await?;
                eprintln!("Blob completed: {} ({} bytes)", digest, total_bytes);
            }
            StatusCode::RANGE_NOT_SATISFIABLE => {
                // 文件可能已经完整下载
                eprintln!("Range not satisfiable, file may already be complete");
            }
            _ => {
                return Err(anyhow!("Failed to download blob: {}", status));
            }
        }

        Ok(())
    }
}

/// 解析 WWW-Authenticate 头
fn parse_www_authenticate(header: &str) -> Result<AuthConfig> {
    // 格式: Bearer realm="...",service="...",scope="..."
    let header = header.strip_prefix("Bearer ").unwrap_or(header);

    let mut realm = String::new();
    let mut service = None;
    let mut scope = None;

    for part in header.split(',') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');

            match key {
                "realm" => realm = value.to_string(),
                "service" => service = Some(value.to_string()),
                "scope" => scope = Some(value.to_string()),
                _ => {}
            }
        }
    }

    if realm.is_empty() {
        return Err(anyhow!("Missing realm in WWW-Authenticate header"));
    }

    Ok(AuthConfig { realm, service, scope })
}

/// 解析镜像引用
pub fn parse_image_ref(image_ref: &str) -> Result<ImageReference> {
    // 默认值
    let default_registry = "registry-1.docker.io";
    let default_reference = "latest";

    // 分离 registry 部分（如果有）
    let (registry, rest) = match image_ref.split_once('/') {
        Some((first, rest)) if first.contains('.') || first.contains(':') => (first, rest),
        _ => (default_registry, image_ref),
    };

    // 分离 tag 部分
    let (repository, reference) = match rest.rsplit_once(':') {
        Some((repo, tag)) => (repo, tag),
        None => (rest, default_reference),
    };

    // 添加 library/ 前缀（Docker Hub 官方镜像需要）
    let repository = if registry == "registry-1.docker.io" && !repository.contains('/') {
        format!("library/{}", repository)
    } else {
        repository.to_string()
    };

    let result = ImageReference {
        registry: registry.to_string(),
        repository: repository.to_string(),
        reference: reference.to_string(),
    };

    Ok(result)
}
