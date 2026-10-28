param(
    [string]$SourceRoot = (Join-Path $PSScriptRoot '../..'),
    [string]$Node = 'node',
    [string]$Cargo = 'cargo',
    [string]$ReadOnlyNodeModules,
    [string]$TargetDirectory,
    [switch]$Offline,
    [switch]$Release
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $SourceRoot).Path
if (-not $TargetDirectory) { $TargetDirectory = Join-Path $root '.work/cargo-target' }
if ($ReadOnlyNodeModules -and -not (Test-Path -LiteralPath (Join-Path $root 'node_modules'))) {
    New-Item -ItemType Junction -Path (Join-Path $root 'node_modules') -Target (Resolve-Path -LiteralPath $ReadOnlyNodeModules).Path | Out-Null
}
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = $TargetDirectory
Push-Location -LiteralPath $root
try {
    $tsc = Join-Path $root 'node_modules/typescript/bin/tsc'
    $vite = Join-Path $root 'node_modules/vite/bin/vite.js'
    foreach ($project in @('packages/presentation/tsconfig.json', 'apps/planner/tsconfig.json')) {
        Write-Output "TYPECHECK $project"
        & $Node $tsc --noEmit -p $project
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    Write-Output 'FRONTEND_BUILD apps/planner (configLoader runner avoids writing shared node_modules)'
    & $Node $vite build apps/planner --config apps/planner/vite.config.ts --configLoader runner
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $cargoArgs = @('check','-p','sf-desktop-host','-p','sf-mcp-bridge','--lib','--bins','--features','sf-desktop-host/custom-protocol','--locked','--target-dir',$TargetDirectory,'-j','2')
    if ($Release) { $cargoArgs = @('build','--release','-p','sf-desktop-host','-p','sf-mcp-bridge','--lib','--bins','--features','sf-desktop-host/custom-protocol,sf-desktop-host/nfr-devtools','--locked','--target-dir',$TargetDirectory,'-j','1') }
    if ($Offline) { $cargoArgs += '--offline' }
    Write-Output ('CARGO_CHECK ' + ($cargoArgs -join ' '))
    & $Cargo @cargoArgs
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
