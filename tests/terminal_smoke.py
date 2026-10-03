#!/usr/bin/env python3
"""測試用 PTY fixture；不屬於產品 runtime。"""
import atexit
import re
import fcntl
import json
import os
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time

BINARY = os.path.abspath(sys.argv[1])

class Terminal:
    # 仍開著的 PTY：等待時要持續讀取，否則子程序寫滿緩衝區（macOS 很小）就會卡住。
    live = []
    def __init__(self, args, env):
        self.master, self.slave = os.openpty()
        # 從 master 讀取終端設定：macOS 在 session leader 結束時會 revoke slave，之後無法再讀。
        self.original = termios.tcgetattr(self.master)
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack('HHHH',24,80,0,0))
        def setup():
            os.setsid()
            fcntl.ioctl(0, termios.TIOCSCTTY,0)
        self.child = subprocess.Popen(args,stdin=self.slave,stdout=self.slave,stderr=self.slave,env=env,preexec_fn=setup)
        self.output = bytearray()
        self.mark = 0
        Terminal.live.append(self)
        atexit.register(self.cleanup)
    @classmethod
    def pump(cls, duration):
        """等待 duration 秒，期間持續讀取所有仍開著的 PTY。"""
        end = time.monotonic()+duration
        while (remaining := end-time.monotonic()) > 0:
            masters = {t.master: t for t in cls.live}
            if not masters:
                time.sleep(remaining)
                return
            ready,_,_ = select.select(list(masters),[],[],min(0.05,remaining))
            for master in ready:
                try: masters[master].output.extend(os.read(master,65536))
                except OSError: pass
    def wait_exit(self, timeout=5):
        """等子程序結束，期間持續讀取輸出（關閉 tty 時 macOS 會等輸出排空）。"""
        deadline = time.monotonic()+timeout
        while self.child.poll() is None:
            assert time.monotonic()<deadline,('child did not exit',self.output.decode('utf-8','replace')[-2000:])
            Terminal.pump(0.05)
        self.read(0)
    def read(self, duration=0.1):
        end = time.monotonic()+duration
        while time.monotonic()<end:
            ready,_,_ = select.select([self.master],[],[],min(0.05,max(0,end-time.monotonic())))
            if ready:
                try: self.output.extend(os.read(self.master,65536))
                except OSError: break
        return self.output.decode('utf-8','replace')
    def cleanup(self):
        if self.child.poll() is None:
            self.child.kill()
            self.wait_exit()
    def wait_text(self, text):
        deadline=time.monotonic()+5
        while text.replace(' ', '') not in re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', self.read()[self.mark:]).replace(' ', ''):
            assert self.child.poll() is None, self.output.decode('utf-8','replace')
            assert time.monotonic()<deadline,(text,self.output.decode('utf-8','replace')[-2000:])
        self.mark = len(self.output.decode('utf-8','replace'))
    def wait_screen(self, text, timeout=5):
        """等到目前畫面出現 text；不受 wait_text 的比對位置影響（同一次重畫可能已包含它）。"""
        deadline=time.monotonic()+timeout
        while text not in '\n'.join(self.screen()):
            assert self.child.poll() is None, self.output.decode('utf-8','replace')
            assert time.monotonic()<deadline,(text,self.screen())
            self.read(0.05)
    def send(self, data):
        os.write(self.master,data)
    def screen(self):
        size=os.get_terminal_size(self.master)
        return screen_snapshot(bytes(self.output),size.columns,size.lines)
    def click_text(self, label):
        import unicodedata
        self.read()
        for row, text in enumerate(self.screen()):
            if label in text:
                prefix=text[:text.index(label)]
                column=sum(0 if unicodedata.combining(c) else 2 if unicodedata.east_asian_width(c) in ['W','F'] else 1 for c in prefix)
                self.send(f'\x1b[<0;{column+1};{row+1}M\x1b[<0;{column+1};{row+1}m'.encode())
                return
        raise AssertionError((label,self.screen()))
    def finish(self, data=b'\x1b'):
        self.send(data)
        self.wait_exit()
        self.read()
        assert self.child.returncode==0,self.output.decode('utf-8','replace')
        assert termios.tcgetattr(self.master)==self.original,'raw/echo/icanon flags not restored'
        for sequence in [b'\x1b[?1049l',b'\x1b[?25h',b'\x1b[?2004l']:
            assert sequence in self.output,('missing terminal restore',sequence)
        Terminal.live.remove(self)
        os.close(self.master);os.close(self.slave)

