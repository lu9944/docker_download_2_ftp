use serde::{Deserialize, Serialize};

/// Docker Registry V2 Manifest 响应
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestResponse {
    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub config: Descriptor,
    pub layers: Vec<Descriptor>,
    pub annotations: Option<std::collections::HashMap<String, String>>,
}

/// Docker Registry V2 Manifest List 响应（多架构）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestListResponse {
    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,
    pub schema_version: Option<u32>,
    pub manifests: Vec<PlatformDescriptor>,
}

/// Blob 描述符
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Descriptor {
    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,
    pub size: u64,
    pub digest: String,
    pub platform: Option<Platform>,
}

/// 平台描述符（用于 Manifest List）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformDescriptor {
    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,
    pub size: u64,
    pub digest: String,
    pub platform: Platform,
}

/// 平台信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Platform {
    pub architecture: String,
    pub os: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

/// Bearer Token 响应
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenResponse {
    pub token: String,
    #[serde(rename = "access_token")]
    pub access_token: Option<String>,
    #[serde(rename = "expires_in")]
    pub expires_in: Option<u64>,
}

/// 从 WWW-Authenticate 头解析的认证配置
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub realm: String,
    pub service: Option<String>,
    pub scope: Option<String>,
}

/// 镜像引用解析结果
#[derive(Debug, Clone)]
pub struct ImageReference {
    pub registry: String,
    pub repository: String,
    pub reference: String,
}

/// 下载进度信息
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub current_layer: usize,
    pub total_layers: usize,
}
