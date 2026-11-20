#define _GNU_SOURCE
#include <stddef.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
_Static_assert(sizeof(struct msghdr) == 48 && _Alignof(struct msghdr) == 8, "msghdr size");
_Static_assert(offsetof(struct msghdr, msg_namelen) == 8, "namelen");
_Static_assert(offsetof(struct msghdr, msg_iov) == 16, "iov");
_Static_assert(offsetof(struct msghdr, msg_iovlen) == 24, "iovlen");
_Static_assert(offsetof(struct msghdr, msg_control) == 32, "control");
_Static_assert(offsetof(struct msghdr, msg_controllen) == 40, "controllen");
_Static_assert(offsetof(struct msghdr, msg_flags) == 44, "flags");
_Static_assert(sizeof(struct ip_mreq_source) == 12, "ip_mreq_source");
_Static_assert(offsetof(struct ip_mreq_source, imr_sourceaddr) == 4, "sourceaddr");
_Static_assert(offsetof(struct ip_mreq_source, imr_interface) == 8, "interface");
