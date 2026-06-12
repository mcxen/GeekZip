# GeekZip — 技术规范 (SPEC)

> **版本**: 0.1.0-alpha  
> **日期**: 2026-06-12  
> **产品定位**: 专为网盘下载场景设计的跨平台智能归档工具 — 解压/压缩/密码管理一体化

---

## 1. 产品概述

### 1.1 一句话定位

**GeekZip** 是一款专为网盘下载场景设计的跨平台智能归档工具，支持 macOS + Windows，实现"一拖即解、智能识别、密码自填、递归解压"的无感体验。

### 1.2 核心差异化

| 竞品痛点 | GeekZip 解法 |
|---------|-------------------|
| 扩展名被篡改/含中文干扰 | 干扰字符清理 + Magic Bytes 识别 |
| 不知道密码 | 文件名提取 → 历史密码 → 密码本 → 字典爆破 |
| 套娃压缩包 | 自动递归解压，支持深度限制与中间层清理 |
| 分卷文件散落 | 自动同目录匹配 + 按序合并 |
| 操作路径长 | 右键菜单 / 拖拽 / 监控全自动 |
| 解压后一团乱 | AI 分析 + 自动整理/清理/去重 |

### 1.3 目标用户

- 经常从网盘下载资源的设计师、开发者、学生
- 需要批量处理归档文件的企业 IT 人员
- 对安全性有要求（密码管理、防 zip bomb）的技术用户

---

## 2. 功能规范

### 2.1 功能全景图

```
┌─────────────────────────────────────────────────────────────┐
│                      GeekZip                                │
├─────────────────────────────────────────────────────────────┤
│  ARCHIVES          TOOLS              AI ASSISTANT          │
│  ├─ Inbox          ├─ Compress        ├─ Analyze             │
│  ├─ Processing     ├─ Extract         ├─ Organize            │
│  ├─ Extracted      ├─ Encrypt         ├─ Clean               │
│  └─ Favorites      ├─ Batch            └─ Duplicates          │
│                     └─ Password Manager                       │
├─────────────────────────────────────────────────────────────┤
│  系统状态栏: CPU / MEM / THREADS / FILES/S / SPEED / RATIO    │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 核心功能模块

#### 模块 A: 解压引擎 (Extract)

| 子功能 | 说明 | 优先级 |
|--------|------|--------|
| **格式支持** | ZIP / RAR(4/5) / 7Z / TAR / GZ / BZ2 / XZ / ZSTD / LZ4 | P0 |
| **Magic Bytes 识别** | 不依赖文件扩展名，读取文件头判断真实格式 | P0 |
| **干扰字符清理** | 去除扩展名中的中文（如"删除"、"去掉"等） | P0 |
| **无后缀识别** | 文件名完全没有点号，靠 Magic Bytes 判断 | P0 |
| **自动补后缀** | 识别后自动重命名，加上正确扩展名 | P1 |
| **分卷识别** | 识别 `.001/.002` 或 `.part1/.part2` 等模式 | P0 |
| **分卷合并** | 自动按序合并为完整文件 | P0 |
| **无点号分卷** | 如 `苹果7z002`，通过同目录文件名相似度分组 | P0 |
| **解压到当前目录** | 默认行为 | P0 |
| **解压到指定目录** | 用户可选 | P0 |
| **解压到同名文件夹** | 创建同名目录再解压进去 | P1 |
| **解压并删除源文件** | 解压成功后自动删除压缩包 | P0 |
| **覆盖策略** | 覆盖/跳过/重命名 | P0 |
| **解压后打开文件夹** | 自动打开目标目录 | P1 |
| **校验文件完整性** | CRC/MD5/SHA 校验 | P1 |

**解压流程状态机**:

```
[Queued] → [Analyzing] → [Password_Required] → [Extracting] → [Verifying] → [Completed]
              ↓                    ↓                    ↓              ↓
         [Error_Invalid]      [Error_Password]      [Error_I/O]     [Error_Corrupt]
```

#### 模块 B: 压缩引擎 (Compress)

| 子功能 | 说明 | 优先级 |
|--------|------|--------|
| **格式输出** | ZIP / 7Z / TAR / GZ / BZ2 / XZ | P0 |
| **压缩级别** | Store / Fast / Normal / Maximum / Ultra | P0 |
| **分卷压缩** | 按指定大小分卷 | P1 |
| **密码加密** | AES-256 / ZIPCrypto | P0 |
| **固实压缩** | 7Z Solid mode | P1 |
| **添加恢复记录** | 防数据损坏 | P2 |
| **压缩后删除源文件** | 可选 | P1 |
| **创建自解压** | Windows SFX | P2 |

#### 模块 C: 密码管理 (Password Manager)

| 子功能 | 说明 | 优先级 |
|--------|------|--------|
| **手动输入密码** | 弹窗输入 | P0 |
| **密码本字典尝试** | 加载 `.txt` 密码列表，多线程并行尝试 | P0 |
| **文件名密码提取** | 自动从文件名中提取 `[密码:xxx]` 模式 | P0 |
| **历史密码记忆** | 成功的密码存入本地 SQLite，下次优先使用 | P0 |
| **内置常用密码** | 预置网盘常见分享密码（1234、0000、提取码等） | P1 |
| **密码本管理** | 添加/删除/导入/导出密码列表 | P1 |
| **密码生成器** | 强密码生成，自定义长度与字符集 | P1 |
| **密码安全存储** | 使用系统 Keychain (macOS) / Credential Manager (Windows) | P0 |
| **密码尝试超时** | 单个密码 30 秒时限 | P0 |

**密码尝试优先级**:

```
1. 文件名提取 (O(1))
2. 历史密码 (本地 SQLite，按使用频率排序)
3. 用户手动输入
4. 内置常用密码 (Top 20)
5. 密码本字典 (多线程并行)
```

#### 模块 D: 递归解压 (Recursive Extract)

| 子功能 | 说明 | 优先级 |
|--------|------|--------|
| **自动递归解压** | 解压出的压缩包继续解压，直到没有为止 | P0 |
| **递归深度限制** | 默认最大 10 层，可配置 | P0 |
| **递归解压并删除** | 每层解压后删除中间压缩包，只留最终文件 | P0 |
| **循环检测** | 检测 A.zip → B.zip → A.zip 循环 | P0 |
| **解压报告** | 完成后显示层数、用了什么密码、最终产物 | P1 |
| **递归压缩炸弹防护** | 单文件大小上限 / 总解压大小上限 | P0 |

**递归解压工作流**:

```
用户触发 "智能解压"
    ↓
