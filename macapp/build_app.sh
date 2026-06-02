#!/usr/bin/env bash
#
# build_app.sh - mdir を macOS ネイティブアプリ (.app) としてビルドする
#
# 手順:
#   1. Rust 製 TUI 本体 (dirtools) を release ビルド
#   2. SwiftUI ラッパー (MdirMac) を SwiftPM で release ビルド
#   3. Mdir.app バンドルを組み立て、mdir バイナリを同梱
#   4. （署名鍵があれば）ad-hoc コード署名
#
# 使い方:
#   ./build_app.sh            # ビルドして macapp/dist/mdir.app を生成
#   ./build_app.sh --open     # ビルド後にアプリを起動
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_DIR="${REPO_ROOT}/dirtools"
DIST_DIR="${SCRIPT_DIR}/dist"
APP_NAME="mdir"
APP_BUNDLE="${DIST_DIR}/${APP_NAME}.app"

OPEN_AFTER_BUILD=false
for arg in "$@"; do
    case "$arg" in
        --open) OPEN_AFTER_BUILD=true ;;
        *) echo "unknown option: $arg" >&2; exit 1 ;;
    esac
done

echo "==> [1/4] Rust TUI 本体を release ビルド"
# CARGO_TARGET_DIR が外部（CI やサンドボックス等）で設定されていると
# 成果物が想定外の場所に出力され、古いバイナリを同梱してしまう。
# ここでは出力先を必ずリポジトリ内 target/ に固定する。
( cd "${RUST_DIR}" && CARGO_TARGET_DIR="${RUST_DIR}/target" cargo build --release )
MDIR_BIN="${RUST_DIR}/target/release/mdir"
if [[ ! -x "${MDIR_BIN}" ]]; then
    echo "error: mdir バイナリが見つかりません: ${MDIR_BIN}" >&2
    exit 1
fi

echo "==> [2/4] SwiftUI ラッパーを release ビルド"
( cd "${SCRIPT_DIR}" && swift build -c release )
SWIFT_BIN="${SCRIPT_DIR}/.build/release/MdirMac"
if [[ ! -x "${SWIFT_BIN}" ]]; then
    echo "error: MdirMac バイナリが見つかりません: ${SWIFT_BIN}" >&2
    exit 1
fi

echo "==> [3/4] ${APP_NAME}.app バンドルを組み立て"
rm -rf "${APP_BUNDLE}"
mkdir -p "${APP_BUNDLE}/Contents/MacOS"
mkdir -p "${APP_BUNDLE}/Contents/Resources"

cp "${SCRIPT_DIR}/Info.plist" "${APP_BUNDLE}/Contents/Info.plist"
cp "${SWIFT_BIN}" "${APP_BUNDLE}/Contents/MacOS/MdirMac"
cp "${MDIR_BIN}" "${APP_BUNDLE}/Contents/Resources/mdir"
chmod +x "${APP_BUNDLE}/Contents/MacOS/MdirMac" "${APP_BUNDLE}/Contents/Resources/mdir"

# アイコンがあれば同梱（任意）
if [[ -f "${SCRIPT_DIR}/AppIcon.icns" ]]; then
    cp "${SCRIPT_DIR}/AppIcon.icns" "${APP_BUNDLE}/Contents/Resources/AppIcon.icns"
fi

echo "==> [4/4] コード署名 (ad-hoc)"
# 個人利用・友人配布向けの ad-hoc 署名。Developer ID があれば CODESIGN_IDENTITY で上書き可。
CODESIGN_IDENTITY="${CODESIGN_IDENTITY:--}"
codesign --force --deep --sign "${CODESIGN_IDENTITY}" "${APP_BUNDLE}" 2>/dev/null \
    && echo "    signed with identity: ${CODESIGN_IDENTITY}" \
    || echo "    署名をスキップしました（codesign が使えない環境）"

echo ""
echo "完成: ${APP_BUNDLE}"
echo "起動:  open \"${APP_BUNDLE}\""

if [[ "${OPEN_AFTER_BUILD}" == true ]]; then
    open "${APP_BUNDLE}"
fi
