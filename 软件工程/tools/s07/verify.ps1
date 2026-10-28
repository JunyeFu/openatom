$ErrorActionPreference = 'Stop'
$productRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$outputDir = Join-Path $productRoot '.work/s07/verification'
$resultPath = Join-Path $PSScriptRoot 'output/verification.json'
New-Item -ItemType Directory -Force -Path $outputDir, (Split-Path $resultPath) | Out-Null
Set-Location -LiteralPath $productRoot
$results = [Collections.Generic.List[object]]::new()

function Invoke-Verified([string]$Key, [string]$File, [string[]]$Arguments) {
    $stdout = Join-Path $outputDir "$Key.stdout.txt"
    $stderr = Join-Path $outputDir "$Key.stderr.txt"
    $at = Get-Date -Format o
    Write-Output "S07: $Key"
    & $File @Arguments 1> $stdout 2> $stderr
    $code = $LASTEXITCODE
    $results.Add([ordered]@{
        key = $Key; command = (@($File) + $Arguments) -join ' '; started_at = $at
        finished_at = (Get-Date -Format o); exit_code = $code
        stdout_ref = ".work/s07/verification/$Key.stdout.txt"
        stderr_ref = ".work/s07/verification/$Key.stderr.txt"
    })
    [ordered]@{ status = 'running'; checks = @($results) } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $resultPath -Encoding utf8
    if ($code -ne 0) {
        Get-Content -LiteralPath $stderr -Tail 25
        throw "S07 $Key failed with exit $code"
    }
}

try {
    Invoke-Verified 'rustc-version' 'rustc' @('--version')
    Invoke-Verified 'cargo-version' 'cargo' @('--version')
    Invoke-Verified 'node-version' 'node' @('--version')
    Invoke-Verified 'npm-version' 'npm.cmd' @('--version')
    Invoke-Verified 'npm-tree' 'npm.cmd' @('ls', '--all', '--json')
    Invoke-Verified 'cargo-metadata' 'cargo' @('metadata', '--locked', '--no-deps', '--format-version', '1')
    Invoke-Verified 'format' 'cargo' @('fmt', '--all', '--', '--check')
    Invoke-Verified 'generate-types' 'npm.cmd' @('run', 'generate:types')
    Invoke-Verified 'typecheck' 'npm.cmd' @('run', 'typecheck')
    Invoke-Verified 'desktop-build' 'npm.cmd' @('run', 'build')
    Invoke-Verified 'rust-test-entry' 'npm.cmd' @('run', 'test:rust')
    Invoke-Verified 'engineering-smoke' 'npm.cmd' @('test')
    [ordered]@{
        document_kind = 'S07_actual_product_engineering_verification'
        status = 'passed'; verified_at = (Get-Date -Format o); checks = @($results)
        scope = '工程壳/本机WebView2启动/本地资源与私有工程IPC、类型生成和测试入口；没有业务操作或产品功能验收'
        business_operations_implemented = 0; business_frontend_implemented = $false
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $resultPath -Encoding utf8
    Write-Output 'S07 verification passed'
} catch {
    [ordered]@{ status = 'failed'; failed_at = (Get-Date -Format o); checks = @($results); error = $_.Exception.Message } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $resultPath -Encoding utf8
    throw
}
