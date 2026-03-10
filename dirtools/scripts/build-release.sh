#!/usr/bin/env bash
# Mac と Windows 用のリリースビルドを作成する。
# 必要: rustup, Mac では x86_64-pc-windows-gnu ターゲット
#   rustup target add x86_64-pc-windows-gnu
set -e
cd "$(dirname "$0")/.."
mkdir -p dist

echo "==> クリーン（古いビルドを削除）..."
cargo clean

echo "==> Mac (ネイティブ) リリースビルド..."
cargo build --release
ARCH=$(uname -m)
cp "target/release/mdir" "dist/mdir-macos-${ARCH}"

echo "==> Windows (x86_64) クロスビルド..."
cargo build --release --target x86_64-pc-windows-gnu
cp "target/x86_64-pc-windows-gnu/release/mdir.exe" "dist/mdir-windows-x86_64.exe"

echo ""
echo "完了. dist/ に以下が出力されました:"
ls -la dist/
echo ""
echo "※ 実行時は必ずこのビルドのバイナリを指定すること（PATH の mdir は古い可能性あり）:"
echo "  ./target/release/mdir --version   # 0.1.1 と出れば改修版"
echo "  ./target/release/mdir"
echo "  または dist/mdir-macos-${ARCH}"
