# TDengine Viewer

一款简洁、高性能的 TDengine 图形化管理工具，界面风格参考 Navicat。基于 Tauri 2 构建，原生 Rust 后端 + Web 前端，安装包小巧、启动迅速，支持 Windows、macOS 与 Linux。

## 功能特性

- **连接管理**：多连接配置（本地持久化）、WebSocket 连接（taosAdapter）、服务器版本显示
- **对象浏览**：左侧树形浏览器展示连接、数据库、超级表 / 普通表 / 视图，支持关键字过滤；右键数据库可直接新建基于该库的查询窗口
- **SQL 查询**：多标签查询工作区、CodeMirror 编辑器（语法高亮 / 自动补全）、**表名智能提示**（输入部分表名如 `metr` 自动匹配 `metric_history_data` 等）、执行选中语句或全部、编辑器 / 结果集高度可拖拽调整
- **查询结果**：多结果集切换、点击列头本地排序、**虚拟滚动**（仅渲染可见行，大数据量流畅滚动）
- **表设计器**：字段查看与维护（新增 / 删除 / 修改，基于 `ALTER TABLE`）、右侧同步展示建表语句（DDL）
- **数据浏览**：分页浏览表数据、字段类型渲染、点击列头**远程排序**（服务端 ORDER BY，支持全量数据排序）、**Navicat 式列筛选**（包含 / 等于 / 大于等 8 种操作符，多列条件叠加）、虚拟滚动渲染
- **表管理**：新建 / 删除 / 清空表
- **界面**：暗色 / 亮色主题切换、侧边栏宽度可拖拽调整（自动记忆）、现代化简洁设计

## 技术栈

| 层 | 技术 |
| --- | --- |
| 框架 | Tauri 2 |
| 后端 | Rust（taos-rs，WebSocket） |
| 前端 | Vue 3 + TypeScript |
| UI 组件 | Naive UI |
| SQL 编辑器 | CodeMirror 6 |

## 下载安装

前往 [Releases](https://github.com/xhuanlee/taos-viewer/releases) 下载对应平台安装包：

| 平台 | 产物 |
| --- | --- |
| macOS（Apple Silicon） | `.dmg`（aarch64） |
| macOS（Intel） | `.dmg`（x86_64） |
| Windows | `.msi` / `.exe` |
| Linux | `.deb` / `.AppImage` |

> macOS 用户首次打开如遇"无法验证开发者"提示，请右键应用选择「打开」，或在系统设置中允许。

## 环境要求

- [Node.js](https://nodejs.org/) ≥ 18
- [Rust](https://www.rust-lang.org/)（stable 工具链）
- Tauri 2 系统依赖：[官方文档](https://tauri.app/start/prerequisites/)
- TDengine 服务器需启用 **taosAdapter**（默认端口 `6041`，提供 WebSocket 接口）

## 本地开发

```bash
# 安装依赖
npm install

# 以开发模式运行（自动启动 Vite + Tauri 窗口）
npm run tauri dev
```

## 构建打包

```bash
npm run tauri build
```

- **macOS**：生成 `.app` 与 `.dmg`（`src-tauri/target/release/bundle/`）
- **Windows**：生成 `.msi` / `.exe` 安装包（需在 Windows 机器上执行）

## 使用说明

1. 启动应用后点击左上角「新建连接」
2. 填写 taosAdapter 地址（如 `localhost:6041`）与用户名密码（默认 `root` / `taosdata`）
3. 连接成功后即可浏览数据库对象、执行 SQL、设计表结构、浏览表数据

## 已知限制

- 当前 taos WS 驱动（taos-ws）暂不支持 `DECIMAL` 类型的二进制结果解析，应用已自动处理，**无需手动 CAST**：
  - 浏览表数据与执行简单 `SELECT` 查询时，DECIMAL 列会被自动改写为 `CAST(列 AS VARCHAR)`，用户无感知
  - 若驱动仍解析失败，将自动降级到 taosAdapter REST API 兜底执行
- 同一连接上的查询按串行执行（规避旧版 taosAdapter 的并发竞态问题）

## 项目结构

```
├── src/                    # Vue 3 前端
│   ├── components/         # 界面组件（侧边栏、查询页、表设计器等）
│   ├── stores/             # Pinia 状态管理
│   └── api/                # Tauri 命令封装
└── src-tauri/              # Rust 后端
    └── src/
        ├── commands.rs     # Tauri 命令（连接 / 查询 / 元数据）
        ├── decimal_compat.rs # DECIMAL 自动 CAST 改写 + REST 兜底
        ├── state.rs        # 连接状态管理
        └── models.rs       # 数据模型
```

## License

[Apache-2.0](LICENSE)
