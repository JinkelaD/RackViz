# RackViz 磁盘占用分析与清理方案

> ⚠️ **数字过期标注（2026-09-16）**：本文 `target/` 实测为 1.6 GB，现已膨胀至 **约 3.1 GB**（新增 `[profile.release] strip="symbols"` 需全量重编后才回落）；项目已建立 Git（HEAD 见 00-总览），本文「无 .git 兜底」的风险描述已失效。**方法论（测量命令、硬链接口径、删 target 的代价评估）仍然有效**。
>
> **性质说明**：本文为**只读分析**成果。撰写过程中未对任何文件执行删除、移动、修改操作，仅执行 `du` / `ls` / `find` / `cat` 与只读统计脚本。
> 所有数字均来自本机实测（测量时间：本次会话；环境 Windows + Git Bash）。
> 单位口径：`du -h` 输出记为 `G`/`M`（二进制 GiB/MiB），Python 脚本统计的文件表观大小记为 `MiB`。两者差异源于**硬链接重复计数**，文中已逐处标注。

---

## 0. 执行摘要（先看这里）

| 项 | 结论 |
|---|---|
| 项目总体 | **1.8G**（`du -sh .`） |
| `src-tauri/target/release` | **1.6G**，占项目 **≈89%** |
| release 内部构成 | `deps/` **1.4G（87.5%）** + `build/` **194M（12.2%）** + `.fingerprint/` **2.7M（0.17%）** + `incremental/` **0** + `examples/` **0** |
| 产物类型构成（表观大小） | `.rlib` 779.9 MiB（44.0%）/ `.rmeta` 449.5 MiB（25.4%）/ `.pdb` 290.7 MiB（16.4%）/ `.exe` 132.8 MiB（7.5%）/ `.dll` 70.4 MiB（4.0%）/ `.lib` 35.3 MiB（2.0%） |
| 最大 crate | `windows` 172.1 MiB、`windows-sys` 107.6 MiB、`tauri-utils` 86.1 MiB、`rackviz_lib` 70.4 MiB、`tauri-macros` 46.7 MiB |
| `.pdb` 专项 | **135 个，304,836,608 字节 = 290.7 MiB** |
| 最终产物 | `rackviz.exe` **17,867,776 字节（17.0 MiB）**；无 `.msi` / `.nsis`（`bundle/` 目录不存在） |
| `release` profile | `src-tauri/Cargo.toml` **未配置 `[profile.release]`**，fingerprint 记录 `"rustflags":[]` → 使用 Cargo 默认值 |
| 能否删 `target/release` | **技术上能删，100% 可再生**；但代价是**全量重编 584 个编译单元（Cargo.lock 共 500 个包）**，且本项目**无 `.git` 兜底**，删除后本机唯一的 `rackviz.exe` 会消失 |
| 是否影响已安装程序 | **否**——当前无 `rackviz` 进程运行，`Program Files` 及 `AppData\Local/Roaming` 下均无已安装副本，`bundle/` 未生成，exe 仅存在于 `target/release` |
| 保守方案释放 | **≈464 MiB**（仅清 `.pdb` + `node_modules` + `dist`），**零 Rust 重编成本** |
| 激进方案释放 | **≈1.77G**，项目瘦到 ≈2M，代价：全量 Rust 重编 + `npm install` + `npm run build` |

---

## 1. 测量方法与口径

```bash
# 总体与顶层
cd "D:/Cursor Project/RackViz" && du -sh . && du -sh */ | sort -rh
# release 逐层（含隐藏目录）
cd src-tauri/target/release && du -sh .[!.]*/ */ | sort -rh
cd src-tauri/target/release/deps && ls -lhS | head -45
# pdb / exe 专项
find . -name "*.pdb" -printf "%s %p\n" | sort -rn
find . -name "*.exe" -printf "%s %p\n" | sort -rn
```

**口径修正（重要）**：`target/release` 内存在大量**硬链接**——
- `release/rackviz.exe` ↔ `release/deps/rackviz.exe`（链接数 2）
- `release/rackviz.pdb` ↔ `release/deps/rackviz.pdb`（链接数 2）
- `release/librackviz_lib.rlib` ↔ `release/deps/librackviz_lib-3f238885787673bc.rlib`（链接数 2）
- `build/<pkg>-<hash>/build_script_build.exe` ↔ `build-script-build.exe`（成对，链接数 2）

`du` 对同一 inode 只计一次（因此 `du -sh build` = 194M），而逐文件累加的 Python 脚本会计两次（因此 `build/` 表观 = 346.2 MiB）。**本章后续凡涉及"实际占盘"一律采用 `du` 值。**

---

## 2. 总体分布（实测）

| 路径（相对项目根） | 实测大小 | 说明 |
|---|---:|---|
| `.`（项目根） | **1.8G** | |
| `src-tauri/` | **1.6G** | |
| ├─ `target/` | **1.6G** | 唯一的大头 |
| └─ 源码与配置（其余全部） | ≈1.5M | `gen/` 957K + `icons/` 292K + `Cargo.lock` 126K + `src/` 119K + `templates/` 4.0K + `capabilities/` 1.0K + `Cargo.toml`/`build.rs`/`tauri.conf.json` |
| `frontend/` | **173M** | |
| ├─ `node_modules/` | **172M** | 依赖，可再生 |
| ├─ `dist/` | **1.1M** | 前端构建产物 |
| ├─ `src/` | 185K | **源码，禁止动** |
| ├─ `package-lock.json` | 99K | **锁文件，禁止动** |
| └─ `public/` | 0 | 空目录 |
| `docs/` | 164K | 文档，**禁止动** |
| `build-exe.bat` | 3.4K | |

