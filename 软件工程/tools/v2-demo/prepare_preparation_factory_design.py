"""Extend the original factory diagram with the next exact material branches."""
import xml.etree.ElementTree as ET
from prepare_full_factory import DESIGN, ART, Drawing, write


def specification():
    return {
        'kind': 'exact_material_preparation_branches', 'version': '1.0.0', 'package': 'V2-05-G01',
        'state': 'declared_design_inputs', 'source': 'bags-host03 actual frozen project/modules',
        'hall_geometry_changed': False, 'AI_views_reused': True,
        'branches': [
            {'id': 'SHEETS', 'title': '500×500mm裁张原纸', 'devices': ['paper-unwinder', 'paper-rewinder', 'rotary-sheet-cutter'],
             'cycles': 1, 'yield_per_cycle': 60, 'load': 10,
             'physical_balance': {'input_rolls': 1, 'length_m': 30, 'width_m': .5, 'thickness_m': .0001,
                                  'output_sheets': 60, 'sheet_length_m': .5, 'paper_volume_m3': .0015},
             'states': ['rolled', 'unwound', 'rolled', 'sheet'],
             'identity': 'Unwinding and rewinding preserve one actual item; only cutting transforms it into sixty descendant sheets.'},
            {'id': 'BOARDS', 'title': '500×200mm分切纸板', 'devices': ['board-edge-trimmer', 'sheet-slitter'],
             'cycles': 3, 'yield_per_cycle': 2, 'load': 2,
             'physical_balance': {'input_area_m2': .2376, 'trimmed_area_m2': .2, 'half_area_m2': .1,
                                  'side_scrap_count': 2, 'side_scrap_area_m2': .0108,
                                  'end_scrap_count': 2, 'end_scrap_area_m2': .008, 'thickness_m': .003},
             'states': ['raw', 'trimmed', 'slit_half'],
             'identity': 'Trimmed board and slitter input have equal physical dimensions. wide_sheet is only an animation-state alias; no stock is resized.'},
            {'id': 'CURED', 'title': '500×400mm固化上光纸板', 'devices': ['varnish-coater', 'uv-curer'],
             'cycles': 3, 'yield_per_cycle': 1, 'load': 1,
             'physical_balance': {'length_m': .5, 'width_m': .4, 'thickness_m': .0008,
                                  'coating_volume_m3_per_board': .000004, 'coating_doses': 3},
             'states': ['flat', 'varnished_wet', 'varnished_cured'],
             'identity': 'The same item passes coating and curing; UV requires varnished_wet and yields varnished_cured.'},
        ],
        'media': 'Compose the accepted material geometries/animations; qualify every processing clip by its exact operation. Never replay an upstream animation at another device.',
        'storage_layout': {
            'shipping': {'location': 'PF-SHIPPING', 'origin_xy_m': [-5.5, -2.0], 'type_columns': 4,
                         'type_spacing_xy_m': [3.0, 3.0], 'stack_columns': 3, 'stack_spacing_xy_m': [.65, .65],
                         'stacks': {'PF-packed-case': [1, .25], 'folder-gluer-output': [3, .004],
                                    'paper-core-grinder-output': [1, .11], 'label-peeler-output': [6, .002],
                                    'PF-BAG-finished-bag': [1, .21], 'rotary-sheet-cutter-output': [10, .001],
                                    'sheet-slitter-output': [6, .005], 'varnish-coater-input': [3, .002]}},
            'process_buffers': {'type_spacing_m': .7, 'stack_columns': 3, 'stack_spacing_m': .65,
                                'flat_paper_per_stack': 10, 'flat_paper_display_gap_m': .0005},
            'counts_qualities_and_holders_changed': False,
            'visual_failure': '1.4 shared the first XY slots across product types; boxes/bags also used paper-thin stack pitch.',
        },
        'resources': {'vehicles': 3, 'dispatch_gate': 1, 'window_ms': 7200000},
        'expected': {'manufacturing_jobs': 153, 'transport_trips': 105,
                     'accepted_orders': {'PF-ORDER-CASE': 3, 'PF-ORDER-SLEEVE': 3, 'PF-ORDER-TUBE': 6,
                                         'PF-ORDER-LABEL': 18, 'PF-ORDER-BAG': 3,
                                         'PF-ORDER-SHEETS': 60, 'PF-ORDER-BOARDS': 6, 'PF-ORDER-CURED': 3},
                     'connected_devices': 26, 'independent_cells': 66},
        'acceptance': ['Declared paper area/volume and coating dose balance', 'Real transport and completed jobs from the original Host',
                       'Same item identity and required state across preserve operations', 'Missing coating cannot yield cured accepted boards',
                       'All existing five product orders, previous projects and Runs retained', 'Saved project/fresh Host reopen',
                       'Native material motion and exact template adoption remain separate required exits'],
    }


