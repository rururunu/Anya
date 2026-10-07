<p align="center">
  <img src="./public/mascot/mascot-icon.svg" alt="Anya" width="96" height="96" />
</p>

<h1 align="center">Anya</h1>

<p align="center"><strong>随时唤出的 Windows AI 助手</strong></p>
<p align="center">提问 · 编写代码 · 处理文档</p>

<p align="center">
  <a href="./README.md">English</a> &nbsp;·&nbsp; 简体中文
  &nbsp;·&nbsp; <a href="../../releases">下载安装</a>
  &nbsp;·&nbsp; <a href="./docs/README.zh-CN.md">使用文档</a>
</p>

<p align="center"><sub>Windows 10 / 11 &nbsp;·&nbsp; v0.2.27 &nbsp;·&nbsp; MIT</sub></p>

---

## 能做什么

| 功能           | 用途                                                                             |
| -------------- | -------------------------------------------------------------------------------- |
| **随时提问**   | 双击 Alt 唤出悬浮窗，附加文件或图片。                                            |
| **处理项目**   | 管理工作区与会话，读取代码、执行任务、审查变更。                                 |
| **个性化设置** | 设置本地资料和头像，查看用量活动，并导入或导出个人资料。                         |
| **处理文档**   | 生成、读取、转换与渲染 Word / Excel / PPT，通过卡片交付文件。                    |
| **选择模型**   | 配置 DeepSeek、OpenAI 兼容或 Anthropic 兼容服务商。                              |
| **扩展工具**   | 使用技能、MCP 与插件；电脑操控和 OpenCLI 需手动启用。                            |
| **手机连接**   | 通过 [Companion](https://github.com/rururunu/AnyaAndroid) 对话、审批和传输文件。 |

DeepSeek 模型使用原生集成的 dsh 工具与提示词，并记录缓存用量、响应耗时和重试信息。其他模型使用 Anya 通用工具集。详细说明见 [DeepSeek 链路](./docs/deepseek-harness.zh-CN.md)。

## 开始使用

Alt+Alt 会在弹窗取得焦点前采集选中文字，并以紧凑的行内标签显示引用。Chrome / Edge 可选安装[浏览器选区桥接](./browser-extension/README.zh-CN.md)，增强网页选区和来源 URL 的获取。

**个人资料**设置页会展示本机用量统计，并支持迁移个人资料、设置和 API Key。导出的文件包含敏感信息，请妥善保管。个人资料操作中也提供恢复出厂设置。

输入栏右侧可以切换模型和思考强度；窗口较窄时，部分操作会收起文字，仅显示图标。编辑已发送消息时也使用同一套操作。发现新版本后，可从工作台侧栏或 **设置 → 关于** 查看版本说明，再选择安装。

1. 从 [Releases](../../releases) 下载并安装 MSI。
2. 打开 **设置**，添加模型服务商并配置凭据。
3. 在工作台开始对话，或双击 **Alt** 唤出悬浮窗。

备用快捷键：<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Space</kbd>。

| 模式  | 用途                                 |
| ----- | ------------------------------------ |
| Ask   | 只读问答与调研                       |
| Agent | 使用工具完成任务，按审批策略执行操作 |
| Plan  | 先制定计划，批准后再执行写操作       |
| Image | 使用单独配置的生图服务生成图片       |

Office 排版、转换和公式结果需按产物检查。部分搜索与扩展功能需要额外配置。详见 [Office 工作流](./docs/office.zh-CN.md) 与 [插件说明](./docs/plugin-system.zh-CN.md)。

## 数据与隐私

设置、凭据与聊天记录默认保存在本机。发送消息时，消息和附带上下文会传给你配置的模型服务商。启用搜索、远程 MCP、云同步或手机连接后，相关数据也会传给对应服务或设备。

自定义背景图片以本地文件保存，运行时设置只记录路径；导出便携资料时会包含可读取的图片。

## 开发

需要 Node.js 18+、pnpm、Rust stable、Visual Studio C++ Build Tools 和 WebView2。

```bash
pnpm install
pnpm tauri:dev
```

<details>
<summary>检查与打包命令</summary>

```bash
pnpm check
cd src-tauri
cargo test --lib
cd ..
pnpm tauri:build
```

</details>

技术栈：**Tauri 2 + Vue 3 + TypeScript + Rust**。

## 文档

- [文档索引](./docs/README.zh-CN.md)
- [技术架构](./docs/architecture-overview.zh-CN.md)
- [DeepSeek 链路](./docs/deepseek-harness.zh-CN.md)
- [Office 工作流](./docs/office.zh-CN.md)
- [文件交付](./docs/file-delivery.zh-CN.md)
- [浏览器选区桥接](./browser-extension/README.zh-CN.md)
- [插件开发](./docs/plugin-system.zh-CN.md)
- [发布与更新](./docs/release.zh-CN.md)

IDE 上下文插件：[VS Code](https://marketplace.visualstudio.com/items?itemName=Anya.anya-ide-context) · [IntelliJ Platform](https://plugins.jetbrains.com/plugin/33163-anya-ide-context)。

## 许可与致谢

采用 [MIT 许可证](./LICENSE)。第三方许可见 [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)。

感谢 [Ghost](https://github.com/NORTHTEKDevs/ghost)、[OpenCLI](https://github.com/jackwener/OpenCLI) 以及本项目使用的开源工具与素材。
