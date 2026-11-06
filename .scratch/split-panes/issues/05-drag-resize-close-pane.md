# 05: 分隔条拖拽 + 关格重排

**What to build:** 两件操作树结构的事。一，拖格间分隔条调整相邻两格占比（宽度仅存内存，
不进 settings.json）；二，右键 Close Pane 或格内程序退出时摘掉该叶、父 Split 退化为兄弟
子树（重排不留白）。关到只剩一格，该 session 退化为普通单终端。

**Blocked by:** 03（树布局渲染 — 需先有分隔条可拖、有叶子可关）

**Status:** resolved

- [x] 命中分隔条后拖拽，按指针在父 Split 盒子的相对位置更新 `ratio`（clamp 到 `[MIN_RATIO, 1-MIN_RATIO]`）
- [x] 拖拽宽度仅存内存（不进 settings.json），每 session 各存一份
- [x] sessions 行右键菜单追加 Close Pane，作用于焦点格
- [x] `remove(node, target)`：从树摘掉目标叶，其父 Split 退化为另一子树；非焦点格程序自然退出走同一路径（复用现有 `session_event` 的 Ended/Exited）
- [x] 焦点格被关 → `focus` 回落到树的第一格；关到只剩一格 → 退化为单终端
- [x] 关到该 session 无格（理论不达，最后一格走现有 session 关闭）不 panic
- [x] `scripts/check.sh` 绿

## Comments

- 2026-11-11：`96ba02a` 的功能经 `50fd77b` 合入 local/dev，随后与 04/06 的合并冲突在 `7346788` 解掉。
- 拖拽：`workspace/panes.rs` 的 `draw()` 给每个 Split 挂 `on_drag_move::<DividerDrag>`，分隔条由 Ely 的
  `resize_handle` 画出并携带「从根到该 Split 的 `Vec<Branch>`」，所以只有自己的分隔条动；
  `PaneNode::resize(path, axis, ratio)` 按 `MIN_RATIO = 0.1` clamp。
- 关格：`PaneNode::without(closed, focus)` 返回新树与新焦点；`Session::drop_pane` 同时摘掉视图与订阅，
  焦点叶被关时取树的最左叶；最后一格返回 `None`，`close_pane` 转而调 `close()` 关整行。
- 程序退出：`workspace/events.rs` 的 `Exited` 分支——仍有多个叶时调 `close_pane(index, id, …)`，
  只剩一格时走原有 `close()`，与右键 Close Pane 同一路径。
- 单格退化：`shown_pane()` 对 `PaneNode::Leaf` 直接给视图，不外包分隔条、不画焦点框。
- `cargo test --lib` 436 通过，2 个 `term::shell::windows_tests` 失败为本机既有基线。
