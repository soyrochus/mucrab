#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Replay mouse/keyboard inputs on an isolated Xvfb display and retain captures.

Requires Xvfb, xwininfo, libX11, libXtst and Pillow with XCB support. Run via
`xvfb-run -a python3 -B tools/linux_gui.py LAUNCH_JSON [--replay ACTIONS_JSON]`.
Launch JSON contains argv, cwd, title and output; optional umask controls child
creation modes. Without --replay, read one
JSON action (or an array of actions) per stdin line; finish with {"op":"quit"}.
This driver never attaches to the user's desktop: a separate Xvfb is required.
"""

import argparse
import ctypes as c
import ctypes.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

from PIL import ImageGrab


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("launch", type=Path)
    parser.add_argument("--replay", type=Path)
    args = parser.parse_args()
    spec = json.loads(args.launch.read_text())
    # Xvfb's isolated display numbers normally start at 99; reject :0/Wayland.
    if not re.fullmatch(r":(?:[1-9]\d{1,}|[1-9]\d{1,}\.0)", os.environ.get("DISPLAY", "")):
        raise ValueError("Use an isolated xvfb-run display, never the live desktop")
    x = c.CDLL(ctypes.util.find_library("X11"))
    xt = c.CDLL(ctypes.util.find_library("Xtst"))
    x.XOpenDisplay.argtypes = [c.c_char_p]; x.XOpenDisplay.restype = c.c_void_p
    display = x.XOpenDisplay(None)
    if not display:
        raise ValueError("Could not connect to isolated X display")
    for name, types in {
        "XFlush": [c.c_void_p], "XCloseDisplay": [c.c_void_p],
        "XRaiseWindow": [c.c_void_p, c.c_ulong],
        "XSetInputFocus": [c.c_void_p, c.c_ulong, c.c_int, c.c_ulong],
        "XKeysymToKeycode": [c.c_void_p, c.c_ulong],
    }.items():
        getattr(x, name).argtypes = types
    x.XStringToKeysym.argtypes = [c.c_char_p]; x.XStringToKeysym.restype = c.c_ulong
    xt.XTestFakeMotionEvent.argtypes = [c.c_void_p, c.c_int, c.c_int, c.c_int, c.c_ulong]
    xt.XTestFakeButtonEvent.argtypes = [c.c_void_p, c.c_uint, c.c_int, c.c_ulong]
    xt.XTestFakeKeyEvent.argtypes = [c.c_void_p, c.c_uint, c.c_int, c.c_ulong]
    output = Path(spec["output"]); output.mkdir(parents=True, exist_ok=True)
    actions = []
    log = (output / "application.log").open("w")
    env = dict(os.environ); env.pop("WAYLAND_DISPLAY", None); env.pop("EFRAME_SCREENSHOT_TO", None)
    env.update(spec.get("env", {}))
    process = subprocess.Popen(spec["argv"], cwd=spec["cwd"], env=env, stdout=log, stderr=log, umask=spec.get("umask", -1))

    def key(name, down):
        symbol = x.XStringToKeysym(name.encode("ascii"))
        code = x.XKeysymToKeycode(display, symbol)
        if not code:
            raise ValueError("Unknown X key: " + name)
        xt.XTestFakeKeyEvent(display, code, int(down), 0); x.XFlush(display)

    def perform(action):
        action = {key: value.replace("{run}", str(args.launch.resolve().parent)).replace("{repository}", str(Path(__file__).resolve().parents[1]))
            if isinstance(value, str) else value for key, value in action.items()}
        op = action["op"]
        if op == "quit":
            return False
        actions.append(action)
        if op == "click":
            for modifier in action.get("modifiers", []): key(modifier, True)
            xt.XTestFakeMotionEvent(display, -1, action["x"], action["y"], 0)
            for _ in range(action.get("count", 1)):
                xt.XTestFakeButtonEvent(display, 1, 1, 0); x.XFlush(display); time.sleep(0.10)
                xt.XTestFakeButtonEvent(display, 1, 0, 0); x.XFlush(display); time.sleep(0.10)
            for modifier in reversed(action.get("modifiers", [])): key(modifier, False)
        elif op == "key":
            for name in action["keys"]: key(name, True)
            time.sleep(action.get("hold", 0.08))
            for name in reversed(action["keys"]): key(name, False)
        elif op == "text":
            shifted = {"_": "minus", ":": "semicolon", "~": "grave"}
            names = {"/": "slash", ".": "period", "-": "minus", " ": "space"}
            for char in action["text"]:
                shift = char.isupper() or char in shifted
                name = shifted.get(char, names.get(char, char.lower()))
                if shift: key("Shift_L", True)
                key(name, True); key(name, False)
                if shift: key("Shift_L", False)
        elif op not in ("capture", "wait"):
            raise ValueError("Unknown operation: " + op)
        time.sleep(min(float(action.get("wait", 0.4)), 10.0))
        if "capture" in action or op == "capture":
            name = action.get("capture", action.get("name", "capture"))
            if Path(name).name != name:
                raise ValueError("Capture must be a simple filename stem")
            path = output / (name + ".png")
            ImageGrab.grab(xdisplay=os.environ["DISPLAY"]).save(path)
            print(json.dumps(dict(capture=str(path))), flush=True)
        (output / "actions.json").write_text(json.dumps(actions, indent=2) + "\n")
        return True

    try:
        for _ in range(100):
            if process.poll() is not None:
                raise ValueError("Application exited before its window appeared; see application.log")
            tree = subprocess.check_output(["xwininfo", "-root", "-tree"], text=True)
            matches = [line for line in tree.splitlines() if (title := re.search(r'"([^"]*)"', line))
                and spec["title"] in title.group(1) and (size := re.search(r'(\d+)x(\d+)\+', line))
                and int(size[1]) > 100 and int(size[2]) > 100]
            matches.sort(key=lambda line: (lambda m: int(m[1]) * int(m[2]))(re.search(r'(\d+)x(\d+)\+', line)), reverse=True)
            if matches:
                window = int(matches[0].strip().split()[0], 16)
                attributes = subprocess.check_output(["xwininfo", "-id", hex(window)], text=True)
                if "Map State: IsViewable" not in attributes:
                    time.sleep(0.2)
                    continue
                x.XRaiseWindow(display, window); x.XSetInputFocus(display, window, 1, 0); x.XFlush(display)
                print(matches[0].strip(), flush=True)
                break
            time.sleep(0.2)
        else:
            raise ValueError("Timed out waiting for the requested application window")
        perform(dict(op="capture", name="initial", wait=2))
        if args.replay:
            for action in json.loads(args.replay.read_text()):
                if not perform(action): break
        else:
            for line in sys.stdin:
                batch = json.loads(line)
                batch = batch if isinstance(batch, list) else [batch]
                if not all(perform(action) for action in batch): break
    finally:
        process.terminate()
        try: process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill(); process.wait()
        x.XCloseDisplay(display); log.close()


if __name__ == "__main__":
    main()
