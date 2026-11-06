# 分屏（split panes）— 设计共识

状态：设计阶段，未实施。本文是三阶段流程的「阶段一分析 / 阶段二方案确认」产物，
不是实现授权。写代码前需用户明确进入阶段三。

每节一条决策：**问题 → 决定 → 理由 → 伪代码**。伪代码贴着仓库现有类型
（`Launch` / `Session` / `Workspace` / `Records`）的形状写，供人员审查；
最终 Rust 实现以本文件为准绳，非逐行照抄。

参考源码：
- `src/sessions.rs:29` — `Session`（顶层 session，左侧一行）
- `src/workspace.rs:94` — `Workspace{ sessions: Vec<Session>, active: usize }`
- `src/workspace/session.rs:70` — `duplicate`（同 shell+args+folder 的现成范本）
- `src/term/pty.rs:74` — `Launch{ program, cwd, env, reports, quote }`
- `src/term/status.rs:204` — `Records::shown()`（状态聚合的来源）

---

## D1 — 数据模型：session → pane 树（记 ADR）

**问题**：分屏出来的格，是顶层 session（左侧列表加平级行），还是挂在父 session 下？

**决定**：**session → pane 树**。一个顶层 session 挂一棵 pane 树；左侧 sessions
列表**不**因分屏新增平级行。每个 pane 是独立 pty，但不是顶层 session。

**理由**：用户明确不希望分屏把左侧列表撑成一堆平级行。与仓库既有的 herdr pane 树
（`src/term/herdr`，herdr 的 pane/cwd/焦点树）同构——只是从「跟随外部 herdr」变成
「本地原生 pane 树」。这是难逆转、日后看代码会问「为什么不是平级 tab」的决定，
满足 ADR 三条件（难逆转 + 无上下文会意外 + 真实权衡），记一条 ADR 进 `docs/adr/`。

**伪代码**：

```
// 一个顶层 session 下挂的 pane 树。叶子是真正画终端的格。
enum PaneNode {
    Leaf(PaneId),
    Split {
        axis: Axis,              // Horizontal = 左右切，Vertical = 上下切
        first: Box<PaneNode>,    // 左 / 上
        second: Box<PaneNode>,   // 右 / 下
        ratio: f32,              // 分隔条位置，0..1，拖拽可改
    },
}

// 叶子格的身份与内容。一个 pane = 独立 pty 进程 + 独立视图，但不是顶层 session。
struct Pane {
    id: PaneId,
    terminal: Entity<TerminalView>,   // 复用现有 view；焦点环画在这一层
    pty: Arc<Pty>,
    // pane 不再自带 shell/args/folder/look —— 这些上升到挂它的 Session
}

// 一个顶层 session = 左侧一行 + 一棵 pane 树 + 树的焦点格。
// 现有 Session 的 shell/args/folder/look 字段保留，但语义变为「焦点 pane 的」，
// 见 D5（行状态聚合）与 D2（folder/env 来源）。
struct Session {
    // …现有字段不变（title, name, editing, bell, waiting, …）
    panes: PaneTree,        // 新增：这棵树的形状
    leaves: BTreeMap<PaneId, Pane>,   // 新增：叶子内容
    focus: PaneId,          // 新增：键盘当前进的格
}
```

---

## D2 — 新格跑什么 / 哪个目录 / 哪套 env

**问题**：新分出来的格跑什么程序、在哪个目录、带哪套环境变量？

**决定**：
- **程序**：父格（发起分屏的那格）同 **shell + args**，即 `Launch::program`
  语义 = `duplicate`（`workspace/session.rs:70`）已经做的事。
- **目录**：父格**当前 reported folder**（OSC 7），不是 session 启动时的 `cwd`。
- **env**：父格 pty 的 **`Launch::env` 快照**——不追父 shell 之后 `export` 的增量
  （那需要新加一个 OSC 把运行时环境吐回来，工作量高一档，本轮不做）。

