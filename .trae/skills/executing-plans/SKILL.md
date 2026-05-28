---
name: executing-plans
description: Execute implementation plan task by task. Use when plan is ready and user says "go" — dispatch subagents per task with two-stage review (spec compliance, then code quality).
---

# 按计划执行实现

## 概述

逐任务执行实现计划。每个任务独立执行、审查、提交。

**核心原则：新 subagent 处理每个任务 → 两阶段审查（spec 合规 + 代码质量）→ 连续执行不中断。**

## 何时使用

```
有计划吗？
  ├── 是 → 任务独立吗？
  │         ├── 是 → 使用 executing-plans
  │         └── 否（紧耦合） → 手动执行
  └── 否 → 先 brainstorming + writing-plans
```

## 流程

1. **读取计划** — 提取所有任务，创建 TodoWrite
2. **逐任务执行**：
   - 派发 subagent 实现任务
   - subagent 提问题 → 回答 → 继续
   - subagent 完成 → 自我审查
   - **Spec 合规审查** — 确认代码符合计划
   - **代码质量审查** — 检查代码质量
   - 审查通过 → 标记任务完成
3. **全部完成** → 调用 finishing-branch

## 连续执行原则

**任务之间不要停下来。** "要继续吗？"、进度摘要都是在浪费用户时间。除非遇到阻塞或全部完成，否则持续执行。

## 任务状态处理

subagent 返回四种状态：

| 状态 | 处理方式 |
|------|---------|
| **DONE** | 进入 spec 合规审查 |
| **DONE_WITH_CONCERNS** | 阅读问题，解决后再审查 |
| **NEEDS_CONTEXT** | 提供缺失的上下文，重新派发 |
| **BLOCKED** | 评估阻塞原因，换更强大模型或拆分任务 |

## 审查标准

**Spec 合规审查：**
- 代码是否实现了计划中的所有要求？
- 是否有多余的功能（超出计划范围）？
- ❌ 不通过 → subagent 修正 → 重新审查

**代码质量审查：**
- 命名清晰吗？
- 逻辑简洁吗？
- 文件结构合理吗？
- 有重复代码吗？
- ✅ 通过 → 进入下一任务

## 本项目适配

- 前端：类型检查 `npx tsc --noEmit` 必须通过
- 后端：API 测试端点可正常响应
- 提交：使用 git-commit skill 遵循 conventional commits