1. 文件名预处理 (清理干扰 + 分卷识别)
    ↓
2. 分卷合并 (如需要)
    ↓
3. Magic Bytes 格式确认
    ↓
4. 密码处理 (文件名 → 历史 → 常用 → 密码本)
    ↓
5. 执行解压
    ↓
6. 递归检查 (深度 < 限制? 有压缩包? 无循环?)
    ├── 是 → 回到步骤 1
    └── 否 → 结束
    ↓
7. 善后 (删除源文件? 删除中间层? 记录密码? 系统通知)
    ↓
✅ 任务完成
```

#### 模块 E: AI 助手 (AI Assistant)

| 子功能 | 说明 | 优先级 |
|--------|------|--------|
| **Analyze (分析)** | 扫描解压内容，生成文件类型统计、大小分布、风险报告 | P1 |
| **Organize (整理)** | 按文件类型自动分类到子目录 (Images/Documents/Videos/...) | P1 |
| **Clean (清理)** | 识别并删除临时/无用文件 (Thumbs.db, .DS_Store, 空目录) | P1 |
| **Duplicates (去重)** | 检测并处理重复文件（按内容哈希） | P1 |
| **AI 建议** | 基于内容给出整理建议 | P2 |

> **注**: AI 模块为本地推理，不依赖云端 API，使用轻量级本地模型或规则引擎。

#### 模块 F: 目录监控 (Directory Watcher)

| 子功能 | 说明 | 优先级 |
|--------|------|--------|
| **目录监控** | 监控指定文件夹（如 `~/Downloads`），新压缩包自动处理 | P2 |
| **规则配置** | 按文件大小/格式/来源设置自动处理规则 | P2 |
| **静默模式** | 后台运行，完成后仅通知 | P2 |
| **文件过滤** | 仅处理特定扩展名/大小范围的文件 | P2 |
| **防重复处理** | 已处理文件记录指纹，避免重复解压 | P2 |

#### 模块 G: 批量处理 (Batch)

| 子功能 | 说明 | 优先级 |
|--------|------|--------|
| **批量解压** | 同时处理多个压缩包 | P1 |
| **批量压缩** | 将多个文件/目录分别压缩 | P1 |
| **队列管理** | 暂停/恢复/取消/重新排序 | P1 |
| **并行度控制** | 可配置同时进行的任务数 | P1 |

---

### 2.3 系统集成 — 右键菜单

#### macOS

| 菜单项 | 实现方式 | 优先级 |
|--------|----------|--------|
| 解压到当前目录 | Finder Extension / Quick Action | P0 |
| 解压到「文件名/」 | Finder Extension / Quick Action | P0 |
| 解压并删除源文件 | Finder Extension / Quick Action | P0 |
| 智能解压（自动密码+递归） | Finder Extension / Quick Action | P0 |
| 智能解压并删除 | Finder Extension / Quick Action | P1 |
| 用密码解压... | Finder Extension / Quick Action | P0 |
| 设置... | 打开主应用 | P0 |
| 拖拽到 Dock 图标 | Dock Drop Handler | P1 |

**实现方式**: 
- **推荐**: Finder Sync Extension (Swift/AppKit) → 调用 Rust CLI
- **备选**: Automator Quick Action (更简单，但功能有限)

#### Windows

| 菜单项 | 实现方式 | 优先级 |
|--------|----------|--------|
| 解压到当前目录 | Shell Extension / 注册表 | P0 |
| 解压到「文件名/」 | Shell Extension / 注册表 | P0 |
| 解压并删除源文件 | Shell Extension / 注册表 | P0 |
| 智能解压（自动密码+递归） | Shell Extension / 注册表 | P0 |
| 智能解压并删除 | Shell Extension / 注册表 | P1 |
| 用密码解压... | Shell Extension / 注册表 | P0 |
| 设置... | 打开主应用 | P0 |
| 拖拽到任务栏图标 | 支持 | P1 |

**实现方式**:
- **推荐**: 注册表 + CLI 调用（最简单，兼容性最好）
- **进阶**: COM Shell Extension (Rust/C++ 写 DLL)
- **现代**: Windows 11 新 Context Menu API (仅 Win11+)

#### 右键菜单结构

```
📦 GeekZip
├── 解压到当前目录
├── 解压到「文件名/」
├── 解压并删除源文件
├── ─────────────────
├── 🔮 智能解压（自动密码 + 递归）
├── 智能解压并删除
├── ─────────────────
├── 🔑 用密码解压...
└── ⚙️ 设置...
```

---

### 2.4 用户界面规范

#### 界面设计原则

- **统一暗色风格**：深色背景（#0A0A0F / #111118）+ 绿色为主色调（#00E676 / #00C853）
- **科技感**：等宽字体用于数据，系统字体用于 UI
- **信息密度**：清晰分区，左侧导航 + 中央主区域 + 右侧详情/日志
- **实时反馈**：进度条、实时日志、速度/ETA 显示
- **多任务并行**：任务列表 + 独立进度 + 系统状态栏

#### 页面结构

**1. 首页 / 仪表盘 (Dashboard)**

```
┌─────────────────────────────────────────────────────────────┐
│ [GeekZip] [NORMAL] [PRO] [TERMINAL]    [波形] [⚙] [👤]  │
├─────────────────────────────────────────────────────────────┤
│ 导航栏          │              主区域            │  右侧面板  │
│                 │                               │           │
│  ARCHIVES       │  ┌─────────────────────────┐  │  选中项   │
│  ├─ Inbox (8)   │  │   DROP ARCHIVES HERE    │  │  详情     │
│  ├─ Processing  │  │   [📦 图标]              │  │  - Path   │
│  ├─ Extracted   │  │   OR CLICK TO BROWSE    │  │  - Size   │
│  └─ Favorites   │  │                         │  │  - Files  │
│                 │  │ [IMPORT FILES] [IMPORT  │  │  - Type   │
│  TOOLS          │  │  FOLDER]                │  │  - ...    │
│  ├─ Compress    │  └─────────────────────────┘  │           │
│  ├─ Extract     │                               │  [AI ANALYZE]│
│  ├─ Encrypt     │  PROCESSING (2)                │           │
│  ├─ Batch       │  ┌─────────────────────────┐  │  LOG      │
│  └─ ...         │  │ 📦 Project_Design.zip   │  │  [14:31:22]│
│                 │  │ 2.45 GB • 1284 files    │  │  Archive  │
│  AI ASSISTANT   │  │ ████████████░░ 72%      │  │  opened   │
│  ├─ Analyze     │  │ 12.4 MB/s  ETA 00:12    │  │  ...      │
│  ├─ Organize    │  │ [⏸] [✕]               │  │           │
│  ├─ Clean       │  └─────────────────────────┘  │           │
│  └─ Duplicates  │  ┌─────────────────────────┐  │           │
│                 │  │ 📦 Data_Backup.7z       │  │           │
│  [拖放区]       │  │ 4.12 GB • 5321 files    │  │           │
│  DRAG & DROP    │  │ ███████░░░░░░░ 41%      │  │           │
│  ARCHIVES HERE  │  │ 8.7 MB/s  ETA 00:28     │  │           │
│                 │  │ [⏸] [✕]               │  │           │
│                 │  └─────────────────────────┘  │           │
│                 │                               │           │
│                 │  RECENT ARCHIVES              │           │
│                 │  [📦 UI_Assets] [📦 Source]  │           │
│                 │  [📦 Document] [📦 Logs]     │           │
│                 │                               │           │
├─────────────────────────────────────────────────────────────┤
│ CPU 12% │ MEM 245MB │ THREADS 8 │ FILES/S 134 │ SPEED 12.4 MB/s │
│ RATIO 72% │                    ● ALL SYSTEMS OPERATIONAL       │
└─────────────────────────────────────────────────────────────┘
```

**2. 解压缩页面 (Extract)**

- 左侧：任务列表 / 文件选择
- 中央：解压设置面板
  - Archive File (显示选中文件)
  - Extract To (目标路径，默认当前目录)
  - Options (复选框组)
    - Overwrite existing files
    - Create subfolder
    - Open folder after extraction
    - Verify extracted files
  - Password (可选输入框)
  - [EXTRACT] [CANCEL] 按钮
- 右侧：进度面板 + 实时日志

**3. 压缩页面 (Compress)**

- 左侧：添加的文件/文件夹列表
- 中央：压缩设置
  - Archive Format (ZIP / 7Z / TAR / ...)
  - Compression Level (Store / Fast / Normal / Maximum / Ultra)
  - Options (复选框组)
    - Create solid archive
    - Add recovery record
    - Delete files after compression
  - Password (可选)
  - [COMPRESS] [CANCEL] 按钮
- 右侧：进度 + 日志

**4. 密码管理页面 (Password Manager)**

- 左侧：密码分类列表
- 中央：密码列表（带搜索框）
  - 每行: 图标 | 名称 | 用户名 | 密码 (隐藏) | 操作按钮
  - 示例: Work Laptop, GitHub, Dropbox, Email, Database, Server SSH
- 右侧：密码生成器
  - 长度滑块
  - 字符集选项 (A-Z, a-z, 0-9, !@#)
  - [GENERATE NEW] 按钮
  - 密码详情面板
- 底部: [+ ADD NEW PASSWORD] [EDIT] [DELETE]

**5. 任务完成页面 (Task Complete)**

- 大型成功图标 + "EXTRACTION COMPLETED!"
- 文件名称
- 统计信息: Files Extracted, Total Size, Time Taken, Errors
- 按钮: [OPEN FOLDER] [VIEW LOG] [CLOSE]

**6. 设置页面 (Settings)**

- 通用设置
  - 默认解压位置
  - 解压后是否删除源文件
  - 语言 (中/英)
- 解压设置
  - 递归深度上限 (默认 10)
  - 单文件大小上限 (默认 10GB)
  - 总解压大小上限 (默认 50GB)
  - 密码尝试超时 (默认 30s)
  - 自动保存成功密码
- 通知设置
  - 解压完成通知
  - 错误通知
  - 开机自启 (监控模式)
- 监控设置
  - 启用目录监控
  - 监控目录列表
  - 规则配置

---

## 3. 技术架构

### 3.1 总体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        用户界面层 (Frontend)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ 主窗口 UI    │  │ 右键菜单     │  │ 系统托盘 / 通知栏    │ │
│  │ (React/Vue)  │  │ (原生)       │  │ (Tauri API)          │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ 拖拽区域     │  │ 任务队列     │  │ 设置面板             │ │
│  │ 仪表盘       │  │ 进度组件     │  │ 日志查看器           │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                      Tauri 桥接层 (IPC)                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ Commands API │  │ Events API   │  │ File System API      │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                      Rust 核心层 (Backend)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ 格式识别     │  │ 文件名清理   │  │ 分卷合并             │ │
│  │ (magic)      │  │ (filename)   │  │ (volume)             │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ 解压引擎     │  │ 密码管理     │  │ 递归控制             │ │
│  │ (extract)    │  │ (password)   │  │ (recursive)          │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ 压缩引擎     │  │ 安全防护     │  │ 目录监控             │ │
│  │ (compress)   │  │ (safety)     │  │ (watcher)            │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ 任务调度     │  │ 数据库       │  │ AI 引擎 (本地)       │ │
│  │ (scheduler)  │  │ (sqlite)     │  │ (ai_local)           │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                      平台适配层 (Platform)                      │
│  ┌────────────────────────┐  ┌────────────────────────────┐   │
│  │ macOS                  │  │ Windows                    │   │
│  │ - Finder Extension     │  │ - Shell Extension / 注册表 │   │
│  │ - .dmg 打包            │  │ - .msi 安装包              │   │
│  │ - Keychain 集成        │  │ - Credential Manager       │   │
│  │ - Spotlight 集成        │  │ - 注册表文件关联           │   │
│  │ - Notification Center  │  │ - Action Center            │   │
│  └────────────────────────┘  └────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 技术栈选型

| 层级 | 技术选型 | 理由 |
|------|---------|------|
| **前端框架** | React 18 + TypeScript | 生态成熟，类型安全 |
| **UI 组件库** | Tailwind CSS + Headless UI | 灵活定制，符合暗色科技风格 |
| **状态管理** | Zustand | 轻量，适合 Tauri 场景 |
| **桌面框架** | Tauri v2 | 轻量 (Rust 后端)，安全沙箱，跨平台 |
| **后端语言** | Rust | 性能、安全、C 库绑定 |
| **解压引擎** | `sevenz-rust` + `zip` + `tar` + `flate2` | 覆盖主要格式 |
| **Magic 识别** | `infer` crate | 基于 magic bytes |
| **密码存储** | `keyring` crate | 系统集成 Keychain/Credential Manager |
| **数据库** | `rusqlite` | 本地 SQLite，密码历史/任务记录 |
| **目录监控** | `notify` crate | 跨平台文件系统监控 |
| **分卷合并** | 自定义 Rust 实现 | 顺序读写拼接 |
| **任务调度** | `tokio` + `async` | 异步 IO，并行解压 |
| **IPC 通信** | Tauri Commands + Events | 前后端通信 |
| **打包** | `cargo-bundle` + Tauri Bundler | dmg / msi |
| **本地 AI** | 规则引擎 + `ort` (ONNX Runtime) | 轻量本地推理，无需云端 |
| **日志** | `tracing` + `tracing-subscriber` | 结构化日志，可输出到前端 |
| **配置** | `config` crate | 支持 JSON/TOML/YAML |
| **错误处理** | `anyhow` + `thiserror` | 简洁的错误处理 |

### 3.3 核心 Crate 结构

```rust
// 工作区结构
geekzip/
├── Cargo.toml              # Workspace 定义
├── crates/
│   ├── geekzip-core/       # 核心库
│   │   ├── src/
│   │   │   ├── format/     # 格式识别 (magic bytes)
│   │   │   ├── extract/    # 解压引擎
│   │   │   ├── compress/   # 压缩引擎
│   │   │   ├── password/   # 密码管理
│   │   │   ├── filename/   # 文件名处理
│   │   │   ├── volume/     # 分卷处理
│   │   │   ├── recursive/  # 递归解压
│   │   │   ├── safety/     # 安全防护 (bomb 检测)
│   │   │   ├── task/       # 任务调度
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   │
│   ├── geekzip-cli/        # CLI 工具
│   │   ├── src/main.rs
│   │   └── Cargo.toml
│   │
│   └── geekzip-watcher/    # 目录监控守护进程
│       ├── src/main.rs
│       └── Cargo.toml
│
├── src-tauri/              # Tauri 应用
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/       # IPC 命令
│   │   ├── events/         # 事件发射
│   │   └── tray/           # 系统托盘
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── src/                    # 前端代码
│   ├── components/         # React 组件
│   ├── pages/              # 页面
│   ├── stores/             # Zustand 状态
│   ├── hooks/              # 自定义 Hooks
│   └── App.tsx
│
├── assets/                 # 图标、字体
├── scripts/                # 构建脚本
├── docs/                   # 文档
└── README.md
```

---

## 4. 数据模型

### 4.1 任务 (Task)

```typescript
interface Task {
  id: string;                    // UUID
  type: 'extract' | 'compress' | 'batch';
  status: TaskStatus;
  
