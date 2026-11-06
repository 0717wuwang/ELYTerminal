# 01: 预重构 — 单 pane 树基线（expand 式铺地基）

**What to build:** 为分屏铺平 1:N 的数据地基，但**行为零变化**。引入 `PaneNode` 树与
`Session.focus`，让「现有每个 session 一格终端」成为一棵退化的单叶树。做完这票，用户
看到的应用与之前逐像素一致，但 Session 内部已能容纳一棵 pane 树。

**Blocked by:** None（can start immediately）

**Status:** done
- [ ] 引入 `PaneNode`（`Leaf(EntityId)` | `Split{ axis, first, second, ratio }`），`PaneId` 复用 `TerminalView` 的 `entity_id`
- [ ] `Session` 增 `panes: PaneNode`（初始为 `Leaf(terminal.entity_id())`）与 `focus: EntityId`（初始为该 terminal）
- [ ] `Workspace::add` 建树时按单叶构造，events 订阅、folder/shell/args/look 派生路径不变
- [ ] 渲染仍只画 active session 的那一格（退化树的唯一叶子），与改动前逐像素一致
- [ ] 现有全部测试通过；`scripts/check.sh`（fmt + clippy `-D warnings` + tests + 500 行规则）绿
- [ ] 无新增公共 API 文档需求（纯内部重构）

## Comments
- 完成（行为零变化）。`PaneNode`/`PaneId`/`Axis` 在 `src/sessions/pane.rs`；`Session` 增 `panes: PaneNode` 与 `focus: PaneId`，`Workspace::add` 按单叶构造（`PaneNode::leaf(id)`）。渲染仍走 `session.terminal`，逐像素一致。
- 六条验收全过：fmt 绿、`clippy -D warnings` 绿、`cargo test` 413 通过。仅剩 2 个 `term::shell::windows_tests` 失败（cmd/powershell 报目录），已用 stash 在干净树上复现，属环境相关、与本次重构无关。
- 双轴 code review 后，删除了规格未要求的 `PaneNode::focus()` 及其测试（生产无调用方，属投机代码，也是 `Session.focus` 与派生 focus 双真相来源的隐患）；`Session.focus` 字段为唯一权威，正如规格所定。`Split`/`Axis` 按规格保留（后续分屏票在其上构建）。
- 2026-11-11 验收：`688c120` 引入 `PaneNode`/`PaneId`/`Axis` 与 `Session.panes`/`focus`，单叶退化树逐像素一致；状态补记为 done。Standards 双轴复审未见本票遗留问题。
