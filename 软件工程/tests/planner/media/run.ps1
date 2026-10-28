$ErrorActionPreference='Stop'
$mediaRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
Set-Location -LiteralPath $mediaRoot
$mediaRuntime='scenarios/媒体表现/runtime'
$mediaPython='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
$mediaNode='C:/Program Files/nodejs/node.exe'
& $mediaPython -B -X utf8 modules/media/p302-controlled/generate.py *> "$mediaRuntime/assets-generate-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Controlled asset generation failed'}
$mediaSim=(Get-ChildItem target/debug/deps/libsf_simulation-*.rlib | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$mediaDomain=(Get-ChildItem target/debug/deps/libsf_domain-*.rlib | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$mediaJson=(Get-ChildItem target/debug/deps/libserde_json-*.rlib | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
@{simulation=$mediaSim;domain=$mediaDomain;serde_json=$mediaJson}|ConvertTo-Json|Set-Content "$mediaRuntime/actual-libraries.json" -Encoding utf8
& 'C:/Users/fujunye/.cargo/bin/rustc.exe' --edition=2024 tests/planner/media/engine_consumer.rs -L dependency=target/debug/deps --extern "sf_simulation=$mediaSim" --extern "sf_domain=$mediaDomain" --extern "serde_json=$mediaJson" -o "$mediaRuntime/engine_consumer.exe" *> "$mediaRuntime/engine-build-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Actual Engine consumer compilation failed'}
& ./scenarios/媒体表现/runtime/engine_consumer.exe *> "$mediaRuntime/engine-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Actual Engine expectations/equality failed'}
& $mediaNode --test tests/planner/media/media.test.mjs *> "$mediaRuntime/unit-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Media projection/decoder tests failed'}
& $mediaNode node_modules/typescript/bin/tsc --noEmit -p packages/presentation/tsconfig.json *> "$mediaRuntime/package-typecheck-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Presentation typecheck failed'}
& $mediaNode node_modules/typescript/bin/tsc --noEmit -p apps/planner/tsconfig.json *> "$mediaRuntime/app-typecheck-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Planner typecheck failed'}
& $mediaNode tests/planner/media/build.mjs *> "$mediaRuntime/build-final.raw.txt"
if($LASTEXITCODE -ne 0){throw 'Independent harness build failed'}
Write-Output 'P302 own-scope validation passed; real Native/industrial/IX/NFR/release remain separate.'
