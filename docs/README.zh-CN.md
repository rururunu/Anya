# Anya 文档索引

<p align="center">
  <a href="./README.md">English</a>
  &nbsp;·&nbsp;
  <a href="./README.zh-CN.md">简体中文</a>
</p>

产品主页：[../README.zh-CN.md](../README.zh-CN.md)

这里是文档地图——产品介绍请看根目录 README；按任务打开对应指南即可。

| 文档                                                         | 读者                | 适用场景                                                                                                               |
| ------------------------------------------------------------ | ------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| [技术架构总览](./architecture-overview.zh-CN.md)             | 贡献者              | 分层、进程拓扑、Ask/Agent/Plan/Image、前缀缓存规则、Companion 网关与文件 HTTP、工作区检索、时间线、持久化、模块地图    |
| [插件系统](./plugin-system.zh-CN.md)                         | 插件作者 / 贡献者   | 技术选型、初衷、架构关系、运行流程、契约原语、开发教学、API 参考                                                       |
| [电脑操控](./computer-use.zh-CN.md)                          | 贡献者 / Agent 作者 | 官方 `computer-use`：launch → 快捷键 → UIA → 像素、截图+控件树、Windows playbook                                       |
| [Office 工作流](./office.zh-CN.md)                           | 用户 / 贡献者       | 保存文件技能、内置 JavaScript 运行时、打包与验证边界                                                                   |
| [DeepSeek 原生工具链路](./deepseek-harness.zh-CN.md)         | 用户 / 贡献者       | dsh 契约、模型隔离、Rust 执行器、请求诊断与移植边界                                                                    |
| [文件交付卡片](./file-delivery.zh-CN.md)                     | 用户 / 贡献者       | 显式交付、历史恢复、打开与预览、彩色图标及许可                                                                         |
| [OpenCLI](./opencli.zh-CN.md)                                | 贡献者 / Agent 作者 | 官方 `opencli`：站点适配器 + 已登录 Chrome Browser Bridge（与 Computer Use 互补）                                      |
| [发布与远程更新](./release.zh-CN.md)                         | 发版负责人          | 签名、`latest.json`、GitHub Releases、CI                                                                               |
| [Companion（安卓）](https://github.com/rururunu/AnyaAndroid) | 用户 / 手机         | 手机远程：配对、对话、审批、文件。[架构](https://github.com/rururunu/AnyaAndroid/blob/main/docs/ARCHITECTURE.zh-CN.md) |

```mermaid
flowchart LR
  User[用户 / README] --> Arch[架构]
  User --> Plug[插件系统]
  User --> CU[电脑操控]
  User --> Rel[发布]
  User --> Comp[Companion]
  Dev[贡献者] --> Arch
  Dev --> Plug
  Dev --> CU
  Dev --> Rel
  Dev --> Comp
```

行为变更时，请更新上表对应文档，并保持中英文姊妹篇（`*.zh-CN.md`）结构同步。
