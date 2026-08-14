# Anya 文档索引

<p align="center">
  <a href="./README.md">English</a>
  &nbsp;·&nbsp;
  <a href="./README.zh-CN.md">简体中文</a>
</p>

产品主页：[../README.zh-CN.md](../README.zh-CN.md)

这里是文档地图——产品介绍请看根目录 README；按任务打开对应指南即可。

| 文档                                                         | 读者          | 适用场景                                                                                                               |
| ------------------------------------------------------------ | ------------- | ---------------------------------------------------------------------------------------------------------------------- |
| [技术架构总览](./architecture-overview.zh-CN.md)             | 贡献者        | 分层、进程拓扑、Ask/Agent/Plan、Companion 网关与文件 HTTP、RAG、时间线、持久化                                         |
| [发布与远程更新](./release.zh-CN.md)                         | 发版负责人    | 签名、`latest.json`、GitHub Releases、CI                                                                               |
| [Companion（安卓）](https://github.com/rururunu/AnyaAndroid) | 用户 / 手机   | 手机远程：配对、对话、审批、文件。[架构](https://github.com/rururunu/AnyaAndroid/blob/main/docs/ARCHITECTURE.zh-CN.md) |
| 截图资源（`image/`）                                         | 用户 / README | 根目录 README 引用的截图                                                                                               |

```mermaid
flowchart LR
  User[用户 / README] --> Arch[架构]
  User --> Rel[发布]
  User --> Comp[Companion]
  Dev[贡献者] --> Arch
  Dev --> Rel
  Dev --> Comp
```

行为变更时，请更新上表对应文档，并保持中英文姊妹篇（`*.zh-CN.md`）结构同步。
