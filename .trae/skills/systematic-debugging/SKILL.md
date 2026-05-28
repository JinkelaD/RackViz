---
name: systematic-debugging
description: Systematic 4-phase debugging: root cause → pattern analysis → hypothesis testing → implementation. Use for ANY bug, test failure, or unexpected behavior BEFORE proposing fixes.
---

# 系统性调试

## 核心原则

**永远先找到根因，再尝试修复。** 症状修复就是失败。

铁律：
```
NO FIXES WITHOUT ROOT CAUSE INVESTIGATION FIRST
```

如果你还没完成 Phase 1，就不能提修复方案。

## 四阶段流程

### Phase 1：根因调查（修复前必做）

1. **仔细读错误信息** — 不要跳过；错误信息往往包含解决方案
2. **稳定复现** — 能可靠触发吗？确切步骤是什么？
3. **检查最近变更** — 什么改动可能导致问题？（git diff, 最近提交）
4. **多组件系统中收集证据** — 在每个组件边界记录输入/输出，定位故障层
5. **追踪数据流** — 坏数据从哪里来？上溯直到找到源头

### Phase 2：模式分析

1. **找到正常运行的参照** — 代码库中类似的、正常工作的代码
2. **对比差异** — 列举每个不同之处，不要假设"这不重要"
3. **理解依赖** — 还需要什么组件、配置、环境？

### Phase 3：假设和验证

1. **形成单一假设** — "我认为 X 是因为 Y"
2. **最小改动测试** — 一次只改一个变量
3. **验证后继续** — 有效 → Phase 4；无效 → 新假设

### Phase 4：实现修复

1. **创建失败的测试用例** — 使用 tdd skill
2. **在根因处理修复** — 不是症状
3. **验证修复** — 测试通过？其他测试不破坏？
4. **如果修复无效**：
   - 试了 < 3 个修复 → 回到 Phase 1
   - **试了 ≥ 3 个修复 → 停下来，质疑架构！** 这是架构问题的信号

## 红牌警告

如果你发现自己在想：
- "快速改一下，后面再调查"
- "试试改 X 看看效果"
- "一次改多个东西，一起测"
- "我知道问题在哪，直接修"
- **"再试一个修复"（已经试了 2+ 个）**

→ **立即停止。回到 Phase 1。**

## 本项目常见调试场景

**后端 500 错误：**
```bash
# 检查 API 日志
# 检查数据库状态
python3 -c "import sqlite3; conn=sqlite3.connect('backend/rackviz.db'); ..."
```

**前端编译错误：**
```bash
cd frontend && npx tsc --noEmit  # 获取完整错误信息
```

**前后端联调问题：**
1. 确认后端正常运行：`python3 -c "import requests; r=requests.get('http://127.0.0.1:8000/api/racks/'); print(r.status_code)"`
2. 确认前端 console 无报错
3. 逐层排查：API → Hook → Component
