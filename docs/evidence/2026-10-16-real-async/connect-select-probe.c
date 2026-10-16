#include <proto/exec.h>
#include <proto/bsdsocket.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <sys/time.h>
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
struct Library *SocketBase;
int main(void) {
 SocketBase=OpenLibrary("bsdsocket.library",4);
 if(!SocketBase) return 20;
 for(int attempt=0;attempt<2;attempt++) {
  int fd=socket(AF_INET,SOCK_STREAM,0), enabled=1;
  int nb=IoctlSocket(fd,(long)(int)FIONBIO,(char *)&enabled);
  struct sockaddr_in address;memset(&address,0,sizeof address);
  address.sin_len=sizeof address;address.sin_family=AF_INET;address.sin_port=htons(18762);
  address.sin_addr.s_addr=htonl(0x0a000202);
  int rc=connect(fd,(struct sockaddr *)&address,sizeof address);
  printf("attempt=%d fd=%d nonblock=%d connect=%d errno=%d\n",attempt,fd,nb,rc,Errno());
  fd_set r,w,x; FD_ZERO(&r);FD_ZERO(&w);FD_ZERO(&x);
  FD_SET(fd,&r);FD_SET(fd,&w);FD_SET(fd,&x);
  struct timeval timeout={2,0}, before,after;gettimeofday(&before,NULL);
  rc=WaitSelect(fd+1,&r,&w,&x,&timeout,NULL);gettimeofday(&after,NULL);
  printf("select=%d errno=%d readable=%d writable=%d exception=%d elapsed_us=%ld\n",rc,Errno(),!!FD_ISSET(fd,&r),!!FD_ISSET(fd,&w),!!FD_ISSET(fd,&x),(long)((after.tv_sec-before.tv_sec)*1000000+after.tv_usec-before.tv_usec));
  int error=0;socklen_t size=sizeof error;rc=getsockopt(fd,SOL_SOCKET,SO_ERROR,&error,&size);
  printf("getsockopt=%d so_error=%d errno=%d\n",rc,error,Errno());
  CloseSocket(fd);
 }
 CloseLibrary(SocketBase);return 0;
}
