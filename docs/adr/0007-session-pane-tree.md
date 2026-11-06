# 0007 — session → pane 树，每 session 内存各存布局

- 状态：已接受（Accepted）
- 日期：2026-11-05
- 关联：`.scratch/split-panes/spec.md` 的 D1（本 ADR 落的就是 D1）
- 实现票：`.scratch/split-panes/issues/01`–`06`（数据模型 / 菜单分屏 / 树布局 / 焦点 / 拖拽关格 / 状态聚合）
- 后续：`.scratch/split-panes/issues/08`（每格目录、焦点菜单路径、最小尺寸提示）

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
- 每个 pane 是**独立 pty 进程 + 独立视图**，但**不是**顶层 session。pane 的身份是它的
  `PaneId`（复用该格 `TerminalView` 的 entity id）；视图与事件订阅按 `PaneId` 存在
  `Session.views` / `Session.pane_events` 里，没有单独的 `Pane` 结构。
- 每个顶层 session 在**内存**里各存一份 pane 布局树 + 拖拽宽度（`Session.panes: PaneNode`
  / `Session.focus: PaneId`，见 `src/sessions/pane.rs`）。切走再切回原样恢复；**不进**
  `settings.json`。
- 形状（`src/sessions/pane.rs` 已落地；spec D1 的伪代码把 `Axis` 写作 Horizontal /
  Vertical，实现取 `LeftRight` / `TopBottom`）：

```
pub enum Axis { LeftRight, TopBottom }   // 左右切 / 上下切
pub enum PaneNode {
    Leaf(PaneId),
    Split { axis: Axis, first: Box<PaneNode>, second: Box<PaneNode>, ratio: f32 },
}
// Session（src/sessions.rs）持有：
//   pub panes: PaneNode,                              // 树的形状（根即整棵树）
//   pub views: BTreeMap<PaneId, Entity<TerminalView>>, // 各格视图
//   pub pane_events: BTreeMap<PaneId, Subscription>,
//   pub focus: PaneId,      // 键盘当前进的格
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

## 追加：窗格键可改绑（2026-11-04）

7 个动作（右切 / 下切 / 关格 / 焦点左右上下）各自的键存在 `Settings` 的
`key_split_right` 等 7 个 `Option<String>` 里，经 `crate::settings::panes::PaneKey`
映射到字段，`None` 取平台默认（`alt-\`、`alt-/`、`alt-w`、`alt-方向键`），`Some("")`
表示不绑定，由 `validate()` 用 `Keystroke::parse` 校验并拒绝两个动作撞同一键。

改绑走 gpui 的**禁用绑定**通道而不是 `clear_key_bindings()`：`Unbind(action_name)`
作为 `KeyBinding` 的 action 只压制同一 keystroke 上的既有绑定，且
`Keymap::bindings_for_action` 会把被压制的绑定从展示列表里过滤掉，所以设置页
「快捷键」只显示真正生效的那一个键。理由：`clear_key_bindings()` 会连 Ely 在
`init()` 里注册的 Tab / Shift-Tab / F6 焦点绑定一并清掉，而 Ely 的 `setup()` 是
私有的，重放只能整体重跑 `ely_gpui_component::init`，那会再把 `theme::Theme`
重置成默认主题。`workspace::keys::apply_pane_keys` 因此读 keymap 里该动作现存的
keystroke 逐条补 `Unbind`（去重，幂等），再 `bind_keys` 新键。

已知权衡：`alt-\`、`alt-/`、`alt-w`、`alt-方向键` 在 keymap 层先于终端，
shell 的 backward-kill-word（`alt-backspace`）不受影响，但 zsh 的 vi 模式
`alt-\`/`alt-w` 一类绑定、以及任何程序从 `Option as Alt` 收到的
`ESC \`、`ESC /`、`ESC w`、`ESC [ D` 都收不到了；要还原，改键或清空该动作的键。
`alt-` 系还在 macOS 上紧跟 Option 的 `alt-⇧-/`（`–`）与 Option 组合键（`alt-a = å`）。

### 2026-11-04：焦点跟着新格走，方向键先走兄弟再跨列，到边循环

分屏后焦点落到**新 pane**（`workspace::split` 末尾调 `focus_pane`），否则新起的
程序收不到键盘、焦点环也还停在旧格。`PaneNode::neighbour` 改成从**最近的**
同轴祖先往上找：先走同一列/行里的兄弟，跨列才跨出去——B1 左跳先到 B 再到 A1，
而不是沿着根一路跳到 A1。到了边的另一头就绕回（`round_about`）：向右/向下
绕到全树的第一个叶子，向左/向上绕到最后一个；焦点本来就在那头时按叶子序走一格，
这样每个方向都能一直按键循环，任何格都不会「卡住」。
