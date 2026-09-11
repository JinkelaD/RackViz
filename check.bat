@echo off
:: ============================================================================
::  RackViz 一键检查脚本  (check.bat)
::
::  用法：直接双击本文件，或在 cmd 中执行  check.bat
::  步骤：npm ci -> npx tsc --noEmit -> cargo check -> cargo test
::  任一步失败即以退出码 1 结束；全部通过打印 [OK]。
::
::  与 build.bat 相同，本脚本会自行定位并注入 MSVC 编译环境。
:: ============================================================================
setlocal
cd /d "%~dp0"

echo ============================================================================
echo   RackViz 代码检查
echo ============================================================================
echo.

:: ---------------------------------------------------------------------------
:: 步骤 1/4  准备 MSVC 编译环境
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
:: 步骤 2/4  前端类型检查
:: ---------------------------------------------------------------------------
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>nul
if errorlevel 1 goto :no_cargo

where npm >nul 2>nul
if errorlevel 1 goto :no_npm
echo [2/4] 前端类型检查 ...

cd /d "%~dp0frontend"
call npm ci --no-audit --no-fund
if errorlevel 1 (
    echo        [WARN] npm ci 失败，package-lock.json 可能未同步，回退到 npm install
    call npm install --no-audit --no-fund
)
if errorlevel 1 goto :frontend_failed

call npx tsc --noEmit
if errorlevel 1 goto :tsc_failed
cd /d "%~dp0"

:: ---------------------------------------------------------------------------
:: 步骤 3/4  Rust 编译检查
:: ---------------------------------------------------------------------------
echo [3/4] cargo check ...
cd /d "%~dp0src-tauri"
cargo check
if errorlevel 1 goto :check_failed

:: ---------------------------------------------------------------------------
:: 步骤 4/4  Rust 单元测试
:: ---------------------------------------------------------------------------
echo [4/4] cargo test ...
cargo test
if errorlevel 1 goto :test_failed
cd /d "%~dp0"

echo.
echo ============================================================================
echo   [OK] 全部检查通过
echo ============================================================================
endlocal
exit /b 0

:: ===========================================================================
:: 错误分支
:: ===========================================================================
:no_msvc
echo [FAIL] 未找到 MSVC 编译环境 vcvars64.bat
echo        已尝试 VS2019 / VS2022 的 BuildTools / Community / Professional / Enterprise
echo        解决办法：从开始菜单打开 x64 Native Tools Command Prompt，
echo                  cd 到本目录后手动执行 cargo check 与 cargo test
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
echo [FAIL] 前端依赖安装失败
endlocal
exit /b 1

:tsc_failed
echo [FAIL] tsc --noEmit 类型检查未通过
endlocal
exit /b 1

:check_failed
echo [FAIL] cargo check 未通过
endlocal
exit /b 1

:test_failed
echo [FAIL] cargo test 未通过
endlocal
exit /b 1