def main():
    with tempfile.TemporaryDirectory(prefix='tmux-manager-pty-') as root:
        config=os.path.join(root,'config.toml')
        env=dict(os.environ,TERM='xterm-256color',TMUX_MANAGER_CONFIG_FILE=config)
        env.pop('ANTHROPIC_API_KEY',None);env.pop('TMUX',None);env.pop('TMUX_PANE',None)
        t=Terminal([BINARY,'prompts'],env)
        t.wait_text('Prompt Slots');t.send(b'n');t.wait_text('Ctrl-S')
        t.send('中文🙂 標題'.encode()+b'\t' + b'test' +b'\t')
        t.send(b'\x1b[200~'+'第一行\r\n第二行🙂\n'.encode()+b'\x1b[201~')
        Terminal.pump(0.1);t.send(b'\x13');t.wait_text('已儲存')
        t.finish()
        with open(os.path.join(root,'prompts.toml'),encoding='utf-8') as file:
            content=file.read();assert '中文🙂 標題' in content and '第二行🙂' in content,content
        t=Terminal([BINARY,'prompts'],env);t.wait_text('中文🙂 標題')
        fcntl.ioctl(t.slave,termios.TIOCSWINSZ,struct.pack('HHHH',12,40,0,0));os.kill(t.child.pid,signal.SIGWINCH)
        t.read();t.finish(b'\x03')
        socket=os.path.join(root,'tmux.socket')
        t=Terminal([BINARY,'--socket',socket],env);t.wait_text('Sessions')
        t.send(b'p');t.wait_text('未指定')
        t.send(b'n');t.wait_text('Ctrl-S')
        t.send('管理器內新增'.encode()+b'\t\t'+ '不用離開管理器'.encode()+b'\x13');t.wait_text('已儲存')
        t.send(b'\x1b');t.wait_text('Sessions')
        with open(os.path.join(root,'prompts.toml'),encoding='utf-8') as file:
            content=file.read();assert '管理器內新增' in content and '不用離開管理器' in content,content
        t.send(b'\x1b[<0;5;2M\x1b[<0;5;2m');t.wait_text('未指定')
        t.send(b'q');t.wait_text('Sessions');t.finish(b'q')
        subprocess.run(['tmux','-S',socket,'-f','/dev/null','new-session','-d','-s','smoke','-x','80','-y','24'],check=True)
        try:
            subprocess.run(['tmux','-S',socket,'send-keys','-t','smoke','printf "WORKSPACE-PREVIEW-MARKER\\n"','Enter'],check=True)
            deadline=time.monotonic()+3
            while 'WORKSPACE-PREVIEW-MARKER' not in subprocess.check_output(['tmux','-S',socket,'capture-pane','-p','-t','smoke']).decode():
                assert time.monotonic()<deadline,'workspace did not print preview fixture'
                time.sleep(0.02)
            t=Terminal([BINARY,'--socket',socket],env);t.wait_text('smoke')
            # 列表與 Preview 可能在同一次重畫出現，直接檢查目前畫面。
            t.wait_screen('WORKSPACE-PREVIEW-MARKER')
            t.read(0.4);before=len(t.output)
            t.send(b'\x1b[<35;5;4M'*20+b'\x1b[<0;5;4m\x1b[<32;5;4M\x1b[<2;5;4M\x1b[<0;5;3M')
            t.read(0.5)
            assert b'\x1b[2J' not in t.output[before:],'ignored mouse events cleared and redrew the full manager screen'
            original_preview=next(i for i,line in enumerate(t.screen()) if 'Preview' in line)
            divider=original_preview-1
            t.send(f'\x1b[<0;20;{divider+1}Mp'.encode());t.wait_text('未指定')
            t.send(b'\x1b[<0;20;14m\x1b');t.wait_text('Sessions');t.read(0.2)
            t.send(f'\x1b[<0;20;{original_preview+3}M\x1b[<32;20;14M\x1b[<0;20;14m'.encode())
            t.read(0.2)
            assert next(i for i,line in enumerate(t.screen()) if 'Preview' in line)==original_preview,'prompt mode left a stale divider drag'
            t.send(f'\x1b[<0;20;{divider+1}M\x1b[<32;20;13M\x1b[<0;20;13m'.encode())
            t.read(0.3)
            dragged_preview=next(i for i,line in enumerate(t.screen()) if 'Preview' in line)
            assert dragged_preview>original_preview,'drag did not resize the panels'
            t.send(b']');t.read(0.2)
            assert next(i for i,line in enumerate(t.screen()) if 'Preview' in line)>dragged_preview,'keyboard resize did not move divider'
            t.send(b'\\');t.read(0.2)
            assert next(i for i,line in enumerate(t.screen()) if 'Preview' in line)==original_preview,'automatic layout was not restored'
            # 加大間距的選單較高；此 Preview 回歸使用足夠左右空間，讓 marker 完整可見。
            fcntl.ioctl(t.slave,termios.TIOCSWINSZ,struct.pack('HHHH',24,104,0,0));os.kill(t.child.pid,signal.SIGWINCH);t.read(0.2)
            t.send(b'\r');t.wait_text('Session 操作');t.read(0.2)
            assert 'Sessions' in '\n'.join(t.screen()) and 'Preview' in '\n'.join(t.screen()),'menu replaced background panels'
            assert not subprocess.check_output(['tmux','-S',socket,'list-clients','-F','#{session_name}']).strip(),'first Enter attached instead of showing menu'
            command = "printf '%s\\n' " + ' '.join(f'modal-{index:02}' for index in range(14)) + ' MODAL-PREVIEW-UPDATE'
            subprocess.run(['tmux','-S',socket,'send-keys','-t','smoke',command,'Enter'],check=True)
            # ratatui 只重畫有變動的格子，新文字可能被拆成數段輸出；直接檢查目前畫面。
            t.wait_screen('MODAL-PREVIEW-UPDATE');t.read(0.2)
            assert 'attach' in '\n'.join(t.screen()),'background refresh closed the menu'
            t.send(b'\x1b[<0;5;2M\x1b[<0;5;2m');t.wait_text('[Enter] 操作');t.read(0.2)
            assert 'Prompt Slots' in '\n'.join(t.screen()) and '未指定' not in '\n'.join(t.screen()),'outside header click opened prompts instead of only dismissing'
            assert next(i for i,line in enumerate(t.screen()) if 'Preview' in line)==original_preview,'dismiss changed panel ratio'
            fcntl.ioctl(t.slave,termios.TIOCSWINSZ,struct.pack('HHHH',12,40,0,0));os.kill(t.child.pid,signal.SIGWINCH);t.read(0.2)
            assert 'smoke' in '\n'.join(t.screen()),'selected session disappeared after resize'
            t.click_text('smoke');t.wait_text('attach')
            fcntl.ioctl(t.slave,termios.TIOCSWINSZ,struct.pack('HHHH',24,80,0,0));os.kill(t.child.pid,signal.SIGWINCH);t.read(0.3)
            t.click_text('rename');t.wait_text('Ctrl-U');t.send(b'\x1b');t.wait_text('[p] prompts')
            t.send(b'\x1b');t.wait_text('[Enter] 操作')
            fcntl.ioctl(t.slave,termios.TIOCSWINSZ,struct.pack('HHHH',24,80,0,0));os.kill(t.child.pid,signal.SIGWINCH);t.read(0.2)
            t.click_text('smoke');t.wait_text('[p] prompts')
            t.click_text('rename');t.wait_text('Ctrl-U');t.send(b'\x1b');t.wait_text('[p] prompts')
            t.click_text('rename');t.wait_text('Ctrl-U');t.send(b'\x03');t.wait_text('操作已中止');t.read(0.3);assert t.child.poll() is None,'Ctrl-C in a form exited the manager'
            t.click_text('kill');t.wait_text('確定結束此 session');t.send(b'n');t.wait_text('[p] prompts')
            assert subprocess.run(['tmux','-S',socket,'has-session','-t','smoke']).returncode==0,'mouse kill bypassed confirmation'
            t.click_text('back');t.wait_text('[Enter] 操作')
            t.click_text('smoke');t.wait_text('[p] prompts');t.send(b'p');t.wait_text('明確選擇貼上目的 pane')
            t.send(b'\x1b[<0;5;3M\x1b[<0;5;3m');t.wait_text('Prompt Slots');t.send(b'\x1b');t.wait_text('[p] prompts')
            t.click_text('attach')
            deadline=time.monotonic()+5
            while subprocess.check_output(['tmux','-S',socket,'list-clients','-F','#{session_name}']).strip()!=b'smoke':
                t.read();assert time.monotonic()<deadline,'mouse attach did not attach to selected session'
            t.read();t.send(b'\x02d');t.wait_text('[Enter] 操作');t.finish(b'q')
            subprocess.run(['tmux','-S',socket,'new-session','-d','-s','gone','-x','80','-y','24'],check=True)
            t=Terminal([BINARY,'--socket',socket],dict(env,TMUX_MANAGER_POLL_INTERVAL='60'));t.wait_text('gone');t.read(0.3)
            subprocess.run(['tmux','-S',socket,'kill-session','-t','gone'],check=True)
            t.click_text('gone');t.wait_text('[p] prompts');t.send(b'a');t.wait_text('attach 失敗');t.read(0.3);assert t.child.poll() is None,'failed attach exited the manager';t.finish(b'qq')
        finally:
            subprocess.run(['tmux','-S',socket,'kill-server'],check=False)
    print('PASS: manager內prompt進出/拖曳狀態復原/分隔線拖曳/鍵盤調整與自動/縮放後點選/新增保存/中文多行/重啟/Ctrl-C/浮動操作選單與Preview背景更新/session滑鼠/刪除確認/pane單擊/attach與detach/表單Ctrl-C與attach失敗不退出/終端復原')

