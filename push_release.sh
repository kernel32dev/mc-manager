#!/bin/bash
cargo build --release &&
cp target/release/mc-manager.exe release/mc-manager.exe
git add release/mc-manager.exe
git cm "compiled release"
git push
