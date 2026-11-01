"""Extend the original factory design with a real three-case pallet and wrapping."""
import xml.etree.ElementTree as ET
from prepare_full_factory import DESIGN, ART, Drawing, write


def specification():
    return {
        'kind': 'three_case_pallet_and_wrapping', 'version': '1.0.0', 'package': 'V2-05-G01',
        'source': 'logistics-host02 actual project, scenario and frozen modules; template1.5.0',
        'target_template': '1.6.0', 'hall_geometry_changed': False, 'AI_views_reused': True,
        'devices': ['manual-packing', 'pallet-stretch-wrapper'],
        'flow': 'Actual three cases from vacuum placement -> manual floor pallet loading -> stretch wrapping -> shipping',
        'pallet': {'length_m': 1.2, 'width_m': .8, 'height_m': .12, 'count': 1,
                   'case_count': 3, 'case_type': 'PF-packed-case', 'case_m': [.32, .24, .24],
                   'contents_per_case': 6, 'case_bottom_centers_m': [[-.34, 0, .12], [0, 0, .12], [.34, 0, .12]],
                   'loaded_extent_m': [1.2, .8, .36]},
        'operation': {'builtin': 'pack', 'identity_policy': 'container_with_contents',
                      'input_cases': 3, 'input_empty_pallet': 1, 'output_loaded_pallet': 1,
                      'manual_cycle_ms': {'load': 5000, 'auto': 15000, 'unload': 5000},
                      'resource': 'Existing manual station operator and execution capacity; no concurrent double use',
                      'presentation': 'Floor loading beside the existing manual station; never place a 1.2m pallet on the small case-folding table'},
        'wrapping': {'operation': 'Existing canonical preserve operation', 'film_length_m': 9,
                     'film_width_m': .5, 'film_thickness_m': .000023,
                     'required_state': 'unwrapped', 'final_state': 'wrapped',
                     'identity': 'Same loaded pallet ID and same nested three case/eighteen item IDs'},
        'visual': {'policy': 'Use actual nested case geometry, not six decorative example boxes',
                   'film_envelope': 'Fit to actual one-layer case stack; retain pallet dimensions and device geometry',
                   'lod': 'Simplified default, only nearby primary selection detailed; both variants must show the same payload and state'},
        'station_fixture': {'manual_floor_anchor': 'Named floor_loading scene_node0 in both exact LODs',
                            'manual_floor_offset_m': [2.425, 0, 0],
                            'case_buffer_m': [34.8, 72.3, 0], 'empty_buffer_m': [34.8, 76, 0],
                            'loaded_buffer_m': [34.8, 73.5, 0],
                            'wrapper_carriage_y_m': .18, 'film_center_y_m': .46,
                            'film_vertical_extent_m': [.21, .71], 'box_on_turntable_vertical_extent_m': [.34, .58],
                            'film_material_adaptation': 'MeteredFilmRoll follows low spindle; LiveFilmWeb center0.46m; partial wrap0.24m height; preserve9m length and500mm roll width',
                            'reason': 'Observed missing manual holder anchors and loaded buffer collision; canonical carriage climbed above the one-layer load. Add exact anchors, separate buffer and use the low-load setting without resizing machine geometry.'},
        'stock_policy': 'Remove isolated wrapper load and its six example contents; keep one metered film supply. Supply one empty pallet at loading station. No preloaded replacement pallet.',
        'order_policy': 'Replace PF-ORDER-CASE with PF-ORDER-PALLET=1. Keep the three-case upstream production task without a direct shipping order. Other seven orders unchanged.',
        'expected': {'manufacturing_jobs': 162, 'transport_trips': 119,
                     'accepted_orders': {'PF-ORDER-PALLET': 1, 'PF-ORDER-SLEEVE': 3, 'PF-ORDER-TUBE': 6,
                                         'PF-ORDER-LABEL': 18, 'PF-ORDER-BAG': 3, 'PF-ORDER-SHEETS': 60,
                                         'PF-ORDER-BOARDS': 6, 'PF-ORDER-CURED': 3},
                     'connected_devices': 31, 'independent_cells': 61},
        'expected_derivation': '161 - 1 isolated wrapping + 1 actual wrapping + 1 loading =162; redirect final three case trips to loading and add two pallet trips =119. The manual station was already connected; wrapper adds one connected device.',
        'display_units': '99 other products plus 1 pallet containing 3 cases and 18 inner items; do not compare mixed-unit sum100 with former102 as a yield loss.',
        'negative_cases': {'six_inner_items_missing': {'accepted_pallets': 0, 'reason': 'Only two upstream cases; three required'},
                           'film_missing': {'accepted_pallets': 0, 'reason': 'Loaded pallet remains unwrapped'},
                           'empty_pallet_missing': {'accepted_pallets': 0, 'reason': 'No container material for packing'}},
        'acceptance': ['Three actual cases and eighteen inner identities retained after packing and wrapping',
                       'One metered film consumed only by actual wrapping', 'Exact eight order quantities and full return trips',
                       'Short case, missing film and missing empty pallet cannot produce a shipped pallet',
                       'Exact Host save and fresh reopen',
                       'Native floor loading, wrapped film, nested cargo motion, no overlaps and both LODs',
                       'Ordinary template adoption and saved-project reopen before promoting1.6.0'],
    }


