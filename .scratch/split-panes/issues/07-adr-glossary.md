# 07: ADR + GLOSSARY — 记录 session→pane 树决策（文档，可与实现并行）

**What to build:** 把 D1（session → pane 树 + 每 session 内存布局）这条难逆转的架构决策
落成 ADR，并把首次确定的术语 session / pane / split 树 / focus 格记进 GLOSSARY。本票不碰
任何 `.rs`，与整条实现链（01–06）零冲突，可由任何人立刻开始，不被 01 阻塞。

**Blocked by:** None（can start immediately）

**Status:** done

- [x] `docs/adr/` 新增一条 ADR：记录「session → pane 树、左侧列表不因分屏加平级行、每 session 内存各存布局」的决定、理由（用户不要平级 tab；与既有 herdr pane 树同构）与被否决的替代方案（平级 session tab）
- [x] 新建 `GLOSSARY.md`（或并入既有领域文档），定义 session / pane / split 树 / focus 格四个术语
- [x] ADR 交叉引用 `.scratch/split-panes/spec.md` 的 D1；术语与实现票用词一致
- [x] 纯文档改动，不需跑 `scripts/check.sh` 的 clippy/tests（但过 500 行规则与 fmt 若适用）
