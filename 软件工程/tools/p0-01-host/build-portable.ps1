param([string]$Destination, [switch]$EnableDevTools, [switch]$EnableNfr06SaveFaults)
$ErrorActionPreference = 'Stop'
$productRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
if (-not $Destination) {
    $stamp = [DateTimeOffset]::UtcNow.ToOffset([TimeSpan]::FromHours(8)).ToString('yyyyMMddTHHmmssfff')
    $Destination = Join-Path $productRoot ".work/whole-packages/p001-20261017/portable-$stamp"
}
$packageRoot = [IO.Path]::GetFullPath($Destination)
if (Test-Path -LiteralPath $packageRoot) {
    if (@(Get-ChildItem -LiteralPath $packageRoot -Force).Count -ne 0) {
        throw "Choose an empty destination; existing files are retained: $packageRoot"
    }
} else { New-Item -ItemType Directory -Path $packageRoot -Force | Out-Null }
$proofRoot = Join-Path $packageRoot 'build-proof'
New-Item -ItemType Directory -Path $proofRoot | Out-Null
Set-Location -LiteralPath $productRoot
$checks = [Collections.Generic.List[object]]::new()

function Build-Step([string]$Name, [string]$Program, [string[]]$Arguments) {
    $started = [DateTimeOffset]::UtcNow.ToString('o')
    $log = Join-Path $proofRoot "$Name.log"
    try {
        $ErrorActionPreference = 'Continue'
        & $Program @Arguments *> $log
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = 'Stop' }
    $checks.Add([ordered]@{ name=$Name; command=(@($Program)+$Arguments)-join ' '; exit_code=$code; started_at=$started; finished_at=[DateTimeOffset]::UtcNow.ToString('o'); raw_file="build-proof/$Name.log" })
    [ordered]@{ status='building'; checks=@($checks); actual_runtime=$null } | ConvertTo-Json -Depth 8 |
        Set-Content -LiteralPath (Join-Path $packageRoot 'portable-build.json') -Encoding utf8
    if ($code -ne 0) { Get-Content -LiteralPath $log -Tail 24; throw "Build failed: $Name ($code); original output retained in $log" }
}

Build-Step 'frontend' 'npm.cmd' @('run','build:web','--workspace','@superfactory/planner')
Copy-Item -LiteralPath (Join-Path $productRoot 'apps/planner/dist/index.html') -Destination (Join-Path $proofRoot 'frontend-index.html')
$nativeFeatures = 'sf-desktop-host/custom-protocol'
if ($EnableDevTools) { $nativeFeatures += ',sf-desktop-host/nfr-devtools' }
if ($EnableNfr06SaveFaults) { $nativeFeatures += ',sf-desktop-host/nfr06-save-faults' }
Build-Step 'native-and-bridge' 'cargo.exe' @('build','-p','sf-desktop-host','-p','sf-mcp-bridge','--bin','superfactory','--bin','superfactory-mcp-bridge','--release','--features',$nativeFeatures,'--locked','--offline','-j','1','--message-format=json')
$compilerFrontier = [Collections.Generic.List[object]]::new()
foreach ($buildName in @('native-and-bridge')) {
    foreach ($line in Get-Content -LiteralPath (Join-Path $proofRoot "$buildName.log")) {
        if ($line.StartsWith('{"reason":')) {
            $message = $line | ConvertFrom-Json
            if ($message.reason -eq 'compiler-artifact') {
                $compilerFrontier.Add([ordered]@{ build=$buildName; package_id=$message.package_id; target=$message.target; filenames=$message.filenames; features=$message.features; executable=$message.executable; fresh=$message.fresh })
            } elseif ($message.reason -eq 'build-script-executed') {
                $compilerFrontier.Add([ordered]@{ build=$buildName; package_id=$message.package_id; out_dir=$message.out_dir; env=$message.env; cfgs=$message.cfgs; linked_libs=$message.linked_libs; linked_paths=$message.linked_paths })
            }
        }
    }
}
ConvertTo-Json -InputObject @($compilerFrontier) -Depth 12 | Set-Content -LiteralPath (Join-Path $proofRoot 'compiler-frontier.json') -Encoding utf8
Copy-Item -LiteralPath (Join-Path $productRoot 'target/release/superfactory.exe') -Destination $packageRoot
Copy-Item -LiteralPath (Join-Path $productRoot 'target/release/superfactory-mcp-bridge.exe') -Destination $packageRoot
$startHere = @'
超级工厂 0.1.0 — Windows 11 x64 portable candidate