def attach_diagram(path, spec):
    dr = Drawing(); dr.file = ET.parse(path).getroot(); dr.counter = 13000
    for page in list(dr.file):
        if page.get('id') == 'pallet-wrapping-detail': dr.file.remove(page)
    dr.page('10 三箱装托、缠膜与整托发运', 2020, 1080)
    dr.file[-1].set('id', 'pallet-wrapping-detail')
    dr.box('整托闭环｜三箱真实装托，再缠膜发运', 60, 25, 1900, 60, font=28, style='strokeColor=none;fontStyle=1;')
    dr.box('沿用原厂房、12区与92台设备位置；同布局AI三视图继续有效。本页细化Z10人工装托与缠绕工位，接续第09页成品输送。', 60, 105, 1900, 75, font=20, style='strokeColor=none;align=left;')
    nodes = [('上游成品箱 · Z11', '3箱，每箱6件内装物\n320×240×240mm'),
             ('人工装托 · Z10', '3箱＋1只空托盘 → 1托\n复用人工工位与操作员'),
             ('托盘缠绕 · Z10', '1托＋9米计量膜\n未缠绕 → 已缠绕'),
             ('整托发运', '验收1托，内含原3箱\n18件内装物身份不变')]
    ids = [dr.box(t+'\n'+d, 60+i*485, 250, 430, 160, '#dcebd9' if i in (0,3) else '#edf3f8', 22, key='PAL-'+str(i)) for i,(t,d) in enumerate(nodes)]
    for a,b,label in zip(ids,ids[1:],['3趟单箱运输','1趟整托运输','1趟整托运输']): dr.edge(a,b,label)
    dr.box('装载几何\n1200×800×120mm托盘；3箱沿长边单层排列，箱间20mm。\n箱底高120mm、总高360mm；人工地面装托，不能放到小折箱台上。\n实际箱件随托盘运动；移除旧示例箱，缠膜包络适配实际箱堆。',60,480,920,190,'#edf3f8',21,style='align=left;')
    dr.box('订单与计数\n原3箱发运订单改为1托订单，原3箱仍全部生产。\n其余7类订单共99件保持不变；展示“99件＋1托（含3箱）”。\n制造161＋1＝162次；运输117＋2＝119趟。',1040,480,920,190,'#dcebd9',21,style='align=left;')
    dr.box('必要边界验证\n少6件内装物：只能产出2箱，不得装出1托。\n无空托盘：不得装托；无计量膜：不得形成已缠绕入库托盘。',60,740,920,150,'#f3e7cf',21,style='align=left;')
    dr.box('发布门槛\nHost数量、嵌套身份、耗膜、保存重开 → Native地面装托与转运\n→ 普通入口创建模板、运行与重开。仅通过后发布模板1.6.0。',1040,740,920,150,'#f3e7cf',21,style='align=left;')
    dr.box('本页定义模板1.6.0的实现与验收要求，实际进度以原开发控制计划为准。31台联线、61个独立单元；整厂闭环及原15项缺口继续保留。',60,950,1900,70,font=20,style='strokeColor=none;align=left;')
    for cell in dr.root.findall('mxCell'):
        if cell.get('value'): cell.set('value',cell.get('value').replace('\n','<br>'))
        if cell.get('style'): cell.set('style',cell.get('style').replace('html=0','html=1'))
    dr.save(path)


if __name__ == '__main__':
    spec=specification(); write(DESIGN/'pallet-line-spec.json',spec)
    attach_diagram(ART/'纸包装整厂区域与设备分配.drawio',spec)
    print('Original design extended with actual three-case packing and wrapping requirements.')