**理由**：`duplicate` 已经验证了「同 shell+args+reported folder」这条路；
`Launch::env` 是 spawn 时快照，干净、够用，符合「携带原始屏的地址环境」的字面。

**伪代码**：

```
fn split_leaf(session: &Session, parent_pane: PaneId, axis: Axis, cx) -> Launch {
    let src = &session.leaves[&parent_pane];
    let src_view = src.terminal.read(cx);
    Launch {
        // 同 shell + args：直接复用 session 顶层的（= duplicate 语义）
        program: program_from(session.shell, session.args),  // None ⇒ 登录 shell
        // 父格当前 reported folder（OSC 7），不是启动 cwd
        cwd: src_view.folder().clone(),
        // 父格 pty 的 env 快照 —— 不含父 shell 运行时 export 的增量
        env: session.launch_env.clone(),
        reports: session.reports,
        quote: src.pty.quote,
    }
}
// 说明：session 需多存一份 `launch_env: Vec<(String,String)>`（首格 spawn 时的
// Launch::env），因为现有 Session 只留了 shell/args，没留 env。
```

---

## D3 — 布局形态：任意二叉切分

**问题**：分屏布局是固定网格，还是任意树状切分？

**决定**：**任意二叉切分**（D1 的 `PaneNode::Split` 树）。横切/竖切、层层嵌套，
非固定网格。

**理由**：固定网格（2×2 之类）实现简单但表达力差，Ghostty/tmux/iTerm 都是树状。
树状让 D4 的拖拽、D6 的关格重排都有统一下手处（对树做局部替换）。

**伪代码**：见 D1 的 `PaneNode`。切一格 = 把该 `Leaf(id)` 原地换成
`Split{ axis, first: Leaf(id), second: Leaf(new_id), ratio: 0.5 }`。

---

## D4 — 分隔条可拖拽，宽度仅存内存

**问题**：格与格之间的分隔条能不能拖？宽度存不存进 settings？

**决定**：**可拖拽**。分隔条画一条 filled hairline（同 sessions/右侧列的既有分隔条
做法），拖拽调整相邻两格占比（改 `Split.ratio`）。**宽度仅存内存**，不进
`settings.json`；每个 session 各留一份，切走再切回原样恢复。

**理由**：与 sessions / 右侧列的拖拽分隔条（`workspace/widths.rs`）体验一致；
分屏布局是易变的临时状态，进 settings 会污染配置且跨 session 无意义。每 session
各存一份让 D8 的「切行保留布局」自然成立。

**伪代码**：

```
// 拖拽：命中分隔条后，按指针在父 Split 盒子里的相对位置更新 ratio。
fn on_drag(split: &mut PaneNode, pointer_ratio: f32) {
    if let PaneNode::Split { ratio, .. } = split {
        *ratio = pointer_ratio.clamp(MIN_RATIO, 1.0 - MIN_RATIO);
    }
}

// 布局：把树投影成一个个格子矩形，分隔条画在相邻两格交界。
fn layout(node: &PaneNode, bounds: Rect, out: &mut Vec<(PaneId, Rect)>) {
    match node {
        PaneNode::Leaf(id) => out.push((*id, bounds)),
        PaneNode::Split { axis, first, second, ratio } => {
            let (a, b) = bounds.split_at(*axis, *ratio);   // 留出 hairline 的几 px
            layout(first, a, out);
            layout(second, b, out);
        }
    }
}
```

---

## D5 — 行状态 orb：聚合最坏状态

**问题**：左侧那行的状态 orb 显示焦点格的状态，还是聚合所有格？

**决定**：**聚合最坏状态**（error > blocked > working > waiting > idle > done）。
hover 时 orb 下拉列出**每个格一条**（复用现有 hover 列表机制，`sessions.rs` 的
`LISTED` / `SHORT`）。

**理由**：orb 是行对外的健康信号。若焦点格 idle 但旁边一格 blocked，只显示 idle 会
掩盖问题——仓库既有「隐藏 session blocked 会把标题变强」也是这个精神。
现有 `Session::looks()`（`sessions.rs`）取单格 `records().shown()`；改为对树上
所有叶子做一次 worst-of 归并。

