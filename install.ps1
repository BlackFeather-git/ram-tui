# ==============================================================================
# ram-tui Windows PowerShell installer
# Usage in PowerShell:
#   irm https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main/install.ps1 | iex
# ==============================================================================

$ErrorActionPreference = "Stop"

$InstallDir = "$HOME\.local\bin"
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$RamPath = Join-Path $InstallDir "ram.py"
$CmdPath = Join-Path $InstallDir "ram.cmd"
$Ps1Path = Join-Path $InstallDir "ram.ps1"

Write-Host "==> Installing ram-tui for Windows..." -ForegroundColor Cyan

# 1. Download binary and cryptographic assets
$BaseUri = "https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main"
$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
$TempRam = Join-Path $TempDir "ram"
$TempSig = Join-Path $TempDir "ram.sig"

try {
    Invoke-WebRequest -Uri "$BaseUri/ram" -OutFile $TempRam
    $HashRaw = (Invoke-RestMethod -Uri "$BaseUri/ram.sha256").Trim()
    Invoke-WebRequest -Uri "$BaseUri/ram.sig" -OutFile $TempSig

    # 2. Mandatory SHA-256 Integrity Verification
    $ExpectedHash = $HashRaw.Split()[0].ToUpper()
    if ($ExpectedHash.Length -ne 64) {
        throw "Malformed cryptographic SHA-256 checksum asset."
    }
    $ActualHash = (Get-FileHash -Path $TempRam -Algorithm SHA256).Hash.ToUpper()
    if ($ActualHash -ne $ExpectedHash) {
        throw "Cryptographic SHA-256 integrity verification failed (Expected: $ExpectedHash, Got: $ActualHash)."
    }
    Write-Host "-> Integrity verified: SHA-256 ($($ActualHash.Substring(0, 16))...)" -ForegroundColor Green

    # 3. Mandatory RSA-2048 Signature Verification
    $SigCheckCode = python -c "
import importlib.machinery, importlib.util, sys
try:
    loader = importlib.machinery.SourceFileLoader('ram_mod', r'$TempRam')
    spec = importlib.util.spec_from_loader('ram_mod', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    with open(r'$TempRam', 'rb') as f: data = f.read()
    with open(r'$TempSig', 'r', encoding='utf-8') as f: sig = f.read().strip()
    sys.exit(0 if m.verify_release_signature(data, sig) else 1)
except Exception:
    sys.exit(2)
"
    if ($LASTEXITCODE -ne 0) {
        throw "Maintainer RSA-2048 cryptographic signature verification failed (code: $LASTEXITCODE)."
    }
    Write-Host "-> Signature verified: RSA-2048 PKCS#1 v1.5 (Maintainer Root of Trust)" -ForegroundColor Green

    Move-Item -Path $TempRam -Destination $RamPath -Force
} finally {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}

# Create launcher shims
$CmdContent = "@echo off`r`npython `"$RamPath`" %*"
Set-Content -Path $CmdPath -Value $CmdContent -Force

$Ps1Content = "& python `"$RamPath`" `$args"
Set-Content -Path $Ps1Path -Value $Ps1Content -Force

Write-Host "-> Installed executable and shims to $InstallDir" -ForegroundColor Green

# PATH check
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "Notice: Adding $InstallDir to your User PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
}

Write-Host "Installation complete. Run 'ram' in PowerShell or Windows Terminal to launch." -ForegroundColor Green
