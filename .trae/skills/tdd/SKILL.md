---
name: tdd
description: Enforce test-driven-development: write failing test first, watch it fail, then implement. Use before writing ANY production code — new features, bug fixes, refactoring.
---

# 测试驱动开发 (TDD)

## 核心原则

**先写失败测试，亲眼看到它失败，再写最小代码让它通过。**

铁律：
```
NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
```

如果在测试之前写了代码 → 删除代码，重新开始。

## 适用范围

**始终使用：**
- 新功能
- Bug 修复
- 重构
- 行为变更

**例外（需用户确认）：**
- 一次性原型
- 配置文件
- 纯样式/CSS 修改

## RED-GREEN-REFACTOR 循环

```
RED: 写失败测试
  │
  ▼
验证失败 — 确认是因为功能缺失（不是拼写错误）
  │
  ▼
GREEN: 写最小代码通过测试
  │
  ▼
验证通过 — 所有测试绿色
  │
  ▼
REFACTOR: 清理代码，保持测试绿色
  │
  ▼
下一个测试...
```

### RED — 写失败测试

只写一个最小测试，展示期望行为：

```
✓ 好：
test('提交空邮箱应返回错误', () => {
  const result = submitForm({ email: '' })
  expect(result.error).toBe('邮箱必填')
})

✗ 差：
test('test1')  // 不清楚在测什么
```

### 验证 RED — 亲眼看到失败

```bash
# 前端
npm test -- path/to/test.test.ts
# 后端
pytest tests/path/test.py -v
```

确认：
- 测试失败（不是报错）
- 失败原因符合预期（功能未实现）
- 不是拼写错误或语法错误导致失败

### GREEN — 最小代码

只写刚好让测试通过的代码，不要：
- 加未测试的功能
- 提前优化
- 重构（那是 REFACTOR 阶段的事）

### 验证 GREEN — 亲眼看到通过

确认：
- 新测试通过
- 其他测试仍然通过
- 无警告、无错误

### REFACTOR — 清理

只有在全绿之后：
- 消除重复
- 改善命名
- 提取辅助函数
- 保持测试绿色

## 常见借口 vs 现实

| 借口 | 现实 |
|------|------|
| "太简单了，不需要测试" | 简单代码也会出错，测试 30 秒 |
| "我先写实现，后面补测试" | 后面永远不补；测试立即通过证明不了什么 |
| "手动测试就够了" | 手动测试没有记录、不能复现 |
| "TDD 太慢了" | 调试比写测试慢得多 |
| "已经花了 X 小时，删除太浪费" | 沉没成本谬误。不可信代码是技术债务 |

## 本项目测试方式

**后端 (Python/FastAPI):**
```bash
pytest tests/ -v
```

**前端 (React/TypeScript):**
```bash
cd frontend && npx tsc --noEmit  # 类型检查
npm test -- path/to/test
```

## 最终规则

```
生产代码 → 有测试存在且测试曾失败过
否则 → 不是 TDD
```

未经用户许可，不可例外。
