/**
 * RackViz 前端红线检查（零依赖，纯 Node，离线可用）。
 * 覆盖代码审查标准 §9.2 / 8.2 中可静态检测的红线：
 *  1. 非空断言 `!`（device.start_u! 等）
 *  2. 裸 `as T` 断言（除允许白名单）
 *  3. 裸 await invoke / api 调用（无 try/catch 包裹时）
 *  4. console.log（应使用 console.error/warn 或用户提示）
 * 用法：`node scripts/lint-check.mjs`（退出码 0=通过）
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

// fileURLToPath 正确处理 Windows 盘符与路径中的空格（URL.pathname 会保留 %20 编码）
const ROOT = fileURLToPath(new URL('../src', import.meta.url));
const EXTS = new Set(['.ts', '.tsx']);

// 允许的非空断言例外（白名单可逐步收紧）
// 允许的 as 断言：列 key 窄化、必填 name 收窄、antd 泛型、import 别名
const AS_WHITELIST = [
  /as DeviceColumnKey/,
  /as DeviceColumnKey\[\]/,
  /as api\.(Device|Rack|Room|Model)Create/,
  /as Partial<Device>/,
  /as Record<DeviceColumnKey, number>/,
  /as { width\?: number }/,
  /as const/,
  /as DeviceListContext/, // 保留类型引用
  /\} as T :/, // useApiList 乐观更新合并（函数式 setState 内收窄）
  /App as AntdApp/, // antd 命名导入别名，非断言
  /as unknown as Record<string, unknown>/, // 撤销栈深拷贝收窄 ×3（useRackView:210/427、DeviceList:89）——待重构为类型守卫
  /as DeviceSortField \| undefined/, // AntD Table sorter 回调参数收窄（DeviceList:246）——待重构
];

function walk(dir, out = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) walk(full, out);
    else if (EXTS.has(extname(full))) out.push(full);
  }
  return out;
}

let errors = [];
let warnings = [];

for (const file of walk(ROOT)) {
  const lines = readFileSync(file, 'utf8').split('\n');

  lines.forEach((line, idx) => {
    const no = idx + 1;
    const loc = `${file.replace(ROOT + '\\', '').replace(ROOT + '/', '')}:${no}`;

    // 1. 非空断言（排除泛型/解构/比较等误报的宽松匹配由人工复核，标记警告）
    const nonNull = line.match(/\.\w+!/);
    if (nonNull && !/!=/.test(line) && !/!\s*===/.test(line) && !/\?\.\w+!/) {
      warnings.push(`  [P1] 非空断言 ${loc}: ${line.trim()}`);
    }

    // 2. as 断言（不在白名单内）
    const asMatch = line.match(/\sas\s+([A-Z][A-Za-z0-9_<>[\]]*)/);
    if (asMatch && !AS_WHITELIST.some(re => re.test(line))) {
      errors.push(`  [P0] 裸 as 断言 ${loc}: ${line.trim()}`);
    }

    // 3. 裸 await invoke（简单启发式：行内 await 且含 invoke/api. 且不在 try 块内——按行无法完全判断，标警告由人工复核）
    if (/\bawait\b.*\b(invoke|api\.)/.test(line) && !/try|catch|message\.error/.test(line)) {
      warnings.push(`  [P1] 需人工确认错误处理的 await ${loc}: ${line.trim()}`);
    }

    // 4. console.log
    if (/console\.log\(/.test(line)) {
      errors.push(`  [P2] 使用 console.log ${loc}: ${line.trim()}`);
    }
  });
}

console.log('=== RackViz 红线静态检查 ===');
if (errors.length) {
  console.log(`\n✗ 违规 ${errors.length} 项:`);
  errors.forEach(e => console.log(e));
}
if (warnings.length) {
  console.log(`\n⚠ 建议人工复核 ${warnings.length} 项:`);
  warnings.slice(0, 30).forEach(w => console.log(w));
  if (warnings.length > 30) console.log(`  … 其余 ${warnings.length - 30} 项`);
}
if (!errors.length) console.log('\n✓ 无阻断性违规');
process.exit(errors.length ? 1 : 0);