  // 源文件
  source: {
    path: string;
    size: number;
    format: ArchiveFormat;       // 识别后的真实格式
    originalFormat?: string;    // 原始扩展名
  };
  
  // 目标
  target: {
    path: string;
    strategy: 'current' | 'subfolder' | 'custom';
  };
  
  // 进度
  progress: {
    percent: number;
    currentFile: string;
    speedBytesPerSec: number;
    processedBytes: number;
    totalBytes: number;
    etaSeconds: number;
    filesTotal: number;
    filesProcessed: number;
  };
  
  // 密码
  password?: string;
  passwordAttempts: PasswordAttempt[];
  
  // 递归
  recursive: {
    enabled: boolean;
    currentDepth: number;
    maxDepth: number;
  };
  
  // 选项
  options: {
    overwrite: 'skip' | 'overwrite' | 'rename';
    createSubfolder: boolean;
    openAfterExtract: boolean;
    verifyFiles: boolean;
    deleteAfterExtract: boolean;
    deleteIntermediate: boolean;
  };
  
  // 日志
  log: LogEntry[];
  
  // 时间
  createdAt: number;
  startedAt?: number;
  completedAt?: number;
  
  // 结果
  result?: {
    success: boolean;
    extractedFiles: string[];
    finalPath: string;
    layersUnpacked: number;
    error?: string;
  };
}

