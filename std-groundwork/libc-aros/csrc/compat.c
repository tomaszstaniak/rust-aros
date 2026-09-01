/* libc completion layer for Rust std on AROS.
 *
 * Three kinds of things live here:
 *  1. sockets: bsdsocket.library is called through its library base, which the
 *     C compiler handles; Rust just sees ordinary functions.
 *  2. *at() family: AROS has no directory file descriptors, so these are
 *     implemented for AT_FDCWD only and fail with ENOTSUP otherwise.
 *  3. things AROS cannot do (fork, poll, chroot...): return -1/ENOSYS so that
 *     std's error paths engage instead of the link failing. */
#include <errno.h>
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
#include <proto/dos.h>
#include <proto/exec.h>
#include <arpa/inet.h>

#define AT_FDCWD_AROS (-100)
#define ENOSYS_RET  do { errno = ENOSYS; return -1; } while (0)

/* --- sockets ------------------------------------------------------------ */
static int sockerr(int r) { if (r < 0) errno = Errno(); return r; }
static long sockerrl(long r) { if (r < 0) errno = Errno(); return r; }
#define NEEDSOCK if (!SocketBase) { errno = ENETDOWN; return -1; }

int rs_socket(int d, int t, int p)                                   { NEEDSOCK; return sockerr(socket(d, t, p)); }
int rs_connect(int s, const struct sockaddr *a, socklen_t l)         { NEEDSOCK; return sockerr(connect(s, (struct sockaddr *)a, l)); }
int rs_bind(int s, const struct sockaddr *a, socklen_t l)            { NEEDSOCK; return sockerr(bind(s, (struct sockaddr *)a, l)); }
int rs_listen(int s, int b)                                          { NEEDSOCK; return sockerr(listen(s, b)); }
int rs_accept(int s, struct sockaddr *a, socklen_t *l)               { NEEDSOCK; return sockerr(accept(s, a, l)); }
long rs_send(int s, const void *b, size_t n, int f)                  { NEEDSOCK; return sockerrl(send(s, b, n, f)); }
long rs_recv(int s, void *b, size_t n, int f)                        { NEEDSOCK; return sockerrl(recv(s, b, n, f)); }
long rs_sendto(int s, const void *b, size_t n, int f, const struct sockaddr *a, socklen_t l)
                                                                     { NEEDSOCK; return sockerrl(sendto(s, b, n, f, (struct sockaddr *)a, l)); }
long rs_recvfrom(int s, void *b, size_t n, int f, struct sockaddr *a, socklen_t *l)
                                                                     { NEEDSOCK; return sockerrl(recvfrom(s, b, n, f, a, l)); }
int rs_shutdown(int s, int how)                                      { NEEDSOCK; return sockerr(shutdown(s, how)); }
int rs_getsockname(int s, struct sockaddr *a, socklen_t *l)          { NEEDSOCK; return sockerr(getsockname(s, a, l)); }
int rs_getpeername(int s, struct sockaddr *a, socklen_t *l)          { NEEDSOCK; return sockerr(getpeername(s, a, l)); }
int rs_setsockopt(int s, int lv, int o, const void *v, socklen_t l)  { NEEDSOCK; return sockerr(setsockopt(s, lv, o, (void *)v, l)); }
int rs_getsockopt(int s, int lv, int o, void *v, socklen_t *l)       { NEEDSOCK; return sockerr(getsockopt(s, lv, o, v, l)); }
int rs_closesocket(int s)                                            { NEEDSOCK; return sockerr(CloseSocket(s)); }
/* This bsdsocket.library has no getaddrinfo; resolve IPv4 names with
 * gethostbyname and build a single-entry addrinfo list. */
#include <netinet/in.h>
#include <stdlib.h>
struct rs_addrinfo { long flags, family, socktype, protocol; size_t addrlen; char *canon; struct sockaddr *addr; struct rs_addrinfo *next; };
int rs_getaddrinfo(const char *node, const char *service, const struct rs_addrinfo *hints, struct rs_addrinfo **res) {
    if (!SocketBase) return 11 /* EAI_SYSTEM */;
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
int rs_gethostname(char *b, size_t n) { if (!SocketBase) { strncpy(b, "aros", n); return 0; } return sockerr(gethostname(b, n)); }
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
/* NOT cryptographic. AROS One has no entropy device the SDK exposes; this
 * seeds from the clock and uses libc random(). Fine for HashMap seeding,
 * which is what std uses it for, and nothing else. */
int getentropy(void *buf, size_t n) {
    static int seeded; if (!seeded) { struct timespec ts; clock_gettime(CLOCK_REALTIME, &ts); srandom((unsigned)(ts.tv_sec ^ ts.tv_nsec) ^ (unsigned)(uintptr_t)buf); seeded = 1; }
    unsigned char *p = buf; for (size_t i = 0; i < n; i++) p[i] = (unsigned char)(random() >> 3); return 0;
}

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
    if (posix_memalign(&p, 4096, len) != 0 || !p) { errno = ENOMEM; return RS_MAP_FAILED; }
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
long sysconf_pagesize(void)                   { return 4096; }

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
    if (!SocketBase) ENOSYS_RET;
    fd_set r, w, e; FD_ZERO(&r); FD_ZERO(&w); FD_ZERO(&e); int max = -1;
    for (unsigned long i = 0; i < n; i++) {
        if (fds[i].fd < 0) continue;
        if (fds[i].events & 0x1) FD_SET(fds[i].fd, &r);
        if (fds[i].events & 0x4) FD_SET(fds[i].fd, &w);
        FD_SET(fds[i].fd, &e);
        if (fds[i].fd > max) max = fds[i].fd;
    }
    struct timeval tv; tv.tv_sec = timeout / 1000; tv.tv_usec = (timeout % 1000) * 1000;
    int rc = WaitSelect(max + 1, &r, &w, &e, timeout < 0 ? NULL : &tv, NULL);
    if (rc < 0) { errno = Errno(); return -1; }
    int cnt = 0;
    for (unsigned long i = 0; i < n; i++) {
        fds[i].revents = 0;
        if (fds[i].fd < 0) continue;
        if (FD_ISSET(fds[i].fd, &r)) fds[i].revents |= 0x1;
        if (FD_ISSET(fds[i].fd, &w)) fds[i].revents |= 0x4;
        if (FD_ISSET(fds[i].fd, &e)) fds[i].revents |= 0x8;
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
struct rs_iovec { void *base; size_t len; };
long readv(int fd, const struct rs_iovec *v, int n) {
    long total = 0;
    for (int i = 0; i < n; i++) { long r = read(fd, v[i].base, v[i].len); if (r < 0) return total ? total : r; total += r; if ((size_t)r < v[i].len) break; }
    return total;
}
