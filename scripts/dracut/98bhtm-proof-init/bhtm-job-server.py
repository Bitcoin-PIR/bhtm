#!/usr/bin/python3
"""One-shot job.json receiver for the generic BHTM proof UKI."""

from http.server import BaseHTTPRequestHandler, HTTPServer
import os
import sys


MAX_JOB_BYTES = 1024 * 1024


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: bhtm-job-server.py <port> <output>", file=sys.stderr)
        return 2

    port = int(sys.argv[1])
    output_path = sys.argv[2]
    accepted = {"value": False}

    class Handler(BaseHTTPRequestHandler):
        server_version = "bhtm-job-server/1"

        def do_POST(self) -> None:
            self._receive()

        def do_PUT(self) -> None:
            self._receive()

        def do_GET(self) -> None:
            self.send_response(200)
            self.send_header("content-type", "text/plain")
            self.end_headers()
            self.wfile.write(b"send job.json with POST or PUT /job\n")

        def log_message(self, fmt: str, *args: object) -> None:
            print("[bhtm-job-server] " + fmt % args, file=sys.stderr)

        def _receive(self) -> None:
            if self.path != "/job":
                self.send_error(404, "expected /job")
                return

            length_text = self.headers.get("content-length")
            if length_text is None:
                self.send_error(411, "content-length required")
                return
            try:
                length = int(length_text)
            except ValueError:
                self.send_error(400, "invalid content-length")
                return
            if length <= 0 or length > MAX_JOB_BYTES:
                self.send_error(413, "job.json size invalid")
                return

            body = self.rfile.read(length)
            tmp_path = output_path + ".tmp"
            os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
            with open(tmp_path, "wb") as f:
                f.write(body)
                f.flush()
                os.fsync(f.fileno())
            os.replace(tmp_path, output_path)

            accepted["value"] = True
            self.send_response(200)
            self.send_header("content-type", "text/plain")
            self.end_headers()
            self.wfile.write(b"ok\n")
            print(
                f"[bhtm-job-server] accepted {length} bytes at {output_path}",
                file=sys.stderr,
            )

    httpd = HTTPServer(("0.0.0.0", port), Handler)
    print(f"[bhtm-job-server] listening on 0.0.0.0:{port}", file=sys.stderr)
    while not accepted["value"]:
        httpd.handle_request()
    httpd.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
