[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$EnginePath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$EnginePath = (Resolve-Path $EnginePath).Path
$PipeName = "smbn-smoke-$PID-$([Guid]::NewGuid().ToString('N'))"
$TokenBytes = [byte[]](1..48 | ForEach-Object { Get-Random -Minimum 0 -Maximum 256 })
$Token = [Convert]::ToBase64String($TokenBytes)
[Array]::Clear($TokenBytes, 0, $TokenBytes.Length)
$LogDirectory = Join-Path ([IO.Path]::GetTempPath()) "smbn-smoke-$([Guid]::NewGuid().ToString('N'))"
$ShareDirectory = Join-Path $LogDirectory 'share'
New-Item -ItemType Directory -Force -Path $LogDirectory, $ShareDirectory | Out-Null

function Read-Exactly {
    param([IO.Stream]$Stream, [byte[]]$Buffer)
    $Offset = 0
    while ($Offset -lt $Buffer.Length) {
        $Count = $Stream.Read($Buffer, $Offset, $Buffer.Length - $Offset)
        if ($Count -eq 0) { throw 'Unexpected end of named-pipe stream.' }
        $Offset += $Count
    }
}

$script:RequestId = [uint64]0
function Invoke-EngineRequest {
    param([string]$Command, [object]$Payload)
    $script:RequestId += 1
    $Envelope = [ordered]@{
        version = 1
        id = $script:RequestId
        token = $Token
        command = $Command
        payload = $Payload
    }
    $Bytes = [Text.Encoding]::UTF8.GetBytes(($Envelope | ConvertTo-Json -Depth 20 -Compress))
    $Client = [IO.Pipes.NamedPipeClientStream]::new('.', $PipeName, [IO.Pipes.PipeDirection]::InOut)
    try {
        $Client.Connect(5000)
        $Length = [BitConverter]::GetBytes([uint32]$Bytes.Length)
        $Client.Write($Length, 0, $Length.Length)
        $Client.Write($Bytes, 0, $Bytes.Length)
        $Client.Flush()
        $ResponseLength = New-Object byte[] 4
        Read-Exactly $Client $ResponseLength
        $Count = [BitConverter]::ToUInt32($ResponseLength, 0)
        if ($Count -eq 0 -or $Count -gt 8388608) { throw "Invalid response length: $Count" }
        $ResponseBytes = New-Object byte[] $Count
        Read-Exactly $Client $ResponseBytes
        $Response = [Text.Encoding]::UTF8.GetString($ResponseBytes) | ConvertFrom-Json
        if (-not $Response.ok) { throw "Engine error: $($Response.error.code): $($Response.error.message)" }
        return $Response.payload
    }
    finally {
        $Client.Dispose()
        [Array]::Clear($Bytes, 0, $Bytes.Length)
    }
}

$OldToken = $env:SMBN_IPC_TOKEN
$env:SMBN_IPC_TOKEN = $Token
$Process = $null
try {
    $Process = Start-Process -FilePath $EnginePath -ArgumentList @(
        '--pipe', $PipeName,
        '--parent', $PID.ToString(),
        '--log-dir', ('"{0}"' -f $LogDirectory)
    ) -PassThru -WindowStyle Hidden
    Remove-Item Env:SMBN_IPC_TOKEN -ErrorAction SilentlyContinue

    $Ready = $false
    for ($Attempt = 0; $Attempt -lt 30 -and -not $Ready; $Attempt++) {
        try {
            $Ping = Invoke-EngineRequest 'ping' @{}
            $Ready = $true
        }
        catch {
            if ($Process.HasExited) { throw "Engine exited during startup with code $($Process.ExitCode)." }
            Start-Sleep -Milliseconds 100
        }
    }
    if (-not $Ready) { throw 'Engine pipe did not become ready.' }

    $Config = [ordered]@{
        server = [ordered]@{
            netbios_name = 'SMBNTEST'
            workgroup = 'WORKGROUP'
            authentication = 'integrated_windows'
            enable_smb1 = $false
            enable_smb2 = $true
            enable_smb3 = $true
            inactivity_timeout_seconds = 30
            reject_remote_subnets = @()
            allow_remote_subnets = @('127.0.0.0/8', '::1/128')
        }
        listeners = @([ordered]@{
            id = 'smoke-direct'
            address = '127.0.0.1'
            port = 41445
            transport = 'direct_tcp'
            netbios_name_service = $false
            enabled = $true
        })
        shares = @([ordered]@{
            id = 'smoke-share'
            name = 'Smoke'
            path = $ShareDirectory
            comment = ''
            enabled = $true
            hidden = $false
            read_only = $true
            read_access = @('Users')
            write_access = @()
        })
        users = @()
        logging = [ordered]@{
            level = 'information'
            max_file_mib = 1
            retained_files = 1
            gui_tail_lines = 50
        }
    }
    $Diagnostics = Invoke-EngineRequest 'diagnostics' @{ config = $Config }
    if ($null -eq $Diagnostics.checks -or $Diagnostics.checks.Count -eq 0) {
        throw 'Diagnostics returned no checks.'
    }
    Invoke-EngineRequest 'shutdown' @{} | Out-Null
    if (-not $Process.WaitForExit(5000)) { throw 'Engine did not exit after shutdown.' }
    if ($Process.ExitCode -ne 0) { throw "Engine exited with code $($Process.ExitCode)." }
    Write-Host "Engine IPC smoke test passed ($($Diagnostics.checks.Count) diagnostic checks)."
}
finally {
    if ($null -ne $Process -and -not $Process.HasExited) {
        Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
    }
    if ($null -eq $OldToken) { Remove-Item Env:SMBN_IPC_TOKEN -ErrorAction SilentlyContinue } else { $env:SMBN_IPC_TOKEN = $OldToken }
    Remove-Item $LogDirectory -Recurse -Force -ErrorAction SilentlyContinue
}