**关键观察**：项目本体（源码 + 文档 + 配置）合计不足 **2M**。1.8G 中 **99.9%** 是可再生的构建产物与依赖。

---

## 3. `src-tauri/target/release` 逐层下钻

### 3.1 目录级（`du`，硬链接已去重）

| 子目录 | 实测大小 | 占 release | 说明 |
|---|---:|---:|---|
| `deps/` | **1.4G** | 87.5% | 所有 crate 的编译产物（`.rlib`/`.rmeta`/proc-macro `.dll`/`.pdb`） |
| `build/` | **194M** | 12.2% | 131 个构建脚本的编译产物与运行输出 |
| `.fingerprint/` | **2.7M** | 0.17% | 584 个编译单元的指纹元数据（JSON + timestamp） |
| `incremental/` | **0** | 0% | 空目录——release profile 默认关闭增量编译 |
| `examples/` | **0** | 0% | 空目录 |
| 根目录散落文件 | ~59M（表观） | — | `librackviz_lib.rlib` 35M、`rackviz.exe` 18M、`rackviz.pdb` 6.8M、2 个 `.d`（3.1K×2）——**全部是 `deps/` 的硬链接，不额外占盘** |
| **合计** | **1.6G** | 100% | |

### 3.2 文件类型构成（表观大小，未去重，总计 1773.4 MiB）

| 扩展名 | 数量 | 大小 | 占比 | 说明 |
|---|---:|---:|---:|---|
| `.rlib` | 428 | **779.9 MiB** | 43.98% | Rust 静态库，依赖编译产物主体 |
| `.rmeta` | 427 | **449.5 MiB** | 25.35% | 元数据，供下游 crate 类型检查用 |
| `.pdb` | 135 | **290.7 MiB** | 16.39% | **Windows 调试符号（专项见 3.4）** |
| `.exe` | 110 | 132.8 MiB | 7.49% | 其中 108 个是 build-script 可执行文件（成对硬链接），仅 1 个是真正的 `rackviz.exe` |
| `.dll` | 28 | 70.4 MiB | 3.97% | proc-macro 动态库（如 `tauri_macros-*.dll` 各 13.4 MiB） |
| `.lib` | 36 | 35.3 MiB | 1.99% | MSVC 导入库（webview2 / windows 相关） |
| `.a` | 3 | 4.7 MiB | 0.26% | libsqlite3-sys 的 C 静态库 |
| `.o` | 3 | 4.5 MiB | 0.25% | C 目标文件 |
| `.d` | 509 | 2.2 MiB | 0.13% | Makefile 依赖描述 |
| `.rs` | 14 | 1.4 MiB | 0.08% | build script 生成的代码（如 webview2 绑定） |
| `.json` | 592 | 0.7 MiB | 0.04% | fingerprint / 输出元数据 |
| 无扩展名 | 1271 | 0.7 MiB | 0.04% | `output`、`root-output`、`*.invoked.timestamp` 等 |
| 其余（`.toml`/`.js`/`.css`/`.exp`/`.rc`/`.html`/`.expr`） | ~380 | <0.5 MiB | <0.05% | |

> **结论**：`.rlib` + `.rmeta` 合计占 **69.3%**，这是 Rust 依赖编译的固有开销，无法通过"选择性删除"压缩——它们要么整体存在（增量可用），要么整体消失（全量重编）。

### 3.3 `deps/` 按 crate 归因 Top 20

聚合同一 crate 名下所有 `.rlib` + `.rmeta` + `.dll` + `.pdb`（表观大小，`deps/` 合计 1368.5 MiB，共 274 个 crate 名 / 1409 个文件）：

