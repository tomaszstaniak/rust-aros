/* Owner-task selector for polling/async-io. Included by compat.c so tagged
 * descriptors share the accepted validation/ownership boundary. Each selector
 * owns an AllocSignal bit. Never clear that bit before WaitSelect: a notify
 * just before entering the wait must remain pending. */
struct rs_selector {
    struct Task *owner;
    ULONG mask;
    BYTE bit;
    volatile int retired;
    struct rs_selector *next;
};
static struct rs_selector *rs_selectors;

static void rs_selector_reap(void) {
    struct rs_selector **link = &rs_selectors;
    while (*link) {
        struct rs_selector *s = *link;
        if (!s->retired) { link = &s->next; continue; }
        *link = s->next;
        SetSignal(0, s->mask);
        FreeSignal(s->bit);
        free(s);
    }
}
int rs_selector_owner(void) { return rs_socket_ready(); }
void *rs_selector_new(void) {
    if (rs_socket_ready() < 0) return NULL;
    rs_selector_reap();
    struct rs_selector *s = calloc(1, sizeof *s);
    if (!s) { errno = ENOMEM; return NULL; }
    s->bit = AllocSignal(-1);
    if (s->bit < 0) { free(s); errno = EAGAIN; return NULL; }
    s->owner = FindTask(NULL); s->mask = (ULONG)1 << s->bit;
    s->next = rs_selectors; rs_selectors = s;
    return s;
}
int rs_selector_check(void *context) {
    struct rs_selector *s = context;
    if (!s || s->retired) { errno = EINVAL; return -1; }
    if (s->owner != FindTask(NULL)) { errno = ENOTSUP; return -1; }
    if (rs_socket_ready() < 0) return -1;
    rs_selector_reap();
    return 0;
}
void rs_selector_free(void *context) {
    struct rs_selector *s = context;
    if (!s) return;
    struct Task *owner = s->owner;
    Forbid();
    s->retired = 1;
    Signal(owner, s->mask);
    Permit();
    /* Do not dereference s after Permit: the owner may already have reaped it. */
    if (owner == FindTask(NULL)) rs_selector_reap();
}
static void rs_selector_shutdown(void) {
    if (FindTask(NULL) != rs_socket_owner) return;
    for (struct rs_selector *s = rs_selectors; s; s = s->next) s->retired = 1;
    rs_selector_reap();
}
int rs_selector_notify(void *context) {
    struct rs_selector *s = context;
    if (!s || s->retired) { errno = EINVAL; return -1; }
    Signal(s->owner, s->mask);
    return 0;
}
int rs_selector_validate(void *context, int fd) {
    if (rs_selector_check(context) < 0) return -1;
    return rs_socket_index(fd) < 0 ? -1 : 0;
}
int rs_selector_wait(void *context, struct rs_pollfd *fds, size_t n, int64_t micros) {
    if (rs_selector_check(context) < 0) return -1;
    if (n && !fds) { errno = EFAULT; return -1; }
    struct rs_selector *s = context;
    fd_set readset, writeset;
    FD_ZERO(&readset); FD_ZERO(&writeset);
    int max = -1;
    for (size_t i = 0; i < n; i++) {
        fds[i].revents = 0;
        int raw = rs_socket_index(fds[i].fd);
        if (raw < 0) return -1;
        if (fds[i].events & ~5) { errno = ENOTSUP; return -1; }
        if (fds[i].events & 1) FD_SET(raw, &readset);
        if (fds[i].events & 4) FD_SET(raw, &writeset);
        if (fds[i].events && raw > max) max = raw;
    }
    struct timeval timeout;
    /* Bound a single timer request without truncating a distant deadline.
       The Rust caller recomputes its remaining deadline on every wait. */
    if (micros > 3600000000LL) micros = 3600000000LL;
    timeout.tv_sec = micros < 0 ? 0 : micros / 1000000;
    timeout.tv_usec = micros < 0 ? 0 : micros % 1000000;
    ULONG signals = s->mask;
    int result = WaitSelect(max + 1, &readset, &writeset, NULL,
                            micros < 0 ? NULL : &timeout, &signals);
    if (result < 0) { errno = Errno(); return -1; }
    int count = 0;
    for (size_t i = 0; i < n; i++) {
        int raw = rs_socket_index(fds[i].fd);
        if (raw < 0) return -1;
        if ((fds[i].events & 1) && FD_ISSET(raw, &readset)) fds[i].revents |= 1;
        if ((fds[i].events & 4) && FD_ISSET(raw, &writeset)) fds[i].revents |= 4;
        if (fds[i].revents) count++;
    }
    return count;
}
