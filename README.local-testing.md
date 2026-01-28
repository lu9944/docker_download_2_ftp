# 本地测试 GitHub Actions

## 方法 1: 使用 act（推荐）

### 安装 act

**macOS (Homebrew):**
```bash
brew install act
```

**Linux:**
```bash
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash
```

**或使用 cargo:**
```bash
cargo install act-cli
```

### 基本使用

```bash
# 查看所有可用的 jobs
act -l

# 运行特定 job
act -j test

# 运行所有 jobs
act push

# 使用特定的 GitHub 镜像（国内更快）
act -j test --container-architecture linux/amd64 -P ubuntu-latest=catthehacker/ubuntu:act-latest

# 查看详细日志
act -j test --verbose

# 不使用缓存
act -j test --use-body=false
```

### 常用参数

| 参数 | 说明 |
|------|------|
| `-j <job>` | 指定要运行的 job 名称 |
| `--bind` | 不使用容器（直接在主机运行） |
| `-n` | 干运行，只显示将要执行的操作 |
| `-v` | 显示详细日志 |
| `--container-daemon-socket` | 连接到 Docker 守护进程 |

### 示例

```bash
# 测试 CI 工作流
act -j test -j build

# 模拟 push 事件
act push

# 使用本地 Docker 镜像加速
act -j test --container-architecture linux/amd64 \
  -P ubuntu-latest=catthehacker/ubuntu:act-20.04
```

---

## 方法 2: 使用 GitHub 免费仓库

如果需要在真实环境中测试：

1. 创建一个测试仓库（可以是私有仓库）
2. 开启 GitHub Actions（免费额度：公开仓库无限，私有仓库每月 2000 分钟）
3. 推送代码并查看 Actions 标签页

---

## 方法 3: 手动执行命令

在本地手动执行工作流中的命令：

```bash
# 格式检查
cargo fmt --all -- --check

# Clippy 检查
cargo clippy --all-targets --all-features -- -D warnings

# 运行测试
cargo test --verbose --all-features

# 构建项目
cargo build --verbose --release
```

---

## 推荐的测试流程

1. **本地使用 `act` 快速迭代** - 快速发现问题
2. **本地手动执行关键命令** - 验证特定步骤
3. **推送到 GitHub** - 最终验证完整流程

---

## act 常见问题

### 权限问题
```bash
# 使用 sudo 运行（Linux）
sudo act -j test
```

### 镜像拉取慢
```bash
# 使用国内镜像
act -j test -P ubuntu-latest=catthehacker/ubuntu:act-latest
```

### 容器架构问题
```bash
# 明确指定架构
act -j test --container-architecture linux/amd64
```

---

## 相关链接

- [act GitHub](https://github.com/nektos/act)
- [GitHub Actions 文档](https://docs.github.com/en/actions)
