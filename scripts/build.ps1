[CmdletBinding()]
param(
    [ValidateSet('baseline', 'x86-64-v3', 'native')]
    [string]$CpuProfile = 'baseline',
    [switch]$SelfContained,
    [switch]$SkipChecks
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$Target = 'x86_64-pc-windows-msvc'
$EngineProject = Join-Path $Root 'engine\Smbn.Engine\Smbn.Engine.csproj'
$Artifacts = Join-Path $Root 'artifacts'
$Variant = if ($SelfContained) { "$CpuProfile-self-contained" } else { $CpuProfile }
$Package = Join-Path $Artifacts "package\smbn-$Variant"
$EngineOut = Join-Path $Artifacts "engine-$Variant"
$CargoTarget = Join-Path $Artifacts "cargo-$Variant"
$Zip = Join-Path $Artifacts "smbn-$Variant.zip"

switch ($CpuProfile) {
    'baseline' {
        $RustFlags = '-C target-cpu=x86-64'
        $CargoProfile = 'release'
    }
    'x86-64-v3' {
        $RustFlags = '-C target-cpu=x86-64-v3'
        $CargoProfile = 'release-amdv3'
    }
    'native' {
        $RustFlags = '-C target-cpu=native'
        $CargoProfile = 'release-amdv3'
    }
}

Push-Location $Root
$OldRustFlags = $env:RUSTFLAGS
$OldTargetDir = $env:CARGO_TARGET_DIR
try {
    if (-not $SkipChecks) {
        & (Join-Path $PSScriptRoot 'check.ps1')
    }

    Remove-Item $Package, $EngineOut -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item $Zip -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $Package, (Join-Path $Package 'assets'), (Join-Path $Package 'engine'), $EngineOut | Out-Null

    $env:RUSTFLAGS = $RustFlags
    $env:CARGO_TARGET_DIR = $CargoTarget

    Write-Host "==> Building Rust GUI ($CpuProfile)"
    & cargo build -p smbn-win --target $Target --profile $CargoProfile
    if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed' }

    $GuiExe = Join-Path $CargoTarget "$Target\$CargoProfile\smbn.exe"
    if (-not (Test-Path $GuiExe)) { throw "Missing GUI binary: $GuiExe" }
    Copy-Item $GuiExe (Join-Path $Package 'smbn.exe')

    $SelfContainedValue = if ($SelfContained) { 'true' } else { 'false' }
    Write-Host "==> Publishing .NET engine (self-contained: $SelfContainedValue)"
    & dotnet publish $EngineProject -c Release -r win-x64 --self-contained $SelfContainedValue -p:PublishSingleFile=false -p:DebugType=None -p:DebugSymbols=false -o $EngineOut
    if ($LASTEXITCODE -ne 0) { throw '.NET publish failed' }
    Copy-Item (Join-Path $EngineOut '*') (Join-Path $Package 'engine') -Recurse -Force

    foreach ($Name in @('README.md', 'README.zh-CN.md', 'HANDOFF.md', 'THIRD_PARTY_NOTICES.md', 'LICENSE-MIT', 'LICENSE-APACHE', 'LICENSE-LGPL-3.0', 'VERSION', 'CHANGELOG.md')) {
        Copy-Item (Join-Path $Root $Name) $Package
    }
    Copy-Item (Join-Path $Root 'docs') (Join-Path $Package 'docs') -Recurse
    Copy-Item (Join-Path $Root 'examples') (Join-Path $Package 'examples') -Recurse

    @(
        "variant=$Variant"
        "rustflags=$RustFlags"
        "target=$Target"
        "self_contained=$SelfContainedValue"
        "built_utc=$([DateTime]::UtcNow.ToString('O'))"
    ) | Set-Content -Encoding UTF8 (Join-Path $Package 'BUILD-INFO.txt')

    Write-Host '==> Verifying package structure'
    foreach ($Required in @(
        (Join-Path $Package 'smbn.exe'),
        (Join-Path $Package 'engine\Smbn.Engine.exe'),
        (Join-Path $Package 'README.zh-CN.md'),
        (Join-Path $Package 'THIRD_PARTY_NOTICES.md')
    )) {
        if (-not (Test-Path $Required)) { throw "Package is missing: $Required" }
    }

    Compress-Archive -Path (Join-Path $Package '*') -DestinationPath $Zip -CompressionLevel Optimal
    $Hash = (Get-FileHash -Algorithm SHA256 $Zip).Hash.ToLowerInvariant()
    "$Hash  $(Split-Path $Zip -Leaf)" | Set-Content -Encoding ASCII "$Zip.sha256"
    Write-Host "Package: $Zip"
    Write-Host "SHA256: $Hash"
}
finally {
    if ($null -eq $OldRustFlags) { Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue } else { $env:RUSTFLAGS = $OldRustFlags }
    if ($null -eq $OldTargetDir) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue } else { $env:CARGO_TARGET_DIR = $OldTargetDir }
    Pop-Location
}
