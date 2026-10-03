#!/usr/bin/env python3
"""舊 supervisor 正在清理（仍持有鎖）時重新 attach，新的 attach 仍須取得底部列。"""
import os,sys
from dock_fixture import dock_fixture
binary=os.path.abspath(sys.argv[1])
# kill-pane 變慢（仍低於單一 tmux 指令 2 秒逾時）：舊 supervisor 清理期間持有鎖的時間拉長，
# 穩定重現交接競態。list-clients 的結果記錄下來，確認 supervisor 已看到 client 才 detach。
WRAPPER='''
if "kill-pane" in args:
    open(os.path.join(ROOT,"killing"),"w").close();time.sleep(1.5)
if "list-clients" in args:
    import subprocess
    out=subprocess.run([REAL]+args,capture_output=True,text=True)
    open(os.path.join(ROOT,"ticks"),"a").write(("clients" if out.stdout.strip() else "none")+"\\n")
    sys.stdout.write(out.stdout);sys.stderr.write(out.stderr);sys.exit(out.returncode)
'''
with dock_fixture('tmux-manager-dock-handoff-','again',wrapper=WRAPPER) as f:
    client=f.manager(binary);f.attach()
    f.wait(lambda:len(f.clients())==1 and len(f.bars())==1,'first attach did not create the bar')
    f.wait(lambda:'clients' in f.lines('ticks'),'supervisor never saw the attached client')
    first=f.bars()[0]
    f.detach()
    f.wait(lambda:os.path.exists(f.path('killing')),'old supervisor never started cleanup')
    f.attach()
    f.wait(lambda:len(f.clients())==1,'reattach failed')
    f.wait(lambda:len(f.bars())==1 and f.bars()[0]!=first,'reattach during the old cleanup ended with no bottom bar')
    f.detach()
    f.wait(lambda:not f.bars(),'detach after handoff left bars')
    client.finish(b'q')
print('PASS: reattach during the old supervisor cleanup still gets the bottom bar')
