/* libc completion layer for Rust std on AROS.
 *
 * Three kinds of things live here:
 *  1. sockets: bsdsocket.library is called through its library base, which the
 *     C compiler handles; Rust just sees ordinary functions.
 *  2. *at() family: AROS has no directory file descriptors, so these are
 *     implemented for AT_FDCWD only and fail with ENOTSUP otherwise.
 *  3. unavailable process/filesystem operations: return -1/ENOSYS so that
 *     std's error paths engage instead of the link failing. */
#include "terminal_abi.h"
#include <errno.h>
#include <stdarg.h>
#include <sys/ioctl.h>
#include <sys/uio.h>
#include <stdint.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <dirent.h>
#include <time.h>
#include <pthread.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <netdb.h>
#include <proto/bsdsocket.h>
#include <bsdsocket/socketbasetags.h>
#include <aros/symbolsets.h>
#include <proto/dos.h>
#include <proto/exec.h>
#include <arpa/inet.h>

#define AT_FDCWD_AROS (-100)
#define ENOSYS_RET  do { errno = ENOSYS; return -1; } while (0)

/* --- sockets ------------------------------------------------------------ */
/* SocketBase is defined here, not taken from the SDK's net.lib: referencing
 * it there pulls in net.lib's autoinit, which puts up "Unable to open
 * bsdsocket.library" and exits when no TCP/IP stack is running. That stopped
 * every std program at startup, network or not. Here the library is opened
 * the same way at startup but is optional: without it SocketBase stays NULL
 * and the socket calls below fail with ENETDOWN. A stack started after the
 * program is not picked up. */
struct Library *SocketBase = NULL;
static struct Task *rs_socket_owner;
static void rs_selector_shutdown(void);
/* BSD and DOS descriptors are separate namespaces in the named old v11.
 * Rust-owned socket handles carry a private tag. They are not raw BSD fds.
 * Only the library-opening task performs socket I/O. Foreign-task drops are
 * queued until its next socket operation or library shutdown. */
#define RS_SOCKET_TAG 0x40000000
struct rs_socket_slot { unsigned char live, pending; int fdflags, status; };
static struct rs_socket_slot rs_socket_slots[FD_SETSIZE];
static int rs_socket_tagged(int fd) { return fd >= RS_SOCKET_TAG; }
static int rs_socket_index(int fd) {
    if (!rs_socket_tagged(fd)) { errno = EBADF; return -1; }
    int raw = fd - RS_SOCKET_TAG;
    if (raw >= FD_SETSIZE ||
        !rs_socket_slots[raw].live || rs_socket_slots[raw].pending) {
        errno = EBADF; return -1;
    }
    return raw;
}
static int rs_socket_drain(void) {
    for (int i = 0; i < FD_SETSIZE; i++) {
        Forbid();
        int pending = rs_socket_slots[i].pending;
        Permit();
        if (!pending) continue;
        if (CloseSocket(i) < 0) { errno = Errno(); return -1; }
        Forbid();
        rs_socket_slots[i].pending = rs_socket_slots[i].live = 0;
        Permit();
    }
    return 0;
}
static int rs_socket_ready(void) {
    if (!SocketBase) { errno = ENETDOWN; return -1; }
    if (FindTask(NULL) != rs_socket_owner) { errno = ENOTSUP; return -1; }
    return rs_socket_drain();
}
static int rs_socket_wrap(int raw) {
    if (raw < 0) { errno = Errno(); return -1; }
    if (raw >= FD_SETSIZE) { CloseSocket(raw); errno = EMFILE; return -1; }
    rs_socket_slots[raw] = (struct rs_socket_slot){1, 0, 0, O_RDWR};
    return RS_SOCKET_TAG + raw;
}
extern STRPTR _ProgramName;

static ULONG rs_open_sockets(void)
{
    rs_socket_owner = FindTask(NULL);
    if ((SocketBase = OpenLibrary("bsdsocket.library", 4)) != NULL)
        SocketBaseTags(SBTM_SETVAL(SBTC_ERRNOPTR(sizeof(errno))), (IPTR)&errno,
                       SBTM_SETVAL(SBTC_LOGTAGPTR), (IPTR)_ProgramName,
                       TAG_END);
    return TRUE;
}

