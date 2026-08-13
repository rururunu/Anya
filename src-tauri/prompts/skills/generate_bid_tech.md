---
name: generate_bid_tech
description: 表驱动生成「综合评分技术部分」技术标 .docx。用于标书、技术标、投标方案、研学方案等需按评分表交付的 Word 文档；强制真表格、半小时日程、数量+质量门禁，完成后须子代理评议。
---

# 技术标专用 Skill（表驱动）

你输出的是**可评分的技术标正文**（真 `.docx`），不是散文汇报，也不是 Markdown 假装成 Word。

工作区已物化（或即将物化）Python 工具包：`.anya/bid_tech/`。
生成脚本必须 `sys.path` 引入该包，并使用其中的 `style` / `tables` / `planner` / `gate` / `quality` / `reference` / `docx_inspect` / `align`。

## 何时使用本 skill

用户要求交付**按招标综合评分表打分的技术标 / 投标技术方案 / 研学方案**——评分点决定了必须有真实表格、量化流程、可核查的门禁，而不是一篇通顺但空洞的说明文。

## 何时不要用本 skill

| 需求 | 改用 |
|---|---|
| 普通 Word 短文档（无评分表约束） | `generate_word` |
| 编辑已有 .docx（修订/批注/OOXML） | `#skill:docx` |
| Markdown ↔ Word/PDF 批量转换 | `#skill:pandoc` |

<example>
用户："帮我写一份公司简介，导出成 Word。"
判断：没有招标评分表、没有硬性门禁要求，属于普通短文档。
正确做法：用 `generate_word`，不要套用本 skill 的门禁流程——那会为一个不需要门禁的任务强加不必要的复杂度。
</example>

<example>
用户："这是招标文件和评分细则，写一份能过打分的技术标。"
判断：评分表驱动、需要真表格与量化流程，正是本 skill 的适用场景。
正确做法：先跑骨架规划（`planner`），再逐章填表，完成后走门禁与 `review_bid_tech` 评议，不得跳过门禁直接宣称完成。
</example>

## 硬性规则（违反即未完成）

1. **先骨架后正文**：用 `planner.build_default_plan`（或从招标文件抽出的章节覆盖）写出目录与每章最低交付物；用 `update_tasks` 跟踪。
2. **有参考稿时先提取画像（不抄正文）**：若用户提供优质 `.docx` 参考，先运行 `reference.extract_reference_profile` / CLI `reference`，再 `plan-from-ref` 校准门禁与表范式；**换项目时以招标文件为准覆盖参考**，参考只提供结构与阈值，不提供固定城市/场馆文案。
3. **路线/场馆以招标文件 + 参考画像为准**：参考稿只告诉「表怎么长、日程粒度」；具体城市/场馆/保费以**本次招标**为准，缺失再联网填格。
4. **禁止用段落冒充表格**：凡行列结构必须 `Document.add_table`（经 `style.add_table` / `tables.*`）。
5. **优先整本 python-docx 生成**；不要用 Office COM `Range.Text` 灌长文。COM 仅用于读取已打开文档的上下文，或插入小表/统一字体。
6. **研究 → 填格**：联网结果写入 `tables.add_venue_research_table` / `add_insurance_research_table` / `add_transport_research_table` 等字段表；禁止把网页摘要堆成「亮点」大段。
7. **完成前门禁（数量 + 质量）**：对产出 `.docx` 运行 `gate.evaluate_gate` / `assert_gate_passed`（默认含 `quality` 反灌水/日程抄袭/空泛地点）。不通过则**补真实差异化日程与流程字段**，禁止只润色措辞后宣称完成。
8. **门禁通过后必须评议**：调用只读子代理 `review_bid_tech`（或 `run_skill` name=`review_bid_tech`），阅读全文并讨论文档合理性；根据严重问题回改后再交卷。
9. 未通过门禁、未完成评议、或未实际写出文件时，**不得**使用「已完成 / 已生成 / 搞定」等完成表述。

## 标准流程

### A. 读懂输入