保持本目录中的 modules 与 report-resources 等目录和两个 exe 的相对位置。
双击 superfactory.exe 启动。手动保存使用“文件→保存工程”；重开选择保存目录。
本包内嵌 React/Three 前端；启动不需要 Node、Rust、Vite 或在线 CDN。
系统依赖 Microsoft Edge WebView2 Runtime。实际验证机器和版本见配套验收结果。
本版工程格式 schema_version=1.0，公开 API api_version=0.1；兼容结果以配套实际验收记录为准。

本地 Agent 桥：先启动应用，在 Agent 面板读取当前 endpoint 和 host_instance_id。
桥命令（替换面板给出的本次实际值）：
  superfactory-mcp-bridge.exe --host-pipe "<endpoint>" --host-instance-id "<host_instance_id>"
在应用中为已连接的会话明确授权所需对象和字段。关闭应用后旧 Host 身份失效。
Agent 面板提供当前路径与身份的可复制 Codex MCP 配置；使用本包内 bridge 的绝对路径。

build-proof 是本次实际构建输出。portable-build.json 的 runtime 状态须由真实启动验证补充。
安装器与另一台目标机器的安装/重开结果另列，不从本机 portable 构建推定。
'@
if ($EnableDevTools) {
    $startHere = "NFR测量候选：release优化与同一产品入口，显式启用Native WebView2 DevTools采集实际Frames；记录诊断开销。不是默认分发配置。`r`n`r`n" + $startHere
}
if ($EnableNfr06SaveFaults) {
    $startHere = "NFR06受控故障测试候选，禁止作为最终分发包。仅在exe旁存在nfr06-save-fault.json且目标路径/旧身份完全匹配时执行一次预定故障；随后调用真实ReplaceFileW和原恢复逻辑，实际错误码据实记录。`r`n`r`n" + $startHere
}
[IO.File]::WriteAllText((Join-Path $packageRoot 'START-HERE.txt'), $startHere, [Text.UTF8Encoding]::new($false))
$configuration = Get-Content -LiteralPath (Join-Path $productRoot 'apps/planner/src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
$resourceFiles = [Collections.Generic.List[object]]::new()
foreach ($mapping in $configuration.bundle.resources.PSObject.Properties) {
    $source = [IO.Path]::GetFullPath((Join-Path (Join-Path $productRoot 'apps/planner/src-tauri') $mapping.Name))
    $target = Join-Path $packageRoot $mapping.Value
    if (Test-Path -LiteralPath $source -PathType Container) {
        foreach ($file in Get-ChildItem -LiteralPath $source -File -Recurse) {
            $relative = [IO.Path]::GetRelativePath($source, $file.FullName)
            $output = Join-Path $target $relative
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $output) | Out-Null
            Copy-Item -LiteralPath $file.FullName -Destination $output
            $resourceFiles.Add([ordered]@{ file=[IO.Path]::GetRelativePath($packageRoot,$output); bytes=$file.Length })
        }
    } else {
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target) | Out-Null
        Copy-Item -LiteralPath $source -Destination $target
        $resourceFiles.Add([ordered]@{ file=[IO.Path]::GetRelativePath($packageRoot,$target); bytes=(Get-Item -LiteralPath $target).Length })
    }
}
$entry = Get-Item -LiteralPath (Join-Path $packageRoot 'superfactory.exe')
[ordered]@{ status='built_and_staged'; platform='Windows 11 x64'; package_root=$packageRoot; entry='superfactory.exe'; entry_bytes=$entry.Length; frontend='embedded custom-protocol assets'; native_features=$nativeFeatures; devtools_measurement_candidate=[bool]$EnableDevTools; controlled_save_fault_candidate=[bool]$EnableNfr06SaveFaults; checks=@($checks); resource_mappings=@($configuration.bundle.resources.PSObject.Properties).Count; resource_files=@($resourceFiles); dependency_source_refs=@('Cargo.lock','package-lock.json','apps/planner/src-tauri/tauri.conf.json'); WebView2='Required Windows runtime; actual tested version recorded separately'; actual_runtime=$null; installer_created=$false; human_acceptance=$false; whole_task_done=$false } |
    ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $packageRoot 'portable-build.json') -Encoding utf8
Write-Output $packageRoot