**伪代码**：

```
fn session_look(session: &Session, cx: &App) -> Option<Look> {
    session.leaves.values()
        .map(|pane| {
            let view = pane.terminal.read(cx);
            let failed = view.ended().is_some_and(|code| code != 0);
            look(view.records().shown().map(|(_, r)| r), failed)  // 复用现有单格逻辑
        })
        .max_by(worst_first)   // error > blocked > working > waiting > idle > done
}
// hover 列表：遍历 leaves，每格一行（其 title / message），沿用 LISTED=10 / SHORT=120。
```

---

## D6 — 焦点：accent 边框 + 键盘去向

**问题**：怎么标示键盘当前进的是哪个格？

**决定**：焦点格四周画**主题 accent 色细边框**（类 Ghostty active pane border）。
焦点格同时是左侧高亮行；`rename` / 右键菜单作用于焦点格。每个 session 记自己的
`focus: PaneId`。

**理由**：纯靠左侧高亮行不足以区分同屏多格（Ghostty/tmux/iTerm 都画 active border）。
复用主题 accent 色，不引入新颜色常量（符合「颜色取自 Ely 主题」的既定品味）。

**伪代码**：

```
fn paint(pane: &Pane, is_focus: bool) {
    let border = if is_focus {
        Some(Border::hairline(theme.accent()))   // 焦点格：accent 细边框
    } else { None };
    pane.render(with_border(border));
}
// 键盘事件只派发给 session.focus 指向的叶子；切焦点改写 session.focus。
```

---

## D7 — 嵌套深度封顶：最小尺寸自然封顶

**问题**：分屏嵌套深度无限，还是封顶？

**决定**：**最小格尺寸自然封顶**——单方向不小于放下 10×8 单元格的像素（`MIN_COLUMNS = 10`、
`MIN_LINES = 8`，见 `src/workspace.rs:73`）；达到就拒绝再切并提示。不设固定层数上限。

**理由**：无限嵌套会让格小到放不下 10×8；固定层数（如 3 层）则在不同窗口尺寸下
要么太松要么太紧。用最小尺寸封顶既保证可用性又自然适应窗口大小——与仓库既有的
防刷屏上限品味一致（如 Git 列表 500 行上限）。

**伪代码**：

```
fn can_split(pane_rect: Rect, axis: Axis, cell: (u16, u16)) -> Result<(), SplitError> {
    let min_px = min_cells_for(axis) * cell_size(axis, cell);   // 10 列宽 / 8 行高
    if pane_rect.size(axis) < 2 * min_px {
        return Err(SplitError::TooSmall);   // UI 提示「格太小，无法再分」
    }
    Ok(())
}
```

---

## D8 — 切行时旧布局保留

**问题**：切到另一行后，旧 session 的分屏布局怎么处理？

**决定**：**保留**。每个 session 在内存里各存一份 pane 布局树 + 拖拽宽度；切到
别行再切回，**原样恢复**。不进 `settings.json`。

**理由**：切走即瓦解会让用户每次切行都重新分屏，违背「分屏是 session 的一种持续
状态」的直觉。每 session 独立存，天然隔离，切行互不影响。

**伪代码**：

```
fn select_session(ws: &mut Workspace, index: usize) {
    ws.active = index;   // 现有字段；sessions 列表本身不变（仍是顶层 Vec<Session>）
    // 什么都不用做：旧 session 的 panes/leaves/focus 就挂在它自己的 Session 上，
    // 切回时直接读它自己的树即可。渲染只画 active session 的树。
}
```

---

## D9 — 入口：先菜单，后快捷键

**问题**：分屏 / 关格 / 切焦点用菜单还是快捷键？

**决定**：**先做菜单入口**（sessions 行右键 / pane 相关菜单），快捷键下一轮再加

