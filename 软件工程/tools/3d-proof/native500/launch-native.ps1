param([string]$Attempt = 'attempt-05',[string]$Pass = 'native-01',[string]$Candidate = '',[string]$Mode = 'render',[string]$SelectionFile = '',[int]$Port=27084,[string]$PlanFile = '')
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$proofRoot = Join-Path $projectRoot '.work/native500-quality-20261025'
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Native500WindowPlacement {
 public delegate bool WindowCallback(IntPtr window, IntPtr state);
 [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
 [DllImport("user32.dll")] static extern bool EnumWindows(WindowCallback callback, IntPtr state);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
 public static IntPtr Find(int targetPid) {
  IntPtr result=IntPtr.Zero; long largest=0;
  EnumWindows((window,state)=>{ uint pid; GetWindowThreadProcessId(window,out pid); Rect rect;
   if(pid==targetPid && GetWindowRect(window,out rect)) { long size=(long)(rect.Right-rect.Left)*(rect.Bottom-rect.Top); if(size>largest) { largest=size;result=window; } }
   return true; },IntPtr.Zero);
  return result;
 }
}
'@
$taskScreens = @([System.Windows.Forms.Screen]::AllScreens)
$targetScreen = if ($taskScreens.Count -gt 1) { $taskScreens | Where-Object { -not $_.Primary } | Select-Object -First 1 } else { $taskScreens[0] }
if (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue) { throw "$Port is already in use; do not attach another process." }
$candidateDir = Join-Path $projectRoot '.work/parallel-closeout-20261024/native/windows-C36'
if ($Candidate) { $candidateDir = Join-Path $projectRoot $Candidate }
$inputPath = Join-Path $proofRoot "$Attempt/saved"
$outputPath = Join-Path $proofRoot $Pass
if (Test-Path -LiteralPath $outputPath) { throw 'Native output must be a new directory.' }
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $proofRoot "$Pass-profile"
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port --remote-debugging-address=127.0.0.1"
$env:SUPERFACTORY_TEST_PYTHON = (Get-Command python).Source
if ($SelectionFile) { $env:SUPERFACTORY_TEST_DIRECTORY_SELECTION = (Resolve-Path -LiteralPath $SelectionFile).Path }
$nativePlan = if ($PlanFile) { (Resolve-Path -LiteralPath $PlanFile).Path } else { Join-Path $proofRoot "$Attempt/native-plan.json" }
$nativeArgs = @('--p3-04-public-render-smoke','--render-input',"`"$inputPath`"",'--render-output',"`"$outputPath`"",'--render-bridge',"`"$(Join-Path $candidateDir 'superfactory-mcp-bridge.exe')`"",'--render-views',"`"$nativePlan`"")
if ($Mode -eq 'ordinary') {
 $process = Start-Process -FilePath (Join-Path $candidateDir 'superfactory.exe') -WorkingDirectory $projectRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $proofRoot "$Pass.stdout.log") -RedirectStandardError (Join-Path $proofRoot "$Pass.stderr.log") -PassThru
} else {
 $process = Start-Process -FilePath (Join-Path $candidateDir 'superfactory.exe') -ArgumentList $nativeArgs -WorkingDirectory $projectRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $proofRoot "$Pass.stdout.log") -RedirectStandardError (Join-Path $proofRoot "$Pass.stderr.log") -PassThru
}
$windowDeadline = (Get-Date).AddSeconds(30)
do {
 $taskWindow = [Native500WindowPlacement]::Find($process.Id)
 if ($taskWindow -eq [IntPtr]::Zero) {
  $process.Refresh()
  if ($process.HasExited) { throw "Owned Native PID $($process.Id) exited before creating its window." }
  Start-Sleep -Milliseconds 100
 }
} until ($taskWindow -ne [IntPtr]::Zero -or (Get-Date) -ge $windowDeadline)
if ($taskWindow -eq [IntPtr]::Zero) { throw "Owned Native PID $($process.Id) remains live but no window is ready for monitor placement." }
$taskRect = New-Object Native500WindowPlacement+Rect
[void][Native500WindowPlacement]::GetWindowRect($taskWindow,[ref]$taskRect)
$workArea = $targetScreen.WorkingArea
$taskWidth = [Math]::Min($taskRect.Right-$taskRect.Left,$workArea.Width)
$taskHeight = [Math]::Min($taskRect.Bottom-$taskRect.Top,$workArea.Height)
$placed = [Native500WindowPlacement]::SetWindowPos($taskWindow,[IntPtr]::Zero,$workArea.Left+[int](($workArea.Width-$taskWidth)/2),$workArea.Top+[int](($workArea.Height-$taskHeight)/2),$taskWidth,$taskHeight,0x14)
$actualScreen = [System.Windows.Forms.Screen]::FromHandle($taskWindow)
if (-not $placed -or $actualScreen.DeviceName -ne $targetScreen.DeviceName) { throw "Owned Native window could not be placed on the selected monitor $($targetScreen.DeviceName)." }
@{pid=$process.Id;candidate=$candidateDir;input=$inputPath;output=$outputPath;profile=$env:WEBVIEW2_USER_DATA_FOLDER;cdp="http://127.0.0.1:$Port";started=(Get-Date -Format o);purpose='Exact portable assets and actual Native Run capture; not formal NFR measurement';mode=$Mode;monitor=@{connected=$taskScreens.Count;selected=$actualScreen.DeviceName;primary=$actualScreen.Primary;placed=$placed}} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $proofRoot "$Pass-launch.json") -Encoding utf8
if ($Mode -eq 'ordinary') {
 $readyDeadline = (Get-Date).AddSeconds(30)
 $nativePageReady = $false
 do {
  $process.Refresh()
  if ($process.HasExited) { throw "Owned Native PID $($process.Id) exited before its page was ready; inspect its logs." }
  try {
   $nativeTabs = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/json/list" -TimeoutSec 1
   $nativePageReady = @($nativeTabs | Where-Object { $_.url -eq 'http://tauri.localhost/' }).Count -gt 0
  } catch { $nativePageReady = $false }
  if (-not $nativePageReady) { Start-Sleep -Milliseconds 250 }
 } until ($nativePageReady -or (Get-Date) -ge $readyDeadline)
 if (-not $nativePageReady) { throw "Owned Native PID $($process.Id) remains live but its page is not ready; inspect this same process before proceeding." }
}
Write-Output "Native PID $($process.Id), own profile, public RenderView on actual main Host."
