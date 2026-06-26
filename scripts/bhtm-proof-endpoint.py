#!/usr/bin/env python3
"""Small HTTPS-backend endpoint for BHTM streams and proof artifacts.

Caddy terminates TLS and reverse-proxies to this process. The endpoint serves
archived stripped streams and accepts unauthenticated proof artifact PUTs. It is
deliberately narrow: paths must be rooted under a single configured base dir,
chunk names cannot contain slashes, proof directories are allow-listed, and only
known proof artifact filenames can be read or written.
"""

import argparse
import os
import re
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from urllib.parse import unquote


ALLOWED_PROOF_DIRS = {"proof", "proof-generic"}
ALLOWED_ARTIFACTS = {
    "job.prepared.json",
    "job.json",
    "attestation.bin",
    "report-data.bin",
    "sev-snp-report.bin",
    "stats.json",
    "run.stdout",
    "verify-bhtm-quote.txt",
    "proof-summary.env",
}
CHUNK_RE = re.compile(r"^[A-Za-z0-9_.-]+$")


class Handler(BaseHTTPRequestHandler):
    server_version = "bhtm-proof-endpoint/2"

    def log_message(self, fmt, *args):
        print(
            "%s - - [%s] %s"
            % (self.address_string(), self.log_date_time_string(), fmt % args),
            flush=True,
        )

    def _send_text(self, code, text):
        data = text.encode()
        self.send_response(code)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _parse_path(self):
        path = unquote(self.path.split("?", 1)[0])
        parts = [p for p in path.split("/") if p]
        if len(parts) < 2:
            return None
        chunk = parts[0]
        if not CHUNK_RE.fullmatch(chunk):
            return None
        return chunk, parts[1:]

    def _send_file(self, src):
        if not src.is_file():
            return self._send_text(404, "not found\n")
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(src.stat().st_size))
        self.end_headers()
        with src.open("rb") as f:
            while True:
                data = f.read(1024 * 1024)
                if not data:
                    break
                self.wfile.write(data)

    def do_GET(self):
        path = unquote(self.path.split("?", 1)[0])
        if path == "/healthz":
            return self._send_text(200, "ok\n")

        parsed = self._parse_path()
        if parsed is None:
            return self._send_text(404, "not found\n")
        chunk, rest = parsed
        chunk_dir = self.server.base_dir / chunk

        if rest == ["stream.bhtmbin.zst"]:
            return self._send_file(chunk_dir / "stream.bhtmbin.zst")

        if len(rest) == 2 and rest[0] in ALLOWED_PROOF_DIRS and rest[1] in ALLOWED_ARTIFACTS:
            return self._send_file(chunk_dir / rest[0] / rest[1])

        return self._send_text(404, "not found\n")

    def do_PUT(self):
        parsed = self._parse_path()
        if parsed is None:
            return self._send_text(404, "not found\n")
        chunk, rest = parsed
        if len(rest) != 2 or rest[0] not in ALLOWED_PROOF_DIRS or rest[1] not in ALLOWED_ARTIFACTS:
            return self._send_text(403, "forbidden\n")

        try:
            length = int(self.headers.get("Content-Length", ""))
        except ValueError:
            return self._send_text(411, "content-length required\n")
        if length < 0 or length > self.server.max_put_bytes:
            return self._send_text(413, "too large\n")

        proof_dir = self.server.base_dir / chunk / rest[0]
        proof_dir.mkdir(parents=True, exist_ok=True)
        dst = proof_dir / rest[1]
        tmp = dst.with_suffix(dst.suffix + ".tmp")
        remaining = length
        with tmp.open("wb") as f:
            while remaining:
                data = self.rfile.read(min(1024 * 1024, remaining))
                if not data:
                    return self._send_text(400, "short body\n")
                f.write(data)
                remaining -= len(data)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, dst)
        self._send_text(201, "created\n")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="/root/blockhash-to-muhash/stripped-deltas")
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=18080)
    ap.add_argument("--max-put-bytes", type=int, default=100 * 1024 * 1024)
    args = ap.parse_args()

    srv = ThreadingHTTPServer((args.host, args.port), Handler)
    srv.base_dir = Path(args.base).resolve()
    srv.max_put_bytes = args.max_put_bytes
    print(f"serving {srv.base_dir} on http://{args.host}:{args.port}", flush=True)
    srv.serve_forever()
