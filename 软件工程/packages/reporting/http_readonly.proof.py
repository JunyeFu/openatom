"""Temporary loopback read-only serving of the self-authored DRF05 outputs only."""
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from urllib.parse import urlsplit
import argparse
import json
import os

ROOT = Path(__file__).resolve().parents[2]
REPORT = (ROOT / "docs/04_开发与验收/特征细化/DRF05_四格式试做/output").resolve()
FILES = {
    "/": REPORT / "evaluation.type_fixture.html",
    "/evaluation.type_fixture.html": REPORT / "evaluation.type_fixture.html",
}


class ReadOnlyReport(BaseHTTPRequestHandler):
    def serve(self, head=False):
        path = urlsplit(self.path).path
        target = FILES.get(path)
        if target is None:
            self.send_error(404)
            return
        target = target.resolve()
        if not target.is_relative_to(REPORT) or not target.is_file():
            self.send_error(404)
            return
        data = target.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        if not head:
            self.wfile.write(data)

    def do_GET(self):
        self.serve()

    def do_HEAD(self):
        self.serve(head=True)

    def do_POST(self):
        self.send_error(405)

    do_PUT = do_POST
    do_DELETE = do_POST
    do_PATCH = do_POST


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    args = parser.parse_args()
    assert args.port != 8921
    server = HTTPServer(("127.0.0.1", args.port), ReadOnlyReport)
    print(json.dumps({"pid": os.getpid(), "bind": "127.0.0.1", "port": args.port,
                      "root": str(REPORT), "allowed_paths": list(FILES), "readonly": True}), flush=True)
    try:
        server.serve_forever()
    finally:
        server.server_close()
