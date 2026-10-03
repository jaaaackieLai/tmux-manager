#!/usr/bin/env python3
"""Manager attach 後的真實底部列、active pane 與資源清理。"""
import json,os,subprocess,sys,tempfile,time,unicodedata,fcntl,termios,struct,signal
from terminal_smoke import Terminal
binary=os.path.abspath(sys.argv[1])
with tempfile.TemporaryDirectory(prefix='tmux-manager-dock-') as root:
    socket=os.path.join(root,'socket');config=os.path.join(root,'config.toml')
    with open(os.path.join(root,'prompts.toml'),'w',encoding='utf8') as f:
        for i,(title,body) in enumerate([('單行','prompt-one🙂'),('多行','first\n中文🙂\n')]):
            f.write(('schema_version = 1\n' if i==0 else '')+'[[slots]]\nid = "'+['5b7d3df5-9445-4206-b3c8-941697ed995c','6b7d3df5-9445-4206-b3c8-941697ed995c'][i]+'"\ntitle = '+json.dumps(title,ensure_ascii=False)+'\nbody = '+json.dumps(body,ensure_ascii=False)+'\ntags = []\norder = '+str(i)+'\n')
    helper=os.path.join(root,'reader.py')
    with open(helper,'w') as f:
        f.write("import os,sys,tty\ntty.setraw(0)\nos.write(1,b'\\x1b[?2004hWORKSPACE')\nopen(sys.argv[1]+'.ready','w').close()\nwith open(sys.argv[1],'wb',buffering=0) as f:\n while True: f.write(os.read(0,65536))\n")
    def tmux(*args):return subprocess.check_output(['tmux','-S',socket,*args],text=True)
    def wait(predicate,message):
        end=time.monotonic()+5
        while not predicate():
            assert time.monotonic()<end,message() if callable(message) else message
            Terminal.pump(.04)
    def data(name):
        path=os.path.join(root,name)
        return open(path,'rb').read() if os.path.exists(path) else b''
    def bars():return [s.split('\t') for s in tmux('list-panes','-s','-t','fixture','-F','#{pane_id}\t#{@tmux_manager_dock}').splitlines() if s.split('\t')[-1]]
    first=tmux('-f','/dev/null','new-session','-d','-s','fixture','-x','80','-y','24','-P','-F','#{pane_id}','python3',helper,os.path.join(root,'first')).strip()
    second=tmux('split-window','-h','-t',first,'-P','-F','#{pane_id}','python3',helper,os.path.join(root,'second')).strip()
    try:
        wait(lambda:os.path.exists(os.path.join(root,'second.ready')),'readers did not start')
        env=dict(os.environ,TERM='xterm-256color',TMUX_MANAGER_CONFIG_FILE=config)
        for k in ['TMUX','TMUX_PANE','ANTHROPIC_API_KEY']:env.pop(k,None)
        client=Terminal([binary,'--socket',socket],env)
        client.wait_text('fixture');client.send(b'\r');client.wait_text('created');client.send(b'a')
        wait(lambda:bool(bars()),'manager attach did not show the bottom prompt dock')
        bar=bars()[0][0]
        wait(lambda:'單行' in tmux('capture-pane','-p','-t',bar),'dock buttons not rendered')
        fcntl.ioctl(client.slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,80,0,0));os.kill(client.child.pid,signal.SIGWINCH)
        wait(lambda:tmux('display-message','-p','-t',bar,'#{pane_height}').strip()=='4','resizing the terminal expanded the dock instead of preserving work space')
        def click(title,pane=bar):
            top,left=map(int,tmux('display-message','-p','-t',pane,'#{pane_top} #{pane_left}').split())
            lines=tmux('capture-pane','-p','-t',pane).splitlines()
            row=next(i for i,s in enumerate(lines) if title in s)
            prefix=lines[row].split(title)[0]
            x=sum(0 if unicodedata.combining(c) else 2 if unicodedata.east_asian_width(c) in ['W','F'] else 1 for c in prefix)
            client.send(f'\x1b[<0;{left+x+1};{top+row+1}M\x1b[<0;{left+x+1};{top+row+1}m'.encode())
        assert tmux('display-message','-p','-t',bar,'#{pane_width}').strip()=='80','dock is not full window width'
        assert tmux('display-message','-p','-t',second,'#{pane_active}').strip()=='1','creating dock changed active work pane'
        tmux('set-window-option','-t',first,'synchronize-panes','on')
        tmux('select-pane','-t',first);click('單行')
        expected=b'\x1b[200~prompt-one'+ '🙂'.encode()+b'\x1b[201~'
        wait(lambda:len(data('first'))>=len(expected),'mouse click did not paste to last active work pane')
        assert data('first')==expected,'payload changed, repeated, or Enter was added'
        assert data('second')==b'','paste went to another pane'
        wait(lambda:tmux('display-message','-p','-t',first,'#{pane_active}').strip()=='1','focus did not return to work pane')
        tmux('set-window-option','-t',first,'synchronize-panes','off')
        tmux('select-pane','-t',second);click('單行')
        wait(lambda:len(data('second'))>=len(expected),'dock did not follow changed active work pane')
        assert data('second')==expected
        tmux('select-pane','-t',bar);client.send(b'\r')
        wait(lambda:tmux('display-message','-p','-t',second,'#{pane_active}').strip()=='1','bar keyboard interaction did not return focus')
        Terminal.pump(.1);assert data('second')==expected,'Enter in the bar triggered a prompt'
        # tmux 依 pane 的游標鍵模式送出 CSI 或 SS3 形式的「上」。
        client.send(b'\x1b[A')
        wait(lambda:any(key in data('second')[len(expected):] for key in (b'\x1b[A',b'\x1bOA')),
             lambda:f"direction key was taken from the work pane: {data('second')[len(expected):]!r}")
        up=data('second')[len(expected):]
        click('多行')
        if tmux('display-message','-p','-t',second,'#{bracket_paste_flag}').strip()=='1':
            extra=b'\x1b[200~first\n'+'中文🙂\n'.encode()+b'\x1b[201~'
            wait(lambda:len(data('second'))>=len(expected)+len(up)+len(extra),'multiline prompt did not arrive')
            assert data('second')==expected+up+extra
        else:
            wait(lambda:'尚未貼上' in tmux('capture-pane','-p','-t',bar),'multiline capability failure is invisible')
            assert data('second')==expected+up
        third=tmux('new-window','-d','-t','fixture','-P','-F','#{pane_id}','python3',helper,os.path.join(root,'third')).strip()
        wait(lambda:len(bars())==2,'new window did not get a prompt dock')
        otherbar=next(p[0] for p in bars() if p[0]!=bar)
        tmux('select-window','-t',third);tmux('select-pane','-t',third)
        wait(lambda:'單行' in tmux('capture-pane','-p','-t',otherbar),'new window dock not ready')
        click('單行',otherbar);wait(lambda:data('third')==expected,'new window prompt did not target its work pane')
        prompts_file=os.path.join(root,'prompts.toml')
        with open(prompts_file,encoding='utf8') as f: contents=f.read()
        with open(prompts_file,'w',encoding='utf8') as f:f.write(contents.replace('單行','已更新'))
        wait(lambda:'已更新' in tmux('capture-pane','-p','-t',otherbar),'dock did not reload saved prompt changes')
        click('已更新',otherbar);wait(lambda:data('third')==expected+expected,'reloaded prompt lost its UUID target')
        # A second manager must share the owned bars and cannot clean another attached client.
        second_client=Terminal([binary,'--socket',socket],env)
        second_client.wait_text('fixture');second_client.send(b'\r');second_client.wait_text('created')
        second_client.send(b'p');second_client.wait_text('明確選擇貼上目的 pane');second_client.read(.1)
        assert all(p[0]+' ' not in '\n'.join(second_client.screen()) for p in bars()),'pane picker offered an internal prompt dock as a paste target'
        second_client.send(b'\x1b');second_client.wait_text('[p] prompts');second_client.send(b'a')
        wait(lambda:len(tmux('list-clients','-t','fixture','-F','#{client_name}').splitlines())==2,'second attach failed')
        assert len(bars())==2,'second attach duplicated prompt bars'
        tmux('detach-client','-t',os.ttyname(second_client.slave));second_client.wait_text('Sessions')
        Terminal.pump(.6);assert len(bars())==2,'detaching another client removed bars still in use'
        second_client.finish(b'q')
        tmux('detach-client','-t',os.ttyname(client.slave));client.wait_text('Sessions')
        wait(lambda:len(bars())==0,'detach left owned dock panes running')
        wait(lambda:tmux('show-options','-qv','-t','fixture','mouse').strip()=='','temporary mouse option was not restored')
        assert tmux('has-session','-t','fixture')=='','cleanup killed work session'
        client.finish(b'q')
    finally:
        subprocess.run(['tmux','-S',socket,'kill-server'],check=False,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
print('PASS: manager attach底部常駐列/僅滑鼠/active與last target/focus復原/方向鍵仍到工作區/多行能力/新window/detach清理/mouse設定復原')
