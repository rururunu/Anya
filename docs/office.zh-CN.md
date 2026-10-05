# Office 文档工作流

Office 能力采用保存文件、按需加载技能和内置 JavaScript 文档运行时。不会通过 COM 连接 Microsoft Office，也不采集当前文档、选区或未保存内容。处理前应保存修改或提供文件副本。

## 可用能力

- `load_skill documents`：Word 生成、针对性 OOXML 编辑、样式与表格、页面检查。
- `load_skill spreadsheets`：Excel 分析、保留地址的数据和公式、批量编辑、计算检查。
- `load_skill presentations`：可编辑 PPT 生成、已有演示文稿的针对性编辑、幻灯片检查。
- `read_office_file`：结构化读取保存文件，支持 `path`、`sheet`、`slide`、从 1 开始的 `offset` 和 `limit`（最多 500 条记录）。`read_file` 也使用此读取器；Office 文件的 offset 表示记录位置，而不是文本行号。
- `prepare_office_runtime`：返回准确的可执行文件、模块导入 URL、CLI 和 API 路径。加载 Office 技能时自动准备。

这三个 Office 技能始终可用。`generate_word` 和 `docx` 保留为 `documents` 的兼容快捷入口。原有 `word_*`、`excel_*`、`ppt_*` 实时自动化工具已删除。

## 执行与打包

Anya 内置 Deno，以及包含 docx、ExcelJS、PptxGenJS、JSZip 和 fast-xml-parser 的离线模块，以及 LibreOffice Kit 0.1.5、对应平台引擎和专用 Node 运行时。采用技能、脚本、验证的工作方式，不分发 Codex 的 `@oai/artifact-tool`，也不宣称文档库能力完全一致。

`pnpm build:office-runtime` 生成 `src-tauri/resources/office/`。Tauri 开发和构建命令自动执行此步骤，并将模块和第三方许可声明作为资源打包。运行时优先使用安装资源，再放入 `{workspace}/.anya/office-runtime/v1/`，保留任务脚本。加载文档库、读取和生成文件不需要首次 npm 下载或全局 Node/Python。

Agent 编写可复用 JavaScript 任务脚本，使用返回的 Deno 路径和现有 Shell 生命周期执行，检查文件包、回读内容，再渲染预览进行检查。长命令使用后台作业并等待真实退出。脚本和输出可供任务恢复使用，但不意味着应用重启后自动续跑。

## 验证边界

- `validateOffice` 检查 ZIP CRC、XML 语法和必需文件包部件，不代表完整 OOXML Schema、关系正确性或版面验证。
- `renderOffice` 使用内置 LibreOffice Kit，不需要系统安装 LibreOffice 或 Poppler。使用全新输出目录和进程超时，返回 PDF 和有序 PNG 页面路径。Agent 必须查看图片；渲染成功不等于视觉检查通过。
- `recalculateWorkbook` 使用内置 LibreOffice Kit 计算一个新的 `.xlsx` 副本，需要回读受影响的结果并测试典型输入变化。Excel 专用公式和特性可能存在差异；ExcelJS 本身不计算公式，`fullCalcOnLoad` 也不是计算证据。
- 渲染或计算依赖缺失时，应明确报告未验证范围，不得宣称检查过版面或公式结果。
- 正文或幻灯片文本读取不会覆盖所有图片、页眉、批注、修订、备注和动画；相关任务应进一步检查 OOXML 部件或预览。
- 复杂模板和不支持的对象应通过针对性文件包编辑保留。不能用 ExcelJS 静默重写 VBA `.xlsm` 或不支持的工作簿特性；PptxGenJS 不提供已有 PPT 的完整保真导入。
- 旧 `.doc`、`.xls`、`.ppt` 应使用内置引擎的 `convertOffice` 转换副本；修改扩展名不是格式转换。

Windows 原生引擎使用系统字体，并需要 Microsoft VC++ v14 运行库。结果会报告缺失字体；引擎失败会明确报错，不会切换到系统渲染器。

## 检查

执行 `pnpm build:office-runtime` 后运行 `pnpm test:office-runtime`，检查真实 DOCX/XLSX/PPTX 文件往返和内置 Deno 离线运行。Office 评估样例（`--include-office --filter office`）检查技能加载，不要求安装 Microsoft Office。

## 交付生成文件

生成并完成必要验证后，使用 `present` 声明最终 DOCX/XLSX/PPTX、PDF 或图片文件；可一并交付可编辑源文件和阅读版。脚本与中间产物仅在用户需要时交付。转换与渲染不会自动创建卡片，`present` 成功也不代表文档版式或公式已经验证。见 [文件交付卡片](./file-delivery.zh-CN.md)。
