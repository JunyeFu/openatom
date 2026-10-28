"""Loopback-only design prototype; serves owned UI and read-only existing 3D assets."""
from pathlib import Path
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import unquote, urlsplit

D = Path(__file__).resolve().parent
S = D.parents[2]
VENDOR = S / 'tools/3d-proof/vendor'
MODELS = S / 'tools/3d-proof/output'
class Handler(SimpleHTTPRequestHandler):
    def translate_path(self, path):
        path = unquote(urlsplit(path).path)
        root, tail = (VENDOR, path[8:]) if path.startswith('/vendor/') else (MODELS, path[8:]) if path.startswith('/models/') else (D, path.lstrip('/'))
        target = (root / tail).resolve()
        if not target.is_relative_to(root.resolve()): return str(D / '_unavailable')
        return str(target)

if __name__ == '__main__':
    print('S04 design prototype: http://127.0.0.1:8921/ (not product server)', flush=True)
    ThreadingHTTPServer(('127.0.0.1', 8921), Handler).serve_forever()
