[CmdletBinding()]
param(
    [switch]$SkipRestore
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$EngineProject = Join-Path $Root 'engine\Smbn.Engine\Smbn.Engine.csproj'

Push-Location $Root
try {
    Write-Host '==> Rust formatting'
    & cargo fmt --all
    if ($LASTEXITCODE -ne 0) { throw 'cargo fmt failed' }

    Write-Host '==> Rust core tests'
    & cargo test -p smbn-core
    if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }

    Write-Host '==> Rust Windows GUI tests'
    & cargo test -p smbn-win --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'cargo test for smbn-win failed' }

    Write-Host '==> Rust Windows target checks'
    & cargo check --workspace --all-targets --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'cargo check failed' }

    Write-Host '==> Rust Clippy'
    & cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'cargo clippy failed' }

    if (-not $SkipRestore) {
        Write-Host '==> .NET restore'
        & dotnet restore $EngineProject
        if ($LASTEXITCODE -ne 0) { throw 'dotnet restore failed' }
    }

    Write-Host '==> .NET build'
    & dotnet build $EngineProject -c Release --no-restore -p:TreatWarningsAsErrors=true
    if ($LASTEXITCODE -ne 0) { throw 'dotnet build failed' }

    Write-Host '==> .NET engine IPC smoke test'
    $EngineExe = Join-Path $Root 'engine\Smbn.Engine\bin\Release\net8.0-windows\Smbn.Engine.exe'
    & (Join-Path $PSScriptRoot 'smoke-engine.ps1') -EnginePath $EngineExe

    Write-Host 'All checks passed.'
}
finally {
    Pop-Location
}
