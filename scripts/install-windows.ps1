#Requires -Version 5.1
<#
.SYNOPSIS
    Installs kernriftc and kernrift on Windows.
.DESCRIPTION
    Downloads and runs the official rustup installer if Rust is not present,
    then clones the KernRift repository and installs the binaries via cargo.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File install-windows.ps1
#>

$ErrorActionPreference = 'Stop'
$Repo = 'https://github.com/Pantelis23/KernRift'

function Write-Info  ($msg) { Write-Host "=> $msg" -ForegroundColor Cyan }
function Write-Ok    ($msg) { Write-Host "v  $msg" -ForegroundColor Green }
function Write-Warn  ($msg) { Write-Host "!  $msg" -ForegroundColor Yellow }

# ── 1. check for conflicting system Rust ──────────────────────────────────────
$sysRustc = Get-Command rustc -ErrorAction SilentlyContinue
if ($sysRustc -and $sysRustc.Source -notlike '*\.cargo\*') {
    Write-Warn "Rust found outside of .cargo\bin: $($sysRustc.Source)"
    Write-Warn "This may be a system-managed Rust that is too old for KernRift."
    Write-Warn "Uninstall it and re-run this script, or install via rustup manually."
    exit 1
}

# ── 2. install rustup if missing ──────────────────────────────────────────────
$cargoEnv = "$env:USERPROFILE\.cargo\env.ps1"
$rustupExe = "$env:USERPROFILE\.cargo\bin\rustup.exe"

if (-not (Test-Path $rustupExe)) {
    Write-Info 'Downloading rustup-init.exe...'
    $rustupInit = "$env:TEMP\rustup-init.exe"
    $rustupUrl  = 'https://win.rustup.rs/x86_64'

    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupInit -UseBasicParsing

    Write-Info 'Running rustup-init (minimal profile, no default toolchain)...'
    & $rustupInit -y --profile minimal --default-toolchain none --no-modify-path
    if ($LASTEXITCODE -ne 0) { Write-Error 'rustup-init failed.'; exit 1 }
} else {
    Write-Ok "rustup already installed: $(& $rustupExe --version 2>&1)"
}

# put cargo/bin on PATH for the rest of this session
$cargoBin = "$env:USERPROFILE\.cargo\bin"
if ($env:PATH -notlike "*$cargoBin*") {
    $env:PATH = "$cargoBin;$env:PATH"
}

# ── 3. clone and install ──────────────────────────────────────────────────────
Write-Info "Installing kernriftc and kernrift from $Repo ..."
$tmpDir = Join-Path $env:TEMP "kernrift-install-$([System.IO.Path]::GetRandomFileName())"
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
    git clone --depth=1 $Repo "$tmpDir\kernrift"
    if ($LASTEXITCODE -ne 0) { Write-Error 'git clone failed.'; exit 1 }

    cargo install --path "$tmpDir\kernrift\crates\kernriftc" --locked
    if ($LASTEXITCODE -ne 0) { Write-Error 'cargo install kernriftc failed.'; exit 1 }

    cargo install --path "$tmpDir\kernrift\crates\kernrift" --locked
    if ($LASTEXITCODE -ne 0) { Write-Error 'cargo install kernrift failed.'; exit 1 }
} finally {
    Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
}

# ── 4. verify ─────────────────────────────────────────────────────────────────
Write-Host ''
foreach ($bin in @('kernriftc', 'kernrift')) {
    $found = Get-Command $bin -ErrorAction SilentlyContinue
    if ($found) {
        Write-Ok "$bin -> $($found.Source)"
    } else {
        Write-Warn "$bin not found on PATH."
        Write-Warn "Add $cargoBin to your PATH, then open a new terminal."
    }
}

Write-Host ''
Write-Ok 'Done! Try: kernriftc --help'