static void rs_close_sockets(void)
{
    if (SocketBase) {
        rs_socket_drain();
        rs_selector_shutdown();
        CloseLibrary(SocketBase);
        SocketBase = NULL;
    }
}

ADD2INIT(rs_open_sockets, 10);
ADD2EXIT(rs_close_sockets, 10);

static int sockerr(int r) { if (r < 0) errno = Errno(); return r; }
static long sockerrl(long r) { if (r < 0) errno = Errno(); return r; }
#define NEEDSOCK if (rs_socket_ready() < 0) return -1
#define SOCKET_FD(s) NEEDSOCK; int raw = rs_socket_index(s); if (raw < 0) return -1

int rs_socket(int d, int t, int p) { NEEDSOCK; return rs_socket_wrap(socket(d, t, p)); }
int rs_connect(int s, const struct sockaddr *a, socklen_t l) { SOCKET_FD(s); return sockerr(connect(raw, (struct sockaddr *)a, l)); }
int rs_bind(int s, const struct sockaddr *a, socklen_t l) { SOCKET_FD(s); return sockerr(bind(raw, (struct sockaddr *)a, l)); }
int rs_listen(int s, int b) { SOCKET_FD(s); return sockerr(listen(raw, b)); }
int rs_accept(int s, struct sockaddr *a, socklen_t *l) { SOCKET_FD(s); return rs_socket_wrap(accept(raw, a, l)); }
long rs_send(int s, const void *b, size_t n, int f) { SOCKET_FD(s); return sockerrl(send(raw, b, n, f)); }
long rs_recv(int s, void *b, size_t n, int f) { SOCKET_FD(s); return sockerrl(recv(raw, b, n, f)); }
long rs_sendto(int s, const void *b, size_t n, int f, const struct sockaddr *a, socklen_t l) {
    SOCKET_FD(s); return sockerrl(sendto(raw, b, n, f, (struct sockaddr *)a, l));
}
long rs_recvfrom(int s, void *b, size_t n, int f, struct sockaddr *a, socklen_t *l) {
    SOCKET_FD(s); return sockerrl(recvfrom(raw, b, n, f, a, l));
}
int rs_shutdown(int s, int how) { SOCKET_FD(s); return sockerr(shutdown(raw, how)); }
int rs_getsockname(int s, struct sockaddr *a, socklen_t *l) { SOCKET_FD(s); return sockerr(getsockname(raw, a, l)); }
int rs_getpeername(int s, struct sockaddr *a, socklen_t *l) { SOCKET_FD(s); return sockerr(getpeername(raw, a, l)); }
int rs_setsockopt(int s, int lv, int o, const void *v, socklen_t l) { SOCKET_FD(s); return sockerr(setsockopt(raw, lv, o, (void *)v, l)); }
int rs_getsockopt(int s, int lv, int o, void *v, socklen_t *l) { SOCKET_FD(s); return sockerr(getsockopt(raw, lv, o, v, l)); }
int rs_closesocket(int s) {
    int raw = rs_socket_index(s); if (raw < 0) return -1;
    if (FindTask(NULL) != rs_socket_owner) {
        Forbid(); rs_socket_slots[raw].pending = 1; Permit(); return 0;
    }
    int rc = sockerr(CloseSocket(raw));
    if (rc == 0) rs_socket_slots[raw].live = 0;
    return rc;
}

