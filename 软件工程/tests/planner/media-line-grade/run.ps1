$ErrorActionPreference='Stop'
$lineGradeRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
Set-Location -LiteralPath $lineGradeRoot
$lineGradeOut='scenarios/媒体表现/runtime-line-grade'
$lineGradeNode='C:/Program Files/nodejs/node.exe'
$lineGradeLibraries=Get-Content -LiteralPath 'scenarios/媒体表现/runtime/actual-libraries.json' -Raw | ConvertFrom-Json
& $lineGradeNode --test tests/planner/media-line-grade/geometry.test.mjs tests/planner/media-line-grade/rebuild.test.mjs *> "$lineGradeOut/verification-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Actual exported geometry or editable-source rebuild failed'}
& 'C:/Users/fujunye/.cargo/bin/rustc.exe' --edition=2024 tests/planner/media-line-grade/engine_consumer.rs -L dependency=target/debug/deps --extern "sf_simulation=$($lineGradeLibraries.simulation)" --extern "sf_domain=$($lineGradeLibraries.domain)" --extern "serde_json=$($lineGradeLibraries.serde_json)" -o "$lineGradeOut/engine_consumer.exe" *> "$lineGradeOut/engine-build.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Pinned actual Engine consumer compilation failed'}
& './scenarios/媒体表现/runtime-line-grade/engine_consumer.exe' *> "$lineGradeOut/engine.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Original Engine expectations or media equality failed'}
& $lineGradeNode node_modules/typescript/bin/tsc --noEmit --incremental false -p apps/planner/tsconfig.json *> "$lineGradeOut/app-typecheck.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Planner typecheck failed'}
& $lineGradeNode tests/planner/media-line-grade/build.mjs *> "$lineGradeOut/build.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Independent harness build failed'}
Write-Output 'Line-grade local checks passed. Existing business coverage unchanged; actual Native/quality/NFR/release gates remain open.'
