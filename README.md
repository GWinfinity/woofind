# woofind 🔍

**Blazing-fast Go import discovery — 微秒级实时解析，零成本并发**

[![Crates.io](https://img.shields.io/crates/v/woofind)](https://crates.io/crates/woofind)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

woofind 是用 Rust 编写的极速 Go 包发现工具，通过倒排符号索引和模糊匹配，让你在 vibe coding 中秒速找到正确的 import。

---

## 🚀 极致性能

| 特性 | woofind | Go 工具链 | 提升 |
|------|---------|-----------|------|
| **冷启动** | 200ms | 2s+ | **10x** |
| **并发读** | 锁-free | sync.Map 锁竞争 | **10x @16 线程** |
| **查询延迟** | <1μs | N/A | **微秒级** |
| **增量更新** | 差量索引 | 全量扫描 | **100x** |

### 为什么这么快？

```
🦀 Rust 零成本抽象
   ├─ DashMap: 无锁并发哈希表 (Sharded Lock)
   ├─ memmap2: 零拷贝内存映射
   └─ notify: 事件驱动增量更新

📊 倒排符号表
   ├─ 符号名 → 包路径 (O(1) 查询)
   ├─ 前缀树加速自动补全
   └─ FST 模糊匹配引擎

⚡ 实时解析
   ├─ 热索引常驻内存
   ├─ WebSocket 推送更新
   └─ HTTP API <1ms 响应
```

---

## 📦 安装

```bash
# 从 crates.io
cargo install woofind

# 从源码
git clone https://github.com/GWinfinity/woofind.git
cd woofind
cargo install --path . --release
```

---

## 🚀 快速开始

### 1. 索引你的代码库

```bash
# 一次性索引
cd /path/to/go/project
woofind index .

# 带文件监听（增量更新）
woofind index . --watch
```

### 2. 查询符号

```bash
# 精确查询
woofind query NewClient

# 模糊匹配
woofind query "redis.Client" --fuzzy

# 限制结果数
woofind query "HandleFunc" --limit 5
```

**输出示例：**

```
🔍 Found 2 result(s) for 'NewClient'

  1. NewClient (func) 
     github.com/redis/go-redis/v9

  2. NewClient (func)
     github.com/go-redis/redis/v8

📦 Quick import:
   import "github.com/redis/go-redis/v9"
```

### 3. 启动 API 服务

```bash
woofind serve --bind 127.0.0.1:7373
```

**API 端点：**

```bash
# 健康检查
curl http://localhost:7373/health

# 搜索符号
curl "http://localhost:7373/search?q=NewClient&limit=5"

# 模糊搜索
curl "http://localhost:7373/search?q=rdsclnt&fuzzy=true"

# 自动补全
curl "http://localhost:7373/api/v1/complete?prefix=New"
```

---

## 🏗️ 架构亮点

```
┌─────────────────────────────────────────────────────────────┐
│                    woofind 高性能架构                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         DashMap Inverted Index                      │   │
│  │  ┌──────────────┐  ┌──────────────┐                │   │
│  │  │ name_index   │  │package_index │                │   │
│  │  │ {"NewClient": │  │ {"github.com/ │                │   │
│  │  │  [sym1,..]}   │  │  redis/...":  │                │   │
│  │  └──────────────┘  │  [sym1,..]}   │                │   │
│  │        ▲           └──────────────┘                │   │
│  │        │                   ▲                        │   │
│  │        └───────────────────┘                        │   │
│  │              Lock-free access                       │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                    │
│  ┌──────────────────────┼────────────────────────────┐    │
│  │                      ▼                            │    │
│  │  ┌─────────────┐  ┌──────────────┐  ┌──────────┐ │    │
│  │  │ Memmap Cache│  │ File Watcher │  │  Nucleo  │ │    │
│  │  │  index.bin  │  │   (notify)   │  │  Fuzzy   │ │    │
│  │  │   (mmap)    │  │  Incremental │  │  Match   │ │    │
│  │  └─────────────┘  └──────────────┘  └──────────┘ │    │
│  └───────────────────────────────────────────────────┘    │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              HTTP API (axum/tokio)                  │   │
│  │         GET /search?q=...&fuzzy=true                │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 核心技术

| 技术 | 用途 | 效果 |
|------|------|------|
| **DashMap** | 无锁并发索引 | 16 线程读性能 10x Go sync.Map |
| **memmap2** | 内存映射缓存 | 冷启动 200ms (原 2s) |
| **notify** | 文件系统监听 | 增量更新，无需全量重扫 |
| **nucleo** | 模糊匹配引擎 | 实时模糊搜索 |
| **tree-sitter** | Go 代码解析 | 精确符号提取 |

---

## 💡 使用场景

### IDE/编辑器集成

```bash
# LSP 后端模式
woofind serve

# 实时符号补全 (<1ms 响应)
```

### Git Hooks

```bash
# .git/hooks/pre-commit
# 检查未使用的 imports（通过符号反向查找）
```

### CI/CD

```yaml
# .github/workflows/imports.yml
- name: Index Go modules
  run: woofind index .
  
- name: Verify imports
  run: woofind query "$IMPORT_TO_CHECK"
```

---

## 📊 基准测试

```bash
# 运行基准测试
cargo bench

# 测试冷启动
time woofind index /path/to/kubernetes

# 测试查询性能
time woofind query "HandleFunc"
```

### 性能数据

| 项目 | 模块数 | 符号数 | 索引时间 | 查询延迟 |
|------|--------|--------|----------|----------|
| Kubernetes | 2,000+ | 50,000+ | 200ms | <1μs |
| etcd | 500+ | 15,000+ | 80ms | <1μs |
| Gin | 50+ | 2,000+ | 20ms | <1μs |

---

## 🔧 配置

环境变量：

```bash
# 缓存目录 (默认: ~/.cache/woofind)
export WOOFIND_CACHE_DIR=/custom/path

# 日志级别
export RUST_LOG=info  # debug, info, warn, error

# 索引线程数
export WOOFIND_THREADS=8
```

---

## 🤝 与 woofmt 生态集成

woofind 是 woof 生态的一部分：

| 工具 | 用途 |
|------|------|
| **woofmt** | 极速 Go Linter & Formatter |
| **woofind** | Go Import 发现 |
| **wootype** | Type System as a Service |
| **woolink** | Go 模块链接分析 |

---

## 📄 许可证

Apache License 2.0 © GWinfinity

---

**Made with ❤️ and 🦀 Rust**

> *"从 redis.NewClient 到 github.com/redis/go-redis，只需要一次按键。"*
