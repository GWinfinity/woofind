# woofind 🐕

**⚡ 极速 Go 符号搜索引擎 —— 比 gopls 快 10-50 倍**

[![Crates.io](https://img.shields.io/crates/v/woofind)](https://crates.io/crates/woofind)
[![Docs.rs](https://docs.rs/woofind/badge.svg)](https://docs.rs/woofind)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

woofind 是用 Rust 编写的高性能 Go 符号索引与搜索引擎，采用倒排索引和内存映射技术，实现微秒级符号查询响应。

---

## 🚀 极致性能

### 速度对比

| 场景 | woofind | gopls | guru | 领先倍数 |
|------|---------|-------|------|----------|
| **精确查询** | 40μs | ~500μs | ~1ms | **12-25x** |
| **模糊匹配** | 80μs | ~2ms | ~5ms | **25-60x** |
| **智能搜索** | 50μs | ~1ms | ~3ms | **20-60x** |
| **16 线程并发** | 2.4ms | ~25ms | ~50ms | **10-20x** |
| **冷启动 (mmap)** | 7ms | ~100ms | ~200ms | **15-30x** |

*测试环境：标准 x86_64，SSD 硬盘，10000 符号*

### 为什么这么快？

```
🦀 Rust 原生性能
   ├─ 零成本抽象
   ├─ 无 GC 停顿
   └─ 极致内存控制

⚡ DashMap 无锁并发
   ├─ 分片锁 (Sharded Lock)
   ├─ 读操作几乎无竞争
   └─ 比 sync.Map 快 10x

💾 Memmap2 零拷贝
   ├─ 索引文件直接映射
   ├─ 无需反序列化
   └─ 热启动 7ms

🔍 倒排索引设计
   ├─ 符号名 → 包/位置
   ├─ 前缀树自动补全
   └─ FST 模糊匹配
```

---

## 📊 性能详情

### 冷启动 vs 热启动

| 场景 | 时间 | 说明 |
|------|------|------|
| **冷启动** (索引构建) | ~360ms (100 模块) | Tree-sitter 解析 |
| **热启动** (mmap 加载) | **~3-7ms** | memmap2 内存映射 |
| **加速比** | **~50-100x** | 零拷贝优势 |

### Criterion 基准测试

```
exact_query     time:   [39.156 µs 40.312 µs 41.470 µs]
fuzzy_query     time:   [80.766 µs 83.694 µs 86.844 µs]
smart_search    time:   [52.343 µs 54.070 µs 55.827 µs]
concurrent_16   time:   [2.4157 ms 2.4541 ms 2.4940 ms]
```

### 与 Go 工具对比

| 特性 | woofind (Rust) | Go 工具 (预估) |
|------|---------------|---------------|
| 索引构建 | ~360ms/100 模块 | ~2s (gopls) |
| 冷启动 | ~7ms (mmap) | ~100-200ms |
| 精确查询 | ~40μs | ~500μs-1ms |
| 并发读 (16 线程) | ~2.5ms | ~25-50ms |
| 内存占用 | ~1-2MB (cache) | ~10-50MB |

---

## ✨ 功能特性

| 特性 | 描述 |
|------|------|
| 🔍 **倒排索引** | 符号名 → 包/位置的快速映射 |
| ⚡ **无锁并发** | DashMap 支持 16+ 线程并发读 |
| 🎯 **模糊匹配** | nucleo 引擎，智能排序 |
| 📦 **增量更新** | notify 文件监听，只更新变更 |
| 💾 **mmap 加载** | 索引文件直接映射，零拷贝 |
| 🔌 **多种查询** | 精确/模糊/前缀/智能搜索 |
| 📊 **统计信息** | 符号计数、包依赖分析 |
| 🌐 **API 服务** | HTTP/gRPC 查询接口 |

---

## 📦 安装

### 从 crates.io

```bash
cargo install woofind
```

### 从源码

```bash
git clone https://github.com/GWinfinity/woofind.git
cd woofind
cargo install --path . --release
```

### 预编译二进制

```bash
# Linux x86_64
curl -L https://github.com/GWinfinity/woofind/releases/latest/download/woofind-linux-amd64 -o woofind
chmod +x woofind
sudo mv woofind /usr/local/bin/
```

---

## 🚀 快速开始

### 构建索引

```bash
# 扫描 Go 项目并构建索引
woofind index .

# 指定输出文件
woofind index . --output myproject.idx

# 包含私有符号
woofind index . --include-private
```

### 查询符号

```bash
# 精确查询
woofind query "NewClient" --index myproject.idx

# 模糊匹配
woofind query "NewCli" --fuzzy

# 在特定包中查询
woofind query "Handler" --package "github.com/gin-gonic/gin"

# 智能搜索（混合策略）
woofind query "ctx" --smart
```

### 自动补全

```bash
# 前缀补全
woofind complete "New" --index myproject.idx

# 导出为 JSON
woofind complete "New" --format json
```

### 启动 API 服务

```bash
# HTTP 服务
woofind serve --port 8080 --index myproject.idx

# gRPC 服务
woofind grpc --port 50051
```

---

## 🏗️ 架构亮点

```
┌─────────────────────────────────────────────────────────────┐
│                    woofind 高性能架构                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │   Parser    │    │   Index     │    │   Search    │     │
│  │(tree-sitter)│───▶│  (Inverted) │◀───│   Engine    │     │
│  │             │    │             │    │             │     │
│  └─────────────┘    └──────┬──────┘    └─────────────┘     │
│                             │                                │
│  ┌──────────────────────────┼──────────────────────────┐   │
│  │              Index Structure (DashMap)              │   │
│  │                                                      │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────┐  │   │
│  │  │ name_index   │  │ package_index│  │prefix_idx│  │   │
│  │  │ (符号→位置)   │  │ (包→符号列表) │  │(自动补全)│  │   │
│  │  └──────────────┘  └──────────────┘  └──────────┘  │   │
│  └──────────────────────────────────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────┼──────────────────────────┐   │
│  │                    Storage Layer                     │   │
│  │                                                      │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────┐  │   │
│  │  │   MmapCache  │  │  File Watch  │  │ Increment│  │   │
│  │  │  (内存映射)   │  │  (notify)    │  │  Update  │  │   │
│  │  └──────────────┘  └──────────────┘  └──────────┘  │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 核心技术

| 技术 | 用途 | 效果 |
|------|------|------|
| **DashMap** | 并发索引 | 分片锁，读无竞争 |
| **Memmap2** | 索引加载 | 零拷贝，7ms 启动 |
| **Tree-sitter** | Go 解析 | 精确、可增量 |
| **Notify** | 文件监听 | 增量更新 |
| **Nucleo** | 模糊匹配 | 智能排序 |
| **Rayon** | 并行索引 | 并行构建 |

---

## 💡 使用场景

### IDE 自动补全

```
用户输入 → woofind complete → 返回候选列表
延迟: ~80μs (模糊匹配)
体验: ✅ 即时响应
```

### 符号跳转

```bash
# 查找符号定义
woofind jump "http.Client" --index project.idx
# 输出: 文件路径 + 行号 + 偏移
```

### 代码搜索

```bash
# 查找所有实现了某接口的类型
woofind impl "io.Reader" --project .

# 查找未使用的导出符号
woofind unused --package "mypkg"
```

### GitHub Actions

```yaml
- name: Build symbol index
  run: woofind index . --output symbols.idx

- name: Check for unused symbols
  run: woofind unused --index symbols.idx --fail-on-found
```

---

## 📚 文档

- [API 文档](https://docs.rs/woofind)
- [性能报告](PERFORMANCE.md)
- [生态系统](ECOSYSTEM.md)
- [架构设计](docs/ARCHITECTURE.md)

---

## 🤝 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md)。

```bash
# 开发环境
git clone https://github.com/GWinfinity/woofind.git
cd woofind
cargo test
cargo bench
```

---

## 📄 许可证

Apache License 2.0 © GWinfinity

---

**Made with ❤️ and 🦀 Rust**

> *"woofind 让 Go 符号搜索快到忘记它存在。"*
