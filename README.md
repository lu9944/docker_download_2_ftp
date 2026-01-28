# Docker Actions Download

纯 Rust 实现的 Docker 镜像下载工具，通过 Docker Registry V2 API 下载镜像并打包成 tar 文件。专为 GitHub Actions 设计，不依赖任何外部 Docker 命令。

## 功能特点

- ✅ **纯 Rust 实现**: 无需安装 Docker 守护进程
- ✅ **Docker API**: 直接调用 Docker Registry V2 API
- ✅ **断点续传**: 支持大文件中断后继续下载
- ✅ **流式传输**: 高效的内存使用，支持超大镜像
- ✅ **多架构支持**: 自动选择 linux/amd64 架构
- ✅ **认证支持**: 支持 Docker Hub Personal Access Token

## GitHub Actions 使用

### 方式一：作为可重用工作流（推荐）

直接在你的项目中引用此工作流，无需复制代码：

```yaml
name: My CI/CD

on:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  download-docker-image:
    uses: your-org/docker_actions_download/.github/workflows/download-image.yml@master
    with:
      image_ref: nginx:latest
      upload_ftp: false
    secrets:
      DOCKER_HUB_USERNAME: ${{ secrets.DOCKER_HUB_USERNAME }}
      DOCKER_HUB_TOKEN: ${{ secrets.DOCKER_HUB_TOKEN }}
```

#### 高级用法：下载后上传到 FTP

```yaml
jobs:
  download-docker-image:
    uses: your-org/docker_actions_download/.github/workflows/download-image.yml@master
    with:
      image_ref: postgres:16
      upload_ftp: true
      ftp_server: ftp.example.com
      ftp_username: your-username
    secrets:
      DOCKER_HUB_TOKEN: ${{ secrets.DOCKER_HUB_TOKEN }}
      FTP_PASSWORD: ${{ secrets.FTP_PASSWORD }}
```

#### 下载后使用镜像

```yaml
jobs:
  download:
    uses: your-org/docker_actions_download/.github/workflows/download-image.yml@master
    with:
      image_ref: redis:7-alpine
    secrets:
      DOCKER_HUB_TOKEN: ${{ secrets.DOCKER_HUB_TOKEN }}

  load:
    needs: download
    runs-on: ubuntu-latest
    steps:
      - name: Download artifact
        uses: actions/download-artifact@v4
        with:
          name: docker-image-${{ needs.download.outputs.run-number }}
          path: /tmp

      - name: Load Docker image
        run: docker load < /tmp/*.tar.gz
```

### 方式二：手动触发（在本仓库）

#### 1. 配置 Secrets

在 GitHub 仓库设置中添加 Secrets：
- **Settings** → **Secrets and variables** → **Actions** → **New repository secret**

添加以下 secrets（可选）：
- `DOCKER_HUB_USERNAME`: Docker Hub 用户名
- `DOCKER_HUB_TOKEN`: Docker Hub Access Token

### 2. 触发 Workflow

进入 **Actions** → **Download Docker Image** → **Run workflow**

输入参数：
- **image_ref**: Docker 镜像引用（如 `nginx:latest`）
- **upload_ftp**: 是否上传到 FTP（可选）
- **ftp_server**: FTP 服务器地址（可选）

### 3. 使用示例

#### 手动触发下载

```yaml
- name: Download Docker image
  env:
    IMAGE_REF: nginx:latest
    DOCKER_HUB_USERNAME: ${{ secrets.DOCKER_HUB_USERNAME }}
    DOCKER_HUB_TOKEN: ${{ secrets.DOCKER_HUB_TOKEN }}
  run: ./target/release/docker-actions-download
```

#### 下载并上传 FTP

