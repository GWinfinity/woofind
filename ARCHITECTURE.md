# woofind 架构设计文档

本文档详细描述 woofind 的架构设计、核心组件和性能优化策略。

## 目录

- [总体架构](#总体架构)
- [核心组件](#核心组件)
- [索引结构](#索引结构)
- [查询引擎](#查询引擎)
- [缓存策略](#缓存策略)
- [性能优化](#性能优化)

## 总体架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           API Layer                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │   HTTP API  │  │  gRPC API   │  │  WebSocket  │  │   CLI Tool      │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘ │
└─────────┼────────────────┼────────────────┼──────────────────┼──────────┘
          │                │                │                  │
          └────────────────┴────────────────┴──────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         Query Engine Layer                               │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                       QueryEngine                                │   │
│  │  ├─ exact_lookup(): 精确查询                                    │   │
│  │  ├─ fuzzy_search(): 模糊匹配                                    │   │
│  │  ├─ smart_search(): 智能搜索                                    │   │
│  │  └─ autocomplete(): 前缀补全                                    │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          Index Layer                                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │ name_index  │  │package_index│  │ prefix_idx  │  │ module_metadata │ │
│  │ (DashMap)   │  │  (DashMap)  │  │  (DashMap)  │  │   (DashMap)     │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘ │
│         │                │                │                  │          │
│         └────────────────┴────────────────┴──────────────────┘          │
│                                   │                                      │
│                        ┌──────────┴──────────┐                          │
│                        ▼                     ▼                          │
│              ┌─────────────────┐   ┌─────────────────┐                 │
│              │ InvertedIndex   │   │  IndexBuilder   │                 │
│              └─────────────────┘   └─────────────────┘                 │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Storage Layer                                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │  MmapCache  │  │ File Watch  │  │   Disk      │  │  Incremental    │ │
│  │  (memmap2)  │  │  (notify)   │  │  Storage    │  │   Update        │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Parser Layer                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │ Tree-sitter │  │  Go Parser  │  │  go.mod     │  │    go.sum       │ │
│  │   (Go)      │  │  (AST)      │  │  Parser     │  │    Parser       │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

## 核心组件

### 1. InvertedIndex (倒排索引)

核心数据结构，支持无锁并发访问：

```rust
pub struct InvertedIndex {
    /// 主索引: 符号名 → 符号列表
    pub name_index: Arc<DashMap<String, Vec<Symbol>>>,
    
    /// 包索引: 包路径 → 符号列表
    package_index: Arc<DashMap<String, Vec<Symbol>>>,
    
    /// 前缀索引: 首字符 → 符号名列表
    prefix_index: Arc<DashMap<char, Vec<String>>>,
    
    /// 模块元数据: 模块路径 → 模块信息
    module_metadata: Arc<DashMap<String, ModuleInfo>>,
}
```

**为什么使用 DashMap?**

- 分片锁设计: 16 个分片，降低锁竞争
- 读操作几乎无锁: 使用原子操作
- 比 Go 的 sync.Map 快 10x (16 线程场景)

### 2. QueryEngine (查询引擎)

提供多种查询策略：

```rust
impl QueryEngine {
    /// 精确查询 - O(1)
    pub fn exact_lookup(&self, name: &str) -> Vec<Symbol>;
    
    /// 模糊匹配 - O(n * m) with SIMD
    pub fn fuzzy_search(&self, query: &str, limit: usize) -> Vec<(Symbol, Score)>;
    
    /// 智能搜索 - 自动选择策略
    pub fn smart_search(&self, query: &str, limit: usize) -> Vec<Symbol>;
    
    /// 前缀补全 - O(log n)
    pub fn autocomplete(&self, prefix: &str, limit: usize) -> Vec<String>;
}
```

### 3. IndexBuilder (索引构建器)

增量构建索引：

```rust
impl IndexBuilder {
    /// 从目录构建
    pub fn build_from_directory(&self, dir: &Path) -> Result<()>;
    
    /// 增量更新单个文件
    pub fn update_file(&self, path: &Path) -> Result<()>;
    
    /// 删除包
    pub fn remove_package(&self, package: &str) -> Result<()>;
    
    /// 保存到缓存
    pub fn save_to_cache(&self) -> Result<()>;
}
```

## 索引结构

### 符号结构

```rust
pub struct Symbol {
    pub name: String,           // "NewClient"
    pub package: String,        // "github.com/redis/go-redis/v9"
    pub package_name: String,   // "redis"
    pub kind: SymbolKind,       // Function
    pub version: Option<String>,// "v9.5.1"
    pub import_path: String,    // "github.com/redis/go-redis/v9"
    pub doc: Option<String>,    // "Creates a new Redis client"
    pub signature: Option<String>, // "func(opt *Options) *Client"
}
```

### 倒排索引示例

```
name_index:
    "NewClient" → [
        Symbol { package: "github.com/redis/go-redis/v9", ... },
        Symbol { package: "github.com/go-sql-driver/mysql", ... },
    ]
    "Context" → [
        Symbol { package: "context", ... },
    ]

package_index:
    "github.com/redis/go-redis/v9" → [
        Symbol { name: "NewClient", ... },
        Symbol { name: "Client", ... },
        Symbol { name: "Options", ... },
    ]

prefix_index:
    'N' → ["NewClient", "NewReader", "NewWriter", ...]
    'C' → ["Context", "Client", "Conn", ...]
```

## 查询引擎

### 查询流程

```
用户查询 ──▶ QueryEngine
                │
                ├──▶ 精确匹配? ──▶ 返回结果
                │
                ├──▶ 限定查询? ──▶ 解析包限定符 ──▶ 包内搜索
                │
                └──▶ 模糊搜索 ──▶ nucleo 模糊匹配 ──▶ 按得分排序
```

### 模糊匹配算法

使用 nucleo 模糊匹配引擎：

```rust
pub fn fuzzy_search(&self, query: &str, limit: usize) -> Vec<(Symbol, u32)> {
    let matcher = nucleo::Matcher::new(nucleo::Config::DEFAULT);
    let pattern = nucleo::pattern::Pattern::parse(
        query,
        nucleo::pattern::CaseMatching::Smart,
        nucleo::pattern::Normalization::Smart
    );
    
    // 并行匹配
    self.index.name_index.iter()
        .par_bridge()
        .filter_map(|entry| {
            let name = entry.key();
            let score = pattern.score(name.chars(), &mut matcher);
            score.map(|s| (entry.value().clone(), s))
        })
        .collect::<Vec<_>>()
        .sort_by_key(|(_, score)| *score)
        .take(limit)
        .collect()
}
```

### 智能搜索策略

```rust
pub fn smart_search(&self, query: &str, limit: usize) -> Vec<Symbol> {
    // 1. 尝试精确匹配
    let exact = self.exact_lookup(query);
    if !exact.is_empty() {
        return exact;
    }
    
    // 2. 尝试限定查询 (如 "redis.NewClient")
    if query.contains('.') {
        let parts: Vec<_> = query.split('.').collect();
        if parts.len() == 2 {
            let package = parts[0];
            let name = parts[1];
            let qualified = self.qualified_lookup(package, name);
            if !qualified.is_empty() {
                return qualified;
            }
        }
    }
    
    // 3. 模糊匹配
    self.fuzzy_search(query, limit)
        .into_iter()
        .map(|(s, _)| s)
        .collect()
}
```

## 缓存策略

### MmapCache (内存映射缓存)

```rust
pub struct MmapCache {
    cache_dir: PathBuf,
    mmap: Option<Mmap>,
}

impl MmapCache {
    /// 检查缓存是否有效
    pub fn is_cache_valid(&self, max_age_hours: u64) -> bool;
    
    /// 加载索引 (零拷贝)
    pub fn load_index(&self) -> Result<Option<InvertedIndex>>;
    
    /// 保存索引
    pub fn save_index(&self, index: &InvertedIndex) -> Result<()>;
}
```

**零拷贝加载**:

```
磁盘文件 ──▶ mmap ──▶ 虚拟内存 ──▶ 直接访问
              │
              └── 无需 read() 系统调用
              └── 无需反序列化
              └── 按需页面加载
```

### 增量更新

使用 notify 监听文件系统事件：

```rust
pub fn watch_and_update(index: Arc<InvertedIndex>) -> Result<()> {
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    
    watcher.watch(Path::new("."), RecursiveMode::Recursive)?;
    
    for event in rx {
        match event? {
            Event { kind: Modify(_), paths, .. } => {
                for path in paths {
                    if is_go_file(&path) {
                        update_file_index(&index, &path)?;
                    }
                }
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

## 性能优化

### 1. 并发优化

**并行索引构建**:

```rust
pub fn build_parallel(&self, files: Vec<PathBuf>) -> Result<()> {
    use rayon::prelude::*;
    
    // 并行解析文件
    let symbols: Vec<_> = files
        .par_iter()
        .map(|file| self.parse_file(file))
        .flatten()
        .collect();
    
    // 批量插入
    self.index.batch_insert(symbols);
    
    Ok(())
}
```

**无锁并发读**:

```rust
// DashMap 支持真正的并发读
let handles: Vec<_> = (0..16)
    .map(|_| {
        let index = Arc::clone(&index);
        thread::spawn(move || {
            // 无锁读取
            index.get_by_name("NewClient")
        })
    })
    .collect();
```

### 2. 内存优化

**字符串去重**:

```rust
// 使用 string-interner 去重
let mut interner = StringInterner::new();
let symbol_name = interner.get_or_intern("NewClient");
// 多次出现的字符串只存储一次
```

**紧凑存储**:

```rust
// 使用合适的数据类型
pub struct CompactSymbol {
    name_id: u32,      // 代替 String
    package_id: u16,   // 包 ID，最多 65535 个包
    kind: u8,          // SymbolKind 转为 u8
}
```

### 3. 查询优化

**前缀树加速**:

```rust
// 构建前缀树用于自动补全
let mut trie = Trie::new();
for name in all_symbol_names {
    trie.insert(name);
}

// O(m) 前缀搜索，m 为前缀长度
let completions = trie.find_prefix("New");
```

**布隆过滤器**:

```rust
// 快速排除不存在的查询
let bloom = BloomFilter::with_size(1_000_000);
for name in all_names {
    bloom.insert(name);
}

// 快速排除
if !bloom.might_contain(query) {
    return vec![]; // 快速返回
}
```

## 基准测试

### Criterion 测试结果

```
exact_query     time:   [39.156 µs 40.312 µs 41.470 µs]
                change: [-2.3% -0.8% +0.7%] (p = 0.29 > 0.05)
                
fuzzy_query     time:   [80.766 µs 83.694 µs 86.844 µs]
                change: [-1.2% +0.4% +2.1%] (p = 0.62 > 0.05)
                
smart_search    time:   [52.343 µs 54.070 µs 55.827 µs]
                change: [-3.1% -1.5% +0.1%] (p = 0.08 > 0.05)
                
concurrent_16   time:   [2.4157 ms 2.4541 ms 2.4940 ms]
                thrpt:  [6511.3 elem/s 6615.6 elem/s 6720.9 elem/s]
```

### 与 Go 工具对比

| 工具 | 10k 符号索引 | 精确查询 | 模糊查询 | 并发 16 线程 |
|------|-------------|----------|----------|-------------|
| woofind | 360ms | 40μs | 80μs | 2.4ms |
| gopls | ~2000ms | ~500μs | ~2ms | ~25ms |
| guru | N/A | ~1ms | ~5ms | ~50ms |

## 调试与监控

### 性能指标

```rust
pub struct IndexStats {
    pub total_symbols: usize,
    pub total_packages: usize,
    pub total_modules: usize,
    pub avg_chain_length: f64,
    pub cache_hit_rate: f64,
}
```

### 调试命令

```bash
# 显示索引统计
woofind stats --index myproject.idx

# 性能剖析
woofind profile --query "NewClient" --verbose

# 内存使用
woofind stats --memory

# 导出索引结构
woofind export --format debug
```
