/* libc completion layer for Rust std on AROS.
 *
 * Three kinds of things live here:
 *  1. sockets: bsdsocket.library is called through its library base, which the
 *     C compiler handles; Rust just sees ordinary functions.
 *  2. *at() family: AROS has no directory file descriptors, so these are
 *     implemented for AT_FDCWD only and fail with ENOTSUP otherwise.
 *  3. unavailable process/filesystem operations: return -1/ENOSYS so that
 *     std's error paths engage instead of the link failing.
 *
 * RS_AROS_ABIV1 selects mainline AROS (x86_64 ABIv1). Its posixc already
 * provides openat/unlinkat/fstatat/fdopendir (with real directory
 * descriptors), pread, strerror_r, getpwuid_r and pthread_condattr_setclock;
 * those definitions are left to the system there. It lacks writev for files
 * (only net.lib's socket writev exists), fdatasync and
 * pthread_attr_getguardsize, which are supplied below. */
/* unlinkat and the other *at() declarations of mainline posixc are behind this. */
#ifdef RS_AROS_ABIV1
#define _GNU_SOURCE
#endif
#include "terminal_abi.h"
#include <errno.h>
#include <stdlib.h>
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
#include <limits.h>
#include <dos/dosextens.h>
#include <exec/memory.h>
#include <arpa/inet.h>

/* The selected v11 CRT shares its path-conversion buffer and descriptor array.
 * open/close explicitly exclude threaded use. Serialize Rust's filesystem entry
 * points (including descriptor allocation), preserving errno across unlocking.
 * Blocking reads/writes do not hold this gate; callers synchronize shared files. */
static pthread_mutex_t rs_fs_mutex = PTHREAD_MUTEX_INITIALIZER;
#define RS_FS(type, bad, call) do { \
    int lock_error = pthread_mutex_lock(&rs_fs_mutex); \
    if (lock_error) { errno = lock_error; return (bad); } \
    type result = (call); int saved_error = errno; \
    pthread_mutex_unlock(&rs_fs_mutex); errno = saved_error; return result; \
} while (0)

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

/* --- flock ----------------------------------------------------------------
 * posixc's flock() is a named exec SignalSemaphore per file path, "FLOCK(path)"
 * (both SDKs: compiler/crt/posixc/flock.c). That differs from POSIX in three
 * ways that this layer covers for Rust's File::lock family:
 *  - a semaphore nests for its owner task, so a second descriptor of the same
 *    file in the same task got the lock again instead of EWOULDBLOCK;
 *  - LOCK_UN releases the semaphore whoever calls it (its documented BUG),
 *    and on a descriptor without a lock it releases one it does not hold;
 *  - neither close() nor program exit releases a lock: a program that ends
 *    holding one leaves the file locked until posixc.library is expunged.
 * Entries record descriptor, mode, owner task and the semaphore's path. Only
 * the owner task may unlock (others get EPERM, the semaphore would otherwise
 * be released by a non-owner). close() by the owner unlocks first; the exit
 * code releases what the main task still holds. A lock still held by another
 * task (a thread) at its close or at exit cannot be released safely and stays. */
#include <sys/file.h>
#define RS_FLOCKS 64
struct rs_flock { int fd, mode; struct Task *owner; char *name; };
static struct rs_flock rs_flocks[RS_FLOCKS];
static pthread_mutex_t rs_flock_mutex = PTHREAD_MUTEX_INITIALIZER;
extern int flock(int fd, int operation);

static char *rs_flock_name(int fd) {
    long handle = 0;
    if (__get_default_file(fd, &handle) < 0 || !handle) return NULL;
    char *name = malloc(1024);
    if (name && !NameFromFH((BPTR)handle, (STRPTR)name, 1024)) { free(name); name = NULL; }
    return name;
}
static struct rs_flock *rs_flock_find(int fd) {
    for (int i = 0; i < RS_FLOCKS; i++) if (rs_flocks[i].name && rs_flocks[i].fd == fd) return &rs_flocks[i];
    return NULL;
}
static void rs_flock_drop(struct rs_flock *e) { free(e->name); e->name = NULL; e->fd = -1; }

