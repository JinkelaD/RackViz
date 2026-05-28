---
name: code-review
description: Review code changes for quality, correctness, and adherence to project conventions. Use after completing a task or when user requests review.
---

# 代码审查

## 概述

在任务完成后、进入下一任务前进行系统性代码审查。

## 审查维度

### 1. Spec/需求合规
- 代码是否实现了计划中的所有要求？
- 是否有多余的功能？
- 边界条件是否处理？

### 2. 代码质量
- 命名是否清晰、有意义？
- 逻辑是否简洁直接？
- 是否有重复代码？
- 文件大小是否合理？（>300 行考虑拆分）

### 3. 项目规范
- 是否遵循现有代码风格？
- 是否使用了项目的工具库和工具函数？
- TypeScript 类型是否完整、准确？
- 后端 Schema/Model 是否完整定义？

### 4. 安全性
- 是否有硬编码的秘密或密钥？
- 用户输入是否校验？
- API 参数是否有类型约束？

### 5. 性能（前端）
- 是否有不必要的重渲染？
- useMemo/useCallback 使用是否恰当？
- useEffect 依赖数组是否正确？

## 审查输出格式

```markdown
## Code Review

### 严重问题 (Critical)
- [ ] [file:line] 问题描述 → 修复建议

### 警告 (Warning)
- [ ] [file:line] 问题描述 → 修复建议

### 建议 (Suggestion)
- [ ] [file:line] 建议 → 可选

### 亮点
- ✅ 做得好的地方

### 审查结论
- ✅ 通过 / ⚠ 有条件通过 / ❌ 需要重大修改
```

## 本项目特别关注

**前端 (React/TypeScript)：**
- 组件是否遵循 single responsibility
- Props 类型是否完整
- CSS 是否使用项目变量 (`var(--...)`)
- 是否引入不必要的依赖

**后端 (FastAPI/SQLAlchemy)：**
- Pydantic Schema 是否正确
- SQLAlchemy 模型字段类型是否恰当
- API 响应状态码是否合适