| # | crate | 合计 | 占 deps | 文件构成 |
|---|---|---:|---:|---|
| 1 | `windows` | **172.1 MiB** | 12.58% | rlib 88.1 + rmeta 84.0 |
| 2 | `windows_sys` | **107.6 MiB** | 7.86% | 5 组 rlib/rmeta（feature 组合不同产生多编译单元） |
| 3 | `tauri_utils` | **86.1 MiB** | 6.29% | rlib 40.1+28.9，rmeta 7.4+9.7 |
| 4 | `rackviz_lib` | **70.4 MiB** | 5.14% | 本项目自己的 lib 产物（rlib 34.4+31.6） |
| 5 | `tauri_macros` | **46.7 MiB** | 3.41% | dll 13.4×2 + pdb 9.9×2 |
| 6 | `tauri` | **37.8 MiB** | 2.76% | rlib 10.9×2 + rmeta 8.0×2 |
| 7 | `zerocopy` | 28.3 MiB | 2.07% | rlib 14.2 + rmeta 14.2 |
| 8 | `regex_syntax` | 25.7 MiB | 1.88% | 两组 rlib/rmeta |
| 9 | `webview2_com_sys` | 23.9 MiB | 1.75% | rlib 17.3 + rmeta 6.6 |
| 10 | `rackviz` | 23.7 MiB | 1.74% | **exe 17.0 + pdb 6.7（最终产物）** |
| 11 | `regex_automata` | 20.1 MiB | 1.47% | |
| 12 | `serde_core` | 19.5 MiB | 1.42% | |
| 13 | `winnow` | 19.1 MiB | 1.40% | 4 组 |
| 14 | `cargo_toml` | 17.3 MiB | 1.26% | 由 `tauri-build` 引入 |
| 15 | `brotli` | 17.1 MiB | 1.25% | |
| 16 | `syn` | 16.4 MiB | 1.20% | rlib 12.0 + rmeta 4.4 |
| 17 | `toml` | 15.5 MiB | 1.14% | 4 组 |
| 18 | `tokio` | 15.5 MiB | 1.13% | rlib 8.5 + rmeta 6.8 |
| 19 | `deranged` | 15.2 MiB | 1.11% | |
| 20 | `serde_with` | 14.5 MiB | 1.06% | |

**归因解读**：
- **`windows` + `windows_sys` = 279.7 MiB（占 deps 20.4%）** —— Tauri 在 Windows 上的固有代价（`tauri`/`tao`/`webview2` 依赖完整的 Win32 绑定），无法避免。
- **`tauri` 全家桶（tauri + tauri_utils + tauri_macros + tauri_build + webview2_com_sys）≈ 210 MiB**。
- **`rackviz_lib` 本身 70.4 MiB**，其中出现了两个不同 hash（-3f238885787673bc 与 -c7906f068f400a13），均为当前有效单元（见 3.6 说明）。

### 3.4 `.pdb` 调试符号专项（重点）

```
实测：find . -name "*.pdb" | wc -l        → 135
      find . -name "*.pdb" -printf "%s\n" | awk 求和 → 304,836,608 字节 = 290.7 MiB
```

**Top 15（字节，实测）**：

| 大小 | 文件 |
|---:|---|
| 10,432,512 | `deps/tauri_macros-3a8fe9ff9b2f62de.pdb` |
| 10,424,320 | `deps/tauri_macros-b2c95ed3a886e7dc.pdb` |
| 8,409,088 | `build/rackviz-c98e4f9e92154be3/build_script_build-c98e4f9e92154be3.pdb` |
| 8,409,088 | `build/rackviz-c98e4f9e92154be3/build_script_build.pdb`（与上一行硬链接） |
| 7,032,832 | `rackviz.pdb` ⇄ `deps/rackviz.pdb`（硬链接对） |
| 5,869,568 | `build/tauri-plugin-fs-299e396dbec8024f/build_script_build*.pdb`（×2 硬链接） |
| 5,738,496 | `build/tauri-plugin-shell-1b5f86d21a7ff46a/build_script_build*.pdb`（×2） |
| 5,730,304 | `build/tauri-plugin-dialog-8f5a115017c4a362/build_script_build*.pdb`（×2） |
| 5,263,360 | `build/tauri-4e0e7056b77f260e/build_script_build*.pdb`（×2） |
| 5,263,360 | `build/tauri-2acd976ea83b754a/build_script_build*.pdb`（×2） |
| 3,837,952 | `deps/askama_derive-0fed9923105025f3.pdb` |
| 3,567,616 | `deps/num_enum_derive-8f8147bdf70a939d.pdb` |
| 3,231,744 | `deps/darling_macro-a14d50dda1a25b6b.pdb` |
| 3,026,944 | `deps/schemars_derive-438f214adaaad1ff.pdb` |
| 3,002,368 | `deps/serde_derive-fd653a84de653762.pdb` |
| … | 其余 120 个，合计约 180 MiB |

**硬链接去重后真实占盘**：`build/` 下 108 个 pdb 中一半是硬链接副本，实际占盘远小于 290.7 MiB 名义值。以 `build/` 为例：表观 199.8 MiB pdb → 去重后约 100 MiB。**保守估计 `.pdb` 的真实磁盘占用约 190–210 MiB**（`build/` 贡献约 100 MiB + `deps/` 贡献约 100 MiB）。

### 3.5 最终产物

```
rackviz.exe      17,867,776 字节（17.0 MiB）   ← release/ 根目录，硬链接至 deps/rackviz.exe
rackviz.pdb       7,032,832 字节（ 6.7 MiB）
librackviz_lib.rlib  35M（根目录，硬链接）
```

- **无 `.msi` / `.nsis`**：`src-tauri/target/release/bundle/` 目录**不存在**。
  虽然 `tauri.conf.json` 中 `bundle.active = true`、`targets = ["msi","nsis"]`，但打包只在执行 `tauri build` 时触发；本项目 `build-exe.bat` 第 6 步执行的是 **`cargo build --release`**，因此从未产生安装包。
- 即：**1.6G 中真正"有用的东西"只有 17 MiB 的 exe**。

### 3.6 `build/` 与 `.fingerprint/` 专项

`build/`（131 个子目录，去重后 **192.3 MiB**，`du` 报 **194M**）：

