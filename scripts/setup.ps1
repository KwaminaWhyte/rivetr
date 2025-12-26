# Rivetr Setup Script for Windows
# This script sets up everything needed to run Rivetr

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Rivetr Setup Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if running as administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

# Function to check if a command exists
function Test-Command {
    param($Command)
    try {
        Get-Command $Command -ErrorAction Stop | Out-Null
        return $true
    } catch {
        return $false
    }
}

# Check prerequisites
Write-Host "[1/5] Checking prerequisites..." -ForegroundColor Yellow

# Check for Rust
if (Test-Command "cargo") {
    $rustVersion = (rustc --version)
    Write-Host "  ✓ Rust is installed: $rustVersion" -ForegroundColor Green
} else {
    Write-Host "  ✗ Rust is not installed" -ForegroundColor Red
    Write-Host "    Please install Rust from https://rustup.rs" -ForegroundColor Red
    Write-Host "    Run: winget install Rustlang.Rustup" -ForegroundColor Yellow
    exit 1
}

# Check for Git
if (Test-Command "git") {
    $gitVersion = (git --version)
    Write-Host "  ✓ Git is installed: $gitVersion" -ForegroundColor Green
} else {
    Write-Host "  ✗ Git is not installed" -ForegroundColor Red
    Write-Host "    Please install Git from https://git-scm.com" -ForegroundColor Red
    Write-Host "    Run: winget install Git.Git" -ForegroundColor Yellow
    exit 1
}

# Check for Docker or Podman
$hasDocker = Test-Command "docker"
$hasPodman = Test-Command "podman"

if ($hasDocker) {
    Write-Host "  ✓ Docker is installed" -ForegroundColor Green
    # Check if Docker daemon is running
    try {
        docker info 2>&1 | Out-Null
        Write-Host "  ✓ Docker daemon is running" -ForegroundColor Green
    } catch {
        Write-Host "  ⚠ Docker is installed but not running" -ForegroundColor Yellow
        Write-Host "    Please start Docker Desktop" -ForegroundColor Yellow
    }
} elseif ($hasPodman) {
    Write-Host "  ✓ Podman is installed" -ForegroundColor Green
} else {
    Write-Host "  ⚠ No container runtime found (Docker or Podman)" -ForegroundColor Yellow
    Write-Host "    Rivetr will start but deployments won't work" -ForegroundColor Yellow
    Write-Host "    Install Docker Desktop: https://www.docker.com/products/docker-desktop" -ForegroundColor Yellow
    Write-Host "    Or install Podman: winget install RedHat.Podman" -ForegroundColor Yellow
}

Write-Host ""

# Create data directory
Write-Host "[2/5] Creating data directory..." -ForegroundColor Yellow
$dataDir = Join-Path $PSScriptRoot "..\data"
if (-not (Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
    Write-Host "  ✓ Created data directory: $dataDir" -ForegroundColor Green
} else {
    Write-Host "  ✓ Data directory already exists" -ForegroundColor Green
}

Write-Host ""

# Create config file if not exists
Write-Host "[3/5] Setting up configuration..." -ForegroundColor Yellow
$configFile = Join-Path $PSScriptRoot "..\rivetr.toml"
$exampleConfig = Join-Path $PSScriptRoot "..\rivetr.example.toml"

if (-not (Test-Path $configFile)) {
    if (Test-Path $exampleConfig) {
        Copy-Item $exampleConfig $configFile
        Write-Host "  ✓ Created rivetr.toml from example config" -ForegroundColor Green
        Write-Host "  ⚠ Please edit rivetr.toml to customize settings" -ForegroundColor Yellow
    } else {
        Write-Host "  ✗ Example config not found" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "  ✓ Configuration file already exists" -ForegroundColor Green
}

Write-Host ""

# Build the project
Write-Host "[4/5] Building Rivetr..." -ForegroundColor Yellow
$projectRoot = Join-Path $PSScriptRoot ".."
Push-Location $projectRoot

try {
    Write-Host "  Building in release mode (this may take a few minutes)..." -ForegroundColor Cyan
    cargo build --release 2>&1 | ForEach-Object { Write-Host "  $_" }

    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ Build successful" -ForegroundColor Green
    } else {
        Write-Host "  ✗ Build failed" -ForegroundColor Red
        exit 1
    }
} finally {
    Pop-Location
}

Write-Host ""

# Print success message
Write-Host "[5/5] Setup complete!" -ForegroundColor Yellow
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Rivetr is ready to use!" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "To start Rivetr:" -ForegroundColor White
Write-Host "  .\target\release\rivetr.exe --config rivetr.toml" -ForegroundColor Yellow
Write-Host ""
Write-Host "Or for development:" -ForegroundColor White
Write-Host "  cargo run -- --config rivetr.example.toml" -ForegroundColor Yellow
Write-Host ""
Write-Host "Then open http://localhost:8080 in your browser" -ForegroundColor White
Write-Host "You'll be prompted to create your admin account on first visit." -ForegroundColor White
Write-Host ""

# Ask if user wants to start now
$response = Read-Host "Would you like to start Rivetr now? (y/n)"
if ($response -eq "y" -or $response -eq "Y") {
    Write-Host ""
    Write-Host "Starting Rivetr..." -ForegroundColor Cyan
    $exePath = Join-Path $projectRoot "target\release\rivetr.exe"
    & $exePath --config (Join-Path $projectRoot "rivetr.toml")
}
