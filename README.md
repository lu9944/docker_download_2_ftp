# Rust GitHub Actions Templates

这是一套完整的 Rust 项目 GitHub Actions 工作流模板。

## 工作流说明

### 1. CI (`.github/workflows/ci.yml`)

持续集成工作流，包含：

- **多版本测试**: 在 stable、beta、nightly 版本上运行测试
- **代码格式检查**: 使用 `cargo fmt` 检查代码格式
- **代码质量检查**: 使用 `cargo clippy` 进行静态分析
- **跨平台构建**: 支持 Linux、Windows、macOS (x86_64 和 ARM64)
- **依赖缓存**: 加速构建过程

触发条件:
- 推送到 `main` 或 `develop` 分支
- 针对 `main` 或 `develop` 分支的 Pull Request

### 2. Release (`.github/workflows/release.yml`)

发布工作流，包含：

- **自动构建**: 推送 tag 时自动构建 release 版本
- **二进制打包**: 创建 tar.gz 格式的发布包
- **自动发布**: 创建 GitHub Release 并上传构建产物

触发条件:
- 推送以 `v` 开头的 tag (如 `v1.0.0`)

### 3. Security (`.github/workflows/security.yml`)

安全审计工作流，包含：

- **依赖漏洞扫描**: 使用 `cargo-audit` 检查依赖漏洞
- **许可证检查**: 使用 `cargo-deny` 检查许可证合规性
- **定期扫描**: 每天自动运行安全检查

触发条件:
- 每天定时运行
- 推送到 `main` 或 `develop` 分支
- 针对 `main` 或 `develop` 分支的 Pull Request

## 使用方法

1. 将 `.github` 目录复制到你的 Rust 项目根目录

2. 根据需要调整工作流配置:
   - 修改触发分支名称
   - 添加/删除构建目标
   - 调整测试和检查参数

3. 如果使用 security 工作流，需要先创建 `Cargo.lock`:
   ```bash
   cargo generate-lockfile
   ```

4. (可选) 安装并配置 `cargo-deny`:
   ```bash
   cargo install cargo-deny
   cargo deny init
   ```

## 环境变量

可在工作流中使用的环境变量:

- `CARGO_TERM_COLOR`: 终端颜色输出 (默认: always)
- `RUST_BACKTRACE`: 错误回溯 (默认: 1)

## 自定义建议

- 添加更多测试平台 (如 ARM Linux)
- 集成代码覆盖率工具 (如 tarpaulin)
- 添加性能基准测试
- 集成 Docker 镜像构建
- 添加部署步骤

## 相关链接

- [Rust 文档](https://www.rust-lang.org/)
- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [cargo-audit](https://github.com/RustSec/cargo-audit)
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
