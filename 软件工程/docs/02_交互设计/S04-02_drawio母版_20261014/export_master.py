"""顺序等待draw.io导出，避免GUI程序并行缓存争用造成旧图混入。"""
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as E

D=Path(__file__).resolve().parent
CLI=r'D:/MathModelingTools/software/drawio-31.3.2/draw.io.exe'
NAMES=['母版_精细化预览','左侧收起_预览','右侧收起_预览','两侧收起_预览','面板调宽_预览','设备字段_精细化预览','未应用离开_预览','时间轴隐藏_预览','描述隐藏_预览','底部双面板隐藏_预览','窗口菜单_预览','时间轴隐藏菜单_预览','描述隐藏菜单_预览','双面板隐藏菜单_预览']
NAMES+=['主轴展开_预览','流水线细化_预览','设备运行缺项_预览','输入吸附联动_预览','多厂房供货_预览','设备表现创作_预览']
NAMES+=['简化档_预览','区域档_预览','视图菜单_预览','修订冲突_预览','多阶段表现_预览','文字放大_预览']
SOURCE=D/'industrial-workbench-master.drawio'
assert len(NAMES)==len(E.parse(SOURCE).getroot().findall('diagram'))
indices=[int(x) for x in sys.argv[1:]] if len(sys.argv)>1 else list(range(1,len(NAMES)+1))
for index in indices:
    name=NAMES[index-1]
    output=D/(name+'.png')
    result=subprocess.run([CLI,'-x','-f','png','-p',str(index),'--width','2400','-o',str(output),str(SOURCE)],capture_output=True,text=True,encoding='utf-8',creationflags=0x08000000)
    if result.returncode!=0: raise RuntimeError(f'{name}: '+result.stderr)
    if output.stat().st_mtime<SOURCE.stat().st_mtime: raise RuntimeError('Stale export: '+name)
    print(f'{index}/{len(NAMES)}: {name}',flush=True)
