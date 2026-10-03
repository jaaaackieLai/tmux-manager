#!/usr/bin/env python3
"""慢速 tmux、多 windows 的啟動不可強制留下 bars/mouse 設定。"""
import os,signal,sys
from dock_fixture import dock_fixture
binary=os.path.abspath(sys.argv[1])
# 只讓建立底部列的 split-window 變慢（6 個 window 共約 7 秒）；穩定後每輪查詢維持正常速度。
WRAPPER='''
if "split-window" in args:
    time.sleep(1.2)
'''
with dock_fixture('tmux-manager-dock-start-','slow',windows=6,wrapper=WRAPPER) as f:
    client=f.manager(binary);f.attach()
    if '--cancel' in sys.argv:
        f.wait(lambda:bool(f.bars()),'startup did not allocate any owned bar')
        os.kill(f.supervisor(),signal.SIGTERM)
        f.wait(lambda:not f.bars(),'termination during startup left partial dock resources')
        f.wait(lambda:f.mouse()=='','cancelled startup left mouse setting')
        # 底部列只是輔助：啟動被中止仍照常 attach，回到 manager 後在狀態列說明。
        f.wait(lambda:len(f.clients())==1,'cancelled dock startup blocked the attach')
        f.tmux('detach-client','-s','slow');client.wait_text('底部 prompt 列未啟動');client.finish(b'q')
    else:
        f.wait(lambda:len(f.clients())==1,'slow tmux startup was aborted and left partial dock resources instead of attaching')
        assert len(f.bars())==6,'not every window got its bottom bar'
        f.detach()
        f.wait(lambda:not f.bars(),'detaching slow session left owned bars')
        f.wait(lambda:f.mouse()=='','slow startup left temporary mouse setting')
        client.finish(b'q')
print('PASS: 6 windows/slow tmux startup/no forced timeout leak/detach owner cleanup/mouse restore')
