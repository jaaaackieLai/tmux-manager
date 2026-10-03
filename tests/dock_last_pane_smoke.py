#!/usr/bin/env python3
"""視窗內最後一個工作 pane 結束時，底部列要一併移除，讓 tmux 照常關閉該 window。"""
import os,sys
from dock_fixture import dock_fixture
binary=os.path.abspath(sys.argv[1])
with dock_fixture('tmux-manager-dock-last-pane-','lonely',windows=2) as f:
    client=f.manager(binary);f.attach()
    f.wait(lambda:len(f.clients())==1 and len(f.bars())==2,'attach did not create a bar in every window')
    def windows():return f.tmux('list-windows','-t','lonely','-F','#{window_id}').splitlines()
    first=windows()[0]
    work=next(line.split()[0] for line in f.tmux('list-panes','-t',first,'-F','#{pane_id} #{@tmux_manager_dock}').splitlines() if len(line.split())==1)
    f.tmux('kill-pane','-t',work)
    f.wait(lambda:first not in windows(),'a window holding only the prompt bar was kept open')
    assert len(f.bars())==1,'the other window lost its bar'
    f.detach()
    f.wait(lambda:not f.bars(),'detach left bars')
    client.finish(b'q')
print('PASS: closing the last work pane closes its window instead of leaving a lone prompt bar')
