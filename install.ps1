# KernRift Self-Hosted Compiler Installer for Windows
# Run: powershell -ExecutionPolicy Bypass -File install.ps1

$ErrorActionPreference = "Stop"
$Version = "1.0.0"
$InstallDir = "$env:LOCALAPPDATA\KernRift\bin"

Write-Host "=== KernRift Self-Hosted Compiler Installer ==="
Write-Host "Version: $Version"
Write-Host "Platform: Windows x86_64"
Write-Host "Install to: $InstallDir"
Write-Host ""

# Architecture check
$arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
if ($arch -ne "AMD64" -and $arch -ne "ARM64") {
    Write-Host "error: unsupported architecture: $arch" -ForegroundColor Red
    exit 1
}

# Create install directory
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# Check for prebuilt binary
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Binary = ""

if (Test-Path "$ScriptDir\dist\krc-windows-x86_64.exe") {
    $Binary = "$ScriptDir\dist\krc-windows-x86_64.exe"
}

if ($Binary) {
    Write-Host "Found prebuilt binary: $Binary"
    Copy-Item $Binary "$InstallDir\krc.exe"
} else {
    Write-Host "No prebuilt Windows binary found."
    Write-Host ""
    Write-Host "To install on Windows:"
    Write-Host "  1. Install the Rust KernRift compiler:"
    Write-Host "     cargo install --git https://github.com/Pantelis23/KernRift kernriftc"
    Write-Host "  2. Build the self-hosted compiler:"
    Write-Host "     kernriftc --emit=hostexe build\krc.kr -o krc.exe"
    Write-Host "  3. Copy krc.exe to $InstallDir"
    Write-Host ""
    Write-Host "Or download a prebuilt binary from GitHub Releases."
    exit 1
}

# Add to PATH if not already there
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$InstallDir;$UserPath", "User")
    Write-Host "Added $InstallDir to user PATH"
    Write-Host "Restart your terminal for PATH changes to take effect."
}

Write-Host ""
Write-Host "Installed: $InstallDir\krc.exe"
Write-Host ""
Write-Host "Usage:"
Write-Host "  krc program.kr                  # compile to fat binary"
Write-Host "  krc --arch=x86_64 prog.kr      # compile for x86_64"
Write-Host "  krc -o output.exe prog.kr      # specify output"
Write-Host "  krc check prog.kr              # run analysis"
Write-Host "  krc lc prog.kr                 # living compiler report"
Write-Host "  krc --version                  # show version"
Write-Host ""
Write-Host "=== Installation complete ==="
