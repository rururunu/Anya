# 电脑操控（Computer use）

<p align="center">
  <a href="./computer-use.md">English</a>
  &nbsp;·&nbsp;
  <a href="./computer-use.zh-CN.md">简体中文</a>
</p>

官方 **Agent** 插件 `computer-use`（仅 Windows，**默认关闭**）。没有工作台界面——授予 `computer` 后在聊天里让 Anya 去做，或输入 `#plugin:computer-use`。会改桌面的工具走现有审批；选 **本会话允许** 才不会每点一次都弹确认。

它不是「只看截图猜坐标」的 VLM。截图是眼睛，UI Automation 是骨骼，`launch` / 快捷键是快通道。

## 1. 混合执行优先级

插件提示词 + 内置 playbook 要求模型按这层梯子走，越靠上越快、越准：

| 层级 | 方式                               | 典型工具                                                                           |
| ---- | ---------------------------------- | ---------------------------------------------------------------------------------- |
| 1    | ShellExecute：打开应用、文件或 URI | `launch`（`mspaint`、`notepad`、`C:\doc.docx`、`ms-settings:display`）             |
| 2    | 应用 / 系统快捷键                  | `key`（`ctrl+s`、`win+e`、`alt+f4`）                                               |
| 3    | UIA 语义                           | `click_control`（优先 InvokePattern，否则点可点中心）、`set_value`（ValuePattern） |
| 4    | 像素兜底                           | 画布、框选、没有名字的区域用 `click` / `drag` / `scroll`                           |

不要在开始菜单里找图标。不要每点一次就截一张图。截一次用来瞄准，其余调用放在**同一轮**发出（会改桌面的工具按顺序执行）。

## 2. 双通道：JPEG + UIA 树

`screenshot` 默认截**前台窗口**（`scope: desktop` 为整个虚拟屏）。工具结果包含：

1. JPEG（长边缩到 1280px）。`click` / `move` / `scroll` 的 `x,y` 是**这张图**里的像素，原点左上。
2. 最多 40 个可交互 UIA 控件：名称、AutomationId、**图像坐标**矩形。`click_control` 的 `index` / `name` / `id` 用这份列表。

列表被截断时再用 `find_control`（匹配 Name **或** AutomationId）。具名点击的坐标来自 UIA 边界框 / `GetClickablePoint`，不是模型估像素。

JPEG → 屏幕：`screen = image / scale + 窗口原点`，`scale = image_w / window_w`。截图、UIA 矩形、SendInput 都用**物理像素**。结果里带窗口 **DPI**，避免把 125%/150%/4K 当成 CSS 像素。

`click` / `click_control` 之后用 `ElementFromPoint` 和前台窗口标题做反馈（不再整窗重编码 JPEG）。若前台像 Anya，结果会写明——应 `focus_window` 目标应用，而不是继续盲点。

## 3. 内置 Windows playbook

`plugin.json` 的 `contributes.agent.skills: ["skills/windows.md"]` 在授予 `agent.prompt` 时追加到系统提示（不必再 `load_skill`）。覆盖画图、记事本、资源管理器、设置、计算器、截图、任务管理器与常见对话框（中英控件名）。

源文件：[`src-tauri/plugins/computer-use/skills/windows.md`](../src-tauri/plugins/computer-use/skills/windows.md)。官方插件启动时复制进 AppData。

## 4. 代码地图

| 部分                        | 路径                                                                                    |
| --------------------------- | --------------------------------------------------------------------------------------- |
| 清单、工具 schema、playbook | `src-tauri/plugins/computer-use/`                                                       |
| 分发与实现                  | `core/plugins/computer/`（`ops.rs`、`win/{capture,uia,snapshot,launch,observe,input}`） |
| 提示词拼接（`skills` 文件） | `core/plugins/prompt.rs`                                                                |
| Agent 工具名                | `plugin_computer-use__*`                                                                |

有 UI 且授予 `computer` 的插件也可走同一套 `computer.*` host RPC。官方插件是 `role: agent`——对话里的 Agent 直接调工具。

相关：[插件系统](./plugin-system.zh-CN.md)（`contributes.agent.skills`、权限 `computer`）。
