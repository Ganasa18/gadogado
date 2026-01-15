#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OCR_ROOT="$ROOT_DIR/src-tauri/resources/ocr"
BIN_DIR="$OCR_ROOT/linux"
LIB_DIR="$BIN_DIR/lib"
TESSDATA_DIR="$OCR_ROOT/tessdata"

mkdir -p "$BIN_DIR" "$LIB_DIR" "$TESSDATA_DIR"

if ! command -v tesseract >/dev/null 2>&1; then
  if command -v apt-get >/dev/null 2>&1; then
    echo "Installing Tesseract via apt..."
    sudo apt-get update
    sudo apt-get install -y tesseract-ocr
  elif command -v dnf >/dev/null 2>&1; then
    echo "Installing Tesseract via dnf..."
    sudo dnf install -y tesseract
  elif command -v pacman >/dev/null 2>&1; then
    echo "Installing Tesseract via pacman..."
    sudo pacman -Sy --noconfirm tesseract
  else
    echo "Package manager not found. Install tesseract manually and rerun."
    exit 1
  fi
fi

TESSERACT_BIN="$(command -v tesseract)"
cp "$TESSERACT_BIN" "$BIN_DIR/tesseract"
chmod +x "$BIN_DIR/tesseract"

if command -v ldconfig >/dev/null 2>&1; then
  LIB_TESS=$(ldconfig -p | awk '/libtesseract\.so/{print $NF; exit}')
  LIB_LEPT=$(ldconfig -p | awk '/liblept\.so/{print $NF; exit}')
  if [ -n "$LIB_TESS" ]; then
    cp "$LIB_TESS" "$LIB_DIR/"
  fi
  if [ -n "$LIB_LEPT" ]; then
    cp "$LIB_LEPT" "$LIB_DIR/"
  fi
fi

if ! command -v pdftoppm >/dev/null 2>&1; then
  if command -v apt-get >/dev/null 2>&1; then
    echo "Installing poppler via apt..."
    sudo apt-get install -y poppler-utils
  elif command -v dnf >/dev/null 2>&1; then
    echo "Installing poppler via dnf..."
    sudo dnf install -y poppler-utils
  elif command -v pacman >/dev/null 2>&1; then
    echo "Installing poppler via pacman..."
    sudo pacman -Sy --noconfirm poppler
  fi
fi

if command -v pdftoppm >/dev/null 2>&1; then
  cp "$(command -v pdftoppm)" "$BIN_DIR/pdftoppm"
  chmod +x "$BIN_DIR/pdftoppm"
fi

curl -L "https://github.com/tesseract-ocr/tessdata_best/raw/main/eng.traineddata" -o "$TESSDATA_DIR/eng.traineddata"
curl -L "https://github.com/tesseract-ocr/tessdata_best/raw/main/ind.traineddata" -o "$TESSDATA_DIR/ind.traineddata"

echo "OCR bundle prepared in $OCR_ROOT"
