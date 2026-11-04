/* Runtime regression: observe real pthread completion under the SDK's lock.
 * Internal state is diagnostic only; this probe never changes runtime state. */
#include "pthread_intern.h"
#include <stdio.h>
#ifndef RUNTIME_RUN_ID
#error RUNTIME_RUN_ID required
#endif
static volatile int release_worker;
static void *worker(void *unused) {
    (void)unused;
    for (;;) {
        Forbid(); int go = release_worker; Permit();
        if (go) return NULL;
        Delay(1);
    }
}
static int wait_state(pthread_t id, int finished) {
    for (int tries = 0; tries < 250; ++tries) {
        Forbid();
        int ok = finished ? GetThreadInfo(id)->finished : !GetThreadInfo(id)->task;
        Permit();
        if (ok) return 1;
        Delay(1);
    }
    return 0;
}
int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    printf("RUN %s detach begin slots=%d\n", RUNTIME_RUN_ID, PTHREAD_THREADS_MAX);
    pthread_t first = 0;
    for (int phase = 0; phase < 2; ++phase) {
        for (int i = 0; i < 64; ++i) {
            Forbid(); release_worker = 0; Permit();
            pthread_t id;
            int rc = pthread_create(&id, NULL, worker, NULL);
            if (rc) { printf("FAIL create phase=%d cycle=%d rc=%d\n", phase, i, rc); return 20; }
            if (!first) first = id;
            if (id != first) { printf("FAIL slot-not-reused cycle=%d id=%lu first=%lu\n", i, (unsigned long)id, (unsigned long)first); return 20; }
            if (phase == 0) {
                rc = pthread_detach(id);
                Forbid(); release_worker = 1; Permit();
            } else {
                Forbid(); release_worker = 1; Permit();
                if (!wait_state(id, 1)) { puts("FAIL worker-not-finished"); return 20; }
                rc = pthread_detach(id);
            }
            if (rc) { printf("FAIL detach rc=%d\n", rc); return 20; }
            if (!wait_state(id, 0)) { printf("FAIL leaked-slot phase=%d cycle=%d\n", phase, i); return 20; }
        }
        printf("PASS detach-%s cycles=64 slot=%lu\n", phase ? "after" : "before", (unsigned long)first);
    }
    printf("RUN %s detach pass\n", RUNTIME_RUN_ID);
    return 0;
}
