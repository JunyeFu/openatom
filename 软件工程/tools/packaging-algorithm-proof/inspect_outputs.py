"""独立读取实际SVG；检查毫米、图层、几何长度/面积和连接来源。"""
from pathlib import Path
from fractions import Fraction as F
import json
import re
import xml.etree.ElementTree as ET

OUT=Path(__file__).resolve().parent/'output'
NS={'s':'http://www.w3.org/2000/svg'}
def segments(path):
    root=ET.parse(path).getroot();assert root.attrib['width'].endswith('mm') and root.attrib['height'].endswith('mm')
    result={}
    for group in root.findall('s:g',NS):
        values=[]
        for p in group.findall('s:path',NS):
            m=re.fullmatch(r'M ([-\d.]+) ([-\d.]+) L ([-\d.]+) ([-\d.]+)',p.attrib['d']);assert m
            values.append((tuple(map(F,m.groups()[:2])),tuple(map(F,m.groups()[2:])),p.attrib))
        result[group.attrib['id']]=values
    assert set(result)=={'cut','fold','glue'}
    return result
def length(edges):return sum(abs(a[0]-b[0])+abs(a[1]-b[1]) for a,b,_ in edges)
def main():
    source=json.loads((OUT/'manufacturing-source-geometries.json').read_text(encoding='utf-8'))
    first=segments(OUT/'MAN01.svg'); assert length(first['cut'])==64 and length(first['fold'])==10
    ids={row['id'] for row in source['MAN01']['rectangle_sources']}
    for _,_,attrs in first['cut']:
        used=[v for v in (attrs['data-panel-refs']+','+attrs['data-fold-band-refs']).split(',') if v]
        assert used and set(used)<=ids
    assert all(attrs['data-fold-band-ref']=='FB1' and attrs['data-parent-panel-ref']=='MAN01_BASE' and attrs['data-child-panel-ref']=='MAN01_WALL' and attrs['data-width-mm']=='2' for _,_,attrs in first['fold'])
    second=segments(OUT/'MAN02.svg')
    area=abs(sum(a[0]*b[1]-b[0]*a[1] for a,b,_ in second['glue']))/2
    assert area==100
    assert all(attrs['data-connection-ref']=='GLUE1' and attrs['data-allowance-ref']=='L1' and attrs['data-active-step-ref']=='approach' for _,_,attrs in second['glue'])
    public=segments(OUT/'PART_A.svg');public_area=abs(sum(a[0]*b[1]-b[0]*a[1] for a,b,_ in public['glue']))/2
    assert public_area==100
    assert all(attrs['data-panel-ref']=='A' and attrs['data-partner-panel-ref']=='B' and attrs['data-connection-ref']=='GLUE1' and attrs['data-allowance-ref']=='L1' and attrs['data-active-step-ref']=='APPROACH' for _,_,attrs in public['glue'])
    result={'evidence_level':'independent_actual_SVG_readback_for_finite_rectangular_examples',
        'cut_length_mm':64,'fold_length_mm':10,'common_glue_area_mm2':100,'actual_public_job_glue_area_mm2':100,'units_and_layers':'passed','cut_source_ids':'passed','glue_connection_allowance_activation_refs':'passed'}
    (OUT/'svg-readback-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result,ensure_ascii=False))
if __name__=='__main__':main()
