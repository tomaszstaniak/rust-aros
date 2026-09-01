#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <sys/un.h>
#include <netdb.h>
#include <signal.h>
#include <sys/ioctl.h>
struct sockaddr         v_sa;
struct sockaddr_in      v_sin;
struct sockaddr_in6     v_sin6;
struct sockaddr_un      v_sun;
struct sockaddr_storage v_ss;
struct linger           v_linger;
struct addrinfo         v_ai;
struct hostent          v_he;
socklen_t               v_socklen;
sa_family_t             v_saf;
sigset_t                v_sigset;
in_addr_t               v_inaddr;
in_port_t               v_inport;
int main(void){return 0;}
