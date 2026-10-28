$ErrorActionPreference = 'Stop'
Add-Type -Path (Join-Path $PSScriptRoot 'PipePeerNative.cs')
function Assert-Fixture([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}
$outputDir = Join-Path $PSScriptRoot 'output'
$serverIdentity = [Drf04Peer.Native]::ProcessIdentity([Drf04Peer.Native]::GetCurrentProcessId())
$actualInstance = [guid]::NewGuid().ToString()
$cases = [Collections.Generic.List[object]]::new()
foreach ($caseName in @('spoofed_body', 'wrong_instance', 'disconnect')) {
    $serverPipe = $null
    $child = $null
    $caseResult = [ordered]@{ id = $caseName; status = 'running' }
    try {
        $pipeName = 'drf04-peer-' + [guid]::NewGuid().ToString('N')
        $serverPipe = [Drf04Peer.Native]::CreateServer($pipeName)
        $caseResult.pipe_name = $pipeName
        $caseResult.pipe_sddl = $serverPipe.Sddl
        $caseResult.remote_clients_rejected_flag = $true
        $caseResult.first_pipe_instance_flag = $true
        $expectedInstance = $actualInstance
        if ($caseName -eq 'wrong_instance') { $expectedInstance = [guid]::NewGuid().ToString() }
        $clientOutput = Join-Path $outputDir ('pipe-peer-' + $caseName + '-client.json')
        $clientScript = Join-Path $PSScriptRoot 'pipe_peer_client.ps1'
        $arguments = @('-NoLogo', '-NoProfile', '-File', ('"' + $clientScript + '"'), '-PipeName', $pipeName,
            '-ExpectedServerProcessId', $serverIdentity.process_id, '-ExpectedInstance', $expectedInstance,
            '-CaseName', $caseName, '-ResultPath', ('"' + $clientOutput + '"'))
        $stdoutPath = Join-Path $outputDir ('pipe-peer-' + $caseName + '.stdout.txt')
        $stderrPath = Join-Path $outputDir ('pipe-peer-' + $caseName + '.stderr.txt')
        $child = Start-Process -FilePath ([Environment]::ProcessPath) -ArgumentList $arguments -WindowStyle Hidden -PassThru `
            -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
        $caseResult.spawned_bridge_pid = $child.Id
        [Drf04Peer.Native]::Wait($serverPipe.Stream.WaitForConnectionAsync())
        $peer = [Drf04Peer.Native]::ObserveClient($serverPipe.Handle)
        $caseResult.observed_client = $peer
        $checks = [ordered]@{
            spawned_pid_matches_pipe_api = ($peer.pipe_process_id -eq $child.Id)
            process_handle_pid_matches_pipe_api = ($peer.token_identity.process_api_id -eq $peer.pipe_process_id)
            pipe_session_matches_process_and_token = ($peer.pipe_session_id -eq $peer.token_identity.process_session_id -and $peer.pipe_session_id -eq $peer.token_identity.token_session_id)
            same_actual_user = ($peer.token_identity.user_sid -eq $serverIdentity.user_sid)
            same_actual_logon = ($peer.token_identity.logon_sid -eq $serverIdentity.logon_sid)
            same_actual_session = ($peer.pipe_session_id -eq $serverIdentity.process_session_id)
            distinct_real_processes = ($peer.pipe_process_id -ne $serverIdentity.process_id)
        }
        $caseResult.os_checks = $checks
        foreach ($check in $checks.GetEnumerator()) { Assert-Fixture $check.Value ('Server OS identity: ' + $check.Key) }
        [Drf04Peer.Native]::WriteFrame($serverPipe.Stream, (@{ host_instance_id = $actualInstance; peer_observation_complete = $true } | ConvertTo-Json -Compress))
        $body = [Drf04Peer.Native]::ReadFrame($serverPipe.Stream)
        if ($caseName -eq 'disconnect') {
            Assert-Fixture ($null -eq $body) 'Bridge disconnect did not yield actual EOF'
            $caseResult.disconnect_observed = 'actual_zero_byte_read_before_request'
        } else {
            Assert-Fixture ($null -ne $body) 'Bridge closed before request'
            $request = $body | ConvertFrom-Json
            $caseResult.request_body = $request
            $sameInstance = $request.expected_host_instance_id -ceq $actualInstance
            $response = [ordered]@{
                admitted = $sameInstance
                host_instance_id = $actualInstance
                observed_client_pid = $peer.pipe_process_id
                authority_source = 'actual_pipe_peer_process_token'
                error = $null
            }
            if (-not $sameInstance) { $response.error = 'ExpectedHostInstanceMismatch' }
            $caseResult.response = $response
            $caseResult.body_identity_used_for_authority = $false
            if ($caseName -eq 'spoofed_body') {
                Assert-Fixture ($request.claimed_client_pid -ne $peer.pipe_process_id -and $request.claimed_server_pid -ne $serverIdentity.process_id -and $request.claimed_logon_sid -ne $peer.token_identity.logon_sid -and $request.claimed_host_id -ne $actualInstance) 'Spoof fixture did not differ from real identity'
                Assert-Fixture $sameInstance 'Correct expected instance rejected'
                $caseResult.expected_outcome = 'accepted_correct_expected_instance_using_actual_os_identity_despite_spoofed_body'
            } else {
                Assert-Fixture (-not $sameInstance -and $request.claimed_host_id -ceq $actualInstance -and $request.claimed_server_pid -eq $serverIdentity.process_id) 'Wrong-instance fixture not constructed correctly'
                $caseResult.expected_outcome = 'rejected_wrong_expected_instance_even_when_body_claims_actual_host'
            }
            [Drf04Peer.Native]::WriteFrame($serverPipe.Stream, ($response | ConvertTo-Json -Compress))
            $eof = [Drf04Peer.Native]::ReadFrame($serverPipe.Stream)
            Assert-Fixture ($null -eq $eof) 'Bridge did not close pipe after response'
            $caseResult.disconnect_observed = 'actual_zero_byte_read_after_response'
        }
        Assert-Fixture ($child.WaitForExit(10000)) 'Owned bridge did not exit'
        Assert-Fixture ($child.ExitCode -eq 0) 'Owned bridge failed; inspect its result/stderr'
        $clientResult = Get-Content -LiteralPath $clientOutput -Raw -Encoding utf8 | ConvertFrom-Json
        Assert-Fixture ($clientResult.status -eq 'passed' -and $clientResult.pipe_handle_closed -eq $true -and $clientResult.pipe_handle_after_close_error -eq 6) 'Bridge identity/pipe cleanup failed'
        $caseResult.bridge_result = $clientResult
        $caseResult.status = 'passed'
    } catch {
        $caseResult.status = 'failed'
        $caseResult.error = $_.Exception.ToString()
    } finally {
        try {
            if ($null -ne $serverPipe) {
                $serverPipe.Close()
                $caseResult.server_pipe_handle_closed = $serverPipe.Closed
                $caseResult.server_pipe_after_close_error = $serverPipe.AfterCloseError
            }
            if ($null -ne $child) {
                $caseResult.own_child_terminated_on_error = $false
                if (-not $child.HasExited) {
                    # This Process object came only from this iteration's Start-Process.
                    $child.Kill()
                    $child.WaitForExit()
                    $caseResult.own_child_terminated_on_error = $true
                }
                $caseResult.own_child_exited = $child.HasExited
                $processHandle = $child.SafeHandle.DangerousGetHandle()
                $child.Dispose()
                $caseResult.own_child_process_handle_after_close_error = [Drf04Peer.Native]::ClosedError($processHandle)
                $caseResult.own_child_process_handle_closed = $true
            }
        } catch {
            $caseResult.status = 'failed'
            $caseResult.cleanup_error = $_.Exception.ToString()
        }
        $cases.Add($caseResult)
    }
    if ($caseResult.status -eq 'passed') {
        foreach ($streamPath in @($stdoutPath, $stderrPath)) {
            if ((Get-Item -LiteralPath $streamPath).Length -eq 0) { Remove-Item -LiteralPath $streamPath }
        }
    }
    if ($caseResult.status -ne 'passed') { break }
}
$passed = @($cases | Where-Object { $_.status -eq 'passed' }).Count
$result = [ordered]@{
    document_kind = 'real_windows_named_pipe_peer_identity_isolated_trial'
    observed_at = (Get-Date -Format o)
    status = $(if ($passed -eq 3 -and $cases.Count -eq 3) { 'passed' } else { 'failed' })
    runtime = @{ powershell = $PSVersionTable.PSVersion.ToString(); dotnet = [Environment]::Version.ToString(); exe = [Environment]::ProcessPath }
    server_identity = $serverIdentity
    actual_server_instance = $actualInstance
    identity_source = 'GetNamedPipeClient/ServerProcessId + GetNamedPipeClient/ServerSessionId + actual OS peer process token TokenUser/TokenGroups logon SID/TokenSessionId'
    framing = 'actual u32-LE byte length plus UTF-8 JSON on pipe'
    cases_passed = $passed
    cases_total = $cases.Count
    cases = $cases
    dependencies_installed = $false
    locks_modified = $false
    product_host_or_bridge = $false
    actual_product_runtime = $null
    boundary = 'Two real processes and OS pipe/token/EOF/owned-handle operations. Isolated admission predicate only; no product Host session registry, Commands grants, canonical full IpcHello/Welcome, renderer, business continuation or Codex. Cross-logon and remote-client rejection were not attempted.'
}
$result | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $outputDir 'pipe-peer-identity-result.json') -Encoding utf8
[pscustomobject]$result | Select-Object status, cases_passed, cases_total, observed_at | ConvertTo-Json
if ($result.status -ne 'passed') { exit 1 }
