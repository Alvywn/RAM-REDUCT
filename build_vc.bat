@echo off
@setlocal enableextensions

echo Building RAM Reduct (Rust Edition) x64 Release...
cargo build --release --target x86_64-pc-windows-msvc
if %ERRORLEVEL% neq 0 goto end

if not exist "bin\64" mkdir "bin\64"
copy /Y "target\x86_64-pc-windows-msvc\release\ramreduct.exe" "bin\64\ramreduct.exe" >nul
echo Done: bin\64\ramreduct.exe

:end