type TaskStatus = 
  | 'queued' 
  | 'analyzing' 
  | 'password_required' 
  | 'extracting' 
  | 'verifying' 
  | 'completed' 
  | 'cancelled' 
  | 'error';

interface PasswordAttempt {
  password: string;
  source: 'filename' | 'history' | 'builtin' | 'dictionary' | 'manual';
  success: boolean;
  timestamp: number;
}

interface LogEntry {
  timestamp: number;
  level: 'info' | 'warn' | 'error' | 'success';
  message: string;
  detail?: string;
}
```

### 4.2 密码记录 (PasswordRecord)

```typescript
interface PasswordRecord {
  id: string;
  label: string;
  username?: string;
  password: string;           // 加密存储
  website?: string;
  notes?: string;
  category: string;
  
  // 使用统计
  usageCount: number;
  lastUsedAt: number;
  lastSuccessAt: number;
  
  // 元数据
  createdAt: number;
  updatedAt: number;
}
```

### 4.3 压缩包信息 (ArchiveInfo)

```typescript
interface ArchiveInfo {
  path: string;
  name: string;
  
  // 文件大小
  size: number;
  
  // 格式识别
  format: ArchiveFormat;
  detectedBy: 'extension' | 'magic_bytes';
  
  // 分卷
  isVolume: boolean;
  volumeParts?: string[];
  
