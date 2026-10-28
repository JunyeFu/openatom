"""Exact planar subdivision for finite simple polygons; no holes or curved faces.

All coordinates are Fractions of the supplied numbers. Ear triangles preserve
concavities; patch edge lines partition them without enlarging any patch.
"""
from fractions import Fraction as F


def cross(a, b, c):
    return (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])


def area2(poly):
    return sum(a[0]*b[1]-a[1]*b[0] for a, b in zip(poly, poly[1:]+poly[:1]))


def clean(poly):
    result = []
    for point in poly:
        if not result or point != result[-1]:
            result.append(point)
    if len(result) > 1 and result[0] == result[-1]:
        result.pop()
    changed = True
    while changed and len(result) >= 3:
        changed = False
        for i in range(len(result)):
            if cross(result[i-1], result[i], result[(i+1) % len(result)]) == 0:
                result.pop(i)
                changed = True
                break
    return result


def on_segment(p, a, b):
    return cross(a, b, p) == 0 and all(min(a[k], b[k]) <= p[k] <= max(a[k], b[k]) for k in (0, 1))


def segments_intersect(a, b, c, d):
    values = cross(a, b, c), cross(a, b, d), cross(c, d, a), cross(c, d, b)
    if values[0]*values[1] < 0 and values[2]*values[3] < 0:
        return True
    return any(v == 0 and on_segment(p, x, y) for v, p, x, y in [
        (values[0], c, a, b), (values[1], d, a, b),
        (values[2], a, c, d), (values[3], b, c, d)])


def polygon(points):
    raw = [tuple(F(p[k]) for k in ('x', 'y')) if isinstance(p, dict) else tuple(map(F, p)) for p in points]
    if len(raw) < 3 or len(set(raw)) != len(raw):
        raise ValueError('invalid_simple_polygon')
    for i,b in enumerate(raw):
        a,c=raw[i-1],raw[(i+1)%len(raw)]
        if cross(a,b,c)==0 and sum((b[k]-a[k])*(c[k]-b[k]) for k in (0,1))<=0:
            raise ValueError('invalid_simple_polygon')
    for i in range(len(raw)):
        a, b = raw[i], raw[(i+1) % len(raw)]
        for j in range(i+1, len(raw)):
            if j == i+1 or (i == 0 and j == len(raw)-1):
                continue
            if segments_intersect(a, b, raw[j], raw[(j+1) % len(raw)]):
                raise ValueError('invalid_simple_polygon')
    poly = clean(raw)
    if len(poly) < 3 or area2(poly) == 0:
        raise ValueError('invalid_simple_polygon')
    return poly if area2(poly) > 0 else list(reversed(poly))


def inside(point, poly):
    # Boundary-inclusive exact winding test.
    winding = 0
    for a, b in zip(poly, poly[1:]+poly[:1]):
        if on_segment(point, a, b):
            return True
        if a[1] <= point[1] < b[1] and cross(a, b, point) > 0:
            winding += 1
        elif b[1] <= point[1] < a[1] and cross(a, b, point) < 0:
            winding -= 1
    return winding != 0


def triangles(poly):
    remaining = list(poly)
    result = []
    while len(remaining) > 3:
        for i, b in enumerate(remaining):
            a, c = remaining[i-1], remaining[(i+1) % len(remaining)]
            if cross(a, b, c) <= 0:
                continue
            if any(all(cross(u, v, p) >= 0 for u, v in [(a, b), (b, c), (c, a)])
                   for p in remaining if p not in (a, b, c)):
                continue
            result.append([a, b, c])
            remaining.pop(i)
            break
        else:
            raise ValueError('polygon_triangulation_unresolved')
    result.append(remaining)
    assert sum(area2(t) for t in result) == area2(poly)
    return result


def clip(poly, a, b, sign=1):
    result = []
    for u, v in zip(poly, poly[1:]+poly[:1]):
        du, dv = sign*cross(a, b, u), sign*cross(a, b, v)
        if du >= 0:
            result.append(u)
        if (du < 0 < dv) or (dv < 0 < du):
            ratio = du/(du-dv)
            result.append(tuple(u[k]+ratio*(v[k]-u[k]) for k in (0, 1)))
    result = clean(result)
    return result if len(result) >= 3 and area2(result) > 0 else []


def centroid(poly):
    return tuple(sum(p[k] for p in poly)/len(poly) for k in (0, 1))


def rectangle(points):
    poly = polygon(points)
    xs, ys = sorted({p[0] for p in poly}), sorted({p[1] for p in poly})
    if len(poly) != 4 or len(xs) != 2 or len(ys) != 2 or set(poly) != {(x, y) for x in xs for y in ys}:
        return None
    return [xs[0], xs[1], ys[0], ys[1]]


def partition(points, thickness, patches):
    poly, thickness = polygon(points), F(thickness)
    if thickness <= 0:
        raise ValueError('nonpositive_panel_thickness')
    regions = triangles(poly)
    parsed = []
    for patch in patches:
        shape, depth = polygon(patch['polygon']), F(patch['limit'])
        if depth < 0:
            raise ValueError('negative_overlap_limit')
        if depth >= thickness:
            raise ValueError('full_thickness_contact_allowance')
        if patch['face'] not in ('front', 'back'):
            raise ValueError('invalid_contact_face')
        parsed.append((patch, shape, depth))
        for a, b in zip(shape, shape[1:]+shape[:1]):
            regions = [piece for region in regions for piece in (clip(region, a, b), clip(region, a, b, -1)) if piece]
    # This also rejects a patch edge crossing the exterior of a concave panel:
    # containment of patch vertices alone would not be sufficient.
    memberships = [[inside(centroid(region), shape) for _, shape, _ in parsed] for region in regions]
    for j, (_, shape, _) in enumerate(parsed):
        if sum(area2(region) for region, flags in zip(regions, memberships) if flags[j]) != area2(shape):
            raise ValueError('patch_outside_panel')
    z0, z1 = -thickness/2, thickness/2
    cuts = {z0, z1}
    slabs = []
    for patch, _, depth in parsed:
        slab = (z1-depth, z1) if patch['face'] == 'front' else (z0, z0+depth)
        slabs.append(slab)
        cuts.update(slab)
    levels = sorted(cuts)
    cells = []
    for region, flags in zip(regions, memberships):
        for lo, hi in zip(levels, levels[1:]):
            mid = (lo+hi)/2
            labels = frozenset(patch['id'] for j, (patch, _, _) in enumerate(parsed)
                               if patch['active'] and flags[j] and slabs[j][0] <= mid <= slabs[j][1])
            cells.append((region, (lo, hi), labels))
    if parsed and not any(not labels for _, _, labels in cells):
        raise ValueError('pair_allowance_covers_entire_panel')
    assert sum(area2(region)*(hi-lo)/2 for region, (lo, hi), _ in cells) == area2(poly)*thickness/2
    return cells
