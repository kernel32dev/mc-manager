cd ..
cargo build --release
if %errorlevel% neq 0 exit /b %errorlevel%
copy target\release\mc-manager.exe release\mc-manager.exe
git restore --staged .
git add release/mc-manager.exe
git cm "compiled release"
git push
echo "pushed release successfully"
pause
