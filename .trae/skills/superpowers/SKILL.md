---
name: superpowers
description: Core methodology entry skill. Use at the START of every conversation. Enforces structured workflow: brainstorming → plans → TDD implementation → review → finish. Never skip; even simple tasks benefit from structure.
---

# Superpowers — 软件工程方法论

## 概述

Superpowers 是一套用于驾驭 AI 编码助手的完整方法论。它强制结构化工作流：**brainstorming → writing-plans → TDD 实现 → code-review → finishing**。

**核心原则：在写任何代码之前，先思考、先设计、先计划。**

## 适用范围

**每次对话都必须先检查 Superpowers。** 即使是最简单的任务。

当你收到任务时，按以下优先级检查：

1. **用户是否有明确的设计/规格？** → 跳过 brainstorming，进入 writing-plans
2. **这是全新的功能/模块？** → 启动 brainstorming skill
3. **这是有 spec/plan 的实现任务？** → 使用 TDD + executing-plans
4. **这是 bug 修复？** → 使用 systematic-debugging + TDD
5. **这是代码审查？** → 使用 code-review skill

## 工作流总览

```
用户提出需求
  │
  ▼
[browse-superpowers] — 检查是否已 brainstorm
  │
  ├── 否 → [brainstorming] 理解需求 → 设计 → 用户确认
  │         │
  └── 是 → ─┤
             ▼
          [writing-plans] 编写实现计划 → 用户确认
             │
             ▼
          [executing-plans] 逐任务执行
             │  ├── [test-driven-development] 每次实现前先写测试
             │  ├── [code-review] 任务间审查
             │  └── [systematic-debugging] 遇到 bug 时
             │
             ▼
          [finishing-branch] 完成 → 合并/PR
```

## 技能清单

| 技能 | 用途 | 触发条件 |
|------|------|---------|
| **brainstorming** (内置) | 需求探索、设计方案 | 新功能、无 spec |
| **writing-plans** (内置) | 编写实现计划 | 有设计/spec 后 |
| **tdd** | 测试驱动开发 | 任何代码实现 |
| **executing-plans** | 按计划逐任务执行 | 有计划后 |
| **systematic-debugging** | 系统性调试 | bug、测试失败 |
| **code-review** | 代码审查 | 任务完成后 |
| **finishing-branch** | 完成开发分支 | 所有任务完成后 |

## 技能调用规则

1. **before 任何代码** → 先检查是否已 design + plan
2. **before 任何实现** → 先 invoke tdd skill
3. **before 任何 fix** → 先 invoke systematic-debugging skill
4. **after 每个任务** → code-review
5. **after 所有任务** → finishing-branch

## 红牌警告 — 立即停止

如果你发现自己有以下想法，立即停止并遵循流程：

| 想法 | 正确做法 |
|------|---------|
| "这个太简单了，不需要设计" | 简单任务也有隐藏假设，先设计 |
| "我先写代码，测试后面补" | 必须先测试后代码 |
| "快速改一下，应该没问题" | 系统性调试，找到根因 |
| "我知道问题在哪，直接修" | 先复现，再假设，再修复 |
| "跳过审查吧，改动很小" | 小道改动是 bug 高发区 |

## 项目适配

本项目技术栈：**Python/FastAPI 后端 + React/TypeScript 前端 + SQLite**

- 后端测试：`pytest`（如有配置）
- 前端类型检查：`npx tsc --noEmit`
- 构建：Vite dev server
- 计划文档路径：`docs/plans/YYYY-MM-DD-<topic>.md`
