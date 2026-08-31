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
$Process = $null

function Write-SmokeDiagnostics {
    param([string]$Root)
    Write-Host '--- GUI smoke diagnostics ---'
    if (Test-Path $Root) {
        Get-ChildItem $Root -Recurse -File -ErrorAction SilentlyContinue |
            Where-Object { $_.Extension.ToLowerInvariant() -in @('.log', '.txt', '.json') } |
            ForEach-Object {
                Write-Host "### $($_.FullName)"
                try { Get-Content $_.FullName -ErrorAction Stop | Write-Host } catch { Write-Host "<unreadable: $($_.Exception.Message)>" }
            }
    }
    Write-Host '--- end diagnostics ---'
}

try {
    New-Item -ItemType Directory -Force -Path $Sandbox | Out-Null
    $env:LOCALAPPDATA = $Sandbox

    $Process = Start-Process -FilePath $GuiExe -WorkingDirectory $PackageDirectory -PassThru

    $Deadline = [DateTime]::UtcNow.AddSeconds([Math]::Max(2, $StartupSeconds))
    while ([DateTime]::UtcNow -lt $Deadline) {
        Start-Sleep -Milliseconds 250
        $Process.Refresh()
        if ($Process.HasExited) {
            Write-SmokeDiagnostics $Sandbox
            throw "GUI exited during startup with code $($Process.ExitCode)."
        }
    }

    $LogDirectory = Join-Path $Sandbox 'Smbn\logs'
    $BootstrapLog = Join-Path $LogDirectory 'engine-bootstrap.log'
    if (-not (Test-Path $BootstrapLog)) {
        Write-SmokeDiagnostics $Sandbox
        throw "GUI stayed alive but did not launch the engine; expected bootstrap log: $BootstrapLog"
    }

    Write-Host "GUI startup smoke test passed; process remained alive for $StartupSeconds seconds after launching its packaged engine."
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
    Remove-Item $Sandbox -Recurse -Force -ErrorAction SilentlyContinue
}
