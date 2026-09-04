# ==============================================================================
# ram-tui Windows PowerShell installer
# ==============================================================================

$ErrorActionPreference = "Stop"

$InstallDir = "$HOME\.local\bin"
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$RamPath = Join-Path $InstallDir "ram.exe"

Write-Host "==> Welcome to RAM-TUI v1.0.3!" -ForegroundColor Cyan
Write-Host "==> Notice: RAM-TUI has officially transitioned from Python to a native Rust binary." -ForegroundColor Yellow
Write-Host "==> Installing ram-tui for Windows to $InstallDir..." -ForegroundColor Cyan

# Clean up legacy Python v0.x scripts and shims if present
$LegacyFiles = @("ram.py", "ram.cmd", "ram.ps1", "ram-tui.py", "ram-tui.cmd", "ram-tui.ps1")
foreach ($file in $LegacyFiles) {
    $target = Join-Path $InstallDir $file
    if (Test-Path $target) {
        Remove-Item -Force $target -ErrorAction SilentlyContinue
    }
}

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "Building native binary via Cargo..." -ForegroundColor Green
    cargo build --release -p cli
    Copy-Item "target\release\ram.exe" -Destination $RamPath -Force
    Write-Host "Successfully installed 'ram' to $InstallDir" -ForegroundColor Green
} else {
    Write-Host "Downloading precompiled release binary from GitHub..." -ForegroundColor Green
    $Uri = "https://github.com/BlackFeather-git/ram-tui/releases/latest/download/ram-windows-x86_64.exe"
    $ShaUri = "$Uri.sha256"
    $TmpBin = [System.IO.Path]::GetTempFileName()
    $TmpSha = [System.IO.Path]::GetTempFileName()
    try {
        Invoke-WebRequest -Uri $Uri -OutFile $TmpBin -UseBasicParsing
        $fileSize = (Get-Item $TmpBin).Length
        if ($fileSize -lt 100000) {
            throw "Downloaded binary is invalid or incomplete (size: $fileSize bytes)."
        }

        # Cryptographic SHA-256 integrity verification (fail-closed)
        Write-Host "==> Verifying cryptographic SHA-256 checksum..." -ForegroundColor Cyan
        Invoke-WebRequest -Uri $ShaUri -OutFile $TmpSha -UseBasicParsing
        $ExpectedHash = (Get-Content $TmpSha).Trim().Split()[0].ToUpper()

        if ($ExpectedHash.Length -ne 64) {
            throw "Cryptographic SHA-256 checksum asset is invalid or malformed."
        }

        $ActualHash = (Get-FileHash -Path $TmpBin -Algorithm SHA256).Hash.ToUpper()
        if ($ActualHash -ne $ExpectedHash) {
            throw "Cryptographic SHA-256 checksum mismatch (Expected: $ExpectedHash, Got: $ActualHash)."
        }
        Write-Host "==> Cryptographic integrity verified: SHA-256 ($($ActualHash.Substring(0, 16))...)" -ForegroundColor Green

        Move-Item -Path $TmpBin -Destination $RamPath -Force
        Write-Host "Downloaded and verified 'ram.exe' to $InstallDir" -ForegroundColor Green
    } catch {
        Write-Host "Error: Failed to download precompiled release binary: $_" -ForegroundColor Red
        Write-Host "Please download ram-windows-x86_64.exe directly from https://github.com/BlackFeather-git/ram-tui/releases/latest" -ForegroundColor Yellow
        exit 1
    } finally {
        Remove-Item -Force $TmpBin -ErrorAction SilentlyContinue
        Remove-Item -Force $TmpSha -ErrorAction SilentlyContinue
    }
}

$RamTuiPath = Join-Path $InstallDir "ram-tui.exe"
Copy-Item $RamPath -Destination $RamTuiPath -Force

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "Notice: Adding $InstallDir to your User PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$UserPath", "User")
}
if ($env:Path -notlike "*$InstallDir*") {
    $env:Path = "$InstallDir;$env:Path"
}

Write-Host "Installation complete. Run 'ram' in PowerShell or Windows Terminal to launch." -ForegroundColor Green
