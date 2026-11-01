param(
    [Parameter(Mandatory=$true)][string]$TextFile,
    [Parameter(Mandatory=$true)][string]$OutputWav,
    [int]$Rate = 0
)
$ErrorActionPreference = 'Stop'
$text = [IO.File]::ReadAllText((Resolve-Path -LiteralPath $TextFile).Path, [Text.Encoding]::UTF8)
if (Test-Path -LiteralPath $OutputWav) { throw "输出已存在：$OutputWav" }
$voice = New-Object -ComObject SAPI.SpVoice
$voices = $voice.GetVoices()
$chinese = $null
for ($i=0; $i -lt $voices.Count; $i++) {
    $item = $voices.Item($i)
    if (($item.GetAttribute('Language') -split ';') -contains '804') { $chinese = $item; break }
}
if ($null -eq $chinese) { throw '没有简体中文 SAPI 语音；请先安装 Windows 中文语音。' }
$stream = New-Object -ComObject SAPI.SpFileStream
try {
    $voice.Voice = $chinese
    $voice.Rate = $Rate
    $stream.Format.Type = 22 # SAFT22kHz16BitMono
    $stream.Open([IO.Path]::GetFullPath($OutputWav), 3, $false)
    $voice.AudioOutputStream = $stream
    [void]$voice.Speak($text)
} finally {
    $stream.Close()
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($stream)
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($voice)
}
Get-Item -LiteralPath $OutputWav | Select-Object FullName,Length
