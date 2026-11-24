# 电脑操控

[English](./computer-use.md)

电脑操控内置于 Anya。直接在聊天里描述要操作的 Windows 应用或浏览器。

- 先用 `computer_window` 选择窗口，再用 `computer_see` 或 `computer_find` 定位控件。
- 用 `computer_act` 点击或输入，并检查返回的 `verified`。用 `computer_wait` 和 `computer_assert` 验证结果。
- 画布可以使用 `computer_click` 或 `computer_drag`。浏览器可使用 `computer_browser` 和 `computer_tab`。
- 可修改桌面状态的操作遵循当前会话的工具审批设置。

Anya 在一轮会话首次使用电脑操控时启动独立的 [Ghost MCP](https://github.com/NORTHTEKDevs/ghost) 进程，并通过私有 stdio 连接调用它。首装和更新会校验官方 Windows x64 压缩包的 SHA-256、MCP 握手及工具可用性。每 24 小时最多检查一次，检查在启动操控进程前进行。此前验证过的版本保存在本机，候选版本无法使用时自动尝试旧版。首次使用需要能够下载官方发布包。

同一时间只有一轮对话可以操控桌面。点击浮层上的 **结束** 会取消该轮会话并停止 Ghost。轮次结束后，Anya 回收进程并清除窗口锚点。超时或连接中断时，可观察目标窗口确认最后一次操作的结果。

## 终端

工作台右侧边栏的终端标签页打开本地 PowerShell。交互终端继承 Anya 当前 Windows 用户的权限和环境变量，可访问该用户有权使用的文件、磁盘及可执行程序。终端优先从当前会话的工作区启动。切换标签页或收起侧栏时，终端会话继续运行。Agent 在聊天中使用独立的内置 shell 工具，受当前聊天的工具审批模式约束。

Rust 实现位于 `src-tauri/src/core/computer/`，终端命令位于 `src-tauri/src/commands/terminal.rs`。
