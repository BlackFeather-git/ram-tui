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

Write-Host "⚡ Installing ram-tui for Windows..." -ForegroundColor Cyan

Invoke-WebRequest -Uri "https://raw.githubusercontent.com/BlackFeather-git/ram-tui/main/ram" -OutFile $RamPath

# Create launcher shims
$CmdContent = "@echo off`r`npython `"$RamPath`" %*"
Set-Content -Path $CmdPath -Value $CmdContent -Force

$Ps1Content = "& python `"$RamPath`" `$args"
Set-Content -Path $Ps1Path -Value $Ps1Content -Force

Write-Host "✅ Installed executable and shims to $InstallDir" -ForegroundColor Green

# PATH check
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "⚠️  Adding $InstallDir to your User PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
}

Write-Host "🚀 Installation complete! Run 'ram' in PowerShell or Windows Terminal to launch." -ForegroundColor Green
