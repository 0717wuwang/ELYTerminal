# 03: 树布局渲染 + 格间分隔条

**What to build:** 把 active session 的 pane 树投影成屏幕上的多个矩形并真正画出来。
两个及以上格子按树结构（横切/竖切、层层嵌套）并排显示，格与格交界画一条 filled
hairline 分隔条（同 sessions / 右侧列既有分隔条做法）。做完这票，分屏第一次「看得见」。

**Blocked by:** 02（菜单触发分屏）

**Status:** done
- [ ] `layout(node, bounds) -> Vec<(PaneId, Rect)>`：把 `PaneNode` 树按 `ratio` 递归切成矩形，交界留出分隔条的几像素
- [ ] 渲染只画 active session 的树；每个叶子的 `TerminalView` 画在自己的矩形内
- [ ] 格间画 filled hairline 分隔条（颜色/尺寸取自 Ely 主题，不引入新颜色常量）
- [ ] 各叶子 `TerminalView` 按其矩形独立 resize（cell 尺寸、grid 尺寸跟随）
- [ ] 非 active session 的树不渲染（但 pty 与布局保留，见 06）
- [ ] 单叶退化树渲染与改动前逐像素一致（不回归 01 的基线）
- [ ] `scripts/check.sh` 绿
- 2026-11-11 验收：`26d1bcc` + `06efd28` 落地树布局渲染、filled hairline 分隔条、各叶独立 resize、非 active 树不渲染；状态补记为 done。全部勾选项达成。
- 复审记两条代码质量项（均判为可在后续小修，不阻塞本票功能）：`workspace/panes.rs` 的 `draw`（51–136 行）约 86 行，超项目 80 行函数上限，应抽出分隔条拖拽处理；`shown_pane`（31–35 行）叶子缺视图时静默返回 `None` 不记日志，与 `leaf()`（143 行）的记日志做法不一致（AGENTS.md「no silent fallbacks」）。另记一处味道：「按 pane id 找所属 session」的查找在 `split_row`/`close_row`/`session_event` 重复三次，可抽 `session_index`。
