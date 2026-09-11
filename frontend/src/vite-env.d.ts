/// <reference types="vite/client" />

// TS 7 起 `noUncheckedSideEffectImports` 默认为 true：
// 对 './styles/global.css' 这类「副作用导入」要求存在模块声明。
// 该文件引用 Vite 客户端类型，提供 *.css / 静态资源 / import.meta.env 的声明。