  // 内容预览
  fileCount?: number;
  totalSize?: number;
  compressedSize?: number;
  compressionRatio?: number;
  
  // 加密
  encrypted: boolean;
  encryptionMethod?: string;  // 'AES-256', 'ZIPCrypto', etc.
  
  // 校验
  integrity?: 'verified' | 'unverified' | 'corrupt';
  
  // 时间
  createdAt?: number;
  modifiedAt?: number;
  
  // 密码提示
  passwordHint?: string;
}

type ArchiveFormat = 
  | 'zip' | 'rar' | 'rar5' | 'sevenz' | 'tar' 
  | 'gz' | 'bz2' | 'xz' | 'zstd' | 'lz4'
  | 'unknown';
```

### 4.4 设置 (Settings)

```typescript
interface Settings {
  // 通用
  language: 'zh' | 'en';
  theme: 'dark' | 'light' | 'auto';
  
  // 解压
  defaultExtractPath: string;        // 'current' | 绝对路径
  defaultOverwrite: 'skip' | 'overwrite' | 'rename';
  defaultCreateSubfolder: boolean;
  defaultOpenAfterExtract: boolean;
  defaultDeleteAfterExtract: boolean;
  defaultDeleteIntermediate: boolean;
  
  // 递归
  recursiveEnabled: boolean;
  recursiveMaxDepth: number;
  
  // 安全
  singleFileSizeLimit: number;       // bytes, default 10GB
  totalExtractSizeLimit: number;     // bytes, default 50GB
  passwordTimeoutSeconds: number;    // default 30
  
  // 密码
  autoSavePasswords: boolean;
  useBuiltinPasswords: boolean;
  usePasswordDictionary: boolean;
  passwordDictionaryPath?: string;
  
  // 通知
  showCompletionNotification: boolean;
  showErrorNotification: boolean;
  
  // 监控
  enableWatcher: boolean;
  watchPaths: string[];
  watcherRules: WatcherRule[];
  startOnLogin: boolean;
  
  // 性能
  maxConcurrentTasks: number;
  maxThreads: number;
}

interface WatcherRule {
  id: string;
  enabled: boolean;
  path: string;
  formats: ArchiveFormat[];
  minSize?: number;
  maxSize?: number;
  action: 'extract' | 'smart_extract';
  autoDelete: boolean;
}
```

---

## 5. 核心算法与逻辑

### 5.1 文件名智能处理

```rust
/// 干扰字符模式
static INTERFERENCE_PATTERNS: &[&str] = &[
    "删除", "去掉", "勿", "不要", "取消",
    "delete", "remove", "cancel", "drop",
];

/// 清理文件名中的干扰字符
fn clean_filename(name: &str) -> String {
    let mut cleaned = name.to_string();
    for pattern in INTERFERENCE_PATTERNS {
        cleaned = cleaned.replace(pattern, "");
    }
    // 清理多余空格和特殊字符
    cleaned = cleaned.trim().to_string();
    cleaned
}

/// 从文件名提取密码
/// 支持模式: [密码:xxx] / pwd:xxx / pass:xxx / 密码xxx
fn extract_password_from_filename(name: &str) -> Option<String> {
    // Regex: 各种常见密码标注模式
    let patterns = [
        r"[\[\(【]\s*密码[\:：]\s*(.+?)\s*[\]\)】]",
        r"[\[\(【]\s*pwd[\:：]\s*(.+?)\s*[\]\)】]",
        r"[\[\(【]\s*pass[\:：]\s*(.+?)\s*[\]\)】]",
        r"密码\s*[\:：]\s*(\S+)",
        r"pwd\s*[\:：]\s*(\S+)",
    ];
    // ...
}

