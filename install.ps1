# ==============================================================================
# ram-tui Windows PowerShell installer
# ==============================================================================

$ErrorActionPreference = "Stop"

$InstallDir = "$HOME\.local\bin"
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$RamPath = Join-Path $InstallDir "ram.exe"

Write-Host "==> Welcome to RAM-TUI v1.0.1!" -ForegroundColor Cyan
Write-Host "==> Notice: RAM-TUI has officially transitioned from Python to a native Rust binary." -ForegroundColor Yellow
Write-Host "==> Installing ram-tui for Windows to $InstallDir..." -ForegroundColor Cyan

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "Building native binary via Cargo..." -ForegroundColor Green
    cargo build --release -p cli
    Copy-Item "target\release\ram.exe" -Destination $RamPath -Force
    Write-Host "Successfully installed 'ram' to $InstallDir" -ForegroundColor Green
} else {
    Write-Host "Downloading precompiled release binary from GitHub..." -ForegroundColor Green
    $Uri = "https://github.com/BlackFeather-git/ram-tui/releases/latest/download/ram-windows-x86_64.exe"
    try {
        Invoke-WebRequest -Uri $Uri -OutFile $RamPath -UseBasicParsing
        $fileSize = (Get-Item $RamPath).Length
        if ($fileSize -lt 100000) {
            Remove-Item -Force $RamPath -ErrorAction SilentlyContinue
            throw "Downloaded binary is invalid or incomplete (size: $fileSize bytes)."
        }
        Write-Host "Downloaded 'ram.exe' to $InstallDir" -ForegroundColor Green
    } catch {
        Write-Host "Error: Failed to download precompiled release binary: $_" -ForegroundColor Red
        Write-Host "Please download ram-windows-x86_64.exe directly from https://github.com/BlackFeather-git/ram-tui/releases/latest" -ForegroundColor Yellow
        exit 1
    }
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "Notice: Adding $InstallDir to your User PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
}

Write-Host "Installation complete. Run 'ram' in PowerShell or Windows Terminal to launch." -ForegroundColor Green