def screen_snapshot(data, width=80, height=24):
    import unicodedata
    cells = [[' ']*width for _ in range(height)]
    row=column=0
    text=data.decode('utf-8','replace');index=0
    while index<len(text):
        if text[index]=='\x1b':
            match=re.match(r'\x1b\[([0-?]*)([ -/]*)([@-~])',text[index:])
            if match:
                raw=match.group(1).lstrip('?');parts=[int(x) if x.isdigit() else 0 for x in raw.split(';')];value=parts[0] or 1;kind=match.group(3)
                if kind in ['H','f']:row=(parts[0] or 1)-1;column=((parts[1] if len(parts)>1 else 1) or 1)-1
                elif kind=='A':row=max(0,row-value)
                elif kind=='B':row=min(height-1,row+value)
                elif kind=='C':column=min(width-1,column+value)
                elif kind=='D':column=max(0,column-value)
                elif kind=='G':column=value-1
                elif kind=='d':row=value-1
                elif kind=='J' and parts[0] in [2,3]:cells=[[' ']*width for _ in range(height)]
                elif kind=='K' and 0<=row<height:
                    for c in range(0 if parts[0] in [1,2] else column,width if parts[0] in [0,2] else column+1):cells[row][c]=' '
                elif kind=='X' and 0<=row<height:
                    for c in range(column,min(width,column+value)):cells[row][c]=' '
                index+=len(match.group(0));continue
            if index+2<len(text) and text[index+1] in ['(',')']:index+=3;continue
            index+=2;continue
        char=text[index];index+=1
        if char=='\r':column=0;continue
        if char=='\n':row=min(height-1,row+1);continue
        if ord(char)<32 or not (0<=row<height):continue
        size=0 if unicodedata.combining(char) else 2 if unicodedata.east_asian_width(char) in ['W','F'] else 1
        if 0<=column<width:
            cells[row][column]=char
            if size==2 and column+1<width:cells[row][column+1]=''
        column+=size
        if column>=width:column=0;row=min(height-1,row+1)
    return [''.join(line) for line in cells]

if __name__ == "__main__":
    main()
