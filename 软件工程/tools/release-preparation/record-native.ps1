# Run only after the coordinator releases the selected Native UI for recording.
# C12 WebView2 title capture froze on the first frame; use the observed window
# rectangle with Desktop Duplication and keep that window visible during capture.
param(
    [Parameter(Mandatory=$true)][string]$FfmpegExe,
    [Parameter(Mandatory=$true)][string]$WindowTitle,
    [Parameter(Mandatory=$true)][string]$OutputFile,
    [Parameter(Mandatory=$true)][int]$Seconds,
    [int[]]$CaptureRegion
)
$ErrorActionPreference = 'Stop'
if (-not [System.IO.File]::Exists($FfmpegExe)) { throw 'Use an already installed FFmpeg executable.' }
if ($Seconds -le 0) { throw 'Seconds must be positive.' }
$taskOutput = [System.IO.Path]::GetFullPath($OutputFile)
# One named window, visible pointer, no microphone/desktop audio, no overwrite.
# Padding preserves the whole picture when the window has an odd pixel size.
$taskInput = @('-f','gdigrab','-framerate','30','-draw_mouse','1','-i',"title=$WindowTitle")
if ($CaptureRegion) {
    if ($CaptureRegion.Count -ne 4 -or $CaptureRegion[2] -le 0 -or $CaptureRegion[3] -le 0) {
        throw 'CaptureRegion must be observed output-0-relative x,y,width,height.'
    }
    $taskInput = @('-f','lavfi','-i',("ddagrab=output_idx=0:framerate=30:offset_x={0}:offset_y={1}:video_size={2}x{3},hwdownload,format=bgra" -f $CaptureRegion[0],$CaptureRegion[1],$CaptureRegion[2],$CaptureRegion[3]))
}
$taskArgs = @('-hide_banner','-nostats','-n') + $taskInput + @('-t',"$Seconds",'-vf','pad=ceil(iw/2)*2:ceil(ih/2)*2',
    '-c:v','libx264','-preset','medium','-crf','20','-pix_fmt','yuv420p',
    '-movflags','+faststart',$taskOutput)
& $FfmpegExe @taskArgs
if ($LASTEXITCODE -ne 0) { throw "FFmpeg capture failed: $LASTEXITCODE" }
