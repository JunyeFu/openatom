param(
    [Parameter(Mandatory)][string]$PipeName,
    [Parameter(Mandatory)][uint32]$ExpectedServerProcessId,
    [Parameter(Mandatory)][string]$ExpectedInstance,
    [Parameter(Mandatory)][string]$CaseName,
    [Parameter(Mandatory)][string]$ResultPath
)
$ErrorActionPreference = 'Stop'
Add-Type -Path (Join-Path $PSScriptRoot 'PipePeerNative.cs')
function Assert-Fixture([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}
$clientPipe = $null
$rawHandle = [IntPtr]::Zero
$result = [ordered]@{ case = $CaseName; role = 'isolated_bridge_process'; status = 'running' }
try {
    $current = [Drf04Peer.Native]::ProcessIdentity([Drf04Peer.Native]::GetCurrentProcessId())
    $result.current_identity = $current
    $clientPipe = [IO.Pipes.NamedPipeClientStream]::new('.', $PipeName, [IO.Pipes.PipeDirection]::InOut, [IO.Pipes.PipeOptions]::Asynchronous)
    $clientPipe.Connect(10000)
    $rawHandle = $clientPipe.SafePipeHandle.DangerousGetHandle()
    $peer = [Drf04Peer.Native]::ObserveServer($rawHandle)
    $result.observed_server = $peer
    $checks = [ordered]@{
        bootstrap_pid_matches_pipe_api = ($peer.pipe_process_id -eq $ExpectedServerProcessId)
        process_handle_pid_matches_pipe_api = ($peer.token_identity.process_api_id -eq $peer.pipe_process_id)
        pipe_session_matches_process_and_token = ($peer.pipe_session_id -eq $peer.token_identity.process_session_id -and $peer.pipe_session_id -eq $peer.token_identity.token_session_id)
        same_actual_user = ($peer.token_identity.user_sid -eq $current.user_sid)
        same_actual_logon = ($peer.token_identity.logon_sid -eq $current.logon_sid)
        same_actual_session = ($peer.pipe_session_id -eq $current.process_session_id)
        distinct_real_processes = ($peer.pipe_process_id -ne $current.process_id)
    }
    $result.os_checks = $checks
    foreach ($check in $checks.GetEnumerator()) { Assert-Fixture $check.Value ('Bridge OS identity: ' + $check.Key) }
    $hello = [Drf04Peer.Native]::ReadFrame($clientPipe) | ConvertFrom-Json
    $result.server_ready = $hello
    if ($CaseName -ne 'disconnect') {
        $request = [ordered]@{
            wire_version = 1
            expected_host_instance_id = $ExpectedInstance
            claimed_client_pid = $ExpectedServerProcessId
            claimed_server_pid = 1
            claimed_user_sid = 'S-1-0-0'
            claimed_logon_sid = 'S-1-0-0'
            claimed_host_id = 'body-forged-host'
        }
        if ($CaseName -eq 'wrong_instance') {
            # A body claim of the real instance cannot repair the wrong expected instance.
            $request.claimed_host_id = $hello.host_instance_id
            $request.claimed_server_pid = $ExpectedServerProcessId
        }
        $result.request_body = $request
        [Drf04Peer.Native]::WriteFrame($clientPipe, ($request | ConvertTo-Json -Compress))
        $response = [Drf04Peer.Native]::ReadFrame($clientPipe) | ConvertFrom-Json
        $result.response = $response
        if ($CaseName -eq 'spoofed_body') {
            Assert-Fixture ($response.admitted -eq $true -and $response.authority_source -eq 'actual_pipe_peer_process_token') 'Correct expected instance did not use actual OS peer authority'
        } else {
            Assert-Fixture ($response.admitted -eq $false -and $response.error -eq 'ExpectedHostInstanceMismatch') 'Wrong expected instance was not rejected'
        }
    }
    $result.status = 'passed'
} catch {
    $result.status = 'failed'
    $result.error = $_.Exception.ToString()
} finally {
    if ($null -ne $clientPipe) {
        try {
            $clientPipe.Dispose()
            $result.pipe_handle_after_close_error = [Drf04Peer.Native]::ClosedError($rawHandle)
            $result.pipe_handle_closed = $true
        } catch {
            $result.status = 'failed'
            $result.cleanup_error = $_.Exception.ToString()
        }
    }
    $result | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $ResultPath -Encoding utf8
}
if ($result.status -ne 'passed') { exit 1 }
