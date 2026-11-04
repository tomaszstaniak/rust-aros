/* The platform layer jitterentropy-library expects, for AROS.
 *
 * It replaces the release's own jitterentropy-base-user.h, which reaches for
 * POSIX and Linux facilities (sysfs, sysconf, mlock, sched_yield) AROS does
 * not have. What the library needs from a platform is small:
 *   - a high-resolution time stamp: the time stamp counter on x86, the
 *     virtual counter on aarch64. The library measures its own suitability
 *     at jent_entropy_init() and refuses a timer that is too coarse; it is
 *     never overridden here;
 *   - memory, cleared on release. AROS has no locked secure memory;
 *   - the number of CPUs and the cache size: 1 and "unknown", which makes
 *     the library use its default memory size;
 *   - no FIPS switch, and a yield that does nothing (the internal timer
 *     thread, the only user of it, is not built). */
#ifndef _JITTERENTROPY_BASE_USER_H
#define _JITTERENTROPY_BASE_USER_H

#include <sys/types.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <limits.h>
#include <errno.h>

#ifndef JENT_SECURE_MEMORY_SIZE_MAX
#define JENT_SECURE_MEMORY_SIZE_MAX 2097152
#endif

#if defined(__x86_64__)
static inline void jent_get_nstime(uint64_t *out)
{
	uint64_t lo, hi;
	__asm__ __volatile__("rdtsc" : "=a" (lo), "=d" (hi));
	*out = lo | (hi << 32);
}
#elif defined(__i386__)
static inline void jent_get_nstime(uint64_t *out)
{
	unsigned long long val;
	__asm__ __volatile__("rdtsc" : "=A" (val));
	*out = val;
}
#elif defined(__aarch64__)
static inline void jent_get_nstime(uint64_t *out)
{
	uint64_t v;
	__asm__ __volatile__("mrs %0, cntvct_el0" : "=r" (v));
	*out = v;
}
#else
#  error "jitterentropy on AROS: no time stamp for this CPU"
#endif

static inline void jent_memset_secure(void *s, size_t n)
{
	memset(s, 0, n);
	__asm__ __volatile__("" : : "r" (s) : "memory");
}

static inline void *jent_zalloc(size_t len)
{
	return calloc(1, len);
}

static inline void jent_zfree(void *ptr, size_t len)
{
	if (!ptr)
		return;
	jent_memset_secure(ptr, len);
	free(ptr);
}

static inline int jent_fips_enabled(void)
{
	return 0;
}

static inline long jent_ncpu(void)
{
	return 1;
}

static inline uint32_t jent_cache_size_roundup(int all_caches)
{
	(void)all_caches;
	return 0;
}

static inline void jent_yield(void)
{
}

static inline uint64_t rol64(uint64_t x, int n)
{
	return ( (x << (n&(64-1))) | (x >> ((64-n)&(64-1))) );
}

#endif /* _JITTERENTROPY_BASE_USER_H */
