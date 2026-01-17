$ErrorActionPreference = "Stop"

$root = Resolve-Path "$PSScriptRoot/.."
$ocrRoot = Join-Path $root "src-tauri/resources/ocr"
$binDir = Join-Path $ocrRoot "windows"
$tessdataDir = Join-Path $ocrRoot "tessdata"

New-Item -ItemType Directory -Force -Path $binDir, $tessdataDir | Out-Null

function Download-File($url, $dest) {
  Write-Host "Downloading $url"
  Invoke-WebRequest -Uri $url -OutFile $dest
}

function Resolve-TesseractPath {
  if ($env:TESSERACT_PATH -and (Test-Path $env:TESSERACT_PATH)) {
    return $env:TESSERACT_PATH
  }

  $cmd = Get-Command tesseract -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }

  $paths = @(
    "${env:ProgramFiles}\Tesseract-OCR\tesseract.exe",
    "${env:ProgramFiles(x86)}\Tesseract-OCR\tesseract.exe"
  )

  foreach ($path in $paths) {
    if (Test-Path $path) { return $path }
  }

  $regPaths = @(
    "HKLM:\SOFTWARE\Tesseract-OCR",
    "HKLM:\SOFTWARE\WOW6432Node\Tesseract-OCR"
  )

  foreach ($regPath in $regPaths) {
    if (Test-Path $regPath) {
      $install = (Get-ItemProperty $regPath -ErrorAction SilentlyContinue).InstallDir
      if ($install) {
        $candidate = Join-Path $install "tesseract.exe"
        if (Test-Path $candidate) { return $candidate }
      }
    }
  }

  return $null
}

$tesseractPath = Resolve-TesseractPath
if (-not $tesseractPath) {
  if (Get-Command winget -ErrorAction SilentlyContinue) {
    Write-Host "Installing Tesseract via winget..."
    winget install --id "UB-Mannheim.TesseractOCR" -e --accept-package-agreements --accept-source-agreements
    $tesseractPath = Resolve-TesseractPath
  }
}

if (-not $tesseractPath) {
  throw "Tesseract not found. Install it manually, then rerun this script."
}

$installDir = Split-Path -Parent $tesseractPath
Copy-Item "$installDir\*" $binDir -Recurse -Force

function Resolve-PdftoppmPath {
  if ($env:PDFTOPPM_PATH -and (Test-Path $env:PDFTOPPM_PATH)) {
    return $env:PDFTOPPM_PATH
  }
  $cmd = Get-Command pdftoppm -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }
  return $null
}

$pdftoppmPath = Resolve-PdftoppmPath
if (-not $pdftoppmPath) {
  Write-Host "Downloading Poppler (pdftoppm)..."
  $popplerUrl = "https://github.com/oschwartz10612/poppler-windows/releases/download/v25.12.0-0/Release-25.12.0-0.zip"
  $zipPath = Join-Path $env:TEMP "poppler.zip"
  $extractPath = Join-Path $env:TEMP "poppler"
  if (Test-Path $extractPath) { Remove-Item $extractPath -Recurse -Force }
  Download-File $popplerUrl $zipPath
  Expand-Archive -Force -Path $zipPath -DestinationPath $extractPath
  $popplerRoot = Get-ChildItem $extractPath -Directory | Select-Object -First 1
  if ($popplerRoot) {
    $popplerBin = Join-Path $popplerRoot.FullName "Library\bin"
    $candidate = Join-Path $popplerBin "pdftoppm.exe"
    if (Test-Path $candidate) {
      Copy-Item "$popplerBin\*.dll" $binDir -Force
      Copy-Item $candidate $binDir -Force
      $pdftoppmPath = Join-Path $binDir "pdftoppm.exe"
    }
  }
}

if ($pdftoppmPath) {
  Write-Host "pdftoppm ready at $pdftoppmPath"
} else {
  Write-Host "pdftoppm not found; OCR on scanned PDFs may fail."
}

$engUrl = "https://github.com/tesseract-ocr/tessdata_best/raw/main/eng.traineddata"
$indUrl = "https://github.com/tesseract-ocr/tessdata_best/raw/main/ind.traineddata"
Download-File $engUrl (Join-Path $tessdataDir "eng.traineddata")
Download-File $indUrl (Join-Path $tessdataDir "ind.traineddata")

Write-Host "OCR bundle prepared in $ocrRoot"