def attach_diagram(path, spec):
    dr = Drawing(); dr.file = ET.parse(path).getroot(); dr.counter = 10000
    for page in list(dr.file):
        if page.get('id') == 'preparation-line-detail': dr.file.remove(page)
    dr.page('07 原纸、纸板与上光联线', 2020, 1250); dr.file[-1].set('id', 'preparation-line-detail')
    dr.box('原纸与表面处理｜三条有明确物料关系的并行路线', 60, 25, 1900, 55, font=28, style='strokeColor=none;fontStyle=1;')
    dr.box('沿用104×90×9m厂房、12区和92台设备位置；先设计，再构建实际工程。几何未变，复用同布局AI三视图。', 60, 90, 1900, 45, font=18, style='strokeColor=none;align=left;')
    labels = {
        'SHEETS': [('来料 · Z01\n1卷30m×500mm\n厚0.1mm；已放行', '#edf3f8'), ('放卷 · Z01\nrolled → unwound\n保留同一物品编号', '#f3e7cf'),
                   ('复卷 · Z01\nunwound → rolled\n保留同一物品编号', '#f3e7cf'), ('滚刀裁张 · Z01\n1卷 → 60张\n每张500×500mm', '#f3e7cf'), ('成品入库\n60张裁张原纸\n10张叠放/趟，共6趟', '#dcebd9')],
        'BOARDS': [('来料 · Z01\n3块540×440mm\n厚3mm；已放行', '#edf3f8'), ('修边 · Z01\n500×400mm净板\n每块另产四条边料', '#f3e7cf'),
                   ('纸板分切 · Z01\n1块净板 → 2半板\n厚度和纸量保持', '#f3e7cf'), ('成品入库\n6块500×200mm半板\n2块/趟，共3趟', '#dcebd9')],
        'CURED': [('来料 · Z02\n3块500×400mm\n厚0.8mm；已放行', '#edf3f8'), ('上光 · Z02\n每块消耗4mL光油\nflat → varnished_wet', '#d9e9ee'),
                  ('UV固化 · Z02\n必须先湿上光\n→ varnished_cured', '#d9e9ee'), ('成品入库\n3块固化上光板\n同一物品身份与纸量', '#dcebd9')],
    }
    for index, branch in enumerate(spec['branches']):
        y = 180 + index * 285
        dr.box(branch['title'], 60, y - 35, 1900, 30, font=21, style='strokeColor=none;align=left;fontStyle=1;')
        previous = None
        for i, (text, fill) in enumerate(labels[branch['id']]):
            identity = dr.box(text, 60 + i * 380, y + 15, 295, 140, fill, 19, key=f"PREP-{branch['id']}-{i}")
            if previous: dr.edge(previous, identity, '实际转运')
            previous = identity
        note = {'SHEETS': '纸量：30×0.5×0.0001 = 60×0.5×0.5×0.0001 = 0.0015m³；不是60次裁切循环。',
                'BOARDS': '每块面积：0.2376 = 2×0.1 + 2×0.0108 + 2×0.008m²；边料另存，不计成品。',
                'CURED': '三份光油合计12mL；缺光油时不能上光、固化或接受此产品。其他产品路线保持可运行。'}[branch['id']]
        dr.box(note, 60, y + 175, 1900, 45, font=18, style='strokeColor=none;align=left;')
    dr.box('共用原有3辆运输车和1个调度放行位。设计预期：153次制造、105趟完整运输、102件合格产品；26台已联线，仍有66个独立单元待后续接入。运行、模板、Native画面分别实测后登记。', 60, 1100, 1900, 90, font=20, style='align=left;')
    # New page only; preserve existing pages and IDs.
    for cell in dr.root.findall('mxCell'):
        if cell.get('vertex') == '1' or cell.get('edge') == '1': cell.set('style', cell.get('style').replace('html=0', 'html=1'))
        if cell.get('value'): cell.set('value', cell.get('value').replace('\n', '<br>'))
    for page in list(dr.file):
        if page.get('id') == 'shipping-layout-detail': dr.file.remove(page)
    dr.page('08 成品区分托与展示槽', 2020, 1100); dr.file[-1].set('id', 'shipping-layout-detail')
    dr.box('成品区｜品类分区，平件分叠，箱袋逐件摆放', 60, 30, 1900, 60, font=28, style='strokeColor=none;fontStyle=1;')
    dr.box('成品与发运区：X=69—100m，Y=4—14m。共用库位原点(86,9,0.9)m；四列两行、品类间距3m，未改变厂房或设备位置。', 60, 110, 1900, 75, font=19, style='strokeColor=none;align=left;')
    names = ['装箱成品 · 3箱', '折糊纸套 · 3件', '精磨纸管 · 6件', '印刷标签 · 18张',
             '双提手纸袋 · 3只', '500×500mm裁张原纸 · 60张', '500×200mm分切纸板 · 6块', '500×400mm上光纸板 · 3块']
    for i, ((identity, (per_stack, pitch)), title) in enumerate(zip(spec['storage_layout']['shipping']['stacks'].items(), names)):
        policy = f'每叠 {per_stack} 件\n显示层间距 {pitch * 1000:g} mm' if per_stack > 1 else '逐件分格摆放\n不叠放'
        dr.box(f'{title}\n{policy}\n品类之间独立分区',
               60 + i % 4 * 475, 250 + i // 4 * 300, 425, 205, '#edf3f8' if i % 2 else '#dcebd9', 19, key='STORE-' + identity)
    dr.box('每种物料拥有独立 XY 区域；纸张按已声明的薄片间距分叠。数量、质量、持有位置、运输批量和运行时钟均保持原规则。工序缓存中的净板、侧边料与端边料也分开摆放，不能因共用库位而使用相同坐标。', 60, 890, 1900, 115, font=21, style='align=left;')
    for cell in dr.root.findall('mxCell'):
        if cell.get('vertex') == '1': cell.set('style', cell.get('style').replace('html=0', 'html=1'))
        if cell.get('value'): cell.set('value', cell.get('value').replace('\n', '<br>'))
    dr.save(path)


if __name__ == '__main__':
    spec = specification(); write(DESIGN / 'preparation-line-spec.json', spec)
    attach_diagram(ART / '纸包装整厂区域与设备分配.drawio', spec)
    print('Original factory design extended with three exact material branches; seven additional devices.')
