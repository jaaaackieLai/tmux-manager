#!/usr/bin/env python3
"""真實 tmux.conf / prefix P / 滑鼠 popup fixture。"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from terminal_smoke import Terminal, screen_snapshot

binary=os.path.abspath(sys.argv[1])
with tempfile.TemporaryDirectory(prefix='tmux-manager-popup-') as root:
    socket=os.path.join(root,'socket')
    config=os.path.join(root,'config.toml')
    body='中文🙂\n第二行\n'
    with open(os.path.join(root,'prompts.toml'),'w',encoding='utf-8') as file:
        file.write('schema_version = 1\n[[slots]]\nid = "5b7d3df5-9445-4206-b3c8-941697ed995c"\ntitle = "中文多行"\nbody = '+json.dumps(body,ensure_ascii=False)+'\ntags = []\norder = 0\n')
    spaced=os.path.join(root,'path with spaces')
    os.mkdir(spaced);test_binary=os.path.join(spaced,'tmux-manager');shutil.copyfile(binary,test_binary);os.chmod(test_binary,0o755)
    helper=os.path.join(root,'reader.py')
    with open(helper,'w') as file:
        file.write("import os,sys,tty\ntty.setraw(0)\nos.write(1,b'\\x1b[?2004h')\nopen(sys.argv[1]+'.ready','w').close()\nwith open(sys.argv[1],'wb',buffering=0) as f:\n while True: f.write(os.read(0,65536))\n")
    def tmux(*args):
        return subprocess.check_output(['tmux','-S',socket,*args],text=True)
    first=tmux('-f','/dev/null','new-session','-d','-s','fixture','-x','80','-y','24','-P','-F','#{pane_id}','python3',helper,os.path.join(root,'first')).strip()
    second=tmux('split-window','-h','-P','-F','#{pane_id}','python3',helper,os.path.join(root,'second')).strip()
    try:
        for name in ['first.ready','second.ready']:
            deadline=time.monotonic()+3
            while not os.path.exists(os.path.join(root,name)):
                assert time.monotonic()<deadline
                time.sleep(0.02)
        tmux('bind-key','R','display-message','keep-existing-binding')
        snippet=subprocess.check_output([test_binary,'--config-file',config,'bindings','--print'],text=True)
        bindings=os.path.join(root,'bindings.conf')
        with open(bindings,'w') as file: file.write(snippet)
        tmux('source-file',bindings)
        assert 'keep-existing-binding' in tmux('list-keys','-T','prefix')
        tmux('set-option','-g','mouse','on')
        tmux('select-pane','-t',first)
        env=dict(os.environ,TERM='xterm-256color');env.pop('ANTHROPIC_API_KEY',None)
        client=Terminal(['tmux','-S',socket,'attach-session','-t','fixture'],env)
        client.wait_text('fixture');client.send(b'\x02P');client.wait_text('Prompt Slots')
        client.read(0.1)
        screen=screen_snapshot(client.output)
        matches=[i for i,line in enumerate(screen) if '> 中文多行' in line]
        assert len(matches)==1,screen
        row=matches[0]+1
        column=screen[matches[0]].index('> 中文多行')+2
        client.send(f'\x1b[<0;{column};{row}M\x1b[<0;{column};{row}m'.encode())
        capabilities=tmux('display-message','-p','-t',first,'#{bracket_paste_flag}').strip()
        if capabilities=='1':
            expected=b'\x1b[200~'+body.encode()+b'\x1b[201~'
            deadline=time.monotonic()+3
            while not os.path.exists(os.path.join(root,'first')) or os.path.getsize(os.path.join(root,'first'))<len(expected):
                assert time.monotonic()<deadline,'popup click did not paste into original pane'
                client.read(0.05)
            with open(os.path.join(root,'first'),'rb') as file: assert file.read()==expected,'duplicate/changed paste'
        else:
            client.wait_text('尚未貼上')
            assert os.path.getsize(os.path.join(root,'first'))==0
            client.send(b'\x1b')
        assert os.path.getsize(os.path.join(root,'second'))==0,'paste leaked to another pane'
        client.read(1.1);tmux('detach-client','-s','fixture');client.child.wait(timeout=5)
        assert client.child.returncode==0
    finally:
        subprocess.run(['tmux','-S',socket,'kill-server'],check=False)
print('PASS: 真實 binding/path with spaces/prefix P/滑鼠單擊/原 pane ID/socket/其他 binding 保留')
