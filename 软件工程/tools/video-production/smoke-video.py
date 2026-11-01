"""Only exercises local media tools; never captures a desktop or product."""
import argparse
import json
import re
import shutil
import subprocess
import wave
from datetime import datetime, timezone
from pathlib import Path


def run(args, cwd=None):
    result = subprocess.run(args, cwd=cwd, capture_output=True, encoding='utf-8', errors='replace')
    if result.returncode:
        raise RuntimeError(f'{args[0]} exited {result.returncode}:\n{result.stdout}\n{result.stderr}')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', required=True)
    parser.add_argument('--ffmpeg', default=r'D:\MathModelingTools\envs\cumcm\Lib\site-packages\imageio_ffmpeg\binaries\ffmpeg-win-x86_64-v7.1.exe')
    args = parser.parse_args()
    out = Path(args.output_dir).resolve()
    out.mkdir(parents=True, exist_ok=False)
    narration = '这是超级工厂的视频工具测试。画面、字幕和旁白用于检查制作环境，不是产品运行实录。'
    (out / 'narration.txt').write_text(narration, encoding='utf-8')
    run(['pwsh.exe', '-NoProfile', '-File', str(Path(__file__).with_name('synthesize-narration.ps1')),
         '-TextFile', str(out / 'narration.txt'), '-OutputWav', str(out / 'narration.wav')])
    with wave.open(str(out / 'narration.wav'), 'rb') as wav:
        duration = wav.getnframes() / wav.getframerate()
        sample_rate = wav.getframerate()
    # A local test font copy keeps Windows drive colons out of filter syntax.
    shutil.copyfile(r'C:\Windows\Fonts\msyh.ttc', out / 'font.ttc')
    (out / 'title.txt').write_text('超级工厂 · 视频工具检查\n工具测试片，非产品实录', encoding='utf-8')
    (out / 'captions.srt').write_text(
        '1\n00:00:00,000 --> 00:00:05,800\n这是超级工厂的视频工具测试。\n\n'
        '2\n00:00:05,800 --> 00:00:20,000\n检查字幕、旁白与编码；不作为产品验收画面。\n', encoding='utf-8')
    vf = ("drawbox=x=230:y=270:w=14:h=360:color=0xff8a22:t=fill,"
          "drawtext=fontfile=font.ttc:textfile=title.txt:fontcolor=0xecf2f4:fontsize=58:"
          "line_spacing=28:x=290:y=340,"
          "subtitles=captions.srt:force_style='FontName=Microsoft YaHei,FontSize=24,"
          "PrimaryColour=&H00FFFFFF,OutlineColour=&H00201B15,BorderStyle=1,Outline=2,MarginV=46'")
    ff = args.ffmpeg
    encoded = run([ff, '-nostdin', '-hide_banner', '-n', '-f', 'lavfi', '-i',
                   'color=c=0x142833:s=1920x1080:r=30', '-i', 'narration.wav',
                   '-vf', vf, '-c:v', 'libx264', '-preset', 'veryfast', '-crf', '22',
                   '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-b:a', '160k',
                   '-t', str(round(duration + 0.3, 3)), '-movflags', '+faststart', 'tool-smoke.mp4'], out)
    (out / 'encode.log').write_text(encoded.stderr, encoding='utf-8')
    decoded = run([ff, '-nostdin', '-hide_banner', '-i', 'tool-smoke.mp4',
                   '-progress', 'pipe:1', '-f', 'null', '-'], out)
    (out / 'decode.log').write_text(decoded.stdout + '\n' + decoded.stderr, encoding='utf-8')
    run([ff, '-nostdin', '-hide_banner', '-n', '-ss', '2', '-i', 'tool-smoke.mp4',
         '-frames:v', '1', '-update', '1', 'preview.png'], out)
    frame_counts = re.findall(r'^frame=(\d+)', decoded.stdout, re.MULTILINE)
    times = re.findall(r'^out_time_us=(\d+)', decoded.stdout, re.MULTILINE)
    receipt = {
        'checked_at_utc': datetime.now(timezone.utc).isoformat(),
        'scope': 'synthetic_media_tool_smoke_only', 'product_capture': False,
        'ffmpeg': ff, 'video': str(out / 'tool-smoke.mp4'),
        'resolution': [1920, 1080], 'fps': 30,
        'video_codec': 'h264', 'audio_codec': 'aac', 'pixel_format': 'yuv420p',
        'sapi_voice_language': 'zh-CN', 'narration_sample_rate': sample_rate,
        'narration_duration_seconds': round(duration, 3),
        'decoded_frames': int(frame_counts[-1]),
        'decoded_video_duration_seconds': int(frame_counts[-1]) / 30,
        'progress_out_time_seconds': int(times[-1]) / 1_000_000,
        'encode_exit_code': 0, 'full_decode_exit_code': 0,
        'preview': str(out / 'preview.png'),
        'visual_check': 'pending', 'auditory_check': 'not_performed',
        'not_evidence_for': ['product_function', 'desktop_capture', 'Remotion_render', 'final_film']
    }
    (out / 'receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(receipt, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
