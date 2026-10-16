#!/usr/bin/env python3
"""Finite loopback HTTPS certificate gate; private keys never leave the host."""
import argparse, concurrent.futures, datetime, hashlib, json, signal, socket, ssl, subprocess, threading, time
from pathlib import Path
NAME='rust-aros.test'
RESPONSE=b'HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\nverified'
def prepare(directory,openssl):
    if directory.exists(): raise ValueError('refusing existing fixture directory')
    directory.mkdir(parents=True,mode=0o700)
    def run(*args,ok=True):
        r=subprocess.run([openssl,*map(str,args)],cwd=directory,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        if ok and r.returncode: raise ValueError(r.stdout.decode())
        return r
    root_ext='basicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\nsubjectKeyIdentifier=hash\n'
    leaf_ext='basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:'+NAME+'\n'
    (directory/'root.ext').write_text(root_ext); (directory/'leaf.ext').write_text(leaf_ext)
    for name in ['root','other','valid','expired']:
        run('genpkey','-algorithm','RSA','-pkeyopt','rsa_keygen_bits:2048','-out',name+'.key')
        (directory/(name+'.key')).chmod(0o600)
        run('req','-new','-key',name+'.key','-subj','/CN='+name,'-out',name+'.csr')
        command=['x509','-req','-in',name+'.csr','-out',name+'.pem','-sha256','-set_serial',str({'root':1,'other':2,'valid':3,'expired':4}[name]),'-extfile','root.ext' if name in ['root','other'] else 'leaf.ext']
        if name in ['root','other']: command+=['-signkey',name+'.key']
        else: command+=['-CA','root.pem','-CAkey','root.key']
        command+=['-not_before','20000101000000Z' if name=='expired' else '20200101000000Z','-not_after','20010101000000Z' if name=='expired' else '20400101000000Z']
        run(*command)
    run('verify','-CAfile','root.pem','-verify_hostname',NAME,'valid.pem')
    for args in [('root.pem','wrong.test','valid.pem'),('root.pem',NAME,'expired.pem'),('other.pem',NAME,'valid.pem')]:
        if not run('verify','-CAfile',args[0],'-verify_hostname',args[1],args[2],ok=False).returncode: raise ValueError('negative certificate unexpectedly accepted')
    metadata={'status':'prepared-not-run','openssl':run('version').stdout.decode().strip(),'cert_sha256':{n:hashlib.sha256((directory/n).read_bytes()).hexdigest() for n in ['root.pem','other.pem','valid.pem','expired.pem']}}
    (directory/'fixture.json').write_text(json.dumps(metadata,indent=2)+'\n')
    return metadata

def serve(directory,run_id,deadline):
    stop=threading.Event(); signal.signal(signal.SIGINT,lambda *_:stop.set()); signal.signal(signal.SIGTERM,lambda *_:stop.set())
    lock=threading.Lock(); records=[]; errors=[]; accepted=0; entropy=0
    def record(**item):
        with lock: records.append(item); print(json.dumps(dict(run_id=run_id,**item)),flush=True)
    contexts={}
    for port,name in [(18771,'valid'),(18772,'expired'),(18774,'valid'),(18776,'valid')]:
        ctx=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER); ctx.minimum_version=ssl.TLSVersion.TLSv1_2
        ctx.load_cert_chain(directory/(name+'.pem'),directory/(name+'.key')); contexts[port]=ctx
    listeners={}; futures=[]
    def connection(raw,port):
        secured=None
        try:
            raw.settimeout(5)
            if port==18775:
                hello=b''
                while True:
                    block=raw.recv(8192)
                    if not block: break
                    hello+=block
                    if len(hello)>65536: raise ValueError('oversized ClientHello')
                if len(hello)<5: raise ValueError('cancelled before ClientHello')
                record(kind='cancelled-handshake',port=port,bytes=len(hello)); return
            secured=contexts[port].wrap_socket(raw,server_side=True)
            if port==18776:
                if secured.recv(1): raise ValueError('unexpected HTTP during live-drop case')
                record(kind='live-drop',port=port,protocol=secured.version()); return
            request=b''
            while b'\r\n\r\n' not in request:
                block=secured.recv(2048)
                if not block: raise ValueError('EOF before HTTP headers')
                request+=block
                if len(request)>8192: raise ValueError('oversized HTTP request')
            if request!=b'GET /gate HTTP/1.1\r\nHost: rust-aros.test\r\nConnection: close\r\n\r\n': raise ValueError('unexpected request')
            protocol=secured.version(); secured.sendall(RESPONSE)
            if port==18774:
                # Deliberately truncate transport without an authenticated alert.
                fd=secured.detach(); socket.socket(fileno=fd).close(); secured=None
                record(kind='truncated',port=port,protocol=protocol)
            else:
                raw=secured.unwrap(); secured=None; raw.close()
                record(kind='verified-http',port=port,protocol=protocol,bytes=len(request))
        except ssl.SSLError as e:
            # A certificate rejection is only accepted if it precedes HTTP.
            if secured is not None or port==18774: errors.append(str(e))
            record(kind='certificate-alert',port=port,error=str(e))
        except Exception as e:
            errors.append(repr(e)); record(kind='error',port=port,error=repr(e))
        finally:
            if secured is not None: secured.close()
            raw.close()
    import selectors
    selector=selectors.DefaultSelector()
    try:
        for port in [18771,18772,18773,18774,18775,18776]:
            listener=socket.socket(); listener.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1); listener.bind(('127.0.0.1',port)); listener.listen(16); listener.setblocking(False)
            listeners[port]=listener; selector.register(listener,selectors.EVENT_READ,port)
        record(kind='ready',ports=list(listeners),deadline=deadline)
        end=time.monotonic()+deadline
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as workers:
            while not stop.is_set() and time.monotonic()<end:
                for key,_ in selector.select(.1):
                    raw,_=key.fileobj.accept(); port=key.data
                    if port==18773:
                        entropy+=1; raw.close(); errors.append('entropy-failure binary connected')
                    else: accepted+=1; futures.append(workers.submit(connection,raw,port))
            if not stop.is_set(): errors.append('fixture deadline expired')
            for f in futures: f.result()
        kinds=[r.get('kind') for r in records]
        if accepted!=10 or kinds.count('cancelled-handshake')!=1 or kinds.count('live-drop')!=1 or kinds.count('verified-http')!=4 or kinds.count('certificate-alert')!=3 or kinds.count('truncated')!=1 or entropy!=0: errors.append('unexpected connection/result counts')
        record(kind='completed',accepted=accepted,entropy_connections=entropy,errors=errors)
        return not errors
    finally:
        selector.close()
        for s in listeners.values(): s.close()

def main():
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument('directory',type=Path); parser.add_argument('--prepare',action='store_true'); parser.add_argument('--openssl',default='/opt/homebrew/bin/openssl'); parser.add_argument('--run-id'); parser.add_argument('--deadline',type=int,default=180)
    args=parser.parse_args()
    if args.prepare: print(json.dumps(prepare(args.directory,args.openssl),indent=2))
    else:
        if not args.run_id: parser.error('--run-id required')
        raise SystemExit(0 if serve(args.directory,args.run_id,args.deadline) else 1)
if __name__=='__main__': main()
