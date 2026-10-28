param(
    [Parameter(Mandatory=$true)][int]$TargetProcessId,
    [Parameter(Mandatory=$true)][string]$ExpectedExecutable,
    [string]$FFmpeg = 'D:\MathModelingTools\envs\cumcm\Lib\site-packages\imageio_ffmpeg\binaries\ffmpeg-win-x86_64-v7.1.exe',
    [string]$OutputFile,
    [int]$Seconds = 20,
    [switch]$Capture
)
# Without -Capture, prints geometry only. Run after the shared Native lease is returned.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class SFVideoWindow {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h,ref POINT p);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h,IntPtr a,int x,int y,int w,int z,uint f);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h,int c);
  [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
}
'@
[void][SFVideoWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$process = Get-Process -Id $TargetProcessId
if ($process.Path -ine (Resolve-Path -LiteralPath $ExpectedExecutable).Path) { throw '目标进程路径与候选不符。' }
$handle = $process.MainWindowHandle
if ($handle -eq [IntPtr]::Zero) { throw '目标没有可用主窗口。' }
$screens = [Windows.Forms.Screen]::AllScreens
$screen = if ($screens.Count -gt 1) { $screens | Where-Object { -not $_.Primary } | Select-Object -First 1 } else { $screens[0] }
$area = $screen.WorkingArea
if ($Capture) {
    if (-not $OutputFile) { throw '录制需要 -OutputFile。' }
    if (Test-Path -LiteralPath $OutputFile) { throw '不覆盖已有录片。' }
    [void][SFVideoWindow]::ShowWindow($handle,9)
    [void][SFVideoWindow]::SetWindowPos($handle,[IntPtr]::Zero,$area.Left,$area.Top,$area.Width,$area.Height,0x0014)
    Start-Sleep -Milliseconds 500
}
$rect = New-Object 'SFVideoWindow+RECT'
$point = New-Object 'SFVideoWindow+POINT'
if (-not [SFVideoWindow]::GetClientRect($handle,[ref]$rect)) { throw '无法获取客户区。' }
if (-not [SFVideoWindow]::ClientToScreen($handle,[ref]$point)) { throw '无法获取客户区屏幕坐标。' }
$width = $rect.R-$rect.L
$height = $rect.B-$rect.T
if ($width -le 16 -or $height -le 16) { throw '目标窗口几何无效。' }
[pscustomobject]@{
    mode = $(if ($Capture) { 'capture' } else { 'plan_only' })
    target_pid = $TargetProcessId; executable = $process.Path
    primary_monitor = ($screens | Where-Object Primary).DeviceName
    selected_monitor = $screen.DeviceName; client_x = $point.X; client_y = $point.Y
    client_width = $width; client_height = $height; seconds = $Seconds
} | ConvertTo-Json
if (-not $Capture) { return }
# Region capture avoids the observed frozen WebView title-capture frame. Never attach user Chrome.
& $FFmpeg -nostdin -hide_banner -n -f gdigrab -draw_mouse 1 -framerate 30 -offset_x $point.X -offset_y $point.Y -video_size "${width}x${height}" -i desktop -t $Seconds -an -vf 'pad=ceil(iw/2)*2:ceil(ih/2)*2' -c:v libx264 -preset veryfast -crf 18 -pix_fmt yuv420p -movflags +faststart $OutputFile
if ($LASTEXITCODE -ne 0) { throw "FFmpeg录制失败：$LASTEXITCODE" }
