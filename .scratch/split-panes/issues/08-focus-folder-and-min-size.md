# T08 — 焦点格兑现：目录来源 / 菜单与会话查找 / 最小尺寸提示

**Status:** done

收尾清单（评审发现 D2、D6、D7 只部分兑现）在本票一齐补上。

- [x] `Session.folders: BTreeMap<PaneId, PathBuf>`：每格最近一次 OSC 7，`drop_pane` 随格移除
- [x] 分屏新格 `cwd` 取 `folders[target]`（目标格自己的目录），不再只认首格 `folder`
- [x] `Session::focus_folder()`：`duplicate` 与右键「New session here」取焦点格目录
- [x] `Workspace::session_index(view)`：按实体找会话，替换 `split_row` / `close_row` / `session_event` 的三份重复查找
- [x] 右键菜单的 Close session 走 `session_index`，分屏目标取 `focused_pane()`
- [x] 重命名不再绑实体 id：按「哪个会话开着重命名框」定位，`end_rename(keep, window, cx)`
- [x] D7：`split_shortage` + `refuse_split`，按格实际像素与 10×8 单元格判定，`PromptLevel::Warning` 提示并记 warn 日志
- [x] `split/tests.rs`：按列、按行、恰好容纳、不可用 cell 四种情形
- [x] `scripts/check.sh` 绿（fmt + clippy -D warnings + 482 测试）

**仍不做（见 spec「明确的未决 / 后续轮」）**：运行时 env 增量（新 OSC）、herdr 焦点跟随与本地 pane 树的共存、`duplicate` / `here` 与分屏合并为同一路径（现共用 `open` 与 `focus_folder`，语义仍是新的顶层 session）。真机 GUI 尚未驱动提示框。
