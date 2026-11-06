# 02: 菜单触发分屏 — 真正二分出第二格

**What to build:** 右键 sessions 行的菜单新增 Split Right（竖切，左右）与 Split Down
（横切，上下）。选中后在焦点格旁真 spawn 一个新 pty，构成二叉树的两个叶子。此时布局
渲染仍是 T03 的活，本票先保证「第二格的进程、env、目录、程序」都正确立起来。

**Status:** done（D7 提示与焦点路径见 T08）

- [x] sessions 行右键菜单（复用现有 `ContextMenu`）追加 Split Right / Split Down，作用于焦点格
- [x] 新格 `Launch`：同 shell + args（复用 duplicate 语义）、父格当前 reported folder（OSC 7）、父格 pty 的 `Launch::env` 快照（不追父 shell 运行时 export 增量）
- [x] `Session` 需多存首格 spawn 时的 `launch_env`（现有 Session 只留 shell/args）
- [x] 把焦点格的 `Leaf(id)` 原地换成 `Split{ axis, first: Leaf(id), second: Leaf(new_id), ratio: 0.5 }`，新 pty + view 挂为新叶
- [x] 新格 events 订阅并入现有 `session_event` 路径
- [x] 达到最小尺寸（单方向放不下 10×8 单元格 ×2）时拒绝再切并提示（嵌套深度靠最小尺寸自然封顶，不设固定层数）— 见 T08
- [x] `scripts/check.sh` 绿（fmt + clippy -D warnings 绿；2 个 `term::shell::windows_tests` cmd folder 失败为基线既有，与本票无关）
