#!/usr/bin/env python3
"""子程序結束前的輸出超過 PTY 緩衝區時，fixture 仍須讀取輸出讓它結束（macOS 的 PTY 緩衝區很小）。"""
import os,sys
from terminal_smoke import Terminal
env=dict(os.environ)
# 大量輸出後寫出終端還原序列並結束：沒人讀取 master 時會卡在寫入，無法結束。
script="import os\nfor _ in range(64):os.write(1,b'x'*16384)\nos.write(1,b'\\x1b[?1049l\\x1b[?25h\\x1b[?2004l')\n"
Terminal([sys.executable,'-c',script],env).finish(b'')
print('PASS: waiting for exit keeps draining the PTY')
