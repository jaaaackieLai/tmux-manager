#!/usr/bin/env python3
"""底部列 PTY 情境的共用 fixture：隔離的 tmux server、可注入行為的 tmux wrapper 與常用查詢。"""
import contextlib,os,shutil,subprocess,sys,tempfile,time
from terminal_smoke import Terminal
REAL_TMUX=shutil.which('tmux')

class DockFixture:
    def __init__(self,root,session,windows=1,wrapper=None):
        self.root=root;self.session=session;self.socket=os.path.join(root,'socket');self.client=None
        self.tmux('-f','/dev/null','new-session','-d','-s',session,'-x','80','-y','24')
        for _ in range(windows-1):self.tmux('new-window','-d','-t',session)
        path=os.environ['PATH']
        if wrapper is not None:
            # wrapper 是 Python 片段：可讀寫 args（tmux 參數）、ROOT 與 REAL（真正的 tmux），結束後執行 REAL。
            bindir=self.path('bin');os.mkdir(bindir)
            with open(os.path.join(bindir,'tmux'),'w') as f:
                f.write('#!'+sys.executable+'\nimport os,sys,time\nargs=sys.argv[1:]\nROOT='+repr(root)+'\nREAL='+repr(REAL_TMUX)+'\n'
                        +wrapper+'\nos.execv(REAL,["tmux"]+args)\n')
            os.chmod(os.path.join(bindir,'tmux'),0o755);path=bindir+':'+path
        self.env=dict(os.environ,PATH=path,TERM='xterm-256color',TMUX_MANAGER_CONFIG_FILE=self.path('config.toml'))
        for key in ['TMUX','TMUX_PANE','ANTHROPIC_API_KEY']:self.env.pop(key,None)
    def path(self,name):return os.path.join(self.root,name)
    def tmux(self,*args):return subprocess.check_output([REAL_TMUX,'-S',self.socket,*args],text=True)
    def bars(self):return [x for x in self.tmux('list-panes','-s','-t',self.session,'-F','#{@tmux_manager_dock}').splitlines() if x]
    def clients(self):return self.tmux('list-clients','-t',self.session,'-F','#{client_name}').splitlines()
    def mouse(self):return self.tmux('show-options','-qv','-t',self.session,'mouse').strip()
    def lines(self,name):
        try:return open(self.path(name)).read().splitlines()
        except FileNotFoundError:return []
    def manager(self,binary):
        self.client=Terminal([binary,'--socket',self.socket],self.env);self.client.wait_text(self.session)
        return self.client
    def attach(self):self.client.send(b'\r');self.client.wait_text('created');self.client.send(b'a')
    def detach(self):self.tmux('detach-client','-s',self.session);self.client.wait_text('Sessions')
    def wait(self,predicate,message,timeout=30):
        end=time.monotonic()+timeout
        while not predicate():
            if self.client and self.client.child.poll() is None:self.client.read(.05)
            else:time.sleep(.05)
            assert time.monotonic()<end,message
    def supervisor(self):
        processes=subprocess.check_output(['ps','-axo','pid,ppid,args'],text=True)
        return next(int(parts[0]) for line in processes.splitlines() if len(parts:=line.split(None,2))==3 and parts[1]==str(self.client.child.pid) and 'prompt-dock-session' in parts[2])

@contextlib.contextmanager
def dock_fixture(prefix,session,**kwargs):
    with tempfile.TemporaryDirectory(prefix=prefix) as root:
        fixture=DockFixture(root,session,**kwargs)
        try:yield fixture
        finally:subprocess.run([REAL_TMUX,'-S',fixture.socket,'kill-server'],check=False,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
