"""Add the bag SKU/material design to the original factory control diagram."""
import json
import xml.etree.ElementTree as ET
from pathlib import Path
from prepare_full_factory import ROOT, DESIGN, ART, Drawing, read, write


def specification():
    area = .24 * (2 * (.18 + .08) + .01)
    cord_volume = 3 * .36 * .006 * .0004
    patch_volume = .020 * .025 * .001
    return {
        'kind': 'explicit_demonstration_bag_sku', 'version': '1.0.0', 'package': 'V2-05-G01',
        'state': 'declared_design_inputs',
        'source_project': 'families-host03 actual frozen inputs; template1.1.0 remains retained',
        'hall_geometry_changed': False, 'factory_device_count': 92,
        'AI_views': 'Existing same-layout three views remain applicable; no hall/zone/equipment placement changed.',
        'bag': {'width_m': .18, 'depth_m': .08, 'height_m': .20, 'tube_height_m': .24,
                'bottom_fold_m': .04, 'seam_overlap_m': .01, 'web_length_m': .53, 'web_width_m': .24,
                'paper_thickness_m': .0006, 'paper_area_m2': area, 'paper_volume_m3': area * .0006},
        'handles': {'cords_per_pair': 2, 'cord_length_m': .30, 'strips_per_cord': 3,
                    'strip_length_m': .36, 'strip_width_m': .006, 'strip_thickness_m': .0004,
                    'patches_per_pair': 4, 'patch_size_m': [.020, .025, .001],
                    'patch_adhesive': 'purchased_precoated', 'cord_paper_volume_m3': cord_volume,
                    'pair_paper_volume_m3': 2 * cord_volume + 4 * patch_volume},
        'final_paper_volume_m3': area * .0006 + 2 * cord_volume + 4 * patch_volume,
        'body_route': ['paper-bag-tuber', 'manual-packing:flatten_gussets', 'paper-bag-bottomer',
                       'manual-packing:open_bag', 'bag-handle-attacher'],
        'handle_route': ['rope-handle-twister:cord_a', 'rope-handle-twister:cord_b', 'manual-packing:prepare_handle_pair'],
        'shared_manual_station': {'id': 'PF-manual-packing', 'capacity': 1, 'operator_capacity': 1,
                                 'also_serves_existing_case_route': True, 'new_manual_jobs': 9,
                                 'manual_cycle_ms': {'load': 4000, 'auto': 12000, 'unload': 4000}},
        'device_tooling': 'Keep exact accepted device modules. Explicit presentation_override uses mature tuber/bottomer geometry with the configured SKU dimensions, detailed and simplified. Do not scale the whole device or replace accepted geometry in place.',
        'resources': {'vehicles': 3, 'demonstration_window_ms': 7200000},
        'expected': {'bags': 3, 'body_webs': 3, 'strip_lots': 6, 'strips': 18, 'cords': 6, 'patches': 12,
                     'handle_pairs': 3, 'new_manufacturing_jobs': 24, 'total_manufacturing_jobs': 145,
                     'new_transport_trips': 24, 'total_transport_trips': 78,
                     'accepted_all_products': 33, 'connected_devices': 19, 'independent_cells': 73},
        'acceptance': ['No bag or handle components initially supplied as finished products',
                       'One web + two cords/six strips + four patches in each final bag ancestry',
                       'Paper volume conserved through forming, folding, opening and handle assembly',
                       'Only one shared manual station/operator; resource intervals never overlap',
                       'Actual unload/release acceptance, saved project and prior Run exact reopen',
                       'Real material/device configured geometry and phase clocks; current Native visual remains pending'],
    }