| 构建脚本包 | 去重后大小 | 占比 |
|---|---:|---:|
| `webview2-com-sys-36920b67622877bc` | 30.4 MiB | 15.83% |
| `rackviz-c98e4f9e92154be3`（本项目 build.rs） | 17.3 MiB | 8.98% |
| `tauri-plugin-fs-299e396dbec8024f` | 11.4 MiB | 5.94% |
| `tauri-plugin-shell-1b5f86d21a7ff46a` | 11.1 MiB | 5.79% |
| `tauri-plugin-dialog-8f5a115017c4a362` | 11.1 MiB | 5.76% |
| `tauri-2acd976ea83b754a` | 10.1 MiB | 5.24% |
| `tauri-4e0e7056b77f260e` | 10.1 MiB | 5.24% |
| `libsqlite3-sys-c0b140fd7bc95906` | 9.2 MiB | 4.76% |
| `libsqlite3-sys-3206e58e86847950` | 3.7 MiB | 1.92% |
| `vswhom-sys-9d05a69244a7ec9f` | 3.2 MiB | 1.68% |
| 其余 121 个 | ≈84 MiB | 43.7% |

按类型（未去重）：`.pdb` 199.8 MiB（57.71%）/ `.exe` 98.7 MiB（28.52%）/ `.lib` 35.3 MiB（10.18%）/ `.a` 4.7 / `.o` 4.5 / 其余 <2 MiB。
> 注意：`.exe` 与 `.pdb` 均成对硬链接，故表观 346.2 MiB → 实际 194M。

`.fingerprint/`：584 个条目（每个编译单元一个目录），2.7M，**这是增量的"记忆"**。删除它等价于触发全量重编。

`incremental/`：空目录（0 字节）——release profile 默认 `incremental = false`，**本项目没有任何增量编译缓存可供保留加速**。

### 3.7 是否存在"陈旧产物"（可只删一部分？）

用 `.fingerprint` 的 hash 集合反查 `deps/`：

```
fingerprint 条目数：584，唯一 hash：584
deps 中 hash 无法在 fingerprint 中找到的文件：0 个（0.0 MiB）
deps 中存在多个 build hash 的 crate：156 / 274
```

**结论：不存在陈旧（stale）产物。** 那 156 个"多 hash"crate 是**合法的并行编译单元**，原因是：
1. `build-dependencies`（`tauri-build` 及其依赖树：`cargo_toml`、`toml`、`serde`、`glob`…）需要在 **host** 上再编一份；
2. `windows-sys`/`hashbrown`/`getrandom` 等 crate 因依赖方 feature 组合不同，产生多个 feature 变体单元。

> **这意味着没有"中间档"清理**：`deps/` 里的东西要么整体留着（增量可用），要么整体删掉（全量重编）。手工挑删子集只能按文件类型（如 `.pdb`）切，无法按"过期"切。

---

## 4. 构建配置核实

### 4.1 `src-tauri/Cargo.toml` —— 无 `[profile.release]`

实测 `Cargo.toml` 完整内容含 `[package]`、`[dependencies]`、`[build-dependencies]`、`[lib]` 四段，**没有 `[profile.release]` 段**。
`.fingerprint/rackviz-3437d97f02b74c4b/bin-rackviz.json` 中记录 `"rustflags":[]`，确认无自定义 `-C` 参数。

→ 使用 Cargo 默认 release profile：`opt-level = 3`、`debug = false`、`strip` 未启用、`split-debuginfo` 取平台默认值。

**但磁盘上确实存在 135 个 `.pdb`（290.7 MiB）**，这是 MSVC 工具链下 rustc/link.exe 为各编译单元（含 build script 与 proc-macro DLL）生成符号文件的默认行为。
若要从源头压缩，可在 `Cargo.toml` 追加：

```toml
[profile.release]
strip = "symbols"        # 剥离符号表；或 strip = "debuginfo"
```
> ⚠️ 该改动**需要全量重编才能生效**，且效果需实测验证（本次分析未执行编译，不做效果断言）。

### 4.2 `src-tauri/tauri.conf.json`

- `"frontendDist": "../frontend/dist"` —— exe 在构建时把 `dist` 内容嵌入，构建完成后不再依赖 `dist` 目录。
- `"beforeBuildCommand": "cd ../frontend && npm run build"` —— 只有走 `tauri build` 才会自动重建前端；`cargo build --release` **不会**。
- `"bundle": { "active": true, "targets": ["msi","nsis"] }` —— 配置已开启，但未执行过 `tauri build`，`bundle/` 不存在。

### 4.3 `build-exe.bat`

三步：`npm install` → `npm run build` → **`cargo build --release`**。脚本内原注释：`提示: 首次编译需要 10-20 分钟，请耐心等待...`。
产物定位提示：`EXE 位置: %PROJECT_ROOT%src-tauri\target\release\rackviz.exe`，脚本**不会把 exe 拷贝出 target**。

### 4.4 `~/.cargo/config.toml`

仅配置了 crates.io → rsproxy.cn 镜像替换（`replace-with = "rsproxy-sparse"`）与 `git-fetch-with-cli`，无 profile 相关设置。

