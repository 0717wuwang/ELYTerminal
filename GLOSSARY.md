# GLOSSARY — 领域术语

ELYTerminal 的核心领域词汇。与实现票、ADR 用词保持一致；不要漂到同义词。

> 分屏（split panes）目前处于设计阶段、尚未实施。标 **(设计)** 的词条描述的是
> `.scratch/split-panes/spec.md` 定下、尚未落地的形状；落地后随实现更新。

## session

一个**顶层会话**：左侧 sessions 列表里的**一行**，`Workspace.sessions: Vec<Session>`
里的一项。拥有自己的标题、状态 orb、生命周期，挂一棵 pane 树 **(设计)**。

- 实现：`Session`（`src/sessions.rs:34`），由 `Workspace`（`src/workspace.rs:77`）持有。
- 边界：分屏**不**新增 session。session 只由「新建 / 复制 / 在此目录新建」产生。

## pane

分屏后真正画终端的**一格**：**独立 pty 进程 + 独立视图**，但**不是**顶层 session **(设计)**。
叶子 pane 挂一个 `TerminalView` 和一个 `Pty`。

- 实现（设计）：`Pane`，内容存在 `Session.leaves: BTreeMap<PaneId, Pane>`。
- 边界：pane 不自带 shell / args / folder / look——这些上升到**挂它的 session**，
  语义为「焦点 pane 的」。

## split 树（pane 树）

一个 session 内 pane 的**任意二叉切分树**，`PaneNode`，形状决定布局 **(设计)**。
叶子是 pane，内部节点 `Split{ axis, first, second, ratio }` 是一次横切 / 竖切，
`ratio` 是分隔条位置。

- 实现（设计）：`Session.panes: PaneTree`（根 `PaneNode`）。
- 操作：切格 = 把某 `Leaf` 原地换成 `Split`；关格 = 摘掉叶子、父 `Split` 退化为
  兄弟子树。分隔条可拖拽改 `ratio`，宽度仅存内存、每 session 各一份。

## focus 格

一棵 pane 树里**键盘当前进的那个 pane**（`Session.focus: PaneId`）**（设计）**。
四周画主题 accent 细边框；它是左侧高亮行；`rename` / 右键菜单作用于它；键盘事件
只派发给它。

- 边界：每个 session 记**自己的** focus，切 session 不丢各树焦点。
- 状态 orb 是对**所有**叶子聚合的最坏状态，不是只看 focus 格（ADR 0007、spec D5）。