/* Rust bindings call these dispatchers; C SDK file calls remain untouched. */
int rs_fd_close(int fd) { return rs_socket_tagged(fd) ? rs_closesocket(fd) : close(fd); }
long rs_fd_read(int fd, void *b, size_t n) { return rs_socket_tagged(fd) ? rs_recv(fd, b, n, 0) : read(fd, b, n); }
long rs_fd_write(int fd, const void *b, size_t n) { return rs_socket_tagged(fd) ? rs_send(fd, b, n, 0) : write(fd, b, n); }
int rs_fd_ioctl(int fd, unsigned long request, ...) {
    if ((uint32_t)request == (uint32_t)FIOCLEX || (uint32_t)request == (uint32_t)FIONCLEX) {
        int flags = (uint32_t)request == (uint32_t)FIOCLEX ? FD_CLOEXEC : 0;
        if (!rs_socket_tagged(fd)) return fcntl(fd, F_SETFD, flags);
        SOCKET_FD(fd); rs_socket_slots[raw].fdflags = flags; return 0;
    }
    /* Other zero-sized requests have no supported contract here. Do not read
       a nonexistent variadic argument to reject one. */
    if (IOCPARM_LEN(request) == 0) { errno = ENOTTY; return -1; }
    va_list ap; va_start(ap, request); void *arg = va_arg(ap, void *); va_end(ap);
    if (!rs_socket_tagged(fd)) return ioctl(fd, (int)request, arg);
    SOCKET_FD(fd);
    /* IoctlSocket's AROS library boundary expects a canonical LONG request,
       including sign extension for _IOW requests such as FIONBIO. */
    LONG command = (LONG)(int32_t)(uint32_t)request;
    int rc = sockerr(IoctlSocket(raw, command, arg));
    if (rc == 0 && (uint32_t)request == (uint32_t)FIONBIO)
        rs_socket_slots[raw].status = O_RDWR | (*(int *)arg ? O_NONBLOCK : 0);
    return rc;
}
int rs_fd_fcntl(int fd, int cmd, ...) {
    if (cmd < F_DUPFD || cmd > F_SETFL) { errno = EINVAL; return -1; }
    int arg = 0;
    if (cmd != F_GETFD && cmd != F_GETFL) {
        va_list ap; va_start(ap, cmd); arg = va_arg(ap, int); va_end(ap);
    }
    if (!rs_socket_tagged(fd)) {
        if (cmd == F_GETFD || cmd == F_GETFL) return fcntl(fd, cmd);
        return fcntl(fd, cmd, arg);
    }
    SOCKET_FD(fd);
    switch (cmd) {
    case F_GETFD: return rs_socket_slots[raw].fdflags;
    case F_SETFD:
        if (arg & ~FD_CLOEXEC) { errno = EINVAL; return -1; }
        rs_socket_slots[raw].fdflags = arg; return 0;
    case F_GETFL: return rs_socket_slots[raw].status;
    case F_SETFL: {
        if (arg & ~(O_RDWR | O_NONBLOCK)) { errno = ENOTSUP; return -1; }
        int nonblocking = !!(arg & O_NONBLOCK);
        return rs_fd_ioctl(fd, (unsigned long)FIONBIO, &nonblocking);
    }
    default: errno = ENOTSUP; return -1;
    }
}
int rs_fd_dup(int fd) {
    if (!rs_socket_tagged(fd)) return dup(fd);
    errno = ENOTSUP; return -1;
}
int rs_fd_dup2(int fd, int target) {
    if (!rs_socket_tagged(fd) && !rs_socket_tagged(target)) return dup2(fd, target);
    errno = ENOTSUP; return -1;
}
/* This bsdsocket.library has no getaddrinfo; resolve IPv4 names with
 * gethostbyname and build a single-entry addrinfo list. */
#include <netinet/in.h>
#include <stdlib.h>
struct rs_addrinfo { long flags, family, socktype, protocol; size_t addrlen; char *canon; struct sockaddr *addr; struct rs_addrinfo *next; };
int rs_getaddrinfo(const char *node, const char *service, const struct rs_addrinfo *hints, struct rs_addrinfo **res) {
    if (rs_socket_ready() < 0) return 11 /* EAI_SYSTEM */;
    struct sockaddr_in *sin = calloc(1, sizeof *sin);
    struct rs_addrinfo *ai = calloc(1, sizeof *ai);
    if (!sin || !ai) { free(sin); free(ai); return 10 /* EAI_MEMORY */; }
    sin->sin_len = sizeof *sin; sin->sin_family = AF_INET;
    sin->sin_port = htons(service ? (unsigned short)atoi(service) : 0);
    if (!node) sin->sin_addr.s_addr = htonl(INADDR_ANY);
    else if ((sin->sin_addr.s_addr = inet_addr((char *)node)) == 0xFFFFFFFFu && strcmp(node, "255.255.255.255") != 0) {
        struct hostent *he = gethostbyname((char *)node);
        if (!he || he->h_addrtype != AF_INET) { free(sin); free(ai); return 8 /* EAI_NONAME */; }
        memcpy(&sin->sin_addr, he->h_addr_list[0], sizeof sin->sin_addr);
    }
    ai->family = AF_INET; ai->socktype = hints ? hints->socktype : SOCK_STREAM;
    ai->protocol = hints ? hints->protocol : 0; ai->addrlen = sizeof *sin; ai->addr = (struct sockaddr *)sin;
    *res = ai; return 0;
}
void rs_freeaddrinfo(struct rs_addrinfo *a) { while (a) { struct rs_addrinfo *n = a->next; free(a->addr); free(a); a = n; } }
const char *gai_strerror(int e) { (void)e; return "name resolution failed"; }
int rs_gethostname(char *b, size_t n) { if (!SocketBase) { strncpy(b, "aros", n); return 0; } NEEDSOCK; return sockerr(gethostname(b, n)); }
int socketpair(int d, int t, int p, int sv[2])                       { (void)d;(void)t;(void)p;(void)sv; ENOSYS_RET; }