---

## 5. 删除 `target/release` 的影响判断

### 5.1 是否需要完全重新编译？—— **是，且是 100% 全量**

- `deps/` 中的 `.rlib` 就是**依赖 crate 的编译产物**。删除它们，Cargo 必须重新编译每一个依赖。
- 规模依据（实测）：
  - `Cargo.lock` 含 **500 个 `[[package]]`**；
  - `.fingerprint` 含 **584 个编译单元**（因为 build-dependencies 与 feature 变体产生额外单元）；
  - `incremental/` 为空（0 字节），release 无增量缓存可用 → 重编时**没有任何加速路径**；
  - 最重的几个 crate（`windows` 172 MiB、`windows-sys` 108 MiB、`tauri-utils` 86 MiB 产物）都是 opt-level=3 下编译缓慢的大型 crate。
- **时间预估的依据**（不凭空给数字）：
  1. 项目自带 `build-exe.bat` 注释：**首次编译 10–20 分钟**（作者实测口径）；
  2. 584 个单元 / 500 个包，MSVC + opt-level=3；
  3. 依赖源码已在本地缓存（`~/.cargo/registry/src` 1.4G），**重编不需要重新下载**，省去网络时间；
  4. 本次未执行编译，故**不给出本次实测分钟数**——请以脚本注释的 10–20 分钟为准，并按 CPU 核数上下浮动。

### 5.2 会不会影响已安装的运行中的程序？—— **不会**

| 检查项 | 实测结果 |
|---|---|
| `tasklist \| grep -i rackviz` | **无 rackviz 进程运行** |
| `C:\Program Files\RackViz` | 不存在 |
| `C:\Program Files (x86)\RackViz` | 不存在 |
| `%LOCALAPPDATA%\RackViz` | 不存在 |
| `%APPDATA%\RackViz` | 不存在 |
| `target/release/bundle/` | 不存在（未生成 msi/nsis 安装包） |

→ **exe 是"原地产物"，不是"拷贝出去的安装副本"**。删除 `target/` 不会破坏任何已安装程序，但**本机将不再有任何可运行的 `rackviz.exe`**（它是唯一的）。

### 5.3 删除 target 会不会丢失不可再生的东西？—— **不会**

`target/` 下 100% 是从 `Cargo.lock` + 源码 + registry 缓存确定性推导出的产物。可再生性保障：
- `src-tauri/Cargo.lock` 存在（126K，500 个包的精确版本锁定）→ 版本可复现；
- `~/.cargo/registry/src` 有 1.4G 依赖源码、`cache` 有 172M `.crate` 压缩包 → **离线也能重编**；
- `src-tauri/gen/`、`icons/`、`capabilities/` 均在 `target/` 之外，不受影响。

### 5.4 唯一的真实风险：**无 Git 兜底**

实测：**项目根目录下不存在 `.git`**。因此：
- 删除操作**不可回滚**（无 `git checkout` / `git reflog`）；
- 但 `target/`、`node_modules/`、`dist/` 都在 `.gitignore` 中被忽略（实测 `.gitignore` 含 `node_modules/`、`src-tauri/target/`、`frontend/dist/`、`*.exe`），说明它们本就不属于"应受版本控制的东西"，删除在语义上是安全的；
- **风险点仅在于"失去唯一的 exe 与编译缓存"，而非"失去信息"**。

---

## 6. 其他可清理项逐项排查