/// 分卷识别
/// 模式: .001/.002, .part1/.part2, .z01/.z02, 7z.001, 无点号分卷
fn detect_volume_parts(base_path: &Path) -> Option<Vec<PathBuf>> {
    let name = base_path.file_stem()?.to_string_lossy();
    let parent = base_path.parent()?;
    
    // 模式 1: 数字扩展名 .001, .002
    // 模式 2: .part1, .part2, .part3
    // 模式 3: .z01, .z02
    // 模式 4: 7z.001, 7z.002
    // 模式 5: 无点号分卷 (如 "苹果7z002", "苹果7z003")
    
    // 收集同目录下相似文件名
    let mut candidates = vec![];
    for entry in std::fs::read_dir(parent).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if is_similar_volume(&name, &path) {
            candidates.push(path);
        }
    }
    
    // 按序号排序
    candidates.sort_by(|a, b| volume_index(a).cmp(&volume_index(b)));
    
    if candidates.len() > 1 {
        Some(candidates)
    } else {
        None
    }
}
```

### 5.2 Magic Bytes 识别表

| 格式 | Magic Bytes | 偏移 | 说明 |
|------|-------------|------|------|
| ZIP | `50 4B 03 04` | 0 | PK.. |
| ZIP (Empty) | `50 4B 05 06` | 0 | PK.. |
| ZIP (Spanned) | `50 4B 07 08` | 0 | PK.. |
| RAR v4 | `52 61 72 21 1A 07 00` | 0 | Rar!... |
| RAR v5 | `52 61 72 21 1A 07 01 00` | 0 | Rar!.... |
| 7Z | `37 7A BC AF 27 1C` | 0 | 7z... |
| TAR | `75 73 74 61 72` | 257 | ustar |
| GZ | `1F 8B` | 0 | gzip |
| BZ2 | `42 5A 68` | 0 | BZh |
| XZ | `FD 37 7A 58 5A 00` | 0 | \xfd7zXZ. |
| ZSTD | `28 B5 2F FD` | 0 | ... |
| LZ4 | `04 22 4D 18` | 0 | ... |

### 5.3 密码尝试引擎

```rust
/// 密码尝试优先级队列
async fn try_passwords(
    archive: &Path,
    hints: &[String],
    history: &[PasswordRecord],
    dictionary: &[String],
    timeout: Duration,
) -> Result<String, PasswordError> {
    let mut passwords = Vec::new();
    
    // 1. 文件名提取 (O(1))
    for hint in hints {
        passwords.push((hint.clone(), Priority::Filename));
    }
    
    // 2. 历史密码 (按成功率排序)
    for record in history.iter().filter(|r| r.lastSuccessAt > 0) {
        passwords.push((record.password.clone(), Priority::History));
    }
    
    // 3. 内置常用密码
    for builtin in BUILTIN_PASSWORDS {
        passwords.push((builtin.to_string(), Priority::Builtin));
    }
    
    // 4. 密码本字典
    for pwd in dictionary {
        passwords.push((pwd.clone(), Priority::Dictionary));
    }
    
    // 去重并保持顺序
    passwords = deduplicate_keep_order(passwords);
    
    // 并行尝试 (最多 4 线程)
    let semaphore = Arc::new(Semaphore::new(4));
    
    for (password, priority) in passwords {
        let permit = semaphore.clone().acquire_owned().await?;
        let result = tokio::time::timeout(
            timeout,
            try_single_password(archive, password.clone())
        ).await;
        
        drop(permit);
        
        match result {
            Ok(Ok(())) => {
                // 成功！记录密码
                record_success(&password, priority).await?;
                return Ok(password);
            }
            Ok(Err(_)) => continue, // 密码错误
            Err(_) => continue,     // 超时
        }
    }
    
    Err(PasswordError::NotFound)
}
```

### 5.4 安全防护 (Anti-Bomb)

```rust
/// 安全检查
struct SafetyGuard {
    single_file_size_limit: u64,
    total_extract_size_limit: u64,
    max_depth: u32,
}

impl SafetyGuard {
    fn check(&self, archive: &ArchiveInfo) -> Result<(), SafetyError> {
        // 1. 检查单文件大小
        if archive.size > self.single_file_size_limit {
            return Err(SafetyError::FileTooLarge);
        }
        
        // 2. 检查压缩比 (Zip Bomb)
        if let Some(ratio) = archive.compressionRatio {
            if ratio > 1000.0 {
                return Err(SafetyError::SuspiciousCompressionRatio);
            }
        }
        
        // 3. 检查嵌套深度
        if archive.recursiveDepth > self.max_depth {
            return Err(SafetyError::MaxDepthExceeded);
        }
        
        // 4. 检查总解压大小预估
        if let Some(total) = archive.totalSize {
            if total > self.total_extract_size_limit {
                return Err(SafetyError::TotalSizeExceeded);
            }
        }
        
        Ok(())
    }
}

/// 嵌套压缩炸弹特征
fn detect_zip_bomb<R: Read>(reader: &mut R) -> Result<bool, io::Error> {
    // 检查常见的 zip bomb 特征
    // 1. 压缩比异常高
    // 2. 大量重复的小文件
    // 3. 递归压缩
    // 4. 文件名超长
    Ok(false)
}
```

---

## 6. API 接口 (Tauri Commands)

### 6.1 任务管理

```rust
// 创建解压任务
#[tauri::command]
async fn create_extract_task(
    source: Vec<String>,
    options: ExtractOptions,
    app: AppHandle,
) -> Result<String, Error> {
    // 返回 task_id
}

// 取消任务
#[tauri::command]
async fn cancel_task(task_id: String) -> Result<(), Error> {}

// 获取任务列表
#[tauri::command]
async fn get_tasks(status: Option<TaskStatus>) -> Result<Vec<Task>, Error> {}

// 获取任务详情
#[tauri::command]
async fn get_task(task_id: String) -> Result<Task, Error> {}

// 获取任务日志
#[tauri::command]
async fn get_task_logs(task_id: String) -> Result<Vec<LogEntry>, Error> {}
```

### 6.2 文件操作

```rust
// 分析文件 (Magic Bytes + 元数据)
#[tauri::command]
async fn analyze_file(path: String) -> Result<ArchiveInfo, Error> {}

// 批量分析
#[tauri::command]
async fn analyze_files(paths: Vec<String>) -> Result<Vec<ArchiveInfo>, Error> {}

// 选择目录 (调用系统对话框)
#[tauri::command]
async fn select_directory() -> Result<Option<String>, Error> {}

// 选择文件
#[tauri::command]
async fn select_files(multiple: bool) -> Result<Vec<String>, Error> {}
```

### 6.3 密码管理

```rust
// 获取密码列表
#[tauri::command]
async fn get_passwords(category: Option<String>) -> Result<Vec<PasswordRecord>, Error> {}

// 添加密码
#[tauri::command]
async fn add_password(record: PasswordRecord) -> Result<String, Error> {}

// 更新密码
#[tauri::command]
async fn update_password(id: String, record: PasswordRecord) -> Result<(), Error> {}

// 删除密码
#[tauri::command]
async fn delete_password(id: String) -> Result<(), Error> {}

// 导入密码本
#[tauri::command]
async fn import_passwords(path: String) -> Result<ImportResult, Error> {}

// 导出密码本
#[tauri::command]
async fn export_passwords(path: String, format: ExportFormat) -> Result<(), Error> {}

