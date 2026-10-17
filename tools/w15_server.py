#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Disposable loopback SFTP fixture. Requires test-only Paramiko 4.0.0.

Run directory must already contain host.key; only generated test credentials are
accepted. No system users, SSH config, personal keys or external hosts are used.
"""
import argparse,json,os,socket,threading,time
from pathlib import Path
import paramiko
from paramiko.sftp import CMD_INIT,CMD_VERSION
from paramiko import SFTP_OK,SFTP_FAILURE,SFTP_PERMISSION_DENIED,SFTP_NO_SUCH_FILE
PASSWORD='w15-disposable-only'
USER='fixture'
class Auth(paramiko.ServerInterface):
 def check_auth_password(self,user,password):return paramiko.AUTH_SUCCESSFUL if user==USER and password==PASSWORD else paramiko.AUTH_FAILED
 def check_auth_publickey(self,user,key):return paramiko.AUTH_SUCCESSFUL if user==USER and key.get_base64() in [(RUN/name).read_text().split()[1] for name in ['client.pub','encrypted.key.pub']] else paramiko.AUTH_FAILED
 def get_allowed_auths(self,user):return 'publickey,password'
 def check_channel_request(self,kind,channel):return paramiko.OPEN_SUCCEEDED if kind=='session' else paramiko.OPEN_FAILED_ADMINISTRATIVELY_PROHIBITED
class Files(paramiko.SFTPServerInterface):
 def __init__(self,server,*args,**kwargs):super().__init__(server,*args,**kwargs);self.auth=server
 def path(self,path):
  p=ROOT/path.lstrip('/')
  if '..' in Path(path).parts or (p!=ROOT and not p.parent.resolve().is_relative_to(ROOT)):raise PermissionError()
  if '/denied' in path:raise PermissionError()
  if '/slow' in path:time.sleep(6)
  return p
 def call(self,fn):
  try:return fn()
  except OSError as e:return paramiko.SFTPServer.convert_errno(e.errno or 13)
 def list_folder(self,path):
  def impl():
   values=[]
   for p in self.path(path).iterdir():
    a=paramiko.SFTPAttributes.from_stat(p.lstat());a.filename=os.fsencode(p.name);values.append(a)
   return values
  return self.call(impl)
 def lstat(self,path):return self.call(lambda:paramiko.SFTPAttributes.from_stat(self.path(path).lstat()))
 def stat(self,path):return self.lstat(path)
 def open(self,path,flags,attr):
  def impl():
   p=self.path(path)
   if p.is_symlink():raise PermissionError()
   fd=os.open(p,flags,attr.st_mode or 0o600);f=os.fdopen(fd,'r+b' if flags&os.O_RDWR else 'wb' if flags&os.O_WRONLY else 'rb',buffering=0);h=paramiko.SFTPHandle(flags)
   if not flags&os.O_WRONLY:h.readfile=f
   if flags&(os.O_WRONLY|os.O_RDWR):h.writefile=f
   h.stat=lambda:paramiko.SFTPAttributes.from_stat(os.fstat(f.fileno()))
   return h
  return self.call(impl)
 def mkdir(self,path,attr):return self.call(lambda:(self.path(path).mkdir(mode=attr.st_mode or 0o700),SFTP_OK)[1])
 def rename(self,old,new):
  def impl():
   a,b=self.path(old),self.path(new)
   if b.exists() or b.is_symlink():return SFTP_FAILURE
   os.link(a,b);a.unlink();return SFTP_OK
  return self.call(impl)
 def posix_rename(self,old,new):
  def impl():
   os.replace(self.path(old),self.path(new))
   if (RUN/'ambiguous').exists():self.auth.transport.close()
   return SFTP_OK
  return self.call(impl)
class Server(paramiko.SFTPServer):
 def _send_server_version(self):
  t,data=self._read_packet();assert t==CMD_INIT
  msg=paramiko.Message();msg.add_int(3)
  if not (RUN/'no-posix').exists():msg.add_string('posix-rename@openssh.com');msg.add_string('1')
  self._send_packet(CMD_VERSION,msg);return 3
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('run',type=Path);args=p.parse_args();RUN=args.run.resolve();ROOT=(RUN/'remote').resolve();ROOT.mkdir(exist_ok=True)
 key=paramiko.RSAKey.from_private_key_file(str(RUN/'host.key'));sock=socket.socket();sock.bind(('127.0.0.1',0));sock.listen(20);(RUN/'endpoint.json').write_text(json.dumps(dict(host='127.0.0.1',port=sock.getsockname()[1],user=USER)))
 def serve(client):
  transport=paramiko.Transport(client);transport.add_server_key(key);transport.set_subsystem_handler('sftp',Server,Files)
  try:
   auth=Auth();auth.transport=transport;transport.start_server(server=auth)
   while transport.is_active():time.sleep(.05)
  except Exception:pass
  finally:transport.close()
 while True:
  client,_=sock.accept();threading.Thread(target=serve,args=(client,),daemon=True).start()
