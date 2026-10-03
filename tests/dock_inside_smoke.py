#!/usr/bin/env python3
"""Manager 在 tmux 內 switch-client 退出後，底部列仍可用。"""
import json,os,shutil,subprocess,sys,tempfile,time,termios
from terminal_smoke import Terminal
binary=os.path.abspath(sys.argv[1])
with tempfile.TemporaryDirectory(prefix='tmux-manager-dock-inside-') as root:
    socket=os.path.join(root,'socket');config=os.path.join(root,'config.toml')
    folder=os.path.join(root,'binary with spaces');os.mkdir(folder)
    spaced=os.path.join(folder,'tmux-manager');shutil.copyfile(binary,spaced);os.chmod(spaced,0o755)
    with open(os.path.join(root,'prompts.toml'),'w',encoding='utf8') as f:
        f.write('schema_version = 1\n[[slots]]\nid = "5b7d3df5-9445-4206-b3c8-941697ed995c"\ntitle = "InsidePrompt"\nbody = '+json.dumps('inside🙂',ensure_ascii=False)+'\ntags = []\norder = 0\n')
    helper=os.path.join(root,'inside.py')
    with open(helper,'w') as f:
        f.write("import os,subprocess,sys,tty\nenv=dict(os.environ,TMUX_MANAGER_CONFIG_FILE=sys.argv[2]);env.pop('ANTHROPIC_API_KEY',None)\nsubprocess.run([sys.argv[1]],env=env,check=True)\ntty.setraw(0);os.write(1,b'\\x1b[?2004hREADY-WORKSPACE')\nwith open(sys.argv[3],'wb',buffering=0) as f:\n while True:f.write(os.read(0,65536))\n")
    def tmux(*args):return subprocess.check_output(['tmux','-S',socket,*args],text=True)
    first=tmux('-f','/dev/null','new-session','-d','-s','inside','-x','80','-y','24','-P','-F','#{pane_id}','python3',helper,spaced,config,os.path.join(root,'received')).strip()
    env=dict(os.environ,TERM='xterm-256color');env.pop('TMUX',None);env.pop('TMUX_PANE',None)
    try:
        client=Terminal(['tmux','-S',socket,'attach-session','-t','inside'],env)
        client.wait_text('Sessions');client.send(b'\r');client.wait_text('created');client.send(b'a');client.wait_text('READY-WORKSPACE')
        end=time.monotonic()+5
        while True:
            panes=tmux('list-panes','-t',first,'-F','#{pane_id}\t#{pane_top}\t#{@tmux_manager_dock}').splitlines()
            bars=[p.split('\t') for p in panes if p.split('\t')[-1]]
            if bars and 'InsidePrompt' in tmux('capture-pane','-p','-t',bars[0][0]):break
            client.read(.05);assert time.monotonic()<end,'inside manager exit removed or failed to create the dock'
        bar,top,_=bars[0]
        client.send(f'\x1b[<0;5;{int(top)+2}M\x1b[<0;5;{int(top)+2}m'.encode())
        end=time.monotonic()+5;expected=b'\x1b[200~inside'+ '🙂'.encode()+b'\x1b[201~'
        while not os.path.exists(os.path.join(root,'received')) or os.path.getsize(os.path.join(root,'received'))<len(expected):
            client.read(.05);assert time.monotonic()<end,'inside dock did not fill the work pane'
        assert open(os.path.join(root,'received'),'rb').read()==expected
        assert tmux('display-message','-p','-t',first,'#{pane_active}').strip()=='1'
        tmux('detach-client','-s','inside');client.wait_exit()
        end=time.monotonic()+5
        while tmux('list-panes','-t',first,'-F','#{@tmux_manager_dock}').strip():
            assert time.monotonic()<end,'inside session left a bar after detach';Terminal.pump(.05)
        assert termios.tcgetattr(client.master)==client.original
    finally:
        subprocess.run(['tmux','-S',socket,'kill-server'],check=False,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
print('PASS: inside tmux switch-client/manager exit/dock persists/path with spaces/byte exact/focus return/detach cleanup')