1. 读取招标文件与参考文件：`read_file` 会从 `.docx` 抽出纯文本；需要结构摘要时再用 `run_shell` 跑 `bid_tech.docx_inspect.inspect_docx`。
2. 抽出综合评分表技术部分章节与分值；若无法抽出，使用默认十章骨架并在正文注明「章节结构按招标评分表，分值以招标文件为准」。
3. 生成对齐清单：`align.save_checklist(align.default_checklist_for_study_tour(), ...)`，再按招标附件增删项。

### B. 落盘计划（含参考画像）

若用户提供参考 `.docx`：

```bash
python .anya/bid_tech/cli.py reference path/to/参考.docx --out .anya/ref_profile.json --print
python .anya/bid_tech/cli.py plan-from-ref --profile .anya/ref_profile.json --project 本次项目名 --out .anya/bid_plan.json
```

画像会识别日程风格（`day_block` / `mixed` / `half_hour_dense`）并校准门禁，**不会**把参考稿里的深圳/喀什等节点硬套到别的项目；换方案时在 `bid_plan.json` 里改 `route_must_include` / `required_keywords` 即可。

无参考稿时用默认 plan：

```python
import sys
from pathlib import Path
sys.path.insert(0, str(Path(".anya").resolve()))
from bid_tech.planner import build_default_plan, write_outline

plan = build_default_plan(project_name="…项目技术标")
write_outline(plan, ".anya/bid_plan_outline.md")
plan.save(".anya/bid_plan.json")
```

参考画像推荐的表模板（按识别结果选用，勿全抄）：
- `tables.add_day_schedule_table` — 天数 / 时段 / 日程 / 线路 / 酒店
- `tables.add_timeline_table` — 时间 / 具体环节 / 工作内容 / 备注
- `tables.add_schedule_table` — 半小时四列表（仅当 `schedule_gate_mode=half_hour_dense`）

### C. 按章填表（最低交付物）

| 章节 | 最低交付物（真表） |
|---|---|
| 投保方案 | 保险核实表 + 投保/理赔流程五列表 |
| 活动方案 | 行程总览表 + **半小时日程四列表（含负责人）** + 备用方案表 |
| 组织与管理 | 架构表 + 人员配比表 + 阶段流程五列表（事项/岗位/形成材料） |
| 出行方案 | 交通核实表 + 集散/延误流程表 |
| 食宿交通 | 标准与流程表 |
| 安全保障 | **场景×响应动作×第一责任人表** + 流程表 |
| 物资/宣传/场馆 | 物资或场馆核实表 + 流程表 |
| 成果转化 | 成果形态/模板表 |
| 档案管理 | **分类×内容×形成时点表** + 验收配合表 |
| 服务承诺 | 承诺条款表或流程化条款表 |

每章开头用 `style.add_score_lead` 写一句评分对照。

### D. 版式

- 使用 `style.configure_document`（页边距、页眉）
- 标题黑体、正文仿宋、表内宋体（见 `style` 常量）
- 表默认全线框（`apply_grid_borders`）；若招标明确要求三线表，生成脚本里 `style.add_table(..., three_line=True, grid=False)`

### E. 自检（硬门槛，默认值）

运行：

```python
from bid_tech.gate import evaluate_gate, save_report
from bid_tech.align import check_alignment, load_checklist
from bid_tech.planner import load_plan

plan = load_plan(".anya/bid_plan.json")
align_report = check_alignment("output.docx", load_checklist(".anya/bid_align.json"), source="checklist")
report = evaluate_gate("output.docx", plan, align_open_items=align_report.open_items)
save_report(report, ".anya/bid_gate_report.json")
print(report.format_text())
print(align_report.format_text())
if not report.passed:
    raise SystemExit(2)
```

CLI 等价：`python .anya/bid_tech/cli.py gate output.docx --plan .anya/bid_plan.json`

默认阈值（可在 plan JSON 调整，但不得无理由大幅放水）：

- 真表格 ≥ **15**
- `HH:MM-HH:MM`（及同义破折号）日程行 ≥ **80**
- 活动方案 ≥ **4000** 字；组织与管理 ≥ **2500**；档案管理 ≥ **600**；其余见 `planner.default_tech_score_chapters`
- 关键词均出现：`深圳`、`北京`、`责任人`、`形成材料`、`半小时`（若招标路线不含某城市，须先改 `plan.route_must_include` / `required_keywords` 并说明依据）

