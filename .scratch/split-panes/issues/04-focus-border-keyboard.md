# 04: 焦点 — accent 边框 + 键盘路由

**What to build:** 键盘当前进的是哪个格，一眼可辨。焦点格四周画主题 accent 色细边框
（类 Ghostty active pane border），键盘事件只派发给焦点格；点某个格把焦点切到它。
焦点格同时是左侧高亮行；rename / 右键菜单作用于焦点格。

**Blocked by:** 03（树布局渲染 — 需先有多个可见格与它们的矩形）

**Status:** resolved

- [x] `Session.focus: EntityId` 指向当前焦点叶；键盘事件只派发给该叶的 `TerminalView`
- [x] 焦点格四周画主题 accent 色 hairline 边框；非焦点格不画（复用主题 accent，不新增颜色常量）
- [x] 鼠标点某格 → 把 `Session.focus` 切到该格；左侧高亮行、rename、右键菜单随之指向焦点格
- [x] 切到别的 session 再切回，焦点落回该 session 自己记的 `focus`（配合 06 的每 session 各存树）
- [x] 无焦点格被隐藏 / 关闭时 `focus` 指向已不存在叶子的情形（关格的 focus 回落由 05 负责，此处只保证不 panic）
- [x] `scripts/check.sh` 绿

## Comments

- 2026-11-11：合入 `7346788`（merge 04 到已含 05/06 的 local/dev）。
- 键盘路由：`Session::focused_pane()` 取 `focus_of(focus)`，`focused()` 给出该叶的 `TerminalView`；
  `focus_pane()` 切 session 再 `window.focus(该叶的 focus_handle)`，所以键盘只到焦点格。
- 边框：`workspace/panes.rs` 的 `leaf()` 在焦点格上叠 1px `theme.colors.accent` 浮层（叠在终端之上，
  不占格内像素）。单格 session 按 Ghostty 的做法不画框，也不外包一层点击区——只有一格时焦点无可辨。
- `focus_of` 回落：焦点叶不存在时取树的最左叶，故关格、树变化都不 panic。
- 校验：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、注释与 500 行规则均绿；
  `cargo test --lib` 436 通过，2 个 `term::shell::windows_tests` 失败为本机既有基线（PowerShell 提示符测试，
  改动前同样失败）。
- 2026-11-11 验收（Spec 双轴复审）：焦点 accent 边框、键盘只路由焦点格、点击切焦点、单格不画框、`focus_of` 回落最左叶——均按规格落地。两处规格偏差记为待补，未阻断合并：
  - **D6「rename 作用于焦点格」为部分满足**：`close_row` 取 `session.focus`，但 `rename`/`named`/`shown()`/`pid` 仍取 `session.terminal`；`drop_pane` 仅在关掉原 `terminal` 叶时才把 `terminal` 重指到焦点格。分屏后 `session.terminal` 可能指向非焦点格，rename 会作用到非焦点格。
  - **D9 菜单缺 `Focus Next Pane`**：规格伪代码列了该项，菜单只加了 Split Right/Down/Close Pane（快捷键按规格留待下一轮，未实现是对的）。
  - 相关联动：`split.rs` 用 `session.folder` 作新格 cwd，而 `session.folder` 只由首格 OSC7 更新（`events.rs:30` `if first`），非首格聚焦并 cd 后分屏，新格拿到的仍是首格目录——D2「父格当前 reported folder」仅在焦点为首个叶子时成立。
