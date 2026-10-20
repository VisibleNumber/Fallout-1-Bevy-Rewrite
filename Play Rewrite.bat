@echo off
cd /d "%~dp0game"
if exist "..\..\Project Grove\bevy-church\target\debug\fallout-rewrite.exe" (
  start "Fallout Rewrite" "..\..\Project Grove\bevy-church\target\debug\fallout-rewrite.exe"
) else (
  cargo run --manifest-path Cargo.toml
)