| # | 路径 | 实测大小 | 风险 | 判断与理由 | 恢复命令 |
|---|---|---:|---|---|---|
| 1 | `src-tauri/target/release`（含 deps/build/指纹） | **1.6G** | 中 | 全可再生，但删了要全量重编 584 单元，且本机唯一 exe 消失 | `cd "D:/Cursor Project/RackViz/src-tauri" && cargo build --release` |
| 2 | `src-tauri/target/release/**/*.pdb`（135 个） | 名义 290.7 MiB（硬链接去重后实际约 **190–210 MiB**） | 低 | 纯调试符号，运行时不需要；但 `cargo` 不会自动补回（fingerprint 仍视为最新），需 touch 源码后重编 | `cd "D:/Cursor Project/RackViz/src-tauri" && find target/release -name '*.pdb' -delete && touch src/main.rs && cargo build --release` |
| 3 | `src-tauri/target/CACHEDIR.TAG` | 177 B | — | 缓存目录标记，随 target 一起删 | — |
| 4 | `src-tauri/target/.rustc_info.json` | 1.1K | — | 工具链信息缓存，随 target 一起删 | 自动重建 |
| 5 | `src-tauri/target/debug` | **不存在**（target 下只有 `release/`） | — | 从未 debug 构建 | — |
| 6 | `src-tauri/target/release/incremental`、`examples` | **0 字节**（空目录） | 低 | 删了也释放 0 字节，无意义 | — |
| 7 | `frontend/node_modules/` | **172M** | 低-中 | 完全可再生；`package-lock.json` 在位保证版本一致；代价是需要网络 + 数分钟；**删了不影响源码阅读** | `cd "D:/Cursor Project/RackViz/frontend" && npm install` |
| 8 | `frontend/node_modules/.vite` | **不存在** | — | Vite 预构建缓存目录未生成 | — |
| 9 | `frontend/node_modules/.cache` | **不存在** | — | 同上 | — |
| 10 | `frontend/dist/` | **1.1M**（`assets/antd-BGFnEcUb.js` 780K、`vendor-CVyfv31H.js` 158K、`index-*.js` 52K、`index-*.css` 42K、`index.html` 903B） | 低 | **构建产物不是源码**；`.gitignore` 已忽略；已生成的 exe 内嵌了资源，删 dist 不影响现有 exe 运行；但**下次 `cargo build --release`（非 `tauri build`）不会自动重建前端**，需手动 `npm run build` | `cd "D:/Cursor Project/RackViz/frontend" && npm run build` |
| 11 | `frontend/package-lock.json` | 99K | **🚫 禁止删除** | 版本锁定。`package.json` 用的是 `^` 范围（如 `antd: ^5.22.0`、`vite: ^6.4.0`），删了 lock 后 `npm install` 会解析到最新次版本，可能引入不兼容变更；且**无 git 兜底，删了不可恢复** | 不可恢复 |
| 12 | `src-tauri/Cargo.lock` | 126K（500 个包） | **🚫 禁止删除** | 与 11 同理。删掉后 cargo 会重新解析 500 个依赖到"当前最新兼容版本"，在 Tauri 2.x 生态下极易导致编译失败；**无 git 兜底，不可恢复** | 不可恢复 |
| 13 | `src-tauri/gen/` | 957K（`gen/schemas/` 下 4 个 JSON：acl-manifests / capabilities / desktop-schema / windows-schema） | 中 | **是 `tauri-build` 生成的产物**，技术上删了可由构建重新生成；但它是 capabilities 校验与 IDE 补全的依据，且体积仅 957K，**不值得删** | `cd "D:/Cursor Project/RackViz/src-tauri" && cargo build --release` |
| 14 | `src-tauri/icons/` | 292K（`icon.ico` 279K + 3 个 png 各 ~1K） | 低 | 无超大资源（`icon.ico` 279K 属正常量级）；**保留** | — |
| 15 | `frontend/public/` | **0**（空目录） | — | 无超大资源 | — |
| 16 | 日志 / 临时 / 备份文件 | **0 字节** | — | 全项目（排除 `node_modules` 与 `target`）搜索 `*.log` `*.bak` `*.old` `*~` `.DS_Store` `Thumbs.db` `*.tmp` `*.db` `*.tsbuildinfo` → **零命中**，无可清理项 | — |
| 17 | `~/.cargo/registry` | **1.6G** | **🚫 不建议** | **全局共享**，删除会影响机器上所有其他 Rust 项目（全部需要重新下载依赖）。细分实测：`src/` 1.4G（rsproxy.cn 主机 956M + index.crates.io 主机 377M）、`cache/` 172M（rsproxy 115M + crates.io 57M）、`index/` 70M（crates.io 36M + rsproxy 34M） | `cargo fetch`（联网） |
| 18 | `~/.cargo/bin` | 13M | 不建议 | cargo/rustup 可执行文件 | 重装 rustup |
| 19 | `~/.cargo/target` | **不存在** | — | 未配置全局共享 target 目录 | — |
| 20 | `%LOCALAPPDATA%\npm-cache`（`C:\Users\Jinkela\AppData\Local\npm-cache`） | **618M** | 中 | **项目外、全局共享**。清理后 `npm install` 需重新联网下载 | `npm cache clean --force` |
| 21 | `frontend/src/`、`src-tauri/src/`（22 个 `.rs`/119K）、`docs/`、`*.json` 配置 | ≈2M | **🚫 禁止删除** | 后续其他 agent 需要阅读源码继续开发 | 不可恢复 |

---

## 7. 分层清理方案

### 档位 A：立刻可安全删除（低风险，无重建代价）

| 路径 | 释放 | 风险 | 理由 | 恢复方式 | 推荐 |
|---|---:|---|---|---|:---:|
| `src-tauri/target/release/**/*.pdb`（135 个，含 `build/` 与 `deps/`） | **≈190–210 MiB**（名义 290.7 MiB，硬链接去重后） | 低 | 纯调试符号，程序运行不需要；崩溃堆栈将失去符号化能力（不影响正确性） | `cd "D:/Cursor Project/RackViz/src-tauri" && touch src/main.rs && cargo build --release` | ⚠️ **有保留地推荐** |
| `src-tauri/target/release/incremental/`、`examples/` | **0 字节** | 低 | 空目录，删了没收益 | — | ❌ 无意义 |
| 日志/临时/备份文件 | **0 字节** | 低 | 实测零命中 | — | ❌ 无 |

> **关于删 `.pdb` 的保留意见**：这是"手工切 target 内部"的非标准操作。Cargo 不保证支持，`cargo` 后续可能不会主动补回这些文件（fingerprint 仍然新鲜）。收益约 200 MiB，占项目 11%，**性价比一般**。若你后续还要调试本机崩溃，建议保留。

### 档位 B：可删但需重建（耗时 / 需网络）

