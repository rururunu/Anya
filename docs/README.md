# Anya documentation

<p align="center">
  <a href="./README.md">English</a>
  &nbsp;·&nbsp;
  <a href="./README.zh-CN.md">简体中文</a>
</p>

Product home: [../README.md](../README.md)

These pages are the maintainer map — start at the root README for the product story, then open the guide that matches your task.

| Document                                                       | Audience                      | When to open it                                                                                                                    |
| -------------------------------------------------------------- | ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| [Architecture overview](./architecture-overview.md)            | Contributors                  | Layers, process topology, Ask/Agent/Plan/Image, Companion gateway + file HTTP, RAG, timeline, persistence, module map              |
| [Plugin system](./plugin-system.md)                            | Plugin authors / contributors | Tech choices, motivation, architecture, runtime flow, contract primitives, dev tutorial, API reference                             |
| [Computer use](./computer-use.md)                              | Contributors / agent authors  | Official `computer-use` plugin: launch → keyboard → UIA → pixels, screenshot+tree, Windows playbook                                |
| [Releases & remote updates](./release.md)                      | Release managers              | Signing, `latest.json`, GitHub Releases, CI                                                                                        |
| [Companion (Android)](https://github.com/rururunu/AnyaAndroid) | Users / mobile                | Phone remote: pair, chat, approvals, files. [Architecture](https://github.com/rururunu/AnyaAndroid/blob/main/docs/ARCHITECTURE.md) |
| Screenshots (`image/`)                                         | Users / README                | Assets linked from the root README                                                                                                 |

```mermaid
flowchart LR
  User[User / README] --> Arch[Architecture]
  User --> Plug[Plugin system]
  User --> CU[Computer use]
  User --> Rel[Release]
  User --> Comp[Companion]
  Dev[Contributor] --> Arch
  Dev --> Plug
  Dev --> CU
  Dev --> Rel
  Dev --> Comp
```

When behavior changes, update the matching document and keep its Chinese twin (`*.zh-CN.md`) in sync.