int rs_flock(int fd, int op) {
    int base = op & ~LOCK_NB;
    if (base != LOCK_SH && base != LOCK_EX && base != LOCK_UN) { errno = EINVAL; return -1; }
    struct Task *me = FindTask(NULL);
    pthread_mutex_lock(&rs_flock_mutex);
    struct rs_flock *held = rs_flock_find(fd);
    if (held && held->owner != me) { pthread_mutex_unlock(&rs_flock_mutex); errno = EPERM; return -1; }
    if (base == LOCK_UN || held) {
        int rc = 0;
        if (held && (base == LOCK_UN || held->mode != base)) {
            /* Conversion is unlock + lock, which POSIX permits to be non-atomic. */
            rc = flock(fd, LOCK_UN);
            if (rc == 0) rs_flock_drop(held);
        }
        pthread_mutex_unlock(&rs_flock_mutex);
        if (rc != 0 || base == LOCK_UN || (held && held->name)) return rc;
        pthread_mutex_lock(&rs_flock_mutex);
    }
    char *name = rs_flock_name(fd);
    if (!name) { pthread_mutex_unlock(&rs_flock_mutex); errno = EBADF; return -1; }
    for (int i = 0; i < RS_FLOCKS; i++) {
        struct rs_flock *e = &rs_flocks[i];
        if (e->name && e->owner == me && strcmp(e->name, name) == 0 &&
            (base == LOCK_EX || e->mode == LOCK_EX)) {
            pthread_mutex_unlock(&rs_flock_mutex); free(name);
            errno = (op & LOCK_NB) ? EWOULDBLOCK : EDEADLK; return -1;
        }
    }
    struct rs_flock *slot = NULL;
    for (int i = 0; i < RS_FLOCKS && !slot; i++) if (!rs_flocks[i].name) slot = &rs_flocks[i];
    if (!slot) { pthread_mutex_unlock(&rs_flock_mutex); free(name); errno = ENOLCK; return -1; }
    slot->name = name; slot->fd = -2; slot->owner = me; /* reserved while waiting */
    pthread_mutex_unlock(&rs_flock_mutex);
    int rc = flock(fd, op);
    int error = errno;
    pthread_mutex_lock(&rs_flock_mutex);
    if (rc == 0) { slot->fd = fd; slot->mode = base; } else rs_flock_drop(slot);
    pthread_mutex_unlock(&rs_flock_mutex);
    errno = error;
    return rc;
}
static void rs_flock_close(int fd) {
    pthread_mutex_lock(&rs_flock_mutex);
    struct rs_flock *e = rs_flock_find(fd);
    if (e) {
        if (e->owner == FindTask(NULL) && flock(fd, LOCK_UN) == 0) rs_flock_drop(e);
        else e->fd = -3; /* held by another task: the number may be reused */
    }
    pthread_mutex_unlock(&rs_flock_mutex);
}
static void rs_flock_exit(void) {
    struct Task *me = FindTask(NULL);
    for (int i = 0; i < RS_FLOCKS; i++)
        if (rs_flocks[i].name && rs_flocks[i].owner == me && rs_flocks[i].fd >= 0 &&
            flock(rs_flocks[i].fd, LOCK_UN) == 0)
            rs_flock_drop(&rs_flocks[i]);
}
ADD2EXIT(rs_flock_exit, 10);

/* Rust bindings call these dispatchers; C SDK file calls remain untouched. */
int rs_fd_close(int fd) { if (rs_socket_tagged(fd)) return rs_closesocket(fd); rs_flock_close(fd); RS_FS(int, -1, close(fd)); }
long rs_fd_read(int fd, void *b, size_t n) { return rs_socket_tagged(fd) ? rs_recv(fd, b, n, 0) : read(fd, b, n); }
long rs_fd_write(int fd, const void *b, size_t n) { return rs_socket_tagged(fd) ? rs_send(fd, b, n, 0) : write(fd, b, n); }
/* Descriptor-based geometry, also from worker threads. Public packet layout:
 * AROS PR #1763, head 4fd9618699101fa6425146d7d0c0eb7767fe23cf,
 * compiler/include/dos/consoleinfo.h (version 1, still proposed upstream).
 * Fixed-width local definitions let older SDKs build without that header.
 * Older Limpet uses the frozen XtyControl v1 presentation query (operation
 * 13). Neither path exposes handler-owned pointers. In particular, never
 * interpret ACTION_DISK_INFO.id_InUse: it may be a count, not an IOStdReq.
 */
