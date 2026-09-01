/* bsdsocket.library is called through its library base, with the base in a
 * register the compiler manages for us (r12 on x86_64). Rather than teach
 * Rust that convention, let the AROS C compiler emit it: each wrapper here is
 * an ordinary SysV function Rust can declare with extern "C".
 * libnet.a opens SocketBase at startup and closes it at exit. */
#include <proto/bsdsocket.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <sys/time.h>

int  aros_socket(int d, int t, int p)                          { return socket(d, t, p); }
int  aros_connect(int s, const struct sockaddr *a, int l)      { return connect(s, (struct sockaddr *)a, l); }
int  aros_bind(int s, const struct sockaddr *a, int l)         { return bind(s, (struct sockaddr *)a, l); }
int  aros_listen(int s, int b)                                 { return listen(s, b); }
int  aros_accept(int s, struct sockaddr *a, int *l)            { return accept(s, a, (socklen_t *)l); }
long aros_send(int s, const void *b, long n, int f)            { return send(s, b, n, f); }
long aros_recv(int s, void *b, long n, int f)                  { return recv(s, b, n, f); }
int  aros_closesocket(int s)                                   { return CloseSocket(s); }
int  aros_sock_errno(void)                                     { return Errno(); }
int  aros_socket_available(void)                               { return SocketBase != NULL; }