质量门禁（默认开启，勿随意 `--skip-quality`）：

- 禁止灌水短语（如「扩展内容」多次出现）与连环拼接凑字
- 禁止多日半小时日程高度雷同（模板改标题）
- 禁止「主题探究活动A / 参观点」等空泛占位占比过高
- 投保章节建议出现「100元」或「元/人/天」等日保费响应（缺省为 warn）

### F. 评议子代理（门禁通过后强制）

数量+质量门禁通过后，**必须**再开只读子代理评议合理性，再与其结论对齐修改：

```text
review_bid_tech
# 或
run_skill  name=review_bid_tech  read_only=true
```

任务里写明：

- 产出 `.docx` 绝对/工作区相对路径
- `.anya/bid_gate_report.json`（若有）
- 招标硬口径摘要（人数、保费、配比、行程天数等）

收到评议结果后：

1. 与子代理结论**逐条讨论**（可用同一会话续写，或再开一轮澄清）：严重问题必须改稿；次要问题说明取舍。
2. 改稿后**重新跑 gate**；若改动较大，可再跑一次 `review_bid_tech`。
3. 仅当门禁通过且评议结论为「可投标」或严重问题已清零时，才可向用户宣称完成。

### G. 输出给用户

- `.docx` 路径
- 门禁报告摘要（表数量、半小时行数、质量项、是否通过）
- 评议结论摘要（总体判定 + 已处理的严重问题）
- 对齐未闭合项（如有）
- 关键假设（文件名、分值来源）

## 最小生成脚本骨架

```python
# scripts/build_tech_bid.py
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str((ROOT / ".anya").resolve()))

from bid_tech import style, tables, planner, gate, align

OUT = ROOT / "docs" / "技术标-综合评分技术部分.docx"
OUT.parent.mkdir(parents=True, exist_ok=True)

plan = planner.build_default_plan("示例项目技术标")
plan.save(ROOT / ".anya" / "bid_plan.json")

doc = style.configure_document(header_text=plan.project_name)
style.add_heading_cn(doc, plan.project_name, level=1)

# —— 按 plan.chapters 逐章：add_score_lead + 必表 + 少量说明句 ——
# 示例：活动方案日程（真实项目须写满 ≥80 条半小时行，且按日差异化）
style.add_heading_cn(doc, "二、活动方案", level=2)
style.add_score_lead(doc, "活动方案", ["半小时日程", "互动实践"])
tables.add_overview_table(doc, [("第1天", "深圳", "开班与结对", "—")])
tables.add_schedule_table(
    doc,
    [
        ("06:00-06:30", "集合清点", "出发地", "教官"),
        ("06:30-07:00", "登车", "停车场", "交通岗"),
        # … 补齐往返日 + 7 天半小时行（禁止多日复制同一模板）…
    ],
)
tables.add_backup_table(doc, [("暴雨不宜户外", "户外参观", "室内场馆 B 计划")])

doc.save(str(OUT))

checklist = align.default_checklist_for_study_tour()
align.save_checklist(checklist, ROOT / ".anya" / "bid_align.json")
align_report = align.check_alignment(OUT, checklist)
report = gate.evaluate_gate(OUT, plan, align_open_items=align_report.open_items)
gate.save_report(report, ROOT / ".anya" / "bid_gate_report.json")
print(report.format_text())
if not report.passed:
    raise SystemExit("门禁未通过：补表后再交卷")
print("saved:", OUT)
print("下一步：调用 review_bid_tech 只读评议后再宣称完成")
```

## 与 generate_word 的关系

- 一般 Word 短文档 → `generate_word`
- **按评分表交付的技术标 / 投标技术方案** → 本 skill（`generate_bid_tech`）
- 本 skill 内部仍用 python-docx；差别是强制表模板、planner、gate（含 quality）与完成后的 `review_bid_tech`

## 语言

按用户最新消息语言回复；路径、表头字段名、代码标识符保持原样。