**已定案（2026-11-04）**：用户否决 pane 右键菜单，**保留现有菜单栏 / 会话行右键**
（「保留现在的菜单栏方式，加入快捷键」「放弃调整菜单栏」）；同时否决 D9 原拟的
Cmd/Ctrl-Shift-D 一套，改为 **7 个动作**、默认 `alt-\` 右切 / `alt-/` 下切 /
`alt-w` 关格 / `alt-方向键` 切焦点窗格，且在 Settings › 终端 › 分屏 **可录制改键**
（`Option<String>` 字段，空串 = 不绑定，`None` = 平台默认；撞键拒绝并提示）。
（届时按 Ghostty/tmux 习惯定具体键面，如 Cmd/Ctrl-Shift-D 竖切、Shift-E 横切、
Shift-W 关格、方向键或 Cmd/Ctrl-[ ] 切焦点）。

**理由**：菜单先行让交互语义先落地、可审查，快捷键是纯叠加。与仓库既有「右键菜单
先行、快捷键随后」的节奏一致（如 D10 的会话右键菜单）。

**伪代码**：

```
// sessions 行右键菜单追加（复用现有 ContextMenu，见 sessions/menu.rs）
menu.item("Split Right",  |ws| ws.split_focus(Horizontal));
menu.item("Split Down",   |ws| ws.split_focus(Vertical));
menu.item("Close Pane",   |ws| ws.close_focus());
menu.separator();
menu.item("Focus Next Pane", |ws| ws.focus_neighbor(+1));
```

---

## D10 — 关格与重排

**问题**：关掉一格（用户关 or 程序退出）后布局怎么变？

**决定**：从树里摘掉该叶子，其父 `Split` 退化为另一子树（重排）；非焦点格程序退出
同样处理。关到只剩一格，该 session 退化为普通单终端；整行关闭走现有
`sessions::after_close`（`workspace.rs:277`）。

**理由**：tmux/Ghostty 的标准行为——关一格回收其空间给兄弟格，不做留白。
程序自然退出（非用户关）与用户关格同路径，避免两套重排逻辑。

**伪代码**：

```
fn close_pane(session: &mut Session, target: PaneId) {
    session.panes = remove(session.panes, target);   // 父 Split 退化为兄弟子树
    session.leaves.remove(&target);
    if let PaneNode::Leaf(id) = session.panes {
        session.focus = id;   // 退化成单终端
    } else if session.focus == target {
        session.focus = first_leaf(&session.panes);   // 焦点格被关，落到第一格
    }
    // 只剩这一行且它也空了 → 走现有 after_close 关整行
}

fn remove(node: PaneNode, target: PaneId) -> PaneNode {
    match node {
        Leaf(id) if id == target => unreachable!(),   // 由父层摘除
        Leaf(id) => Leaf(id),
        Split { first, second, .. } if first == Leaf(target) => *second,
        Split { first, second, .. } if second == Leaf(target) => *first,
        Split { axis, first, second, ratio } =>
            Split { axis, first: remove(first, target), second: remove(second, target), ratio },
    }
}
```

---

## 待落物（方案阶段，需用户点头后）

- **ADR**：`docs/adr/` 落一条，记录 D1（session → pane 树 + 每 session 内存布局）
  的理由。
- **GLOSSARY**：新术语 `session` / `pane` / `split 树` 首次确定，顺手起
  `GLOSSARY.md`。
- 实现细节（分隔条像素宽度、菜单项文案、拖拽阈值、焦点环取值）属阶段三设计，
  不在本文件展开。

## 明确的未决 / 后续轮

- ~~快捷键具体键面（D9，下一轮）~~：已完成，见 D9 的「已定案」段。
- 运行时 env 快照（D2 提到的新 OSC），本轮明确不做。
- herdr 焦点跟随（`src/term/herdr`）如何与本地 pane 树共存，未讨论。
- 现有 `duplicate` / `here`（`workspace/session.rs`）与分屏的关系：二者仍是「新增
  顶层 session」，与分屏（新增 pane）是两条路径，未合并。
