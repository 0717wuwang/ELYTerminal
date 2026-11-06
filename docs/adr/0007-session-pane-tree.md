# 0007 — session → pane 树，每 session 内存各存布局

- 状态：已接受（Accepted）
- 日期：2026-11-05
- 关联：`.scratch/split-panes/spec.md` 的 D1（本 ADR 落的就是 D1）
- 实现票：`.scratch/split-panes/issues/01`–`06`（数据模型 / 菜单分屏 / 树布局 / 焦点 / 拖拽关格 / 状态聚合）

## 背景

要给 ELYTerminal 加分屏：把一个终端格横切 / 竖切成多格。第一个要回答、
且日后最难逆转的问题是——**分出来的格，在数据模型里算什么？**

两条路：

1. **平级 session tab**：每分一格，左侧 sessions 列表多一行，格即顶层 session。
2. **session → pane 树**：一个顶层 session 挂一棵 pane 树，左侧列表**不**因分屏
   新增平级行；格是 pane，不是顶层 session。

仓库现状（见 `GLOSSARY.md`）：`Workspace{ sessions: Vec<Session>, active: usize }`
（`src/workspace.rs:77`），顶层 session 是左侧一行（`src/sessions.rs:34` 的
`Session`）。同时仓库已经跟着一棵**外部的** herdr pane 树走
（`src/term/herdr/`，herdr 的 pane / cwd / 焦点树）。

## 决定

**session → pane 树**（方案 2）。

- 一个顶层 session 挂一棵 pane 树；树的叶子是真正画终端的格（pane）。
- 左侧 sessions 列表**不**因分屏新增平级行；分屏只是把某个 pane 再切成两格。
- 每个 pane 是**独立 pty 进程 + 独立视图**，但**不是**顶层 session。
- 每个顶层 session 在**内存**里各存一份 pane 布局树 + 拖拽宽度
  （`Session` 新增 `panes: PaneTree` / `leaves: BTreeMap<PaneId, Pane>` /
  `focus: PaneId`）。切走再切回原样恢复；**不进** `settings.json`。

形状（节选自 spec D1，供审查，最终以实现为准）：

```
enum PaneNode {
    Leaf(PaneId),
    Split { axis: Axis, first: Box<PaneNode>, second: Box<PaneNode>, ratio: f32 },
}
struct Session {
    // …现有字段不变（title, name, bell, waiting, …）
    panes: PaneTree,                  // 树的形状
    leaves: BTreeMap<PaneId, Pane>,   // 叶子内容（各挂一个 TerminalView + Pty）
    focus: PaneId,                    // 键盘当前进的格
}
```

## 理由

- **用户明确不要平级 tab**：分屏若把左侧列表撑成一堆平级行，会淹没「哪些是
  真正独立的会话」这一信息。用户要的是「在同一块屏幕里再开一块」，不是「再开
  一个会话」。
- **与既有 herdr pane 树同构**：仓库已有 herdr 的 pane / cwd / 焦点树概念，
  只是过去是「跟随外部 herdr」。本地原生 pane 树与它同构，概念不新增、心智模型
  一致。
- **布局是易变临时状态**：每 session 各存一份、只存内存，让「切行保留布局」
  自然成立；进 settings 会污染配置，且跨 session 的绝对宽度无意义（与
  `src/workspace/widths.rs` 侧栏宽度进 settings 的取舍相反——侧栏宽度是持久偏好，
  分屏布局是会话内临时态）。

## 被否决的替代方案

- **平级 session tab**（方案 1）：每分一格左侧多一行。否决——违背用户明确诉求；
  左侧列表失去「顶层会话」的区分度；关格 / 重排 / 状态聚合都要在「哪些行属于同一次
  分屏」上额外记账，比一棵树更绕。

## 后果

- `Session` 的 shell / args / folder / look 字段语义变为「焦点 pane 的」；行状态 orb
  改为对树上所有叶子做 worst-of 聚合（spec D5），folder / env 取焦点格的
  reported folder（spec D2）。
- 分屏 / 关格 / 切焦点 / 拖拽都是**对树的局部替换**（spec D3 / D4 / D6 / D10），
  拥有统一下手处。
- 这是**难逆转**的决定：一旦 pane 树落地，左侧行、状态聚合、folder 来源、键盘
  派发全部挂在它上面。日后看代码会问「为什么不是平级 tab」，故记此 ADR。
- 满足 ADR 三条件：难逆转 + 无上下文会看不懂 + 真实权衡。
