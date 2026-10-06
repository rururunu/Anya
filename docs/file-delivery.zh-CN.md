# 文件交付卡片

[English](./file-delivery.md)

适用于 Anya v0.2.26。文件卡片用于交付最终产物，代码修改摘要另行展示。

## 使用与显示

模型调用 `present`，传入 `files` 数组，每项包含 `path` 和可选的 `description`。通用工具集和 DeepSeek 工具集都支持；Image 专用模式仍仅提供图像生成工具。一次最多 8 个文件，建议只选最终交付物。相对路径以工具工作区解析，文件必须存在、为普通文件且通过路径权限检查；目录与符号链接被拒绝。

成功结果包含 `version: 1` 和文件列表：原始路径、绝对路径、名称、描述及交付时大小。元数据总量限制为 1 MiB，不复制文件内容、不上传 Companion，也不扫描回复中的路径或自动交付全部修改文件。

回复完成后显示主任务成功交付的卡片，历史重新加载后仍可恢复。同一绝对路径采用最新信息，Windows 路径按大小写与分隔符归一化去重；超过四张可展开。子任务交付不会自动变成主任务卡片，主模型需要再次声明最终文件。

## 打开与预览

图片使用现有图片预览侧栏，也可通过按钮在默认应用中打开。其他文件直接使用系统默认应用；文件夹按钮定位文件。操作过程中禁用重复点击，失败时显示错误并允许重试。

当前没有 Office/PDF 内置预览侧栏或应用选择菜单。Office 技能可以生成预览产物，但不会自动变成卡片预览。历史卡片引用原文件，移动、删除或修改后以当前文件状态为准；名称与大小是交付时快照。

## 图标与维护

卡片沿用 Anya 主题变量、圆角和响应式布局。`DeliveryFileIcon.vue` 按扩展名选择内置 Material Icon Theme SVG，未知文件使用通用文档图标；扩展名只决定外观，不验证 MIME 或内容。素材来源固定为 `735a0166cb72f3514a717f7c17905a77374d5df4`，SVG 保持原样。

许可和来源位于 `src/assets/material-file-icons/`；`public/licenses/material-icon-theme-MIT.txt` 随前端构建分发。更新时同时更新 SVG、许可与来源版本，不需要安装 VS Code 插件或运行时联网。

## 个人资料与重置

个人资料设置页支持导入或导出设置、本地资料、头像及 Token 用量偏好。导出文件可能包含服务商凭据和 API Key，请安全保存和传输；可读取的本地背景图片会嵌入导出文件。恢复出厂设置提供两种范围：仅删除个人信息并保留设置，或删除全部本地应用数据和设置。操作会关闭并重启 Anya；工作区中的项目文件会保留。删除不可撤销，建议先导出需要保留的信息。

## 源码与验证

- Rust：`src-tauri/src/core/tools/present.rs`、`core/chat/agent_loop/tools.rs`、`core/chat/db/tool_activity.rs`。
- 前端：`src/services/chat/presentedFiles.ts`、`src/components/chat/PresentedFiles.vue`、`DeliveryFileIcon.vue`，入口为 `MessageList.vue`。
- 测试：元数据提取、去重和无效结果过滤；卡片测试覆盖打开、定位、图片预览、展开与错误重试。IPC 使用 mock，不等于真实系统应用打开已验证。

相关：[DeepSeek 链路](./deepseek-harness.zh-CN.md) · [Office 工作流](./office.zh-CN.md) · [技术架构](./architecture-overview.zh-CN.md)