// 生成密码
#[tauri::command]
async fn generate_password(options: PasswordGenOptions) -> Result<String, Error> {}
```

### 6.4 设置

```rust
// 获取设置
#[tauri::command]
async fn get_settings() -> Result<Settings, Error> {}

// 保存设置
#[tauri::command]
async fn save_settings(settings: Settings) -> Result<(), Error> {}

// 重置设置
#[tauri::command]
async fn reset_settings() -> Result<(), Error> {}
```

### 6.5 系统事件 (前端订阅)

```typescript
// 任务进度更新
listen('task:progress', (event: { taskId: string; progress: TaskProgress }) => {
  // 更新进度条
});

// 任务状态变更
listen('task:status', (event: { taskId: string; status: TaskStatus }) => {
  // 更新状态
});

// 新日志
listen('task:log', (event: { taskId: string; log: LogEntry }) => {
  // 追加到日志面板
});

// 任务完成
listen('task:completed', (event: { taskId: string; result: TaskResult }) => {
  // 显示完成通知
});

// 监控新文件
listen('watcher:new_file', (event: { path: string; info: ArchiveInfo }) => {
  // 自动处理或提示用户
});

// 系统托盘事件
listen('tray:event', (event: { action: string }) => {
  // 处理托盘点击
});
```

---

## 7. 开发里程碑

### M1: Rust CLI 核心 (Week 1-2)

**目标**: 命令行能完成所有核心操作

- [ ] 项目初始化 (Cargo Workspace + Tauri v2)
- [ ] 格式识别 (Magic Bytes)
- [ ] 基础解压 (ZIP / 7Z / TAR / GZ)
- [ ] 基础压缩 (ZIP / 7Z)
- [ ] 文件名清理与密码提取
- [ ] 分卷识别与合并
- [ ] CLI 接口设计

**产出**: `geekzip-cli` 可命令行解压/压缩

### M2: 智能引擎 (Week 3-4)

**目标**: 智能化处理

- [ ] 密码本系统 (SQLite + keyring)
- [ ] 密码尝试引擎 (多线程)
- [ ] 递归解压 (含循环检测)
- [ ] 安全防护 (Zip Bomb 检测)
- [ ] 文件名无后缀识别
- [ ] 无点号分卷分组

**产出**: CLI 支持智能解压、递归解压、密码自动尝试

### M3: Tauri GUI 基础 (Week 5-6)

**目标**: 主窗口可用

- [ ] 前端项目初始化 (React + Tailwind)
- [ ] 仪表盘布局 (三栏式)
- [ ] 拖放区域实现
- [ ] 任务列表与进度显示
- [ ] 系统状态栏
- [ ] 实时日志面板
- [ ] 设置页面

**产出**: 可拖拽文件解压，有进度和日志

### M4: 完整功能 GUI (Week 7-8)

**目标**: 所有功能页面完成

- [ ] 解压详细页面 (选项、密码)
- [ ] 压缩页面
- [ ] 密码管理页面 (列表、生成器)
- [ ] 任务完成页面
- [ ] AI 助手页面 (Analyze/Organize/Clean/Duplicates)
- [ ] 批量处理队列
- [ ] 多任务并行

**产出**: 所有功能可通过 GUI 操作

### M5: 系统集成 (Week 9-10)

**目标**: 系统级集成

- [ ] macOS Finder Extension / Quick Action
- [ ] Windows 右键菜单 (注册表)
- [ ] 系统托盘 (macOS/Windows)
- [ ] 系统通知
- [ ] 文件关联
- [ ] Dock/任务栏拖拽

**产出**: 右键菜单可用，系统集成完成

### M6: 高级功能与打包 (Week 11-12)

**目标**: 完善与分发

- [ ] 目录监控 (Watcher)
- [ ] 自动模式 / 静默模式
- [ ] 规则引擎
- [ ] 多语言支持
- [ ] 错误处理与恢复
- [ ] 性能优化
- [ ] .dmg 打包 (macOS)
- [ ] .msi 打包 (Windows)
- [ ] 安装程序
- [ ] 文档与 README

**产出**: 可分发的安装包

---

## 8. 安全与隐私

### 8.1 密码安全

- **存储**: 所有密码使用系统级安全存储
  - macOS: Keychain
  - Windows: Credential Manager / DPAPI
  - 本地数据库中的密码字段使用 AES-256-GCM 加密
- **内存**: 密码在内存中使用 `zeroize` crate 清除
- **传输**: IPC 通信中敏感数据使用 Tauri 的安全通道

### 8.2 文件安全

- **沙箱**: Tauri 的安全沙箱限制前端访问
- **权限**: 仅请求必要的文件系统权限
- **校验**: 解压后校验文件完整性
- **清理**: 临时文件自动清理

### 8.3 防恶意压缩包

- Zip Bomb 检测 (压缩比、深度、大小)
- 路径遍历防护 (清理 `../` 路径)
- 符号链接处理安全
- 文件类型白名单 (可选)

---

## 9. 性能指标

| 指标 | 目标 | 测试方式 |
|------|------|----------|
| 启动时间 | < 3s | 冷启动到主窗口显示 |
| 格式识别 | < 100ms / 文件 | 1000 个文件批量识别 |
| 单任务解压速度 | 达到磁盘 IO 上限 | 测试 1GB ZIP 文件 |
| 并发任务数 | 默认 4 个 | 同时解压 4 个 1GB 文件 |
| 密码尝试速度 | > 100 个/秒 | 使用 1000 密码字典测试 |
| 内存占用 | < 500MB (空闲) | 主窗口打开后空闲状态 |
| 内存占用 | < 2GB (峰值) | 解压 10GB 文件时 |
| 包大小 | < 50MB (安装包) | 最终分发包 |

---

## 10. 错误处理规范

### 10.1 错误分类

| 错误类型 | 示例 | 用户提示 | 处理方式 |
|----------|------|----------|----------|
| **格式错误** | 文件损坏、无法识别 | "文件损坏或格式不支持" | 记录日志，跳过 |
| **密码错误** | 密码不正确 | "无法找到正确密码，请手动输入" | 弹窗提示 |
| **IO 错误** | 磁盘空间不足、权限不足 | "磁盘空间不足，请清理后重试" | 暂停任务，通知用户 |
| **安全错误** | Zip Bomb、深度超限 | "检测到可疑压缩包，已停止解压" | 终止任务，警告 |
| **分卷错误** | 分卷缺失 | "分卷文件不完整，请检查" | 提示缺失文件 |
| **循环错误** | 递归循环 | "检测到循环压缩，已停止" | 终止递归分支 |
| **未知错误** | 内部错误 | "发生未知错误，请查看日志" | 记录详细日志 |

### 10.2 错误编码

```rust
enum ArchivexError {
    // 格式
    UnsupportedFormat { path: String, detected: String },
    CorruptArchive { path: String, reason: String },
    
