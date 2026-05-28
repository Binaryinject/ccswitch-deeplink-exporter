# CC Switch Deeplink Exporter

[English](#english) | [中文](#中文)

---

## English

A lightweight desktop tool that reads the [CC Switch](https://github.com/farion1231/cc-switch) configuration database and batch-exports deeplinks for all providers.

### Features

- **Auto-detect** — Reads `~/.cc-switch/cc-switch.db` directly, no manual export needed
- **Multi-app support** — Claude, Codex, Gemini providers
- **Skill repos** — Export deeplinks for all configured Skill repositories
- **One-click copy** — Copy single deeplink or all links at once
- **HTML export** — Generate a standalone shareable HTML page
- **Filter by app** — Quickly filter by Claude / Codex / Gemini / Skills
- **Lightweight** — ~12MB standalone exe, no runtime dependencies

### Screenshots

> Dark-themed UI with provider cards, filter buttons, and copy/export actions.

### Prerequisites

- [CC Switch](https://github.com/farion1231/cc-switch) installed and configured (at least one provider added)
- [Node.js](https://nodejs.org/) v18+
- [Rust](https://rustup.rs/) toolchain (for building from source)

### Development

```bash
# Install dependencies
npm install

# Start dev server with hot reload
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

Output: `src-tauri/target/release/ccswitch-deeplink-exporter.exe`

### How It Works

1. The Rust backend opens `~/.cc-switch/cc-switch.db` (SQLite) in read-only mode
2. Queries the `providers`, `provider_endpoints`, and `skill_repos` tables
3. Constructs `ccswitch://v1/import?...` deeplinks with Base64-encoded config
4. The frontend renders provider cards with copy/import actions

### Deeplink Format

Each provider generates a deeplink like:

```
ccswitch://v1/import?resource=provider&app=claude&name=MyProvider&config=<base64>&configFormat=json
```

The `config` parameter contains the full provider configuration (API key, endpoint, models) encoded in Base64.

### Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | [Tauri v2](https://tauri.app) |
| Backend | Rust + [rusqlite](https://crates.io/crates/rusqlite) |
| Frontend | TypeScript + Vite |
| Styling | Vanilla CSS (dark theme) |

### License

MIT

---

## 中文

一个轻量级桌面工具，自动读取 [CC Switch](https://github.com/farion1231/cc-switch) 配置数据库，批量导出所有供应商的 deeplink。

### 功能特性

- **自动检测** — 直接读取 `~/.cc-switch/cc-switch.db`，无需手动导出
- **多应用支持** — Claude、Codex、Gemini 供应商
- **Skill 仓库** — 导出所有已配置的 Skill 仓库 deeplink
- **一键复制** — 复制单个链接或全部链接
- **导出 HTML** — 生成可分享的独立 HTML 页面
- **按类型筛选** — 快速筛选 Claude / Codex / Gemini / Skills
- **轻量级** — 约 12MB 独立 exe，无运行时依赖

### 前置要求

- 已安装并配置 [CC Switch](https://github.com/farion1231/cc-switch)（至少添加过一个供应商）
- [Node.js](https://nodejs.org/) v18+
- [Rust](https://rustup.rs/) 工具链（从源码构建时需要）

### 开发

```bash
# 安装依赖
npm install

# 启动开发服务器（支持热重载）
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

产物路径：`src-tauri/target/release/ccswitch-deeplink-exporter.exe`

### 工作原理

1. Rust 后端以只读模式打开 `~/.cc-switch/cc-switch.db` (SQLite)
2. 查询 `providers`、`provider_endpoints`、`skill_repos` 表
3. 构造 `ccswitch://v1/import?...` 格式的 deeplink，配置内容使用 Base64 编码
4. 前端渲染供应商卡片，支持复制和导入操作

### Deeplink 格式

每个供应商生成的 deeplink 格式如下：

```
ccswitch://v1/import?resource=provider&app=claude&name=MyProvider&config=<base64>&configFormat=json
```

其中 `config` 参数包含完整的供应商配置（API Key、端点、模型），使用 Base64 编码。

### 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | [Tauri v2](https://tauri.app) |
| 后端 | Rust + [rusqlite](https://crates.io/crates/rusqlite) |
| 前端 | TypeScript + Vite |
| 样式 | 原生 CSS（暗色主题） |

### 项目结构

```
├── src/                    # 前端源码
│   ├── main.ts            # 主逻辑：调用 Rust API、渲染 UI
│   └── style.css          # 暗色主题样式
├── src-tauri/             # Rust 后端
│   ├── src/main.rs        # SQLite 读取 + deeplink 生成
│   ├── Cargo.toml         # Rust 依赖
│   └── tauri.conf.json    # Tauri 配置
├── index.html             # 入口 HTML
├── package.json           # Node.js 依赖
└── vite.config.ts         # Vite 构建配置
```

### 许可证

MIT
