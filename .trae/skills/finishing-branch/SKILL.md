---
name: finishing-branch
description: Complete a development branch: verify tests pass, review changes, and present merge options (merge/PR/keep/discard). Use when all implementation tasks are complete.
---

# 完成开发分支

## 概述

所有实现任务完成后，系统性地收尾。

## 流程

### 1. 验证

```
- npx tsc --noEmit     ✅ 前端类型检查通过
- 后端 API 测试        ✅ 核心端点响应正常
- 前端构建             ✅ 无报错
```

### 2. 审查变更

- 查看变更摘要：`git diff --stat main...HEAD`
- 确认没有意外文件被修改
- 检查是否有调试代码残留

### 3. 呈现选项

向用户展示：

1. **合并到 main** — 直接合并
2. **创建 PR** — 推送到远程并创建 PR
3. **保留分支** — 不合并，保留工作
4. **丢弃** — 放弃所有变更

### 4. 清理

- 删除临时文件、调试脚本
- 确保 `.gitignore` 覆盖新文件类型
- 文档更新（如需要）

## 本项目检查清单

- [ ] `cd frontend && npx tsc --noEmit` 通过
- [ ] 后端 API 未破坏现有功能
- [ ] 前端页面可正常加载
- [ ] 无遗留的 console.log / print 调试语句
- [ ] 无硬编码的 localhost 端口
- [ ] 数据库迁移脚本未遗留
