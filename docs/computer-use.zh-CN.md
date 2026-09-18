# 电脑操控（Computer use）

<p align="center">
  <a href="./computer-use.md">English</a>
  &nbsp;·&nbsp;
  <a href="./computer-use.zh-CN.md">简体中文</a>
</p>

官方 **Agent** 插件 `computer-use`（仅 Windows，**默认关闭**）。没有工作台界面——授予 `computer` 后在聊天里让 Anya 去做，或输入 `#plugin:computer-use`。会改桌面/浏览器的工具走现有审批；选 **本会话允许** 才不会每步都弹确认。

执行引擎：进程内嵌入 **[Ghost](https://github.com/NORTHTEKDevs/ghost)**（`ghost-session` v0.23.4，MIT）。Anya 保留权限、工具审批与 Computer Use HUD；Ghost 负责 UIA、后台焦点策略、动作校验（`verified`）与浏览器 CDP。

## 1. Ghost 风格流水线

| 步骤 | 工具                            | 说明                                                |
| ---- | ------------------------------- | --------------------------------------------------- |
| 1    | `window`                        | `launch` / `focus` / `anchor`——会话锚定目标窗       |
| 2    | `see`                           | 优先 `mode=text`；否则 name/role/enabled/中心点列表 |
| 3    | `act`                           | 按 name/role 点击或输入；读返回里的 **`verified`**  |
| 4    | `wait` / `assert`               | 等元素/值/空闲——少用纯 sleep                        |
| 5    | `browser` + `tab`               | 网页走 CDP，不要像素点网页                          |
| 兜底 | `screenshot` / `click` / `drag` | 仅画布、框选、无名区域                              |

默认后台优先（Ghost focus policy / `GHOST_FOCUS_LOCK`）：尽量不抢用户鼠标。

## 2. 工具面

**桌面：** `see`、`act`、`wait`、`assert`、`window`、`key`/`hotkey`、`scroll`、`drag`、`clipboard`、`screenshot`、`screen_info`。

**浏览器：** `browser`（`launch` \| `attach` \| `tabs` \| `close`）、`tab`（`open` \| `navigate` \| `click` \| `type` \| `text` \| `eval` \| `wait` …）。

**旧名别名：** `list_windows`、`focus_window`、`find_control`、`click_control`、`set_value`、`launch`、`click`、`type`。

只读类（`see` / `assert` / `wait` / `screenshot` / `list_windows` / `find_control` …）免批；改桌面/浏览器的 `act` / `window` / `key` / `browser` / `tab` 等需批（会话允许仍一次放行整类）。

## 3. HUD 与停止

本轮对话首次调用 `plugin_computer-use__*` 时打开辉光 + 顶栏；对话结束或 HUD **结束** 会触发 Ghost 紧急停止、清空窗口锚点并关闭 HUD。

## 4. 内置 playbook 与学习库

`plugin.json` 的 `contributes.agent.skills: ["skills/windows.md"]` 在授予 `agent.prompt` 时追加到系统提示。覆盖记事本、资源管理器、画图、设置、计算器、对话框与 CDP 浏览器流程（中英控件名）。

此外提供 **`playbook` 工具**（Agent S2 检索/写入 + EchoPath 可执行步骤）：成功后把语义步骤存到 `%APPDATA%/Anya/computer-use/playbooks/*.json`；下次 `lookup` 复用；某步 `verified` 失败则 `fail` 降权/隔离，再探索并用同 `id` `save` 打补丁。启用插件时会把本地 playbook 目录索引注入 prompt。

源文件：[`src-tauri/plugins/computer-use/skills/windows.md`](../src-tauri/plugins/computer-use/skills/windows.md)；实现：`core/plugins/computer/playbook/`。

## 5. 代码地图

| 部分                        | 路径                                                             |
| --------------------------- | ---------------------------------------------------------------- |
| 清单、工具 schema、playbook | `src-tauri/plugins/computer-use/`                                |
| Ghost 桥与分发              | `core/plugins/computer/{ghost_bridge,ghost_ops,playbook,mod}.rs` |
| 依赖                        | `Cargo.toml` → `ghost-session`（仅 Windows，`git` tag 锁定）     |
| Agent 工具名                | `plugin_computer-use__*`                                         |

跟随 upstream 需显式 bump tag 并记变更。进程内嵌入会增加二进制体积与首次 COM 初始化成本，属预期。

同一会话不要再开 Ghost MCP，避免双轨抢桌面。已登录网页优先考虑官方插件 [OpenCLI](./opencli.zh-CN.md)，且不要与 Ghost CDP 同时驱动同一 Chrome。

相关：[插件系统](./plugin-system.zh-CN.md)（`contributes.agent.skills`、权限 `computer`）。
