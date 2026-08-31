[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PackageDirectory,
    [int]$StartupSeconds = 5
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$PackageDirectory = (Resolve-Path $PackageDirectory).Path
$GuiExe = Join-Path $PackageDirectory 'smbn.exe'
if (-not (Test-Path $GuiExe)) {
    throw "GUI executable not found: $GuiExe"
}

$Sandbox = Join-Path ([IO.Path]::GetTempPath()) "smbn-gui-smoke-$([Guid]::NewGuid().ToString('N'))"
$OldLocalAppData = $env:LOCALAPPDATA
$OldDotnetRoot = $env:DOTNET_ROOT
$OldDotnetRootX64 = $env:DOTNET_ROOT_X64
$OldMultilevelLookup = $env:DOTNET_MULTILEVEL_LOOKUP
$Process = $null
$StartedAt = Get-Date

function Write-SmokeDiagnostics {
    param([string]$Root, [DateTime]$Since)
    Write-Host '--- GUI smoke diagnostics ---'
    if (Test-Path $Root) {
        Get-ChildItem $Root -Recurse -File -ErrorAction SilentlyContinue |
            Where-Object { $_.Extension.ToLowerInvariant() -in @('.log', '.txt', '.json') } |
            ForEach-Object {
                Write-Host "### $($_.FullName)"
                try { Get-Content $_.FullName -ErrorAction Stop | Write-Host } catch { Write-Host "<unreadable: $($_.Exception.Message)>" }
            }
    }

    Start-Sleep -Milliseconds 750
    try {
        $Events = Get-WinEvent -FilterHashtable @{ LogName = 'Application'; StartTime = $Since } -ErrorAction Stop |
            Where-Object {
                ($_.ProviderName -eq 'Application Error' -or $_.ProviderName -eq 'Windows Error Reporting') -and
                $_.Message -match 'smbn\.exe'
            } |
            Select-Object -First 6
        foreach ($Event in $Events) {
            Write-Host "### Windows event $($Event.Id) / $($Event.ProviderName)"
            Write-Host $Event.Message
        }
    }
    catch {
        Write-Host "<unable to read Application crash events: $($_.Exception.Message)>"
    }
    Write-Host '--- end diagnostics ---'
}

try {
    New-Item -ItemType Directory -Force -Path $Sandbox | Out-Null
    $env:LOCALAPPDATA = $Sandbox

    # A self-contained release must not depend on a machine-wide .NET install.
    # Point the host lookup variables at an intentionally nonexistent directory;
    # the packaged Engine should still reach its IPC-ready state.
    $MissingDotnet = Join-Path $Sandbox 'no-global-dotnet'
    $env:DOTNET_ROOT = $MissingDotnet
    $env:DOTNET_ROOT_X64 = $MissingDotnet
    $env:DOTNET_MULTILEVEL_LOOKUP = '0'
    $StartedAt = Get-Date

    $Process = Start-Process -FilePath $GuiExe -WorkingDirectory $PackageDirectory -PassThru

    $Deadline = [DateTime]::UtcNow.AddSeconds([Math]::Max(2, $StartupSeconds))
    $EngineReady = $false
    $GuiTrace = Join-Path $Sandbox 'Smbn\logs\gui-bootstrap.log'
    while ([DateTime]::UtcNow -lt $Deadline) {
        Start-Sleep -Milliseconds 250
        $Process.Refresh()
        if ($Process.HasExited) {
            Write-SmokeDiagnostics $Sandbox $StartedAt
            throw "GUI exited during startup with code $($Process.ExitCode)."
        }

        if (Test-Path $GuiTrace) {
            $TraceText = Get-Content $GuiTrace -Raw -ErrorAction SilentlyContinue
            if ($TraceText -match '(?m)^stage=engine_launch_failed\s*$') {
                Write-SmokeDiagnostics $Sandbox $StartedAt
                throw 'GUI stayed alive, but the packaged SMB engine exited before reaching IPC readiness.'
            }
            if ($TraceText -match '(?m)^stage=engine_launch_complete\s*$') {
                $EngineReady = $true
            }
        }
    }

    if (-not $EngineReady) {
        Write-SmokeDiagnostics $Sandbox $StartedAt
        throw "GUI stayed alive for $StartupSeconds seconds, but the packaged SMB engine never reached IPC readiness."
    }

    $LogDirectory = Join-Path $Sandbox 'Smbn\logs'
    $BootstrapLog = Join-Path $LogDirectory 'engine-bootstrap.log'
    if (-not (Test-Path $BootstrapLog)) {
        Write-SmokeDiagnostics $Sandbox $StartedAt
        throw "Engine reached IPC readiness without creating its bootstrap log; expected: $BootstrapLog"
    }

    Write-Host "GUI startup smoke test passed; packaged engine reached IPC readiness and the GUI remained alive for $StartupSeconds seconds without a machine-wide .NET runtime."
}
finally {
    if ($null -ne $Process) {
        try {
            $Process.Refresh()
            if (-not $Process.HasExited) {
                Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
                $Process.WaitForExit(5000) | Out-Null
            }
        } catch { }
    }

    if ($null -eq $OldLocalAppData) {
        Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue
    } else {
        $env:LOCALAPPDATA = $OldLocalAppData
    }
    if ($null -eq $OldDotnetRoot) {
        Remove-Item Env:DOTNET_ROOT -ErrorAction SilentlyContinue
    } else {
        $env:DOTNET_ROOT = $OldDotnetRoot
    }
    if ($null -eq $OldDotnetRootX64) {
        Remove-Item Env:DOTNET_ROOT_X64 -ErrorAction SilentlyContinue
    } else {
        $env:DOTNET_ROOT_X64 = $OldDotnetRootX64
    }
    if ($null -eq $OldMultilevelLookup) {
        Remove-Item Env:DOTNET_MULTILEVEL_LOOKUP -ErrorAction SilentlyContinue
    } else {
        $env:DOTNET_MULTILEVEL_LOOKUP = $OldMultilevelLookup
    }
    Remove-Item $Sandbox -Recurse -Force -ErrorAction SilentlyContinue
}
