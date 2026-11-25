@echo off
@setlocal enableextensions

echo [1/2] Building ramreduct.exe...
cargo build --release
if %ERRORLEVEL% neq 0 goto end

if not exist "bin\64" mkdir "bin\64"
copy /Y "target\release\ramreduct.exe" "bin\64\ramreduct.exe" >nul

echo [2/2] Building ramreduct-3.5.3-setup.exe...
cargo build --release --manifest-path installer\Cargo.toml
if %ERRORLEVEL% neq 0 goto end

copy /Y "installer\target\release\ramreduct-setup.exe" "bin\64\ramreduct-3.5.3-setup.exe" >nul
echo Built bin\64\ramreduct.exe and bin\64\ramreduct-3.5.3-setup.exe successfully.

:end