```yaml
- name: Download and upload
  env:
    IMAGE_REF: postgres:16
    DOCKER_HUB_TOKEN: ${{ secrets.DOCKER_HUB_TOKEN }}
    FTP_SERVER: ftp.example.com
    FTP_USERNAME: user
    FTP_PASSWORD: ${{ secrets.FTP_PASSWORD }}
  run: |
    ./target/release/docker-actions-download
    curl -T /mnt/*.tar.gz "ftp://${FTP_USERNAME}:${FTP_PASSWORD}@${FTP_SERVER}/"
```

## 本地使用

### 环境变量

```bash
export IMAGE_REF="nginx:latest"
export DOCKER_HUB_USERNAME="your-username"  # 可选
export DOCKER_HUB_TOKEN="dckr_pat_..."      # 可选

# 创建输出目录
sudo mkdir -p /mnt/download
sudo chown -R $USER:$USER /mnt

# 运行
cargo run --release
```

### 输出

- 下载的镜像层: `/mnt/download/blobs/`
- Manifest 文件: `/mnt/download/manifest.json`
- 压缩包: `/mnt/<image>.tar.gz`

### 示例输出

```
========================================
Docker Image Downloader
========================================
Image: nginx:latest
Output: /mnt/download
========================================

Parsing image reference: nginx:latest
Registry: registry-1.docker.io
Repository: library/nginx
Reference: latest

Authenticating...
Fetching manifest...
Downloading config: sha256:abc123...
Downloading 7 layers...
Layer 1/7: sha256:def456... (12345678 bytes)
...
All layers downloaded successfully!

Creating tar archive...
Archive size: 145.23 MB

All done! Archive saved to: /mnt/nginx_latest.tar.gz
```

## 环境变量说明

| 变量 | 必需 | 说明 |
|------|------|------|
| `IMAGE_REF` | ✅ | Docker 镜像引用，如 `nginx:latest` |
| `DOCKER_HUB_USERNAME` | ❌ | Docker Hub 用户名 |
| `DOCKER_HUB_TOKEN` | ❌ | Docker Hub Token (支持 PAT) |
| `GITHUB_OUTPUT` | ❌ | GitHub Actions 输出文件 |

## 支持的镜像格式

- Docker Hub 官方镜像: `nginx:latest`, `postgres:16`, etc.
- 用户镜像: `username/repo:tag`
- 第三方 Registry: `ghcr.io/repo/image:tag`
- Digest 引用: `nginx@sha256:...`

## 技术架构

```
┌─────────────┐
│  main.rs    │  主入口，环境变量解析
└──────┬──────┘
       │
       ├──────────────────────┐
       │                      │
┌──────▼──────┐      ┌────────▼────────┐
│  registry.rs│      │   download.rs   │
│  - 认证     │◄─────│  - 获取 Manifest│
│  - API请求  │      │  - 下载 Layers  │
└──────┬──────┘      └────────┬────────┘
       │                      │
       └──────────────────────┤
                      ┌───────▼────────┐
                      │     tar.rs     │
                      │  - 打包 tar.gz │
                      └────────────────┘
```

## Docker Registry V2 API

项目基于 Docker Registry V2 API 规范实现，详见 [docs/DockerAPI.md](docs/DockerAPI.md)。

核心流程：
1. 检查 `/v2/` 端点获取认证要求
2. 获取 Bearer Token
3. 获取 Manifest（支持多架构）
4. 下载 Config Blob
5. 下载所有 Layer Blobs（支持断点续传）
6. 打包成 tar.gz

## 开发

### 构建

```bash
cargo build --release
```

### 测试

```bash
cargo test
```

### 代码检查

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

## 依赖项

- `reqwest`: HTTP 客户端
- `tokio`: 异步运行时
- `serde`: JSON 序列化
- `tar`: tar 文件打包
- `flate2`: gzip 压缩
- `anyhow`: 错误处理

## License

MIT

## 相关链接

- [Docker Registry V2 API 规范](https://docs.docker.com/registry/spec/api/)
- [OCI Image Format 规范](https://github.com/opencontainers/image-spec)
