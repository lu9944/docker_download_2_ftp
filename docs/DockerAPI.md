# Docker Registry V2 API 参考文档（下载镜像）

本文档基于 `repo_sync` 项目代码整理，涵盖了下载 Docker 镜像所需的 Docker Registry V2 API 接口。

## 目录

- [1. 认证相关接口](#1-认证相关接口)
- [2. Manifest 接口](#2-manifest-接口)
- [3. Blob 接口](#3-blob-接口)
- [4. 完整的下载流程](#4-完整的下载流程)
- [5. 数据结构](#5-数据结构)

---

## 1. 认证相关接口

### 1.1 检查 API 版本和认证要求

**接口**: `GET /v2/`

**用途**: 检查 Registry 是否支持 V2 API，并触发认证挑战

**请求示例**:
```http
GET /v2/ HTTP/1.1
Host: registry-1.docker.io
```

**响应**:
```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Bearer realm="https://auth.docker.io/token",service="registry.docker.io"
Content-Length: 0
```

**代码位置**: `src/registry.rs:140-150`

---

### 1.2 获取 Bearer Token

**接口**: 从 `WWW-Authenticate` 头的 `realm` 参数获取

**用途**: 获取用于后续请求的 Bearer Token

**请求示例**:
```http
GET /token?service=registry.docker.io&scope=repository:library/nginx:pull HTTP/1.1
Host: auth.docker.io
Authorization: Basic <base64(username:password)>
```

**响应**:
```json
{
  "token": "eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCJ9...",
  "access_token": "eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in": 3600
}
```

**Scope 参数格式**:
- `pull`: `repository:{name}:pull`
- `push`: `repository:{name}:pull,push`

**代码位置**: `src/registry.rs:134-184`

---

### 1.3 Docker Hub Personal Access Token

对于 Docker Hub 的 Personal Access Token（`dckr_pat_*`），可以直接用作 Bearer Token，无需额外请求。

**代码位置**: `src/registry.rs:135-138`

---

### 1.4 WWW-Authenticate 头解析

从认证响应头解析认证参数：

**格式**:
```
WWW-Authenticate: Bearer realm="https://auth.docker.io/token",service="registry.docker.io",scope="repository:library/nginx:pull"
```

**解析结果**:
- `realm`: Token 服务 URL
- `service`: Registry 服务名
- `scope`: 权限范围

**代码位置**: `src/registry.rs:186-209`

---

## 2. Manifest 接口

### 2.1 获取 Manifest

**接口**: `GET /v2/{name}/manifests/{reference}`

**参数**:
- `name`: 镜像名称（如 `library/nginx`）
- `reference`: tag 或 digest（如 `latest` 或 `sha256:...`）

**请求示例**:
```http
GET /v2/library/nginx/manifests/latest HTTP/1.1
Host: registry-1.docker.io
Authorization: Bearer <token>
Accept: application/vnd.docker.distribution.manifest.v2+json, \
        application/vnd.docker.distribution.manifest.list.v2+json, \
        application/vnd.oci.image.manifest.v1+json, \
        application/vnd.oci.image.index.v1+json
```

**Accept 类型说明**:
| Media Type | 说明 |
|------------|------|
| `application/vnd.docker.distribution.manifest.v2+json` | 单架构 Manifest |
| `application/vnd.docker.distribution.manifest.list.v2+json` | 多架构 Manifest List |
| `application/vnd.oci.image.manifest.v1+json` | OCI 单架构 Manifest |
| `application/vnd.oci.image.index.v1+json` | OCI 多架构 Index |

**响应 - 单架构**:
```json
{
  "schemaVersion": 2,
  "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
  "config": {
    "mediaType": "application/vnd.docker.container.image.v1+json",
    "size": 1234,
    "digest": "sha256:abc123..."
  },
  "layers": [
    {
      "mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
      "size": 56789012,
      "digest": "sha256:def456..."
    }
  ]
}
```

**响应 - Manifest List (多架构)**:
```json
{
  "schemaVersion": 2,
  "mediaType": "application/vnd.docker.distribution.manifest.list.v2+json",
  "manifests": [
    {
      "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
      "size": 1234,
      "digest": "sha256:...",
      "platform": {
        "architecture": "amd64",
        "os": "linux"
      }
    }
  ]
}
```

**多架构处理流程**:
1. 发送 GET 请求，Accept 包含多种类型
2. 如果返回 Manifest List，根据 `Content-Type` 判断
3. 从 `manifests` 数组中选择目标架构（如 `linux/amd64`）
4. 使用选中架构的 `digest` 再次请求获取具体 Manifest

**代码位置**: `src/registry.rs:212-286`

---

## 3. Blob 接口

### 3.1 下载 Blob

**接口**: `GET /v2/{name}/blobs/{digest}`

**用途**: 下载镜像层（layer）或配置（config）

**请求示例**:
```http
GET /v2/library/nginx/blobs/sha256:def456... HTTP/1.1
Host: registry-1.docker.io
Authorization: Bearer <token>
Accept-Encoding: identity
```

**断点续传支持**:
```http
Range: bytes=1024000-
```

**响应**:
```http
HTTP/1.1 200 OK
Content-Length: 56789012
Docker-Content-Digest: sha256:def456...
Content-Type: application/octet-stream

<blob data>
```

**断点续传响应**:
```http
HTTP/1.1 206 Partial Content
Content-Length: 55765012
Content-Range: bytes 1024000-56789012/56789012
```

**416 Range Not Satisfiable 处理**:
如果收到 416 状态码，可能表示：
- 文件已完整下载
- 服务器不支持 Range 请求
- 文件大小不匹配

需要检查 `Content-Range` 头进行判断。

**代码位置**: `src/registry.rs:310-437`

---

### 3.2 获取 Config Blob

Config Blob 是镜像的配置信息（环境变量、入口点等），通过相同的 GET 接口获取。

**接口**: `GET /v2/{name}/blobs/{digest}`

**响应**: JSON 格式的配置对象
```json
{
  "config": {
    "Env": ["PATH=/usr/local/sbin:/usr/local/bin"],
    "Cmd": ["/bin/bash"],
    "Image": "sha256:..."
  },
  "rootfs": {
    "type": "layers",
    "diff_ids": ["sha256:..."]
  }
}
```

**代码位置**: `src/registry.rs:289-307`

---

## 4. 完整的下载流程

### 4.1 下载镜像的步骤

```
1. 解析镜像引用
   └──> parse_image_ref("nginx:latest") => ("registry-1.docker.io", "library/nginx", "latest")

2. 认证
   └──> authenticate("library/nginx", "pull")
       ├─ 尝试 GET /v2/library/nginx/manifests/latest
       ├─ 解析 WWW-Authenticate 头
       └─ 获取 Bearer Token

3. 获取 Manifest
   └──> fetch_manifest("library/nginx", "latest")
       ├─ 发送 GET 请求，指定多个 Accept 类型
       ├─ 如果是 Manifest List，选择目标架构（如 linux/amd64）
       └─ 获取最终的 Manifest

4. 下载 Config Blob
   └──> download_layer("library/nginx", config_digest, path)
       └─ GET /v2/library/nginx/blobs/{config_digest}

5. 下载所有 Layer Blobs
   └─> for layer in manifest.layers:
       └─> download_layer("library/nginx", layer.digest, path)
           ├─ 检查是否有部分下载文件（断点续传）
           ├─ 使用 Range 头支持断点续传
           └─ 流式下载到本地文件
```

**代码位置**: `src/utils.rs` 中的 `download_image` 函数

---

### 4.2 断点续传实现

项目实现了完善的断点续传机制：

**步骤**:
1. **检查部分下载**: 检查目标文件是否存在，获取已下载字节数
2. **Range 请求**: 添加 `Range: bytes={start}-` 头
3. **处理 416 状态**: 如果服务器不支持范围请求，从头重新下载
4. **重试机制**: 最多重试 5 次，指数退避（5s, 10s, 15s, 20s, 25s）

**流程图**:
```
开始下载
  │
  ├─> 文件是否存在？
  │    ├─ 是: 获取文件大小作为起始字节
  │    └─ 否: 从 0 开始
  │
  ├─> 添加 Range 头
  │
  ├─> 发送请求
  │
  ├─> 响应状态
  │    ├─ 206 Partial Content: 追加文件，继续下载
  │    ├─ 200 OK: 服务器不支持 Range，从头下载
  │    ├─ 416 Range Not Satisfiable: 检查 Content-Range
  │    └─ 其他错误: 重试
  │
  └─> 流式写入文件
```

**代码位置**: `src/registry.rs:310-437`

---

### 4.3 流式下载

为避免大文件占用内存，使用流式下载：

**实现**:
```rust
let mut stream = resp.bytes_stream();

while let Some(chunk_result) = stream.next().await {
    let chunk = chunk_result?;
    file.write_all(&chunk).await?;
    total_bytes += chunk.len() as u64;
}
```

**进度显示**: 每下载 10% 显示一次进度

**代码位置**: `src/registry.rs:440-516`

---

## 5. 数据结构

### 5.1 ManifestResponse

```rust
pub struct ManifestResponse {
    pub media_type: Option<String>,          // "application/vnd.docker.distribution.manifest.v2+json"
    pub schema_version: u32,                 // 2
    pub config: Descriptor,                  // Config blob 描述
    pub layers: Vec<Descriptor>,             // Layer blobs 描述
    pub annotations: Option<HashMap<String, String>>,
}
```

---

### 5.2 Descriptor

```rust
pub struct Descriptor {
    pub media_type: Option<String>,          // Media type
    pub size: u64,                           // Blob 大小（字节）
    pub digest: String,                      // SHA256 digest (格式: "sha256:...")
    pub platform: Option<Platform>,          // 仅用于 Manifest List
}
```

---

### 5.3 ManifestListResponse

```rust
pub struct ManifestListResponse {
    pub media_type: Option<String>,          // "application/vnd.docker.distribution.manifest.list.v2+json"
    pub schema_version: Option<u32>,
    pub manifests: Vec<PlatformDescriptor>,   // 各架构的 manifest
}
```

---

### 5.4 PlatformDescriptor

```rust
pub struct PlatformDescriptor {
    pub media_type: Option<String>,
    pub size: u64,
    pub digest: String,
    pub platform: Platform,
}
```

---

### 5.5 Platform

```rust
pub struct Platform {
    pub architecture: String,                // "amd64", "arm64", etc.
    pub os: String,                          // "linux", "windows"
    pub variant: Option<String>,             // "v7", "v8", etc.
}
```

---

### 5.6 TokenResponse

```rust
pub struct TokenResponse {
    pub token: String,                       // Bearer token
    pub access_token: Option<String>,        // 优先使用此字段
    pub expires_in: Option<u64>,             // 过期时间（秒）
}
```

---

### 5.7 AuthConfig

从 `WWW-Authenticate` 头解析：

```rust
pub struct AuthConfig {
    pub realm: String,                       // Token 服务 URL
    pub service: Option<String>,             // Registry 服务名
    pub scope: Option<String>,               // 权限范围
}
```

---

## 6. 常用 HTTP 状态码

| 状态码 | 说明 | 使用场景 |
|--------|------|----------|
| 200 OK | 成功 | Blob 下载、完整响应 |
| 206 Partial Content | 部分内容 | 断点续传响应 |
| 401 Unauthorized | 未认证 | 需要认证 |
| 404 Not Found | 不存在 | Blob 或 Manifest 不存在 |
| 416 Range Not Satisfiable | 范围不可满足 | 断点续传文件已完整或不支持 |

---

## 7. 常见 Registry 地址

| Registry | Host | 说明 |
|----------|------|------|
| Docker Hub | `registry-1.docker.io` | 官方 Docker Hub |
| GitHub Container Registry | `ghcr.io` | GitHub 容器镜像仓库 |
| Google Container Registry | `gcr.io` | Google 容器镜像仓库 |
| AWS Elastic Container Registry | `*.dkr.ecr.*.amazonaws.com` | AWS ECR |
| Azure Container Registry | `*.azurecr.io` | Azure ACR |

---

## 8. 重要提示

1. **Always HTTPS**: 生产环境始终使用 HTTPS
2. **Digest 验证**: 下载完成后应验证 SHA256 digest
3. **流式传输**: 大文件必须使用流式下载，避免内存溢出
4. **超时设置**: 大层下载需要较长的超时时间（项目中设置为 30 分钟）
5. **重试机制**: 网络不稳定时需要重试
6. **User-Agent**: 某些 Registry 可能要求设置 User-Agent 头

---

## 9. 项目中的实际应用

### 9.1 命令行用法

```bash
# 下载镜像为 tar 文件
cargo run -- download nginx:latest nginx.tar

# 同步单个镜像到 Harbor
cargo run -- sync nginx:latest

# 批量同步
cargo run -- sync-batch images.json
```

---

### 9.2 配置文件示例

**registries.json**:
```json
{
  "source_registries": [
    {
      "name": "docker-hub",
      "host": "registry-1.docker.io",
      "username": "your-username",
      "password": "dckr_pat_...",
      "use_mirror": true
    }
  ]
}
```

---

## 10. 参考链接

- [Docker Registry V2 API 规范](https://docs.docker.com/registry/spec/api/)
- [OCI Image Format 规范](https://github.com/opencontainers/image-spec)