static int rs_console_size(int fd, struct winsize *ws) {
    struct {
        uint32_t size, version, columns, rows, width, height;
    } geometry = {0};
    struct {
        uint32_t version, operation, reserved[2], values[12];
    } presentation = {1, 13, {0, 0}, {0}};
    _Static_assert(sizeof geometry == 24, "ConsoleGeometry wire size");
    _Static_assert(sizeof presentation == 64, "XtyControl wire size");
    struct winsize result;
    long handle = 0;
    if (!ws) { errno = EFAULT; return -1; }
    if (__get_default_file(fd, &handle) < 0 || !handle) { errno = EBADF; return -1; }
    BPTR file = (BPTR)handle;
    struct FileHandle *fh = BADDR(file);
    if (!IsInteractive(file) || !fh || !fh->fh_Type) { errno = ENOTTY; return -1; }
    /* Arg1 is fh_Arg1, not the BPTR; Arg3 is capacity, Arg4 is version. */
    SIPTR answer = DoPkt((struct MsgPort *)fh->fh_Type, 0x434f4e47L,
                        fh->fh_Arg1, (SIPTR)&geometry, sizeof geometry, 1, 0);
    if (answer == DOSTRUE) {
        if (geometry.size != sizeof geometry || geometry.version != 1 ||
            !geometry.columns || !geometry.rows) {
            errno = ENOTTY; return -1;
        }
    } else {
        /* Only absence of the public operation permits a legacy query.
         * A malformed success or another DOS failure is not negotiable. */
        if (answer != DOSFALSE || IoErr() != ERROR_ACTION_NOT_KNOWN) { errno = ENOTTY; return -1; }
        if (DoPkt((struct MsgPort *)fh->fh_Type, 0x58545901L, fh->fh_Arg1,
                  (SIPTR)&presentation, sizeof presentation, 0x58545931L, 0) != DOSTRUE ||
            presentation.version != 1 || presentation.operation != 13 ||
            presentation.reserved[0] != 0x58545931L || presentation.reserved[1] ||
            !(presentation.values[0] & 8) || !presentation.values[4] ||
            !presentation.values[5]) {
            errno = ENOTTY; return -1;
        }
        /* Unknown bits and future values are deliberately ignored.
         * This protocol reports cells only; pixel dimensions stay zero. */
        geometry.width = geometry.height = 0;
        geometry.rows = presentation.values[4];
        geometry.columns = presentation.values[5];
    }
    if (geometry.rows > USHRT_MAX || geometry.columns > USHRT_MAX ||
        geometry.width > USHRT_MAX || geometry.height > USHRT_MAX) {
        errno = EOVERFLOW; return -1;
    }
    memset(&result, 0, sizeof result);
    result.ws_row = (unsigned short)geometry.rows;
    result.ws_col = (unsigned short)geometry.columns;
    result.ws_xpixel = (unsigned short)geometry.width;
    result.ws_ypixel = (unsigned short)geometry.height;
    *ws = result;
    return 0;
}

