"""Local proof viewer and fixed-output export endpoint; no production engine."""
import argparse
import base64
import json
import os
import subprocess
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parent
KINDS = {"assembly", "robot", "conveyor", "inspection", "sealer", "pallet"}
VIDEO_FRAMES = 24 * 30
VIDEO_PROFILES = {
    "device": {"stem": "device-motion", "frames": VIDEO_FRAMES, "duration": 24},
    "line": {"stem": "L03-full-run", "frames": 143 * 30, "duration": 143},
}
video_lock = threading.Lock()
video_process = None
video_frames = 0
video_log = None
video_profile = VIDEO_PROFILES["device"]
ffmpeg_path = None


class Handler(SimpleHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def log_request(self, code="-", size="-"):
        if self.path != "/record-frame":
            super().log_request(code, size)

    def do_POST(self):
        if self.path in {"/record-start", "/record-frame", "/record-finish", "/record-cancel"}:
            self.record_video()
            return
        if self.path.startswith("/save-model/"):
            kind = self.path.removeprefix("/save-model/")
            length = int(self.headers.get("Content-Length", "0"))
            if kind not in KINDS or not 0 < length <= 64_000_000:
                self.send_error(400)
                return
            output = ROOT / "output"
            output.mkdir(exist_ok=True)
            (output / f"{kind}.glb").write_bytes(self.rfile.read(length))
            self.send_response(200)
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        if self.path != "/save-proof":
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", "0"))
        if not 0 < length <= 80_000_000:
            self.send_error(413)
            return
        data = json.loads(self.rfile.read(length))
        if data["kind"] not in KINDS or data["scene"] not in {"device", "line", "load"} or data["quality"] not in {"high", "low"} or data.get("view", "iso") not in {"iso", "work", "front", "back", "top", "free"}:
            self.send_error(400)
            return
        output = ROOT / "output"
        output.mkdir(exist_ok=True)
        stem = f'{data["scene"]}-{data["kind"]}-{data["quality"]}-{data.get("view", "iso")}'
        files = []
        for name in data["model_sizes"]:
            if name not in KINDS:
                self.send_error(400)
                return
            path = output / f"{name}.glb"
            files.append({"file": path.name, "bytes": path.stat().st_size})
        png = output / f"{stem}.png"
        png.write_bytes(base64.b64decode(data["image"].split(",", 1)[1], validate=True))
        data["metrics"]["saved_files"] = files
        (output / f"{stem}.json").write_text(json.dumps(data["metrics"], ensure_ascii=False, indent=2), encoding="utf-8")
        response = json.dumps({"files": files, "image": png.name}, ensure_ascii=False).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)

    def record_video(self):
        global video_process, video_frames, video_log, video_profile
        with video_lock:
            if self.path == "/record-start":
                if video_process is not None:
                    self.send_error(409, "A recording is already active")
                    return
                length = int(self.headers.get("Content-Length", "0"))
                request = json.loads(self.rfile.read(length)) if length else {}
                profile = request.get("profile", "device")
                if profile not in VIDEO_PROFILES:
                    self.send_error(400, "Unknown recording profile")
                    return
                video_profile = VIDEO_PROFILES[profile]
                output = ROOT / "output"
                output.mkdir(exist_ok=True)
                video_log = (output / f'{video_profile["stem"]}-encode.log').open("wb")
                video_process = subprocess.Popen([
                    ffmpeg_path, "-hide_banner", "-loglevel", "warning", "-y",
                    "-f", "image2pipe", "-framerate", "30", "-vcodec", "png", "-i", "pipe:0",
                    "-an", "-c:v", "libx264", "-preset", "veryfast", "-crf", "18",
                    "-pix_fmt", "yuv420p", "-movflags", "+faststart", str(output / f'{video_profile["stem"]}.mp4'),
                ], stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=video_log,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
                video_frames = 0
                result = {"frames_expected": video_profile["frames"]}
            elif self.path == "/record-frame":
                length = int(self.headers.get("Content-Length", "0"))
                if video_process is None or video_frames >= video_profile["frames"] or not 0 < length <= 8_000_000 or int(self.headers.get("X-Frame-Index", "-1")) != video_frames:
                    self.send_error(400, "Unexpected frame")
                    return
                frame = self.rfile.read(length)
                video_process.stdin.write(frame)
                if video_profile["stem"] == "L03-full-run" and video_frames in {0, video_profile["frames"] - 1}:
                    label = "poster" if video_frames == 0 else "final"
                    (ROOT / "output" / f'L03-full-run.{label}.png').write_bytes(frame)
                video_frames += 1
                result = {"frames_received": video_frames}
            elif self.path == "/record-finish":
                if video_process is None or video_frames != video_profile["frames"]:
                    self.send_error(400, "Recording has incomplete frames")
                    return
                length = int(self.headers.get("Content-Length", "0"))
                request = json.loads(self.rfile.read(length)) if length else {}
                video_process.stdin.close()
                exit_code = video_process.wait()
                video_log.close()
                video_process = None
                if exit_code:
                    self.send_error(500, "Video encoder failed; see output/*-encode.log")
                    return
                result = {"file": video_profile["stem"] + ".mp4", "frames": video_frames, "fps": 30, "duration_seconds": video_profile["duration"], "size": [1920, 1080]}
                if video_profile["stem"] == "device-motion":
                    result.update(scope="GLB_part_motion_preview_no_business_simulation", source_assets=["assembly.glb", "robot.glb"], shots=["assembly_overview", "assembly_spindle_closeup", "robot_overview_gripper", "robot_gripper_closeup"])
                else:
                    result.update(scope="L03_executed_local_business_simulation", source_assets=[kind + ".glb" for kind in sorted(KINDS)], shots=request["shots"], business_result=request["business_result"], coverage=request["coverage"])
                (ROOT / "output" / (video_profile["stem"] + ".json")).write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
            else:
                if video_process is not None:
                    video_process.stdin.close()
                    video_process.wait()
                    video_log.close()
                    video_process = None
                result = {"cancelled": True}
            response = json.dumps(result).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(response)))
            self.end_headers()
            self.wfile.write(response)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=19287)
    parser.add_argument("--ffmpeg", default="D:/MathModelingTools/envs/cumcm/Lib/site-packages/imageio_ffmpeg/binaries/ffmpeg-win-x86_64-v7.1.exe")
    args = parser.parse_args()
    ffmpeg_path = args.ffmpeg
    print(f"3D proof: http://127.0.0.1:{args.port}", flush=True)
    ThreadingHTTPServer(("127.0.0.1", args.port), Handler).serve_forever()
