/* Linked by build-console-geometry.py with the production rs_console_size.
 * Redirect stdout to a log; descriptor 0 remains the console being queried.
 * A terminal without either copied-snapshot contract reports ENOTTY safely.
 */
#include <stdio.h>
#include <errno.h>
#include <string.h>
#include <sys/ioctl.h>
#include <pthread.h>
#include <fcntl.h>
#include <unistd.h>

extern int probe_console_size(int fd, struct winsize *ws);
static int worker_rc, worker_errno;
static struct winsize worker_ws;
static void *worker(void *unused) {
    (void)unused;
    memset(&worker_ws, 0xa5, sizeof worker_ws);
    errno = 0;
    worker_rc = probe_console_size(0, &worker_ws);
    worker_errno = errno;
    return NULL;
}
int main(void) {
    struct winsize ws, before;
    pthread_t thread;
    int failures = 0, rc, saved_errno, fd;
    memset(&ws, 0xa5, sizeof ws); before = ws;
    errno = 0;
    rc = probe_console_size(0, &ws); saved_errno = errno;
    printf("console rc=%d errno=%d rows=%u columns=%u width=%u height=%u\n",
           rc, saved_errno, ws.ws_row, ws.ws_col, ws.ws_xpixel, ws.ws_ypixel);
    if (rc == 0) failures += !ws.ws_row || !ws.ws_col;
    else failures += saved_errno != ENOTTY || memcmp(&ws, &before, sizeof ws) != 0;
    int trc = pthread_create(&thread, NULL, worker, NULL);
    if (!trc) {
        int jrc = pthread_join(thread, NULL);
        printf("thread rc=%d errno=%d rows=%u columns=%u join=%d\n", worker_rc,
               worker_errno, worker_ws.ws_row, worker_ws.ws_col, jrc);
        failures += jrc != 0 || worker_rc != rc;
        if (rc == 0) failures += memcmp(&worker_ws, &ws, sizeof ws) != 0;
        else failures += worker_errno != ENOTTY || memcmp(&worker_ws, &before, sizeof ws) != 0;
    } else { printf("pthread_create=%d\n", trc); failures++; }
    ws = before;
    failures += probe_console_size(99999, &ws) != -1 || errno != EBADF ||
                memcmp(&ws, &before, sizeof ws) != 0;
    failures += probe_console_size(0, NULL) != -1 || errno != EFAULT;
    fd = open("NIL:", O_RDONLY);
    if (fd < 0) { printf("open NIL failed errno=%d\n", errno); failures++; }
    else {
        ws = before;
        failures += probe_console_size(fd, &ws) != -1 || errno != ENOTTY ||
                    memcmp(&ws, &before, sizeof ws) != 0;
        close(fd);
    }
    printf("failures=%d\n", failures);
    return failures ? 5 : 0;
}
