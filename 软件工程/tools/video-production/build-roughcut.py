"""Edit the first real Native recording batch; no generated product imagery."""
import argparse
import json
import re
import subprocess
import wave
from pathlib import Path


def run(args, cwd=None):
    result = subprocess.run(args, cwd=cwd, capture_output=True, encoding='utf-8', errors='replace')
    if result.returncode:
        raise RuntimeError(result.stderr + result.stdout)
    return result


def timestamp(seconds):
    ms = round(seconds * 1000)
    return f'{ms // 3600000:02}:{ms // 60000 % 60:02}:{ms // 1000 % 60:02},{ms % 1000:03}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('recording_root')
    args = parser.parse_args()
    root = Path(args.recording_root).resolve()
    out = root / 'roughcut-01'
    out.mkdir(exist_ok=False)
    ff = r'D:\MathModelingTools\envs\cumcm\Lib\site-packages\imageio_ffmpeg\binaries\ffmpeg-win-x86_64-v7.1.exe'
    voice = Path(__file__).with_name('synthesize-narration.ps1').resolve()
    chapters = [
        ('01-template-and-production.mp4', 0, 12, '模板建厂', '从模板建立纸包装演示工程，预览将创建的厂房、设备和生产方案。'),
        ('01-template-and-production.mp4', 18, 18, '本地方案运行', '采用模板后启动本地方案。工厂中有九十二台设备模型，已经联线的路线与独立演示单元分别记录。'),
        ('02-factory-and-timeline.mp4', 0, 28, '全景与时间轴', '本次实际运行包含八类订单，累计接受一百件。工厂全景帮助观察分区，底部时间轴让工序与等待保持同一时间关系。这里展示实际运行记录的回放。'),
        ('03-device-production.mp4', 2, 20, '设备阶段连续回放', '选中纸袋筒成型机，连续回放加工阶段。观察时点和设备机构一起变化，画面来自这一次运行，没有用生成动画替代。'),
        ('04-save-project.mp4', 0, 9, '保存演示工程', '将演示工程另存到专用目录，原工程和旧运行记录保留。'),
        ('05b-restored-run.mp4', 0, 22, '新进程恢复同一工程', '重新启动同一稳定程序，打开刚保存的工程，选择原来的运行记录。布局与结果完整恢复，全部运行数据已经核对一致。这是本地流程的素材粗剪，其余镜头继续补录。'),
    ]
    srt, ass_events, manifest, concat = [], [], [], []
    elapsed = 0
    for index, (source, start, seconds, title, narration) in enumerate(chapters, 1):
        text = out / f'{index:02}.txt'
        wav = out / f'{index:02}.wav'
        text.write_text(narration, encoding='utf-8')
        run(['pwsh.exe', '-NoProfile', '-File', str(voice), '-TextFile', str(text), '-OutputWav', str(wav)])
        with wave.open(str(wav), 'rb') as audio:
            voice_seconds = audio.getnframes() / audio.getframerate()
        if voice_seconds > seconds:
            raise RuntimeError(f'Chapter{index} narration exceeds real clip duration: {voice_seconds}>{seconds}')
        vf = 'scale=1920:972:force_original_aspect_ratio=decrease:force_divisible_by=2,pad=1920:1080:(ow-iw)/2:0:color=0x142833,setsar=1'
        part = out / f'{index:02}.mp4'
        result = run([ff, '-nostdin', '-hide_banner', '-n', '-ss', str(start), '-i', str(root/source), '-i', str(wav),
                      '-map', '0:v:0', '-map', '1:a:0', '-t', str(seconds), '-vf', vf, '-r', '30',
                      '-c:v', 'libx264', '-preset', 'veryfast', '-crf', '20', '-pix_fmt', 'yuv420p',
                      '-af', 'apad', '-c:a', 'aac', '-b:a', '160k', '-movflags', '+faststart', str(part)])
        (out / f'{index:02}.log').write_text(result.stderr, encoding='utf-8')
        concat.append(f"file '{part.name}'")
        manifest.append({'chapter': title, 'source_clip': str(root/source), 'source_start_seconds': start,
                         'duration_seconds': seconds, 'output_start_seconds': elapsed,
                         'narration': narration, 'voice_seconds': round(voice_seconds, 3), 'playback_speed': 1})
        # SRT breaks follow real recorded chapter timing; bottom band preserves all UI fields.
        sentences = [s+'。' for s in narration.split('。') if s]
        cursor = elapsed
        length = sum(len(s) for s in sentences)
        for sentence in sentences:
            span = seconds * len(sentence) / length
            lines = '\n'.join(sentence[i:i+22] for i in range(0, len(sentence), 22))
            srt.append(f'{len(srt)+1}\n{timestamp(cursor)} --> {timestamp(cursor+span)}\n{lines}\n')
            ass_start = timestamp(cursor).replace(',', '.')[:-1]
            ass_end = timestamp(cursor+span).replace(',', '.')[:-1]
            ass_events.append(f'Dialogue: 0,{ass_start},{ass_end},Default,,0,0,0,,'+lines.replace('\n', r'\N'))
            cursor += span
        elapsed += seconds
    (out/'concat.txt').write_text('\n'.join(concat)+'\n', encoding='utf-8')
    (out/'captions.srt').write_text('\n'.join(srt), encoding='utf-8')
    ass_header = ('[Script Info]\nScriptType: v4.00+\nPlayResX: 1920\nPlayResY: 1080\nWrapStyle: 2\n'
                  '[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n'
                  'Style: Default,Microsoft YaHei,32,&H00FFFFFF,&H00FFFFFF,&H00332814,&H00332814,0,0,0,0,100,100,0,0,1,1,0,2,40,40,18,1\n'
                  'Style: Badge,Microsoft YaHei,20,&H00228AFF,&H00228AFF,&H00332814,&H00332814,0,0,0,0,100,100,0,0,1,0,0,1,20,20,18,1\n'
                  '[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n')
    ass_events.append(f'Dialogue: 0,00:00:00.00,{timestamp(elapsed).replace(",", ".")[:-1]},Badge,,0,0,0,,素材粗剪 · 模板 1.6.0')
    (out/'captions.ass').write_text(ass_header+'\n'.join(ass_events)+'\n', encoding='utf-8')
    run([ff, '-nostdin', '-hide_banner', '-n', '-f', 'concat', '-safe', '0', '-i', 'concat.txt', '-c', 'copy', 'joined.mp4'], out)
    final = out/'superfactory-native-demo-roughcut-01.mp4'
    result = run([ff, '-nostdin', '-hide_banner', '-n', '-i', 'joined.mp4',
                  '-vf', 'ass=captions.ass',
                  '-c:v', 'libx264', '-preset', 'veryfast', '-crf', '20', '-pix_fmt', 'yuv420p',
                  '-c:a', 'copy', '-movflags', '+faststart', final.name], out)
    (out/'final-encode.log').write_text(result.stderr, encoding='utf-8')
    decoded = run([ff, '-nostdin', '-hide_banner', '-i', str(final), '-progress', 'pipe:1', '-f', 'null', '-'])
    (out/'full-decode.log').write_text(decoded.stdout+'\n'+decoded.stderr, encoding='utf-8')
    run([ff, '-nostdin', '-hide_banner', '-n', '-ss', '67', '-i', str(final), '-frames:v', '1', '-update', '1', str(out/'preview.png')])
    receipt = {'scope': 'first_actual_Native_materials_roughcut_only', 'video': str(final), 'captions': str(out/'captions.srt'),
               'source_candidate': json.loads((root/'native-01/launch.json').read_text(encoding='utf-8-sig'))['exe'],
               'template_version': '1.6.0', 'duration_seconds': elapsed, 'resolution': [1920,1080], 'fps': 30,
               'H264_AAC': True, 'synthetic_product_frames': False, 'chapters': manifest,
               'full_decode_exit_code': 0, 'decoded_frames': int(re.findall(r'^frame=(\d+)', decoded.stdout,re.M)[-1]),
               'visual_check': 'pending', 'auditory_check': 'pending', 'final_six_minute_film_complete': False,
               'remaining': ['cloud_and_maintenance', 'Agent', 'professional_calculation', 'full_comparison_reports', 'final_candidate_reconciliation']}
    (out/'receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    print(json.dumps({'video': str(final), 'seconds': elapsed, 'full_decode_exit_code': 0}, ensure_ascii=False))


if __name__ == '__main__':
    main()