| 路径 | 释放 | 风险 | 理由 | 恢复命令 | 推荐 |
|---|---:|---|---|---|:---:|
| `frontend/node_modules/` | **172M** | 低-中 | 完全可再生；`package-lock.json` 保证版本；**不影响源码阅读**，不影响其他 agent 读 `frontend/src/` | `cd "D:/Cursor Project/RackViz/frontend" && npm install` | ✅ **推荐**（若近期不构建前端） |
| `frontend/dist/` | **1.1M** | 低 | 构建产物；`.gitignore` 已忽略；已生成的 exe 内嵌资源不受影响 | `cd "D:/Cursor Project/RackViz/frontend" && npm run build` | ⚠️ 收益仅 1.1M，可顺手删 |
| `src-tauri/target/`（整体） | **1.6G** | 中 | 全可再生，但**全量重编 584 单元**（脚本自述 10–20 分钟），且本机唯一 exe 消失 | `cd "D:/Cursor Project/RackViz/src-tauri" && cargo build --release` | ⚠️ **仅在需要立即腾出 1.6G 时推荐** |
| `%LOCALAPPDATA%\npm-cache` | 618M | 中 | 项目外、全局共享，清了会让所有 npm 项目后续安装变慢 | `npm cache clean --force` | ⚠️ 可选 |

### 档位 C：不要动 🚫

| 路径 | 大小 | 理由 |
|---|---:|---|
| `frontend/src/` | 185K | 源码，后续 agent 要读 |
| `src-tauri/src/`（22 个 `.rs`） | 119K | 源码，后续 agent 要读 |
| `docs/`（含本报告） | 164K | 文档 |
| **`frontend/package-lock.json`** | 99K | **锁版本。删了 `npm install` 会解析到新版本，可能不兼容；无 git 不可恢复** |
| **`src-tauri/Cargo.lock`** | 126K | **锁 500 个包版本。删了 cargo 重新解析，Tauri 2.x 生态下极易编译失败；无 git 不可恢复** |
| `src-tauri/tauri.conf.json`、`capabilities/`、`build.rs`、`Cargo.toml` | ~10K | 构建配置 |
| `src-tauri/gen/` | 957K | 虽是生成物，但 capabilities 校验与 IDE 补全依赖它；957K 不值得冒险 |
| `src-tauri/icons/` | 292K | 打包必需（tauri.conf.json 引用）；无超大文件 |
| `frontend/public/` | 0 | 空目录，无大资源 |
| **`~/.cargo/registry`** | **1.6G** | **全局共享。删了影响机器上所有其他 Rust 项目，需重新下载 1.4G 依赖源码；且本项目重编正需要它提供离线依赖** |
| `~/.cargo/bin` | 13M | 工具链 |
| `src-tauri/target/.fingerprint/` | 2.7M | 单独删 = 触发全量重编，等于删了整个 target 却只省 2.7M |

---

## 8. 保守方案 vs 激进方案

### 方案对比表

| 维度 | 🟢 保守方案 | 🟡 推荐折中方案 | 🔴 激进方案 |
|---|---|---|---|
| **操作** | 删 135 个 `.pdb` + `node_modules/` + `dist/` | 删 `node_modules/` + `dist/`（保留 target 与 pdb） | 删 `src-tauri/target/` + `frontend/dist/` + `node_modules/`（+ 可选清 npm-cache） |
| **释放空间** | **≈364–384 MiB**（pdb 190–210 + node_modules 172 + dist 1.1） | **≈174 MiB** | **≈1.77G**（项目从 1.8G → ≈2M） |
| **Rust 重编成本** | 0（但 pdb 不会自动补回） | **0** | **全量 584 单元**（脚本自述 10–20 分钟） |
| **前端恢复成本** | `npm install`（数分钟，需网络）+ `npm run build`（秒级） | 同左 | 同左 |
| **丢失 exe 风险** | ❌ 无 | ❌ 无 | ⚠️ **有**——本机唯一 `rackviz.exe` 消失，需重编才能再运行 |
| **丢失调试符号** | ⚠️ 是 | ❌ 否 | ⚠️ 是 |
| **影响其他 agent 读源码** | ❌ 不影响 | ❌ 不影响 | ❌ 不影响（`frontend/src`、`src-tauri/src`、`docs/` 均保留） |
| **可逆性** | 需联网 + 重编 | 需联网 | 需联网 + 长时间重编 |
| **是否需要网络** | 是（npm） | 是（npm） | 是（npm）；Rust 部分**离线可完成**（registry 缓存 1.4G 在位） |

### 🟢 保守方案（≈364–384 MiB）

```bash
# 1) 删除所有 Windows 调试符号（约 190–210 MiB 实际占盘）
cd "D:/Cursor Project/RackViz/src-tauri" && find target/release -name "*.pdb" -delete
# 2) 删除前端依赖与构建产物（173M）
cd "D:/Cursor Project/RackViz/frontend" && rm -rf node_modules dist
```
- **代价**：失去崩溃符号化能力；`npm install` 需联网几分钟；`.pdb` 不会自动补回（需 `touch src/main.rs` 后重编）。
- **剩余项目体积**：约 1.43G。

### 🔴 激进方案（≈1.77G）

