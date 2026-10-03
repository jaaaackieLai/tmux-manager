#!/usr/bin/env python3
"""終端斷線或 supervisor 異常結束後，不可留下孤兒底部列或被強制開啟的 mouse。"""
import os,signal,sys
from dock_fixture import dock_fixture
binary=os.path.abspath(sys.argv[1])
with dock_fixture('tmux-manager-dock-orphan-','orph') as f:
    def attached():
        f.attach();f.wait(lambda:len(f.clients())==1 and len(f.bars())==1,'attach did not create the bottom bar')
    # A：終端斷線（SIGHUP 送到前景 process group）時 supervisor 仍須清理。
    client=f.manager(binary);attached()
    os.killpg(client.child.pid,signal.SIGHUP)
    f.wait(lambda:not f.clients(),'hangup did not detach the tmux client')
    f.wait(lambda:not f.bars(),'hangup left orphaned bottom bars')
    f.wait(lambda:f.mouse()=='','hangup left the temporary mouse setting')
    # B：supervisor 被強制結束後，下一次 attach 接手清除舊列並還原原本的 mouse。
    client=f.manager(binary);attached()
    stale=f.bars()[0]
    os.kill(f.supervisor(),signal.SIGKILL)
    f.detach()
    assert f.bars()==[stale] and f.mouse()=='on','fixture did not leave stale resources'
    f.attach()
    f.wait(lambda:len(f.clients())==1 and len(f.bars())==1 and f.bars()!=[stale],'stale bar from a dead supervisor was not replaced')
    f.detach()
    f.wait(lambda:not f.bars(),'detach after recovery left bars')
    f.wait(lambda:f.mouse()=='','recovered supervisor did not restore the original mouse setting')
    client.finish(b'q')
print('PASS: hangup cleanup/stale owner takeover/original mouse restore')
