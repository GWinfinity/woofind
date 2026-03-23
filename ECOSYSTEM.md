# Woo 生态 - 极速 Go 开发工具链

## 生态架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           🐕 Woo 生态系统                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        IDE / AI Agents                              │   │
│  │              (Cursor, VS Code, Claude, GPT-4, etc.)                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     🌐 Unified API Gateway                            │   │
│  │              gRPC / HTTP / WebSocket / LSP                           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│         ┌─────────────┬────────────┼────────────┬─────────────┐            │
│         │             │            │            │             │            │
│         ▼             ▼            ▼            ▼             ▼            │
│  ┌────────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐      │
│  │  woofind   │ │  woofmt  │ │ wootype  │ │ woolink  │ │  woof    │      │
│  │  🔍        │ │  ✨      │ │  🎯      │ │  🔗      │ │  🐕      │      │
│  ├────────────┤ ├──────────┤ ├──────────┤ ├──────────┤ ├──────────┤      │
│  │Import发现  │ │代码格式化 │ │类型检查  │ │模块链接  │ │统一CLI   │      │
│  │符号索引    │ │Lint规则  │ │语义分析  │ │依赖分析  │ │编排调度  │      │
│  │模糊匹配    │ │自动修复  │ │AI类型推断│ │死码检测  │ │工作流    │      │
│  └────────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘      │
│         │             │            │            │             │            │
│         └─────────────┴────────────┴────────────┴─────────────┘            │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     💾 Shared Services Layer                        │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │   │
│  │  │ Index Store  │  │  AST Cache   │  │    Type Universe         │  │   │
│  │  │ (DashMap)    │  │  (Mmap)      │  │    (ECS Storage)         │  │   │
│  │  └──────────────┘  └──────────────┘  └──────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 组件协作

### 1. woofind 🔍 (Import Discovery)

**职责**: 包发现、符号索引、导入管理

```rust
// 生态集成点
pub struct ImportService {
    index: Arc<InvertedIndex>,
    fuzzy_engine: FuzzyMatcher,
}

impl ImportService {
    // 为 woofmt 提供未使用导入检测
    pub fn find_unused_imports(&self, file: &GoFile) -> Vec<Import> {
        // 反向查找符号使用情况
    }
    
    // 为 wootype 提供类型位置解析
    pub fn resolve_type_location(&self, type_name: &str) -> Option<PackagePath> {
        // 查询倒排索引
    }
}
```

**与生态集成**:
- 向 `woofmt` 提供符号索引，用于未使用导入检测
- 向 `wootype` 提供包路径解析，用于类型查找
- 向 `woolink` 提供模块依赖图

### 2. woofmt ✨ (Linter & Formatter)

**职责**: 代码格式化、静态分析、自动修复

```rust
// 生态集成点
pub struct LintEngine {
    rules: Vec<Box<dyn LintRule>>,
    import_service: Arc<ImportService>, // 来自 woofind
    type_service: Arc<TypeService>,     // 来自 wootype
}

impl LintEngine {
    // 使用 wootype 的类型信息进行更深层的分析
    pub fn check_type_errors(&self, file: &GoFile) -> Vec<Diagnostic> {
        let type_info = self.type_service.infer_types(file);
        // 基于类型信息的高级检查
    }
}
```

**与生态集成**:
- 使用 `woofind` 的索引优化导入排序和分组
- 使用 `wootype` 的类型信息进行精确的语义检查
- 向 `woof` 提供统一的格式化输出

### 3. wootype 🎯 (Type System as a Service)

**职责**: 类型检查、语义分析、AI 类型推断

```rust
// 生态集成点
pub struct TypeService {
    universe: TypeUniverse,
    import_resolver: Arc<ImportService>, // 来自 woofind
}

impl TypeService {
    // 为 woofmt 提供类型检查
    pub fn validate_for_linter(&self, file: &GoFile) -> TypeDiagnostics {
        // 快速类型检查模式
    }
    
    // 为 IDE 提供自动补全类型信息
    pub fn get_completion_types(&self, pos: Position) -> Vec<TypeHint> {
        // 基于上下文的类型推断
    }
}
```

**与生态集成**:
- 使用 `woofind` 解析导入包的位置
- 向 `woofmt` 提供类型信息用于高级 Lint 规则
- 向 IDE 提供实时代码补全和类型提示

### 4. woolink 🔗 (Module Linker)

**职责**: 依赖分析、死码检测、模块重构

```rust
// 生态集成点
pub struct LinkAnalyzer {
    module_graph: ModuleGraph,
    symbol_index: Arc<InvertedIndex>, // 来自 woofind
    type_universe: Arc<TypeUniverse>, // 来自 wootype
}

impl LinkAnalyzer {
    // 跨模块死码检测
    pub fn find_dead_code(&self) -> Vec<DeadCode> {
        // 结合符号索引和类型使用信息
    }
    
    // 依赖循环检测
    pub fn detect_cycles(&self) -> Vec<Cycle> {
        // 基于模块图的分析
    }
}
```

