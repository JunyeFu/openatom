"""Package real shared-consumer screenshots, with reproducible crop and exact editable model dependency."""
from pathlib import Path
from PIL import Image
import json,shutil
root=Path(__file__).resolve().parent
runtime=root.parents[2]/'scenarios/媒体表现/runtime-line-grade'
inventory=json.loads((root/'inventory.json').read_text(encoding='utf-8'))
anchors={v['family']:v['anchors'] for v in json.loads((runtime/'image-anchor-capture.json').read_text(encoding='utf-8'))}
packages=[]
for family in ['assembly','rework','packing','robot','conveyor','inspection','sealer','pallet']:
    target=root/'installable'/f'{family}-image';(target/'source').mkdir(parents=True,exist_ok=True)
    source=runtime/'views'/f'{family}-iso-full.png'
    canvas=Image.open(source).convert('RGB').crop((19,248,976,864))
    canvas.save(target/'view.png');canvas.save(target/'source/consumer-canvas.png')
    prior=target/'source/full-consumer.png'
    if prior.exists():prior.unlink()  # own obsolete package copy; original full evidence remains in runtime
    source_info={'model':{'id':f'p302.line-grade.{family}','version':'0.1.0'},'consumer':'frozen MediaPresentationView','camera':'shared fit / nominal iso','phase_ref':'auto','view_u':0,'crop_css_pixels':[19,248,976,864],'actual_run':False,'reference_gauge':False}
    (target/'source/capture.json').write_text(json.dumps(source_info,indent=2),encoding='utf-8')
    (target/'source/crop.py').write_text("from pathlib import Path\nfrom PIL import Image\np=Path(__file__).resolve().parent\nImage.open(p/'consumer-canvas.png').convert('RGB').save(p.parent/'view.png')\n",encoding='utf-8')
    shutil.copy2(root/'LICENSE.txt',target/'LICENSE.txt')
    asset={'id':f'p302.line-grade.{family}.image.png','path':'view.png','kind':'png'}
    rights=[{'asset_ref':asset['id'],'source':'Original self-authored model rendered by frozen shared consumer; source/consumer-canvas.png, capture.json and crop.py; editable geometry is exact module dependency.','license':'CC0-1.0','redistribution_confirmed':True,'missing_reason':None}]
    presentation=json.loads((root/'installable'/family/'module.json').read_text(encoding='utf-8'))['data']['definition']['payload']['data']
    presentation.update(kind='image',source={'kind':'asset','data':asset},nodes=[],poses=[],actions=[],timeline=[],lod={'detailed':None,'simplified':{'kind':'asset','data':asset},'region':'device_icons'})
    presentation['tags'][0]['label']=f'{family} / static GLB view'
    presentation['tags'][0]['payload']['data']='原创GLB声明姿态的静态导出；静态图不表示实际工时、动作或库存。'
    for container in ['anchor_roles','port_anchors']:
        presentation[container]={k:{'kind':'viewport','data':{'x':a['x'],'y':a['y']}} for k in presentation[container] for a in anchors[family] if a['name']==k and 0<=a['x']<=1 and 0<=a['y']<=1}
    files=['view.png','source/consumer-canvas.png','source/capture.json','source/crop.py','LICENSE.txt']
    pkg={'assets':[asset],'definition':{'id':f'p302.line-grade.{family}.image','version':'0.1.0','title':f'P302 原创 {family} 同模型静态图','definition_state':'runtime_candidate','contribution':{'author':'P302 line-grade original author','license':'CC0-1.0','assets':rights},'parameters':[],'attribute_schema':[],'payload':{'category':'media','data':presentation}},'files':[{'path':p,'kind':'asset'} for p in files],'dependencies':[source_info['model']],'rights':rights,'issues':[]}
    (target/'module.json').write_text(json.dumps({'schema_version':'1.0','data':pkg},ensure_ascii=False,indent=2),encoding='utf-8')
    packages.append({'id':pkg['definition']['id'],'version':'0.1.0','directory':f'installable/{family}-image','source_model':source_info['model'],'asset':asset})
inventory['image_packages']=packages
(root/'inventory.json').write_text(json.dumps(inventory,ensure_ascii=False,indent=2),encoding='utf-8')
print(f'{len(packages)} canonical static-image packages from actual shared-consumer captures')
