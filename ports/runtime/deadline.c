/* Link wrapper records the actual deadline, then always calls real pthread code. */
#include <pthread.h>
#include <time.h>
#include <stdio.h>
static unsigned calls, bad;
int __real_pthread_cond_timedwait(pthread_cond_t *, pthread_mutex_t *, const struct timespec *);
int __wrap_pthread_cond_timedwait(pthread_cond_t *c, pthread_mutex_t *m, const struct timespec *deadline) {
    struct timespec now;
    int rc = clock_gettime(CLOCK_REALTIME, &now);
    long delta = deadline->tv_sec - now.tv_sec;
    __sync_fetch_and_add(&calls, 1);
    if (rc || delta < -1 || delta > 4) __sync_fetch_and_add(&bad, 1);
    printf("TIMEDWAIT realtime_delta_seconds=%ld\n", delta); fflush(stdout);
    return __real_pthread_cond_timedwait(c, m, deadline);
}
unsigned deadline_calls(void) { return __sync_fetch_and_add(&calls, 0); }
unsigned deadline_bad(void) { return __sync_fetch_and_add(&bad, 0); }
