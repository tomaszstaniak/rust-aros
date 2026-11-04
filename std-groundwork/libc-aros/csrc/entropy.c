/* POSIX adapter for arospkg's serialized jitterentropy source.
 * Scope: Rust's x86_64 ABIv11 libc compatibility layer. No weak fallback.
 * The backend owns initialization, health checks and collector lifetime. */
#include <errno.h>
#include <stddef.h>
#include <string.h>

extern int rs_jent_poll(void *, unsigned char *, size_t, size_t *);

int getentropy(void *buf, size_t len)
{
    size_t got = 0;
    if (len > 256) { errno = EIO; return -1; }
    if (len == 0) return 0;
    if (!buf) { errno = EFAULT; return -1; }
    if (rs_jent_poll(NULL, buf, len, &got) != 0 || got != len) {
        memset(buf, 0, len);
        __asm__ __volatile__("" : : "r" (buf) : "memory");
        errno = EIO;
        return -1;
    }
    return 0;
}
