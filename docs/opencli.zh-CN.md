# OpenCLI 插件

<p align="center">
  <a href="./opencli.md">English</a>
  &nbsp;·&nbsp;
  <a href="./opencli.zh-CN.md">简体中文</a>
</p>

官方 **Agent** 插件 `opencli`（**默认关闭**）。封装本机 [OpenCLI](https://github.com/jackwener/OpenCLI) CLI，让 Anya 能跑站点适配器，并通过 Browser Bridge 操控**已登录的 Chrome**——与 [电脑操控](./computer-use.zh-CN.md)（Ghost 桌面 + CDP）互补。

## 前置

1. Node.js ≥ 20
2. 在插件 **设置** 里点「一键安装 CLI」，或自行 `npm i -g @jackwener/opencli`
3. OpenCLI Chrome 扩展（Browser Bridge）
4. `opencli doctor` 通过（设置页也可运行）

可选：若 PATH 找不到命令，设置环境变量 `OPENCLI_BIN` 为可执行文件完整路径。

## 工具

| 工具      | 审批 | 说明                             |
| --------- | ---- | -------------------------------- |
| `doctor`  | 否   | 环境检查                         |
| `list`    | 否   | 适配器/命令列表（`--json`）      |
| `run`     | 是   | 站点适配器（`argv` / `command`） |
| `browser` | 是   | `opencli browser <session> …`    |

Agent 工具名：`plugin_opencli__*`。

## 路由

- 已登录网页 / 已知站点 → OpenCLI
- 桌面应用、画图、资源管理器、UIA → Computer Use
- 同一 Chrome：一轮对话里只用一套（不要混用 Ghost CDP 与 OpenCLI Bridge）

## 代码地图

| 部分                      | 路径                         |
| ------------------------- | ---------------------------- |
| 清单、host schema、skills | `src-tauri/plugins/opencli/` |
| 执行器                    | `core/plugins/opencli.rs`    |
| 权限                      | `opencli`                    |

相关：[插件系统](./plugin-system.zh-CN.md)、[电脑操控](./computer-use.zh-CN.md)。