int rs_fd_ioctl(int fd, unsigned long request, ...) {
    if ((uint32_t)request == (uint32_t)FIOCLEX || (uint32_t)request == (uint32_t)FIONCLEX) {
        int flags = (uint32_t)request == (uint32_t)FIOCLEX ? FD_CLOEXEC : 0;
        if (!rs_socket_tagged(fd)) { RS_FS(int, -1, fcntl(fd, F_SETFD, flags)); }
        SOCKET_FD(fd); rs_socket_slots[raw].fdflags = flags; return 0;
    }
    /* Other zero-sized requests have no supported contract here. Do not read
       a nonexistent variadic argument to reject one. */
    if (IOCPARM_LEN(request) == 0) { errno = ENOTTY; return -1; }
    va_list ap; va_start(ap, request); void *arg = va_arg(ap, void *); va_end(ap);
    if (!rs_socket_tagged(fd) && (uint32_t)request == (uint32_t)TIOCGWINSZ) return rs_console_size(fd, arg);
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
        if (cmd == F_GETFD || cmd == F_GETFL) { RS_FS(int, -1, fcntl(fd, cmd)); }
        RS_FS(int, -1, fcntl(fd, cmd, arg));
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
    if (!rs_socket_tagged(fd)) { RS_FS(int, -1, dup(fd)); }
    errno = ENOTSUP; return -1;
}
int rs_fd_dup2(int fd, int target) {
    if (!rs_socket_tagged(fd) && !rs_socket_tagged(target)) { RS_FS(int, -1, dup2(fd, target)); }
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
#ifndef RS_AROS_ABIV1
int openat(int fd, const char *p, int fl, ...) {
    ATCHECK(fd);
    mode_t m = 0;
    if (fl & O_CREAT) { __builtin_va_list ap; __builtin_va_start(ap, fl); m = (mode_t)__builtin_va_arg(ap, int); __builtin_va_end(ap); }
    return open(p, fl, m);
}
int unlinkat(int fd, const char *p, int flags)                        { ATCHECK(fd); return (flags & 0x200 /*AT_REMOVEDIR*/) ? rmdir(p) : unlink(p); }
#endif
int mkdirat(int fd, const char *p, mode_t m)                          { ATCHECK(fd); return mkdir(p, m); }
int linkat(int f1, const char *a, int f2, const char *b, int fl)      { ATCHECK(f1); ATCHECK(f2); (void)fl; return link(a, b); }
int renameat(int f1, const char *a, int f2, const char *b)            { ATCHECK(f1); ATCHECK(f2); return rename(a, b); }
#ifndef RS_AROS_ABIV1
int fstatat(int fd, const char *p, struct stat *st, int fl)           { ATCHECK(fd); return (fl & 0x100 /*AT_SYMLINK_NOFOLLOW*/) ? lstat(p, st) : stat(p, st); }
#endif
int utimensat(int fd, const char *p, const struct timespec ts[2], int fl) { ATCHECK(fd); (void)p;(void)ts;(void)fl; ENOSYS_RET; }
int futimens(int fd, const struct timespec ts[2])                     { (void)fd;(void)ts; ENOSYS_RET; }
#ifndef RS_AROS_ABIV1
DIR *fdopendir(int fd)                                                { (void)fd; errno = ENOTSUP; return NULL; }

/* --- odds and ends std links against ------------------------------------ */
int strerror_r(int e, char *b, size_t n) { const char *m = strerror(e); if (!m) return EINVAL; strncpy(b, m, n); if (n) b[n - 1] = 0; return 0; }
#else
/* Declared only under some feature macros in the mainline headers. */
int posix_memalign(void **memptr, size_t alignment, size_t size);
/* No unbuffered data is kept by posixc; fsync is the strongest request. */
int fdatasync(int fd) { return fsync(fd); }
/* pthread.library has no guard pages and no getter for them. */
int pthread_attr_getguardsize(const pthread_attr_t *a, size_t *s) { (void)a; if (!s) return EINVAL; *s = 0; return 0; }
#endif
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
#ifndef RS_AROS_ABIV1
int pthread_condattr_setclock(pthread_condattr_t *a, clockid_t c)     { (void)a;(void)c; errno = ENOTSUP; return ENOTSUP; }
int getpwuid_r(uid_t u, void *pw, char *b, size_t n, void **r)        { (void)u;(void)pw;(void)b;(void)n; *r = NULL; return ENOENT; }
#endif

/* poll over select: enough for std's connect_timeout and friends */
struct rs_pollfd { int fd; short events; short revents; };

/* One console or file descriptor waiting for input, no sockets: what a
 * terminal program's event loop asks for. WaitForChar waits inside the
 * console handler, so an idle wait costs one packet per slice instead of a
 * wake-up per poll. WaitSelect cannot take DOS handles, so a set mixing them
 * with sockets stays unsupported. */
#define RS_WAIT_SLICE_US 1000000L
static int rs_poll_handle(struct rs_pollfd *p, int timeout) {
    long handle = 0;
    p->revents = 0;
    if (p->events & ~0x5) { errno = ENOTSUP; return -1; }
    if (__get_default_file(p->fd, &handle) < 0 || !handle) { p->revents = 0x20; return 1; }
    BPTR file = (BPTR)handle;
    if (!IsInteractive(file)) {
        /* Files are always ready, as POSIX defines it for regular files. */
        p->revents = p->events & 0x5;
        return p->revents ? 1 : 0;
    }
    if (p->events & 0x4) p->revents |= 0x4; /* console writes do not block */
    if (!(p->events & 0x1)) return p->revents ? 1 : 0;
    if (p->revents) timeout = 0;
    for (;;) {
        long slice = timeout < 0 || timeout >= RS_WAIT_SLICE_US / 1000 ? RS_WAIT_SLICE_US : (long)timeout * 1000;
        if (WaitForChar(file, slice)) { p->revents |= 0x1; return 1; }
        if (IoErr()) { p->revents |= 0x8; return 1; }
        if (timeout >= 0) {
            timeout -= (int)(slice / 1000);
            if (timeout <= 0) return p->revents ? 1 : 0;
        }
    }
}

int poll(struct rs_pollfd *fds, unsigned long n, int timeout) {
    if (n && !fds) { errno = EFAULT; return -1; }
    {
        unsigned long handles = 0, sockets = 0, last = 0;
        for (unsigned long i = 0; i < n; i++) {
            if (fds[i].fd < 0) continue;
            if (rs_socket_tagged(fds[i].fd)) sockets++; else { handles++; last = i; }
        }
        if (handles && sockets) { errno = ENOTSUP; return -1; }
        if (handles > 1) { errno = ENOTSUP; return -1; }
        if (handles == 1) {
            for (unsigned long i = 0; i < n; i++) fds[i].revents = 0;
            return rs_poll_handle(&fds[last], timeout);
        }
    }
    if (rs_socket_ready() < 0) return -1;
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
#ifndef RS_AROS_ABIV1
long pread(int fd, void *b, size_t n, off_t off) {
    off_t cur = lseek(fd, 0, SEEK_CUR); if (cur < 0) return -1;
    if (lseek(fd, off, SEEK_SET) < 0) return -1;
    long r = read(fd, b, n); lseek(fd, cur, SEEK_SET); return r;
}
#endif
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
#ifdef RS_AROS_ABIV1
/* Mainline's only writev is net.lib's, for sockets. Files get the same
 * short-write semantics as readv above. */
static long rs_file_writev(int fd, const struct iovec *v, int n) {
    long total = 0;
    for (int i = 0; i < n; i++) { long r = write(fd, v[i].iov_base, v[i].iov_len); if (r < 0) return total ? total : r; total += r; if ((size_t)r < v[i].iov_len) break; }
    return total;
}
#define writev rs_file_writev
#endif
long rs_fd_writev(int fd, const struct iovec *v, int n) {
    if (!rs_socket_tagged(fd)) return writev(fd, (const struct iovec *)v, n);
    if (n < 0) { errno = EINVAL; return -1; }
    for (int i = 0; i < n; i++) if (v[i].iov_len) return rs_send(fd, v[i].iov_base, v[i].iov_len, 0);
    return 0;
}

#include "selector_aros.inc"

#ifdef RS_AROS_ABIV1
/* Mainline posixc writes the errno of a failed open() to its own stdc base instead of the
 * calling thread's errno (since "stdc: make errno per thread", 956444232f, 2026-11-06; seen on
 * the nightly 20261110): errno keeps whatever an earlier call left, so a missing file read as
 * "File exists" or "No error". errno is cleared before the call; when the call fails and
 * leaves it clear, the reason is taken from the path. With a fixed posixc errno is set and
 * nothing here applies. */
static int rs_open_errno(const char *p, int flags, mode_t mode) {
    struct stat st;
    int fd;
    errno = 0;
    fd = open(p, flags, mode);
    if (fd < 0 && errno == 0) {
        if (stat(p, &st) == 0)
            errno = ((flags & O_CREAT) && (flags & O_EXCL)) ? EEXIST : S_ISDIR(st.st_mode) ? EISDIR : EACCES;
        else if (errno == 0)
            errno = ENOENT;
    }
    return fd;
}
#else
#define rs_open_errno open
#endif

/* Names are distinct from CRT symbols: internal CRT calls cannot re-enter this gate. */
int rs_fs_open(const char *p, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) { va_list ap; va_start(ap, flags); mode = (mode_t)va_arg(ap, int); va_end(ap); }
    RS_FS(int, -1, rs_open_errno(p, flags, mode));
}
int rs_fs_stat(const char *p, struct stat *s) { RS_FS(int, -1, stat(p,s)); }
int rs_fs_lstat(const char *p, struct stat *s) { RS_FS(int, -1, lstat(p,s)); }
int rs_fs_mkdir(const char *p, mode_t m) { RS_FS(int, -1, mkdir(p,m)); }
int rs_fs_rmdir(const char *p) { RS_FS(int, -1, rmdir(p)); }
int rs_fs_unlink(const char *p) { RS_FS(int, -1, unlink(p)); }
int rs_fs_rename(const char *a, const char *b) { RS_FS(int, -1, rename(a,b)); }
int rs_fs_link(const char *a, const char *b) { RS_FS(int, -1, link(a,b)); }
int rs_fs_symlink(const char *a, const char *b) { RS_FS(int, -1, symlink(a,b)); }
ssize_t rs_fs_readlink(const char *p, char *b, size_t n) { RS_FS(ssize_t, -1, readlink(p,b,n)); }
char *rs_fs_realpath(const char *p, char *b) { RS_FS(char *, NULL, realpath(p,b)); }
int rs_fs_chdir(const char *p) { RS_FS(int, -1, chdir(p)); }
char *rs_fs_getcwd(char *b, size_t n) { RS_FS(char *, NULL, getcwd(b,n)); }
DIR *rs_fs_opendir(const char *p) { RS_FS(DIR *, NULL, opendir(p)); }
int rs_fs_closedir(DIR *d) { RS_FS(int, -1, closedir(d)); }
int rs_fs_pipe(int p[2]) { RS_FS(int, -1, pipe(p)); }
int rs_fs_chown(const char *p, uid_t u, gid_t g) { RS_FS(int, -1, chown(p,u,g)); }
int rs_fs_lchown(const char *p, uid_t u, gid_t g) { RS_FS(int, -1, lchown(p,u,g)); }
int rs_fs_chmod(const char *p, mode_t m) { RS_FS(int, -1, chmod(p,m)); }
int rs_fs_openat(int fd, const char *p, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) { va_list ap; va_start(ap, flags); mode = (mode_t)va_arg(ap, int); va_end(ap); }
    RS_FS(int, -1, openat(fd,p,flags,mode));
}
int rs_fs_mkdirat(int fd, const char *p, mode_t m) { RS_FS(int, -1, mkdirat(fd,p,m)); }
int rs_fs_unlinkat(int fd, const char *p, int f) { RS_FS(int, -1, unlinkat(fd,p,f)); }
int rs_fs_linkat(int a, const char *p, int b, const char *q, int f) { RS_FS(int, -1, linkat(a,p,b,q,f)); }
int rs_fs_renameat(int a, const char *p, int b, const char *q) { RS_FS(int, -1, renameat(a,p,b,q)); }
int rs_fs_fstatat(int fd, const char *p, struct stat *s, int f) { RS_FS(int, -1, fstatat(fd,p,s,f)); }