```bash
# 1) 删除全部 Rust 构建产物（1.6G）
cd "D:/Cursor Project/RackViz/src-tauri" && rm -rf target
# 2) 删除前端依赖与构建产物（173M）
cd "D:/Cursor Project/RackViz/frontend" && rm -rf node_modules dist
# 3)（可选，全局，谨慎）清理 npm 缓存
npm cache clean --force
```
- **代价**：本机唯一 `rackviz.exe` 消失；下次运行需完整重编（584 单元，项目脚本自述 10–20 分钟）+ `npm install` + `npm run build`；期间 CPU 长时间满载。
- **剩余项目体积**：约 **2M**（src-tauri 源码 ≈1.5M + frontend/src 185K + 两个 lock 225K + docs 164K）。
- **可再生性**：✅ 完全可再生（`Cargo.lock` + `package-lock.json` + registry 缓存均在位）。

### 我的建议

> **优先执行"保守方案"中的第 2 步 + 折中保留 target**，即：
> ```
> cd "D:/Cursor Project/RackViz/frontend" && rm -rf node_modules dist     # 释放 173M，零 Rust 成本
> ```
> 这 173M 是**纯收益**：完全可再生、不影响任何源码、不影响 `target/` 增量、`package-lock.json` 保证版本一致。
>
> **`target/` 的 1.6G 建议留到最后再决定**：它的唯一真实价值是"省一次 10–20 分钟的全量重编"，而后续其他 agent 很可能需要反复 `cargo build` 验证改动。留着它，等于用 1.6G 磁盘换后续每次编译的分钟级加速与"随时能跑 exe"的能力。
>
> **如果磁盘确实吃紧**，先删 `node_modules`（173M，随时 `npm install` 回来）；还不够再考虑整体删 `target/`——那时请确保已备份 `rackviz.exe` 到项目外的安全位置（例如复制到 `D:/Cursor Project/RackViz_backup/rackviz.exe`），因为**本项目无 `.git`，删了就真没了**。

---

## 9. 附录

### 9.1 复现测量的完整命令

```bash
# 总体
cd "D:/Cursor Project/RackViz" && du -sh . && du -sh */ | sort -rh

# release 逐层（含隐藏目录）
cd "D:/Cursor Project/RackViz/src-tauri/target/release" && du -sh .[!.]*/ */ | sort -rh | head -40
cd "D:/Cursor Project/RackViz/src-tauri/target/release/deps" && ls -lhS | head -45

# pdb / exe
cd "D:/Cursor Project/RackViz/src-tauri/target/release"
find . -name "*.pdb" -printf "%s %p\n" | sort -rn | head -40
find . -name "*.pdb" -printf "%s\n" | awk '{s+=$1} END {printf "total=%d bytes = %.1f MiB\n", s, s/1048576}'
find . -name "*.exe" -printf "%s %p\n" | sort -rn | head -20

# 依赖数量
grep -c '^\[\[package\]\]' "D:/Cursor Project/RackViz/src-tauri/Cargo.lock"     # → 500

# 陈旧产物判定
# .fingerprint 条目数 = 584；deps 中 1409 个文件全部能在 fingerprint 找到对应 hash → 陈旧产物 0 个

# 全局缓存（只读报告，不建议删）
du -sh ~/.cargo/registry                                   # → 1.6G
du -sh ~/.cargo/registry/src ~/.cargo/registry/cache ~/.cargo/registry/index
du -sh "$LOCALAPPDATA/npm-cache"                           # → 618M
```

### 9.2 关键事实清单（供后续 agent 引用）

1. 项目 1.8G，`src-tauri/target/` 独占 1.6G（89%）。
2. `release/deps/` 1.4G + `release/build/` 194M + `.fingerprint` 2.7M，`incremental/` 与 `examples/` 为空。
3. 产物类型：`.rlib` 44.0% / `.rmeta` 25.4% / `.pdb` 16.4% / `.exe` 7.5% / `.dll` 4.0%。
4. `.pdb` 135 个，304,836,608 字节；硬链接去重后实际占盘约 190–210 MiB。
5. `rackviz.exe` 17,867,776 字节；**无 msi/nsis**（`bundle/` 不存在，`build-exe.bat` 只跑 `cargo build --release`）。
6. `Cargo.toml` **无 `[profile.release]`**，fingerprint 记录 `rustflags: []`。
7. `Cargo.lock` 锁定 **500 个包**；`.fingerprint` **584 个编译单元**；无增量缓存。
8. 156/274 个 crate 存在多个 build hash，但**全部是当前有效单元**（build-dependencies host 单元 + feature 变体），**非陈旧产物**。
9. 项目根**无 `.git`**，删除不可回滚。
10. 当前无 `rackviz` 进程运行，`Program Files` / `AppData` 下无已安装副本。
11. 全项目无 `*.log` / `*.bak` / `*.old` / `*~` / `.DS_Store` / `Thumbs.db` / `*.tmp` / `*.db` / `*.tsbuildinfo`。
12. `frontend/node_modules/.vite` 与 `frontend/node_modules/.cache` 均**不存在**。
13. `~/.cargo/registry` 1.6G 为**全局共享**，不建议清理。
14. `src-tauri/gen/`（957K）、`icons/`（292K）、`frontend/public/`（0）均无超大资源。
