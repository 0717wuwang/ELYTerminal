# GLOSSARY — 领域术语

ELYTerminal 的核心领域词汇。与实现票、ADR 用词保持一致；不要漂到同义词。

> 分屏（split panes）的数据模型已落地（`src/sessions/pane.rs` 的 `PaneNode` /
> `Session.panes` / `Session.focus`）。树布局与渲染（票 03）仍在进行。标 **(设计)** 的
> 细节描述 `.scratch/split-panes/spec.md` 定下、随实现收敛的形状。

## session

一个**顶层会话**：左侧 sessions 列表里的**一行**，`Workspace.sessions: Vec<Session>`
里的一项。拥有自己的标题、状态 orb、生命周期，挂一棵 pane 树 **(设计)**。

- 实现：`Session`（`src/sessions.rs:34`），由 `Workspace`（`src/workspace.rs:77`）持有。
- 边界：分屏**不**新增 session。session 只由「新建 / 复制 / 在此目录新建」产生。

## pane

分屏后真正画终端的**一格**：**独立 pty 进程 + 独立视图**，但**不是**顶层 session **(设计)**。
叶子 pane 挂一个 `TerminalView` 和一个 `Pty`。

- 实现：pane 的身份是它的 `PaneId`（复用该格 `TerminalView` 的 entity id）；各格视图存在
  `Session.views: BTreeMap<PaneId, Entity<TerminalView>>`，事件订阅在 `Session.pane_events`。
- 边界：pane 不自带 shell / args / folder / look——这些上升到**挂它的 session**，
  语义为「焦点 pane 的」。

## split 树（pane 树）

一个 session 内 pane 的**任意二叉切分树**，`PaneNode`，形状决定布局 **(设计)**。
叶子是 pane，内部节点 `Split{ axis, first, second, ratio }` 是一次横切 / 竖切，
- 实现：`Session.panes: PaneNode`（根即整棵树，无单独 `PaneTree` 类型），
  `src/sessions/pane.rs`。内部节点的 `axis: Axis` 取 `LeftRight`（左右切）或
  `TopBottom`（上下切）。
- 操作：切格 = 把某 `Leaf` 原地换成 `Split`；关格 = 摘掉叶子、父 `Split` 退化为
  兄弟子树。分隔条可拖拽改 `ratio`，宽度仅存内存、每 session 各一份。

## focus 格

一棵 pane 树里**键盘当前进的那个 pane**（`Session.focus: PaneId`）**（设计）**。
四周画主题 accent 细边框；它是左侧高亮行；`rename` / 右键菜单作用于它；键盘事件
只派发给它。

- 边界：每个 session 记**自己的** focus，切 session 不丢各树焦点。
- 状态 orb 是对**所有**叶子聚合的最坏状态，不是只看 focus 格（ADR 0007、spec D5）。

## 窗格键（pane key）

7 个分屏动作各自的键：右切、下切、关格、焦点左/右/上/下。`PaneKey`（`src/settings/panes.rs`）
把动作映射到 `Settings` 的字段，给默认值与归一化；`workspace::keys::apply_pane_keys` 把
生效的键绑进 keymap，改绑用 `Unbind` 压制旧绑定。

- 实现：默认 `alt-\`、`alt-/`、`alt-w`、`alt-方向键`；`None` = 平台默认，`Some("")` = 不绑定。
- 边界：不碰 Ely 的 Tab / F6 焦点绑定，也不用 `clear_key_bindings()`（理由见 ADR 0007 追加段）。
- 焦点方向上的「邻格」由 `PaneNode::neighbour` 给：从**最近的**同轴祖先取另一侧，再取那一侧
  离得最近的叶子（先在同一列/行内的兄弟之间走，跨列才跨出去）；到了边的另一头就绕回，即循环。
- 分屏后焦点落到**新 pane**，旧格让出焦点环。