/* --- *at family, AT_FDCWD only ------------------------------------------ */
#define ATCHECK(fd) if ((fd) != AT_FDCWD_AROS) { errno = ENOTSUP; return -1; }
int openat(int fd, const char *p, int fl, ...) {
    ATCHECK(fd);
    mode_t m = 0;
    if (fl & O_CREAT) { __builtin_va_list ap; __builtin_va_start(ap, fl); m = (mode_t)__builtin_va_arg(ap, int); __builtin_va_end(ap); }
    return open(p, fl, m);
}
int mkdirat(int fd, const char *p, mode_t m)                          { ATCHECK(fd); return mkdir(p, m); }
int unlinkat(int fd, const char *p, int flags)                        { ATCHECK(fd); return (flags & 0x200 /*AT_REMOVEDIR*/) ? rmdir(p) : unlink(p); }
int linkat(int f1, const char *a, int f2, const char *b, int fl)      { ATCHECK(f1); ATCHECK(f2); (void)fl; return link(a, b); }
int renameat(int f1, const char *a, int f2, const char *b)            { ATCHECK(f1); ATCHECK(f2); return rename(a, b); }
int fstatat(int fd, const char *p, struct stat *st, int fl)           { ATCHECK(fd); return (fl & 0x100 /*AT_SYMLINK_NOFOLLOW*/) ? lstat(p, st) : stat(p, st); }
int utimensat(int fd, const char *p, const struct timespec ts[2], int fl) { ATCHECK(fd); (void)p;(void)ts;(void)fl; ENOSYS_RET; }
int futimens(int fd, const struct timespec ts[2])                     { (void)fd;(void)ts; ENOSYS_RET; }
DIR *fdopendir(int fd)                                                { (void)fd; errno = ENOTSUP; return NULL; }

/* --- odds and ends std links against ------------------------------------ */
int strerror_r(int e, char *b, size_t n) { const char *m = strerror(e); if (!m) return EINVAL; strncpy(b, m, n); if (n) b[n - 1] = 0; return 0; }
/* --- memory mapping ------------------------------------------------------
 * AROS has no MMU-backed mapping. Anonymous maps become plain allocations,
 * and a read-only file map is satisfied by reading the range into memory.
 * That is correct for readers (search tools, parsers, loaders) but it is NOT
 * a real mapping: a shared writable file map would have to write changes back,
 * so it is refused rather than silently losing them. mprotect is likewise
 * refused for anything that asks for real protection, so nothing can mistake
 * this for memory safety it does not have. */
#define RS_PROT_READ   1
#define RS_PROT_WRITE  2
#define RS_MAP_SHARED  1
#define RS_MAP_ANON    0x1000
#define RS_MAP_FAILED  ((void *)-1)
#define RS_MAP_GRANULE 4096

long pread(int fd, void *b, size_t n, off_t off);   /* defined below */

struct rs_map { void *addr; size_t len; };
static struct rs_map rs_maps[64];
static pthread_mutex_t rs_maps_lock = PTHREAD_MUTEX_INITIALIZER;