def attach_diagram(path, spec):
    dr = Drawing(); dr.file = ET.parse(path).getroot(); dr.counter = 9000
    for page in list(dr.file):
        if page.get('id') == 'bag-line-detail': dr.file.remove(page)
    dr.page('06 纸袋联线与物料设计', 2020, 1180)
    dr.file[-1].set('id', 'bag-line-detail')
    dr.box('纸袋联线｜明确规格、展平开袋与提手预制', 60, 25, 1880, 60, '#ffffff', 29, 'strokeColor=none;fontStyle=1;')
    dr.box('沿用同一厂房、12区与92台设备位置；本页是开发设计，运行结果另由实际Run验证。', 60, 90, 1880, 45, '#ffffff', 19, 'strokeColor=none;align=left;')
    stages = [
        ('raw', '计量原纸\n530×240 mm\n3段；来料已放行', '#edf3f8'),
        ('tube', '制袋筒 · Z06\n180×80×240 mm\n围模搭接；1 mL胶', '#f5dfd7'),
        ('flat', '侧褶展平 · Z10\n现有人工台\n折入侧褶；不裁掉纸张', '#dcebd9'),
        ('bottom', '折底糊合 · Z06\n40 mm折底量\n糊底；1 mL胶', '#f5dfd7'),
        ('open', '开袋 · Z10\n现有人工台\n180×80×200 mm袋身', '#dcebd9'),
        ('attach', '粘接双提手 · Z06\n1副提手＋2 mL胶\n不是两条直绳直接上袋', '#f5dfd7'),
        ('ship', '合格纸袋入库\n3只双提手纸袋\n卸货并放行后计数', '#dcebd9'),
    ]
    previous = None; ids = {}
    for i, (key, title, fill) in enumerate(stages):
        ids[key] = dr.box(title, 60 + 275 * i, 170, 220, 140, fill, 18, key='BAG-' + key)
        if previous: dr.edge(previous, ids[key], '实物转运')
        previous = ids[key]
    dr.box('提手支线｜每袋两条纸绳、四片补强片，先成一副提手再送往粘接机', 60, 425, 1880, 60, '#ffffff', 23, 'strokeColor=none;align=left;')
    strips = dr.box('预裁纸条\n6组×3条＝18条\n每条360×6×0.4 mm', 60, 550, 280, 140, '#edf3f8', 19, key='BAG-strips')
    rope = dr.box('纸绳捻制 · Z06\n6次作业→6条纸绳\n成绳长度300 mm', 505, 550, 280, 140, '#f5dfd7', 19, key='BAG-rope')
    prepare = dr.box('弯绳贴片 · Z10\n同一人工台；3次作业\n2条绳＋4片→1副提手', 1010, 550, 280, 140, '#dcebd9', 19, key='BAG-prepare')
    pair = dr.box('3副预制双绳柄\n保留6条绳和12片身份\n一副供一只纸袋', 1435, 550, 280, 140, '#edf3f8', 19, key='BAG-pair')
    patches = dr.box('预涂胶纸质补强片\n12片；20×25×1 mm\n明确采购来料', 1010, 790, 280, 130, '#edf3f8', 19, key='BAG-patches')
    for a, b, text in [(strips, rope, '3条/绳'), (rope, prepare, '2条/托'), (prepare, pair, '1副/批'),
                       (patches, prepare, '4片/副'), (pair, ids['attach'], '1副/袋')]: dr.edge(a, b, text)
    dr.box('共用资源：人工装箱台仍只有1台、1个工位容量、1名操作员。原装箱3次＋纸袋展平3次＋开袋3次＋提手预制3次，共12次人工台作业；必须由同一资源池排队。', 60, 970, 1880, 75, '#ffffff', 20, 'align=left;')
    dr.box('独立预期：新增24次制造、24趟运输、3只纸袋；整厂145次制造、78趟运输、33件合格成品。3辆运输车；两小时演示工作窗。纸张不因折叠而消失，运输次数不计为成品。', 60, 1070, 1880, 70, '#ffffff', 19, 'align=left;')
    dr.save(path)


if __name__ == '__main__':
    spec = specification(); write(DESIGN / 'bag-line-spec.json', spec)
    attach_diagram(ART / '纸包装整厂区域与设备分配.drawio', spec)
    print(json.dumps({'design': str(DESIGN / 'bag-line-spec.json'), 'drawio_page': 6, 'expected': spec['expected']}, ensure_ascii=False))
