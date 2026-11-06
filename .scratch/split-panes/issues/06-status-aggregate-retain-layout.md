# 06: 行状态聚合 + hover + 切行保留布局

**What to build:** 左侧那行的状态 orb 从「只看焦点格」改为「聚合所有格的最坏状态」，
hover 时 orb 下拉列出每个格一条。同时确立「每个 session 各存一份 pane 树 + 拖拽宽度，
切到别行再切回原样恢复」——这两件事都建立在 01 起 Session 自持树的基础上，本票把它们
收口成完整的多 session 多 pane 体验。

**Blocked by:** 03（树布局渲染 — 多 session 各自的树需能渲染）

**Status:** resolved

- [x] `Session::looks()` 由取单格 `records().shown()` 改为对树上所有叶子做 worst-of 归并（error > blocked > working > waiting > idle > done）
- [x] hover 时 orb 下拉列出每个格一条（复用现有 hover 列表机制，沿用 LISTED=10 / SHORT=120）
- [x] 非 active session 的树、ratio、focus 全部保留在各自 Session 上（内存），切走再切回原样恢复
- [x] 分屏布局不写 settings.json；窗口重开不恢复分屏（仅当前运行期保留）
- [x] 非焦点格 blocked 时行 orb 仍反映（不因焦点格 idle 而掩盖），与「隐藏 session blocked 变强标题」既有精神一致
- [x] `scripts/check.sh` 绿

## Comments

- 2026-11-11：`ab78568`(wip) → `98c443d`（功能）经 `87b736a` 合入 local/dev，随后在 `7346788` 解掉与 04/05 的冲突。
- 聚合：`sessions.rs` 的 `looks()` 用 `orb::worst(leaf_views().map(...))`，每叶取 `records().shown()` 的最坏记录
  并带上该叶自己 `ended()` 的失败码；`orb::worst` 按 error > blocked > working > waiting > idle > done 归并。
- hover：`hover(panes, words)` 逐叶列记录，`LISTED.saturating_sub(lines.len())` 保证总数不超过 10，
  超出记一条 `et.status.more`；失败码取首个非零。`mark()` 用 `leaf_views()` 的降级句柄在 tooltip 里实时读。
- 保留：`Session.panes` / `focus` / 各叶 `views` 与 `pane_events` 都在内存，`select()` 不再重建；
  设置里没有任何 pane 字段，重开窗口各行回到单终端。
- `cargo test --lib` 436 通过，2 个 `term::shell::windows_tests` 失败为本机既有基线。
