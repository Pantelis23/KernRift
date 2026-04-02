# KernRift Self-Hosted Compiler Installer for Windows
# Run: irm https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.ps1 | iex

$ErrorActionPreference = "Stop"
$Repo = "Pantelis23/KernRift"
$InstallDir = "$env:LOCALAPPDATA\KernRift\bin"

Write-Host "=== KernRift Self-Hosted Compiler Installer ==="

# Architecture detection
$arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
$ArchName = switch ($arch) {
    "AMD64" { "x86_64" }
    "ARM64" { "arm64" }
    default {
        Write-Host "error: unsupported architecture: $arch" -ForegroundColor Red
        exit 1
    }
}

Write-Host "Platform: windows $ArchName"
Write-Host "Install to: $InstallDir"
Write-Host ""

# Create install directory
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# Download from GitHub releases
$BinaryName = "krc-windows-$ArchName.exe"
$Url = "https://github.com/$Repo/releases/latest/download/$BinaryName"
$Dest = "$InstallDir\krc.exe"

Write-Host "Downloading $BinaryName..."
try {
    Invoke-WebRequest -Uri $Url -OutFile $Dest -UseBasicParsing
} catch {
    Write-Host "error: download failed: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "Alternatively, build from source:"
    Write-Host "  cargo install --git https://github.com/Pantelis23/KernRift-bootstrap kernriftc"
    Write-Host "  kernriftc --emit=hostexe build\krc.kr -o krc.exe"
    Write-Host "  Copy-Item krc.exe $InstallDir\krc.exe"
    exit 1
}

# Download kr runner
$KrBinaryName = "kr-windows-$ArchName.exe"
$KrUrl = "https://github.com/$Repo/releases/latest/download/$KrBinaryName"
$KrDest = "$InstallDir\kr.exe"
Write-Host "Downloading $KrBinaryName..."
try {
    Invoke-WebRequest -Uri $KrUrl -OutFile $KrDest -UseBasicParsing
} catch {
    Write-Host "warning: could not download kr runner: $_" -ForegroundColor Yellow
}

# Add to PATH if not already there
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$InstallDir;$UserPath", "User")
    Write-Host "Added $InstallDir to user PATH"
    Write-Host "Restart your terminal for PATH changes to take effect."
}

Write-Host ""
Write-Host "Installed: $Dest"
Write-Host "Installed: $KrDest"
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