    // 密码
    PasswordNotFound { attempted: usize },
    PasswordRequired,
    
    // IO
    DiskFull { required: u64, available: u64 },
    PermissionDenied { path: String },
    PathNotFound { path: String },
    
    // 安全
    ZipBombDetected { ratio: f64 },
    MaxDepthExceeded { depth: u32 },
    MaxSizeExceeded { size: u64, limit: u64 },
    
    // 分卷
    VolumeMissing { expected: String },
    VolumeOutOfOrder { expected: String, found: String },
    
    // 递归
    RecursiveLoop { path: String },
    
    // 内部
    Internal(String),
}
```

---

## 11. 测试策略

### 11.1 单元测试

- **格式识别**: 每种格式的 Magic Bytes 测试
- **文件名处理**: 各种干扰字符、密码提取模式
- **分卷识别**: 所有分卷命名模式
- **密码引擎**: 优先级队列、并发尝试
- **安全防护**: 边界值、Zip Bomb 样本

### 11.2 集成测试

- **端到端解压**: 各种格式、大小、密码
- **递归解压**: 多层嵌套、循环检测
- **分卷合并**: 大文件分卷合并完整性
- **并发处理**: 多任务并行稳定性

### 11.3 测试样本

```
tests/fixtures/
├── formats/
│   ├── sample.zip          # 标准 ZIP
│   ├── sample.rar          # RAR4
│   ├── sample.rar5         # RAR5
│   ├── sample.7z           # 7Z
│   ├── sample.tar.gz       # TAR.GZ
│   ├── sample.tar.bz2      # TAR.BZ2
│   ├── sample.tar.xz       # TAR.XZ
│   └── sample.zstd         # ZSTD
├── password/
│   ├── encrypted_aes.zip   # AES-256 加密
│   ├── encrypted_zip.zip   # ZIPCrypto 加密
│   ├── [密码:1234].zip     # 文件名含密码
│   └── [pwd:abcd].zip      # 文件名含密码
├── volumes/
│   ├── sample.001          # 001 分卷
│   ├── sample.002
│   ├── sample.part1.rar    # part 分卷
│   ├── sample.part2.rar
│   ├── sample.7z.001       # 7z 分卷
│   ├── sample.7z.002
│   ├── sample.z01          # ZIP 分卷
│   ├── sample.z02
│   └── 苹果7z001           # 无点号分卷
│   └── 苹果7z002
├── recursive/
│   ├── level1.zip          # 包含 level2.zip
│   └── level2.zip          # 包含 level3.zip
├── safety/
│   ├── zip_bomb.zip        # 高压缩比炸弹
│   └── path_traversal.zip  # 包含 ../ 路径
└── clean/
    ├── 去掉.zip             # 干扰字符
    ├── 删除.zip             # 干扰字符
    └── 文件.zip.删除.zip     # 多层干扰
```

---

## 12. 附录

### 12.1 术语表

| 术语 | 说明 |
|------|------|
| **Magic Bytes** | 文件头部用于标识文件格式的字节序列 |
| **Zip Bomb** | 压缩比极高、解压后体积极大的恶意压缩包 |
| **Solid Compression** | 将所有文件作为一个整体压缩，提高压缩率 |
| **SFX** | Self-Extracting Archive，自解压文件 |
| **Keychain** | macOS 系统密码管理器 |
| **Credential Manager** | Windows 系统密码管理器 |
| **Finder Extension** | macOS 文件管理器扩展 |
| **Shell Extension** | Windows 资源管理器扩展 |

### 12.2 参考库

- **Rust 解压**: `sevenz-rust`, `zip`, `tar`, `flate2`, `bzip2`, `xz2`, `zstd`, `lz4`
- **Rust 安全**: `zeroize`, `aes-gcm`, `ring`
- **Rust 文件**: `notify`, `walkdir`, `tempfile`
- **Rust 异步**: `tokio`, `async-trait`
- **Tauri**: `tauri`, `tauri-build`, `tauri-plugin-`
- **前端**: `react`, `tailwindcss`, `zustand`, `lucide-react`

### 12.3 设计资源

- **颜色方案**:
  - 背景: `#0A0A0F` / `#111118` / `#1A1A24`
  - 主色 (绿): `#00E676` / `#00C853` / `#69F0AE`
  - 次色: `#2962FF` (蓝) / `#FF6D00` (橙) / `#D50000` (红)
  - 文字: `#FFFFFF` (主) / `#B0BEC5` (次) / `#607D8B` (灰)
  - 边框: `#2D2D3A` / `#3D3D4F`
  
- **字体**:
  - UI: `SF Pro Display` (macOS) / `Segoe UI` (Windows)
  - 数据: `JetBrains Mono` / `Fira Code`
  - 等宽: `SF Mono` / `Consolas`

- **图标**: `Lucide Icons` + 自定义归档相关图标

---

## 13. 修订记录

| 版本 | 日期 | 变更 |
|------|------|------|
| 0.1.0 | 2026-06-12 | 初始版本，基于用户需求与 UI 设计图综合整理 |

---

> **下一步**: 本 SPEC 确认后，进入 **M1: Rust CLI 核心** 开发阶段，首先初始化项目结构并实现格式识别与基础解压。