void *mmap(void *hint, size_t len, int prot, int flags, int fd, off_t off) {
    (void)hint;
    if (len == 0) { errno = EINVAL; return RS_MAP_FAILED; }
    int anon = (flags & RS_MAP_ANON) || fd < 0;
    if (!anon && (prot & RS_PROT_WRITE) && (flags & RS_MAP_SHARED)) {
        errno = ENOTSUP;                 /* would have to write back; we cannot */
        return RS_MAP_FAILED;
    }
    void *p = NULL;
    if (posix_memalign(&p, RS_MAP_GRANULE, len) != 0 || !p) { errno = ENOMEM; return RS_MAP_FAILED; }
    if (anon) {
        memset(p, 0, len);
    } else {
        size_t got = 0;
        while (got < len) {
            long n = pread(fd, (char *)p + got, len - got, off + (off_t)got);
            if (n < 0) { free(p); return RS_MAP_FAILED; }
            if (n == 0) { memset((char *)p + got, 0, len - got); break; }
            got += (size_t)n;
        }
    }
    pthread_mutex_lock(&rs_maps_lock);
    int slot = -1;
    for (int i = 0; i < 64; i++) if (!rs_maps[i].addr) { slot = i; break; }
    if (slot >= 0) { rs_maps[slot].addr = p; rs_maps[slot].len = len; }
    pthread_mutex_unlock(&rs_maps_lock);
    if (slot < 0) { free(p); errno = ENOMEM; return RS_MAP_FAILED; }
    return p;
}

int munmap(void *addr, size_t len) {
    (void)len;
    pthread_mutex_lock(&rs_maps_lock);
    int found = 0;
    for (int i = 0; i < 64; i++) if (rs_maps[i].addr == addr) { rs_maps[i].addr = NULL; found = 1; break; }
    pthread_mutex_unlock(&rs_maps_lock);
    if (!found) { errno = EINVAL; return -1; }
    free(addr);
    return 0;
}

/* Only the no-op direction is honest: asking for fewer permissions than we
 * can enforce is fine, asking for real protection is not. */
int mprotect(void *addr, size_t len, int prot) {
    (void)addr; (void)len;
    if (prot & ~(RS_PROT_READ | RS_PROT_WRITE)) { errno = ENOTSUP; return -1; }
    return 0;
}
/* Nothing is pageable, so locking is a no-op that cannot be wrong. */
int mlock(const void *addr, size_t len)       { (void)addr;(void)len; return 0; }
int munlock(const void *addr, size_t len)     { (void)addr;(void)len; return 0; }
int msync(void *addr, size_t len, int flags)  { (void)addr;(void)len;(void)flags; return 0; }
int madvise(void *addr, size_t len, int adv)  { (void)addr;(void)len;(void)adv; return 0; }
long sysconf_pagesize(void)                   { return RS_MAP_GRANULE; }
/* memmap2 uses this value to reconstruct the original address at Drop.
 * The old SDK has no page-size answer. Report the copy-map granule through
 * Rust's private boundary; preserve SDK semantics for every other selector. */
long rs_sysconf(int name) {
    if (name == _SC_PAGESIZE || name == _SC_PAGE_SIZE) return sysconf_pagesize();
    return sysconf(name);
}

/* --- not available on AROS ---------------------------------------------- */
/* fork is provided in Rust as a direct alias of vfork: vfork returns twice into
 * the SAME stack frame, so it must not go through a wrapper function that
 * returns (the child would then run on a frame the parent has already left). */
int chroot(const char *p)                                             { (void)p; ENOSYS_RET; }
int mkfifo(const char *p, mode_t m)                                   { (void)p;(void)m; ENOSYS_RET; }
int killpg(pid_t g, int s)                                            { (void)g;(void)s; ENOSYS_RET; }
int setpgid(pid_t a, pid_t b)                                         { (void)a;(void)b; ENOSYS_RET; }
int setgroups(int n, const gid_t *g)                                  { (void)n;(void)g; ENOSYS_RET; }
int lchown(const char *p, uid_t u, gid_t g)                           { return chown(p, u, g); }
/* Returning success here would be a lie with teeth: callers then compute
 * deadlines against a clock the condvar does not use, and timed waits never
 * fire. Fail honestly instead. */
