# Makes src/icons.rs, Lust's built-in icons, from Lucide's SVG files.
#
#   npm pack lucide-static   (or download its .tgz from the npm registry) and unpack it, then
#   python3 tools/lucide-icons.py path/to/package tools/lucide-icons.txt src/icons.rs
#
# lucide-icons.txt lists the icons Lust keeps. Each SVG shape (path, circle, rect, line, ellipse,
# polyline, polygon) is turned into SVG path data, so Lust only has to draw paths.
import json, re, sys
pkg, names_file, out = sys.argv[1], sys.argv[2], sys.argv[3]
version = json.load(open(pkg + '/package.json'))['version']
names = open(names_file).read().split()
# Other names: ones Lucide used before, and a few short ones of Lust's own (the last group).
aliases = [('edit', 'square-pen'), ('home', 'house'), ('unlock', 'lock-open'), ('more-horizontal', 'ellipsis'),
           ('more-vertical', 'ellipsis-vertical'), ('alert-triangle', 'triangle-alert'), ('help-circle', 'circle-help'),
           ('filter', 'funnel'), ('alert-circle', 'circle-alert'), ('check-circle', 'circle-check'), ('x-circle', 'circle-x'),
           ('plus-circle', 'circle-plus'), ('minus-circle', 'circle-minus'),
           ('help', 'circle-help'), ('refresh', 'refresh-cw'), ('sort', 'arrow-up-down'), ('stop', 'square'),
           ('close', 'x'), ('delete', 'trash-2')]

def f(x):
    s = ('%.3f' % float(x)).rstrip('0').rstrip('.')
    return '0' if s in ('-0', '') else s

def elements(svg):
    out = []
    for m in re.finditer(r'<(path|circle|rect|line|ellipse|polyline|polygon)\b([^>]*)/>', svg):
        tag, attrs = m.group(1), dict(re.findall(r'([\w-]+)="([^"]*)"', m.group(2)))
        fill = attrs.get('fill') not in (None, 'none')
        if tag == 'path':
            d = attrs['d']
        elif tag in ('circle', 'ellipse'):
            cx, cy = float(attrs['cx']), float(attrs['cy'])
            rx = float(attrs.get('r', attrs.get('rx', 0)))
            ry = float(attrs.get('r', attrs.get('ry', 0)))
            d = f'M{f(cx-rx)} {f(cy)}A{f(rx)} {f(ry)} 0 1 0 {f(cx+rx)} {f(cy)}A{f(rx)} {f(ry)} 0 1 0 {f(cx-rx)} {f(cy)}Z'
        elif tag == 'rect':
            x, y, w, h = (float(attrs.get(k, 0)) for k in ('x', 'y', 'width', 'height'))
            rx, ry = attrs.get('rx'), attrs.get('ry')
            rx = float(rx if rx is not None else (ry or 0))
            ry = float(ry if ry is not None else rx)
            rx, ry = min(rx, w / 2), min(ry, h / 2)
            if rx == 0 or ry == 0:
                d = f'M{f(x)} {f(y)}H{f(x+w)}V{f(y+h)}H{f(x)}Z'
            else:
                a = f'A{f(rx)} {f(ry)} 0 0 1 '
                d = (f'M{f(x+rx)} {f(y)}H{f(x+w-rx)}{a}{f(x+w)} {f(y+ry)}V{f(y+h-ry)}{a}{f(x+w-rx)} {f(y+h)}'
                     f'H{f(x+rx)}{a}{f(x)} {f(y+h-ry)}V{f(y+ry)}{a}{f(x+rx)} {f(y)}Z')
        elif tag == 'line':
            d = f"M{f(attrs['x1'])} {f(attrs['y1'])}L{f(attrs['x2'])} {f(attrs['y2'])}"
        else:
            pts = [f(p) for p in re.split(r'[\s,]+', attrs['points'].strip())]
            pairs = [pts[i] + ' ' + pts[i + 1] for i in range(0, len(pts) - 1, 2)]
            d = 'M' + 'L'.join(pairs) + ('Z' if tag == 'polygon' else '')
        out.append(('F' if fill else '') + d)
    return out

rows = []
for n in names:
    e = elements(open(f'{pkg}/icons/{n}.svg').read())
    assert e and ';' not in ''.join(e), n
    rows.append((n, ';'.join(e)))
known = {n for n, _ in rows}
for a, to in aliases:
    assert to in known and a not in known, (a, to)
with open(out, 'w') as o:
    o.write("// Lust's icons: a subset of Lucide (https://lucide.dev), version %s, under the ISC licence,\n" % version)
    o.write('// some of them from Feather (MIT licence). Both licences are in LICENSES/lucide.txt.\n//\n')
    o.write('// Made by tools/lucide-icons.py; change tools/lucide-icons.txt and run it again rather than\n')
    o.write('// editing this. Each icon is drawn on a 24 by 24 grid with lines 2 wide and round ends, as\n')
    o.write('// Lucide draws them. Its shapes are SVG path data, with ; between shapes; a shape that starts\n')
    o.write('// with F is filled in.\n\n')
    o.write("/// Each icon's name and its shapes.\npub const ICONS: &[(&str, &str)] = &[\n")
    for n, d in rows:
        o.write('    (%s, %s),\n' % (json.dumps(n), json.dumps(d)))
    o.write('];\n\n/// Other names for some icons: the names Lucide gave them before, and a few short ones.\n')
    o.write('pub const ICON_ALIASES: &[(&str, &str)] = &[\n')
    for a, to in aliases:
        o.write('    (%s, %s),\n' % (json.dumps(a), json.dumps(to)))
    o.write('];\n')
print(len(rows), 'icons,', len(aliases), 'other names')
