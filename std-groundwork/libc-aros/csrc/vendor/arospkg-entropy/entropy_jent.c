/* Entropy for Mbed TLS on AROS systems without a system source (the ABIv11
 * distributions such as AROS One, the i386 line, hosted aarch64), from
 * jitterentropy-library (third_party/jitterentropy).
 *
 * It provides mbedtls_hardware_poll(), the hook Mbed TLS calls when built
 * with MBEDTLS_ENTROPY_HARDWARE_ALT. It depends on neither Mbed TLS headers
 * nor anything of arospkg, so the same file serves the developer package.
 *
 * Rules it keeps:
 *   - jent_entropy_init() decides whether the timer is usable; its answer
 *     is never overridden, and nothing is tuned here;
 *   - any failure (start-up test, allocation, health test) is returned as
 *     a failure, and the caller then must not use TLS; there is no fallback;
 *   - one collector per process, created on first use, used under an exec
 *     semaphore, because jitterentropy must not be used by two tasks at once
 *     on one collector; aros_jent_shutdown() frees it. */
#include <exec/semaphores.h>
#include <proto/exec.h>
#include <jitterentropy.h>
#include <stddef.h>
#include <string.h>

/* The value Mbed TLS uses for a failed source (MBEDTLS_ERR_ENTROPY_SOURCE_FAILED),
 * repeated here so this file needs no Mbed TLS header. */
#define AROS_JENT_SOURCE_FAILED (-0x003C)

static struct SignalSemaphore lock;
static volatile int lock_ready;
static struct rand_data *collector;
static int init_rc = 1;                  /* 1: not tried yet */

static void wipe(void *p, size_t n)
{
    memset(p, 0, n);
    __asm__ __volatile__("" : : "r" (p) : "memory");
}

static void take(void)
{
    /* Forbid() makes the one-time set-up of the semaphore itself safe. */
    Forbid();
    if (!lock_ready) { InitSemaphore(&lock); lock_ready = 1; }
    Permit();
    ObtainSemaphore(&lock);
}

/* 0 once the source is usable; jitterentropy's error code, or -1 when the
 * collector could not be allocated; 1 before the first use. */
int aros_jent_status(void)
{
    return init_rc;
}

int mbedtls_hardware_poll(void *data, unsigned char *output, size_t len, size_t *olen)
{
    ssize_t got;
    int rc = 0;
    (void)data;
    *olen = 0;
    take();
    if (init_rc == 1) {
#ifdef AROS_JENT_FORCE_FAIL
        /* Test builds only: behave as if the start-up test had failed, to
           show what a program does when the source is unusable. */
        init_rc = -2;
#else
        init_rc = jent_entropy_init();
#endif
        if (init_rc == 0 && !(collector = jent_entropy_collector_alloc(0, 0)))
            init_rc = -1;
    }
    if (init_rc != 0) {
        rc = AROS_JENT_SOURCE_FAILED;
    } else {
        got = jent_read_entropy(collector, (char *)output, len);
        if (got < 0 || (size_t)got != len) {
            wipe(output, len);
            rc = AROS_JENT_SOURCE_FAILED;
        } else {
            *olen = len;
        }
    }
    ReleaseSemaphore(&lock);
    return rc;
}

void aros_jent_shutdown(void)
{
    take();
    if (collector) { jent_entropy_collector_free(collector); collector = NULL; }
    init_rc = 1;
    ReleaseSemaphore(&lock);
}
