$ErrorActionPreference = 'Stop'
$productRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$relativeProof = '.work/p0-02-inputs/verification'
$proofRoot = Join-Path $productRoot $relativeProof
New-Item -ItemType Directory -Force -Path $proofRoot | Out-Null
Set-Location -LiteralPath $productRoot
$results = [Collections.Generic.List[object]]::new()

function Invoke-SliceCheck([string]$Key, [string]$Program, [string[]]$Arguments) {
    $started = (Get-Date).ToString('o')
    $stdout = Join-Path $proofRoot "$Key.stdout.txt"
    $stderr = Join-Path $proofRoot "$Key.stderr.txt"
    Write-Output "P0-02 business inputs: $Key"
    # Windows PowerShell 5 wraps native stderr as ErrorRecords, including npm
    # notices. Capture both streams and classify by the actual process exit code.
    try {
        $ErrorActionPreference = 'Continue'
        & $Program @Arguments 1> $stdout 2> $stderr
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = 'Stop'
    }
    $entry = [ordered]@{
        key = $Key; command = (@($Program) + $Arguments) -join ' '
        started_at = $started; finished_at = (Get-Date).ToString('o'); exit_code = $code
        stdout_ref = "$relativeProof/$Key.stdout.txt"
        stderr_ref = "$relativeProof/$Key.stderr.txt"
    }
    if ($Key -eq 'workspace-tests' -and $code -eq 0) {
        $passed = 0
        Select-String -LiteralPath $stdout -Pattern 'test result: ok\. (\d+) passed;' -AllMatches |
            ForEach-Object { foreach ($match in $_.Matches) { $passed += [int]$match.Groups[1].Value } }
        $entry.distinct_tests_executed = $passed
    }
    $results.Add($entry)
    [ordered]@{status='running';checks=@($results)} | ConvertTo-Json -Depth 12 |
        Set-Content (Join-Path $proofRoot 'result.json') -Encoding utf8
    if ($code -ne 0) {
        Get-Content -LiteralPath $stdout -Tail 24
        Get-Content -LiteralPath $stderr -Tail 20
        throw "P0-02 business inputs check failed: $Key ($code)"
    }
}

try {
    Invoke-SliceCheck 'format' 'cargo' @('fmt','-p','sf-domain','-p','sf-commands','-p','sf-desktop-host','--','--check')
    Invoke-SliceCheck 'workspace-tests' 'npm.cmd' @('run','test:rust')
    Invoke-SliceCheck 'generate-contracts' 'npm.cmd' @('run','generate:contracts')
    Invoke-SliceCheck 'typecheck' 'npm.cmd' @('run','typecheck')
    $runtime = 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
    Invoke-SliceCheck 'schema-consumer' $runtime @('-B','-X','utf8','tools/检查工程与Command边界.py','--output','.work/p0-02-inputs/schema-validation-result.json')
    Invoke-SliceCheck 'desktop-build' 'cargo' @('build','-p','sf-desktop-host','--bin','superfactory','--locked')
    Invoke-SliceCheck 'desktop-host-smoke' '.\target\debug\superfactory.exe' @('--p0-01-host-smoke')
    $smoke = Get-Content -LiteralPath (Join-Path $proofRoot 'desktop-host-smoke.stdout.txt') -Encoding utf8 |
        Where-Object { $_ -match '^\{' } | ForEach-Object { $_ | ConvertFrom-Json } |
        Where-Object { $_.kind -eq 'P0_01_host_smoke' }
    if ($null -eq $smoke -or $smoke.passed -ne $true -or $smoke.checks.Count -le 31) {
        throw 'Missing actual desktop business-input smoke receipt'
    }
    $previous = Get-Content -LiteralPath '.work/p0-01-persistence-history/verification/result.json' -Encoding utf8 | ConvertFrom-Json
    $names = @($smoke.checks | ForEach-Object { $_.name })
    foreach ($old in $previous.native_host_smoke.checks) {
        if ($old.name -notin $names) { throw ('Previous native check removed: ' + $old.name) }
    }
    $oldTests = Select-String -LiteralPath '.work/p0-01-persistence-history/verification/workspace-tests.stdout.txt' -Pattern '^test .+ \.\.\. ok$' | ForEach-Object { $_.Line.Trim() }
    $newTests = Select-String -LiteralPath (Join-Path $proofRoot 'workspace-tests.stdout.txt') -Pattern '^test .+ \.\.\. ok$' | ForEach-Object { $_.Line.Trim() }
    if (@($oldTests).Count -ne 79) { throw 'Expected the retained 79-test baseline' }
    foreach ($old in $oldTests) {
        if ($old -notin $newTests) { throw ('Previous Rust test removed: ' + $old) }
    }
    [ordered]@{status='passed';checks=@($results);native_host_smoke=$smoke;whole_task_done=$false} |
        ConvertTo-Json -Depth 100 | Set-Content (Join-Path $proofRoot 'result.json') -Encoding utf8
    Write-Output 'P0-02 business inputs verification passed'
} catch {
    [ordered]@{status='failed';checks=@($results);error=$_.Exception.Message} |
        ConvertTo-Json -Depth 12 | Set-Content (Join-Path $proofRoot 'result.json') -Encoding utf8
    throw
}
