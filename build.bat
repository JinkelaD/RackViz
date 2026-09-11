@echo off
:: ============================================================================
::  RackViz 一键构建脚本  (build.bat)
::
::  用法：直接双击本文件，或在 cmd 中执行  build.bat
::  产出：src-tauri\target\release\rackviz.exe
::
::  本脚本会自行定位并注入 MSVC 编译环境，无需预先打开 Developer Command Prompt。
::  若环境探测失败，请改用「x64 Native Tools Command Prompt」直接执行 cargo 命令。
:: ============================================================================
setlocal
cd /d "%~dp0"

echo ============================================================================
echo   RackViz 构建
echo ============================================================================
echo.

:: ---------------------------------------------------------------------------
:: 步骤 1/4  准备 MSVC 编译环境
::   不依赖 vswhere（本机 VS2019 BuildTools 未向 vswhere 注册），改为按候选路径枚举。
:: ---------------------------------------------------------------------------
where cl.exe >nul 2>nul
if not errorlevel 1 (
    echo [1/4] MSVC 环境已就绪 - cl.exe 已在 PATH
    goto :step_cargo
)

echo [1/4] 正在定位 MSVC 编译环境 ...

set "PF86=%ProgramFiles(x86)%"
set "PF=%ProgramFiles%"
set "VCVARS="

if not defined VCVARS if exist "%PF86%\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%PF86%\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if not defined VCVARS if exist "%PF86%\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%PF86%\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat"
if not defined VCVARS if exist "%PF86%\Microsoft Visual Studio\2019\Professional\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%PF86%\Microsoft Visual Studio\2019\Professional\VC\Auxiliary\Build\vcvars64.bat"
if not defined VCVARS if exist "%PF86%\Microsoft Visual Studio\2019\Enterprise\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%PF86%\Microsoft Visual Studio\2019\Enterprise\VC\Auxiliary\Build\vcvars64.bat"
if not defined VCVARS if exist "%PF%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%PF%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if not defined VCVARS if exist "%PF%\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%PF%\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if not defined VCVARS if exist "%PF%\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%PF%\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat"
if not defined VCVARS if exist "%PF%\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS=%PF%\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat"

if not defined VCVARS goto :no_msvc

echo        使用: %VCVARS%
call "%VCVARS%" >nul
if errorlevel 1 goto :vcvars_failed
echo        完成

:step_cargo
:: ---------------------------------------------------------------------------
:: 步骤 2/4  准备 Rust / Cargo
:: ---------------------------------------------------------------------------
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>nul
if errorlevel 1 goto :no_cargo
echo [2/4] cargo 就绪

:: ---------------------------------------------------------------------------
:: 步骤 3/4  构建前端
::   优先 npm ci 保证可复现；若 lock 与 package.json 不同步则回退 npm install
:: ---------------------------------------------------------------------------
where npm >nul 2>nul
if errorlevel 1 goto :no_npm
echo [3/4] 构建前端 ...

cd /d "%~dp0frontend"
call npm ci --no-audit --no-fund
if errorlevel 1 (
    echo        [WARN] npm ci 失败，package-lock.json 可能未同步，回退到 npm install
    call npm install --no-audit --no-fund
)
if errorlevel 1 goto :frontend_failed

call npm run build
if errorlevel 1 goto :frontend_failed
cd /d "%~dp0"
echo        前端产物: frontend\dist\

:: ---------------------------------------------------------------------------
:: 步骤 4/4  编译 Rust release
:: ---------------------------------------------------------------------------
echo [4/4] 编译 Rust release ...
echo        提示：首次全量编译约 10-20 分钟，之后增量编译很快
echo.

cd /d "%~dp0src-tauri"
cargo build --release
if errorlevel 1 goto :cargo_failed
cd /d "%~dp0"

echo.
echo ============================================================================
echo   [OK] 构建成功
echo ============================================================================
if not exist "src-tauri\target\release\rackviz.exe" goto :no_exe
for %%F in ("src-tauri\target\release\rackviz.exe") do echo   产物: %%~fF  [%%~zF 字节  %%~tF]

endlocal
exit /b 0

:: ===========================================================================
:: 错误分支
:: ===========================================================================
:no_msvc
echo [FAIL] 未找到 MSVC 编译环境 vcvars64.bat
echo        已尝试 VS2019 / VS2022 的 BuildTools / Community / Professional / Enterprise
echo        解决办法：从开始菜单打开 x64 Native Tools Command Prompt，
echo                  cd 到本目录后手动执行 cargo build --release
endlocal
exit /b 1

:vcvars_failed
echo [FAIL] vcvars64.bat 执行失败
endlocal
exit /b 1

:no_cargo
echo [FAIL] 未找到 cargo
echo        预期位置: %USERPROFILE%\.cargo\bin\cargo.exe
echo        请确认已安装 Rust，参见 https://rustup.rs
endlocal
exit /b 1

:no_npm
echo [FAIL] 未找到 npm，请确认已安装 Node.js
endlocal
exit /b 1

:frontend_failed
echo [FAIL] 前端构建失败
endlocal
exit /b 1

:cargo_failed
echo [FAIL] cargo build --release 失败
endlocal
exit /b 1

:no_exe
echo   [WARN] 未找到 rackviz.exe，请检查上方输出
endlocal
exit /b 0
