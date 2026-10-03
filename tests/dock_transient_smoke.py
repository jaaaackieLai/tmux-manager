#!/usr/bin/env python3
"""tmux 指令偶發失敗（例如 window 剛被關閉）時，底部列 supervisor 不可清除所有底部列並結束。"""
import os,sys
from dock_fixture import dock_fixture
binary=os.path.abspath(sys.argv[1])
# 旗標檔存在時，下一次符合條件的呼叫失敗一次；成功的 list-clients 記錄下來，代表 supervisor 又跑了一輪。
WRAPPER='''
for flag,match in [("list-clients","list-clients" in args),("list-panes","list-panes" in args and any(a.startswith("@") for a in args))]:
    path=os.path.join(ROOT,"fail-"+flag)
    if match and os.path.exists(path):
        os.remove(path);sys.stderr.write("can't find window\\n");sys.exit(1)
if "list-clients" in args:
    open(os.path.join(ROOT,"ticks"),"a").write("tick\\n")
'''
with dock_fixture('tmux-manager-dock-transient-','flaky',windows=2,wrapper=WRAPPER) as f:
    client=f.manager(binary);f.attach()
    f.wait(lambda:len(f.bars())==2,'attach did not create a bar in every window')
    for flag in ['list-clients','list-panes']:
        open(f.path('fail-'+flag),'w').close()
        f.wait(lambda:not os.path.exists(f.path('fail-'+flag)),f'{flag} failure was never triggered')
        ticks=len(f.lines('ticks'))
        f.wait(lambda:len(f.lines('ticks'))>ticks,f'supervisor stopped after one failed {flag}')
        assert len(f.bars())==2,f'one failed {flag} removed the bottom bars'
    f.detach()
    f.wait(lambda:not f.bars(),'detach left bars')
    client.finish(b'q')
print('PASS: transient list-clients/list-panes failures keep the bottom bars')
