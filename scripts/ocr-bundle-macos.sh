#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OCR_ROOT="$ROOT_DIR/src-tauri/resources/ocr"
BIN_DIR="$OCR_ROOT/macos"
LIB_DIR="$BIN_DIR/lib"
TESSDATA_DIR="$OCR_ROOT/tessdata"

mkdir -p "$BIN_DIR" "$LIB_DIR" "$TESSDATA_DIR"

if ! command -v tesseract >/dev/null 2>&1; then
  if command -v brew >/dev/null 2>&1; then
    echo "Installing Tesseract via Homebrew..."
    brew install tesseract
  else
    echo "Homebrew not found. Install tesseract manually and rerun."
    exit 1
  fi
fi

TESSERACT_BIN="$(command -v tesseract)"
cp "$TESSERACT_BIN" "$BIN_DIR/tesseract"
chmod +x "$BIN_DIR/tesseract"

if command -v brew >/dev/null 2>&1; then
  BREW_PREFIX="$(brew --prefix tesseract)"
  cp "$BREW_PREFIX/lib/"libtesseract*.dylib "$LIB_DIR/" || true
  cp "$BREW_PREFIX/lib/"liblept*.dylib "$LIB_DIR/" || true
fi

if command -v pdftoppm >/dev/null 2>&1; then
  cp "$(command -v pdftoppm)" "$BIN_DIR/pdftoppm"
  chmod +x "$BIN_DIR/pdftoppm"
elif command -v brew >/dev/null 2>&1; then
  echo "Installing poppler via Homebrew..."
  brew install poppler
  cp "$(command -v pdftoppm)" "$BIN_DIR/pdftoppm"
  chmod +x "$BIN_DIR/pdftoppm"
fi

curl -L "https://github.com/tesseract-ocr/tessdata_best/raw/main/eng.traineddata" -o "$TESSDATA_DIR/eng.traineddata"
curl -L "https://github.com/tesseract-ocr/tessdata_best/raw/main/ind.traineddata" -o "$TESSDATA_DIR/ind.traineddata"

echo "OCR bundle prepared in $OCR_ROOT"
