$ErrorActionPreference = 'Stop'
$s04Path = Join-Path $PSScriptRoot '../../../02_交互设计/S04_可操作设计原型/output/delegated-walkthrough/walkthrough-result.json'
$s04 = Get-Content -LiteralPath $s04Path -Raw -Encoding utf8 | ConvertFrom-Json
if (-not $s04.completed -or $s04.authorization.quote -ne '代替我完成S04-04/S04-05' -or @($s04.flows | Where-Object { $_.status -ne 'passed' }).Count -gt 0) {
    throw 'S04 actual delegated walkthrough completion evidence is missing'
}
$start = [ordered]@{
    actual_start = (Get-Date -Format o)
    S04_evidence = (Resolve-Path -LiteralPath $s04Path).Path
    operator_kind = $s04.operator_kind
    S04_completed = $s04.completed
    S04_actual_user_walkthrough = $s04.actual_user_walkthrough
    product_apps_created = $false
}
$outputDir = Join-Path $PSScriptRoot 'output'
$start | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $outputDir 'launch-preconditions.json') -Encoding utf8
$exe = Join-Path $PSScriptRoot 'target/debug/drf04-s05-desktop-proof.exe'
$proc = $null
try {
    $proc = Start-Process -FilePath $exe -WorkingDirectory $PSScriptRoot -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput (Join-Path $outputDir 'desktop.stdout.txt') `
        -RedirectStandardError (Join-Path $outputDir 'desktop.stderr.txt')
    $childIds = [Collections.Generic.HashSet[int]]::new()
    $timedOut = $false
    $deadline = (Get-Date).AddSeconds(60)
    while (-not $proc.WaitForExit(500)) {
        Get-CimInstance Win32_Process -Filter ('ParentProcessId=' + $proc.Id) | ForEach-Object { [void]$childIds.Add([int]$_.ProcessId) }
        if ((Get-Date) -ge $deadline) { $timedOut = $true; break }
    }
    if ($timedOut) { $proc.Kill($true); $proc.WaitForExit() }
    $exit = [ordered]@{
        actual_end = (Get-Date -Format o)
        actual_host_pid = $proc.Id
        actual_exit_code = $proc.ExitCode
        own_helper_timed_out_and_terminated = $timedOut
        observed_direct_child_pids = @($childIds)
        actual_host_exited = $proc.HasExited
    }
    $exit | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $outputDir 'actual-exit.json') -Encoding utf8
    $exit | ConvertTo-Json -Depth 4
    if ($timedOut -or $proc.ExitCode -ne 0) {
        Get-Content -LiteralPath (Join-Path $outputDir 'desktop.stderr.txt') -Tail 35
        throw 'Actual isolated desktop probe failed; inspect output'
    }
} finally {
    if ($null -ne $proc) {
        if (-not $proc.HasExited) { $proc.Kill($true); $proc.WaitForExit() }
        $proc.Dispose()
    }
}