int pthread_condattr_setclock(pthread_condattr_t *a, clockid_t c)     { (void)a;(void)c; errno = ENOTSUP; return ENOTSUP; }
int getpwuid_r(uid_t u, void *pw, char *b, size_t n, void **r)        { (void)u;(void)pw;(void)b;(void)n; *r = NULL; return ENOENT; }

/* poll over select: enough for std's connect_timeout and friends */
struct rs_pollfd { int fd; short events; short revents; };
int poll(struct rs_pollfd *fds, unsigned long n, int timeout) {
    if (rs_socket_ready() < 0) return -1;
    if (n && !fds) { errno = EFAULT; return -1; }
    fd_set r, w, e; FD_ZERO(&r); FD_ZERO(&w); FD_ZERO(&e);
    int max = -1, invalid = 0;
    for (unsigned long i = 0; i < n; i++) {
        fds[i].revents = 0;
        if (fds[i].fd < 0) continue;
        int raw = rs_socket_index(fds[i].fd);
        if (raw < 0) { fds[i].revents = 0x20; invalid++; continue; }
        /* This adapter accepts socket IN/OUT readiness only. select's
           exceptfds means urgent data, not POLLERR; do not mislabel it. */
        if (!fds[i].events || (fds[i].events & ~0x5)) { errno = ENOTSUP; return -1; }
        if (fds[i].events & 0x1) FD_SET(raw, &r);
        if (fds[i].events & 0x4) FD_SET(raw, &w);
        if (raw > max) max = raw;
    }
    if (invalid) timeout = 0;
    struct timeval tv; tv.tv_sec = timeout / 1000; tv.tv_usec = (timeout % 1000) * 1000;
    int rc = WaitSelect(max + 1, &r, &w, &e, timeout < 0 ? NULL : &tv, NULL);
    if (rc < 0) { errno = Errno(); return -1; }
    int cnt = invalid;
    for (unsigned long i = 0; i < n; i++) {
        if (fds[i].fd < 0 || fds[i].revents) continue;
        int raw = rs_socket_index(fds[i].fd);
        if (raw < 0) { fds[i].revents = 0x20; cnt++; continue; }
        if (FD_ISSET(raw, &r)) fds[i].revents |= 0x1;
        if (FD_ISSET(raw, &w)) fds[i].revents |= 0x4;
        if (fds[i].revents) cnt++;
    }
    return cnt;
}

/* positional and vectored I/O over plain calls */
long pread(int fd, void *b, size_t n, off_t off) {
    off_t cur = lseek(fd, 0, SEEK_CUR); if (cur < 0) return -1;
    if (lseek(fd, off, SEEK_SET) < 0) return -1;
    long r = read(fd, b, n); lseek(fd, cur, SEEK_SET); return r;
}
long pwrite(int fd, const void *b, size_t n, off_t off) {
    off_t cur = lseek(fd, 0, SEEK_CUR); if (cur < 0) return -1;
    if (lseek(fd, off, SEEK_SET) < 0) return -1;
    long r = write(fd, b, n); lseek(fd, cur, SEEK_SET); return r;
}
long readv(int fd, const struct iovec *v, int n) {
    long total = 0;
    for (int i = 0; i < n; i++) { long r = read(fd, v[i].iov_base, v[i].iov_len); if (r < 0) return total ? total : r; total += r; if ((size_t)r < v[i].iov_len) break; }
    return total;
}

long rs_fd_readv(int fd, const struct iovec *v, int n) {
    if (!rs_socket_tagged(fd)) return readv(fd, v, n);
    if (n < 0) { errno = EINVAL; return -1; }
    for (int i = 0; i < n; i++) if (v[i].iov_len) return rs_recv(fd, v[i].iov_base, v[i].iov_len, 0);
    return 0;
}
long rs_fd_writev(int fd, const struct iovec *v, int n) {
    if (!rs_socket_tagged(fd)) return writev(fd, (const struct iovec *)v, n);
    if (n < 0) { errno = EINVAL; return -1; }
    for (int i = 0; i < n; i++) if (v[i].iov_len) return rs_send(fd, v[i].iov_base, v[i].iov_len, 0);
    return 0;
}

#include "selector_aros.inc"
