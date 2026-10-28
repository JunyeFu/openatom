"""Independent PNG observation from actual native/renderer output; no browser model."""
import json
from pathlib import Path
from PIL import Image

root = Path(__file__).resolve().parent
output = root / 'output'
native = json.loads((output / 'desktop-result.json').read_text(encoding='utf-8'))
exited = json.loads((output / 'actual-exit.json').read_text(encoding='utf-8-sig'))
assert native['status'] == 'passed'
assert exited['actual_exit_code'] == 0 and not exited['own_helper_timed_out_and_terminated']
assert native['current']['revision'] == 8 and native['current']['x_m'] == 8
assert native['frozen']['revision'] == 7 and native['frozen']['x_m'] == 2
assert any(s.get('actual_current_before_after_identical') is True for s in native['UI_report']['steps'])
observations = {}
for name, expected_x in [('frozen-r7.png', 128), ('current-r8.png', 512), ('mcp-r7.png', 128)]:
    with Image.open(output / name) as image:
        assert image.size == (640, 360)
        rgb = image.convert('RGB')
        coordinates = [(x, y) for y in range(360) for x in range(640)
                       if (lambda p: p[2] > 200 and p[0] < 30 and 60 < p[1] < 150)(rgb.getpixel((x, y)))]
    assert len(coordinates) > 2000, f'{name}: actual blue geometry missing'
    centroid_x = sum(p[0] for p in coordinates) / len(coordinates)
    centroid_y = sum(p[1] for p in coordinates) / len(coordinates)
    assert abs(centroid_x - expected_x) < 2, f'{name}: pose mismatch {centroid_x}'
    assert abs(centroid_y - 180) < 2
    observations[name] = {'actual_size': [640, 360], 'blue_pixels': len(coordinates),
                          'centroid_x': centroid_x, 'centroid_y': centroid_y,
                          'expected_x_from_fixed_camera_and_world_pose': expected_x}
assert (output / 'frozen-r7.png').read_bytes() == (output / 'mcp-r7.png').read_bytes()
assert (output / 'frozen-r7.png').read_bytes() != (output / 'current-r8.png').read_bytes()
assert len(native['actual_edits']) == 2
assert sum(item['ok'] for item in native['actual_edits']) == 1
assert native['native_agent']['bridge_exit_code'] == 0 and native['owned_pipe_eof']
assert native['owned_pipe_after_close_error'] == 6
result = {'document_kind': 'independent_actual_WebView2_Three_PNG_pixel_observation',
          'status': 'passed', 'images': observations,
          'same_frozen_bytes_returned_through_actual_MCP_image': True,
          'current_editor_pose_is_distinct': True,
          'actual_product_runtime': None}
(output / 'pixel-verification.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps(result, ensure_ascii=False))