**与生态集成**:
- 使用 `woofind` 的模块索引构建依赖图
- 使用 `wootype` 的类型使用信息精确检测死码
- 向 `woofmt` 提供重构建议

### 5. woof 🐕 (Unified CLI)

**职责**: 统一入口、工作流编排、配置管理

```bash
# 统一 CLI 设计
woof check .           # 运行 woofmt lint
woof format .          # 运行 woofmt format
woof typecheck .       # 运行 wootype check
woof imports .         # 运行 woofind index + 分析
woof link .            # 运行 woolink 分析
woof fix .             # 运行所有修复工具

# 组合命令
woof ci .              # CI 模式：lint + typecheck + link
woof dev               # 开发模式：启动所有服务 + watch
```

**统一配置** (`woof.toml`):

```toml
[global]
project_root = "."
target_go_version = "1.21"

[woofmt]
enabled = true
select = ["E", "F", "SA"]
ignore = ["E101"]
auto_fix = true

[wootype]
enabled = true
mode = "strict"
ai_assist = true

[woofind]
enabled = true
index_stdlib = true
fuzzy_threshold = 0.8

[woolink]
enabled = true
dead_code_detection = true
cycle_detection = true

[ecosystem]
shared_cache = true
parallel_jobs = 8
ipc_mode = "unix_socket"
```

## 数据流

```
┌─────────────────────────────────────────────────────────────────┐
│                        典型工作流示例                            │
│                     "保存文件时的完整检查"                        │
└─────────────────────────────────────────────────────────────────┘

用户保存 main.go
       │
       ▼
┌──────────────┐
│   woof dev   │ ◄── 统一 CLI 接收文件变更事件
└──────┬───────┘
       │
       ├──────────────────┬──────────────────┐
       ▼                  ▼                  ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   woofind    │  │   woofmt     │  │   wootype    │
│  (增量索引)   │  │  (格式检查)   │  │  (类型检查)   │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                  │                  │
       │                  ▼                  │
       │         ┌──────────────┐            │
       │         │  需要导入    │────────────┤
       │         │  信息？      │            │
       │         └──────┬───────┘            │
       │                │                    │
       ▼                ▼                    ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  更新符号索引  │  │  查询 woofind │  │  查询 woofind│
│  通知其他服务  │  │  获取导入信息  │  │  获取包路径  │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                  │                  │
       └──────────────────┼──────────────────┘
                          ▼
                   ┌──────────────┐
                   │   聚合结果    │
                   │  推送到 IDE   │
                   └──────────────┘
```

## 性能协同

| 组件 | 核心优化 | 生态贡献 |
|------|----------|----------|
| **woofind** | DashMap + memmap2 | 为全生态提供 O(1) 符号查询 |
| **woofmt** | Parser Pool + AST Cache | 2ms 热运行响应 |
| **wootype** | ECS + Copy-on-Write | 微秒级类型查询 |
| **woolink** | 并行图算法 | 秒级大型项目分析 |
| **woof** | 统一调度 | 避免重复解析，共享 AST |

## 共享服务层

### Shared AST Cache

```rust
pub struct SharedAstCache {
    cache: DashMap<PathBuf, Arc<AstNode>>,
    mmap_storage: MmapCache,
}

// woofmt 解析后，wootype 可以直接使用
// 避免重复解析 Go 源代码
```

### Unified Index

```rust
pub struct UnifiedIndex {
    symbols: Arc<InvertedIndex>,     // 来自 woofind
    types: Arc<TypeUniverse>,        // 来自 wootype
    modules: Arc<ModuleGraph>,       // 来自 woolink
}
```

## 下一步集成计划

### Phase 1: 数据共享 (当前)
- [x] woofind 提供符号索引接口
- [x] woofmt 使用索引优化导入检测
- [ ] wootype 使用索引解析包路径

### Phase 2: 服务协同 (4周内)
- [ ] 实现统一 CLI `woof`
- [ ] 共享 AST Cache
- [ ] 统一配置文件

### Phase 3: 智能融合 (8周内)
- [ ] 类型感知的 Lint 规则
- [ ] 导入感知的类型检查
- [ ] 全链路死码检测

### Phase 4: AI 增强 (12周内)
- [ ] AI 驱动的代码重构建议
- [ ] 智能导入补全
- [ ] 预测性类型错误修复

## 愿景

> **"让 Go 开发快到忘记工具存在"**

通过 woofind + woofmt + wootype + woolink 的协同，构建世界上最快的 Go 语言工具链，为开发者和 AI Agent 提供零延迟的代码智能。
