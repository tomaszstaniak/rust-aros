/* Check the handwritten Jonx FFI against the selected native Pi SDK.
 * This is a compile gate, not a runtime or x86_64 ABI claim. */
#include <stddef.h>
#include <pthread.h>
#include <time.h>

_Static_assert(sizeof(void *) == 8, "LP64 profile required");
_Static_assert(sizeof(time_t) == 4, "Jonx time PAL expects i32 seconds");
_Static_assert(sizeof(struct timespec) == 16, "timespec size");
_Static_assert(offsetof(struct timespec, tv_nsec) == 8, "timespec padding");
_Static_assert(CLOCK_MONOTONIC == 0 && CLOCK_REALTIME == 2, "clock identifiers");
_Static_assert(sizeof(pthread_t) == 4, "thread handle width");
_Static_assert(sizeof(pthread_key_t) == 4, "TLS key width");
_Static_assert(sizeof(pthread_attr_t) == 32, "Jonx spawn glue attribute size");
_Static_assert(offsetof(pthread_attr_t, stacksize) == 8, "stacksize offset");
_Static_assert(offsetof(pthread_attr_t, detachstate) == 16, "detachstate offset");
_Static_assert(offsetof(pthread_attr_t, param) == 20, "scheduler offset");
_Static_assert(offsetof(pthread_attr_t, inheritsched) == 24, "inheritsched offset");
_Static_assert(offsetof(pthread_attr_t, contentionscope) == 28, "scope offset");
_Static_assert(sizeof(pthread_mutex_t) <= 160, "Jonx mutex buffer capacity");
_Static_assert(_Alignof(pthread_mutex_t) <= 8, "Jonx mutex buffer alignment");
_Static_assert(sizeof(pthread_cond_t) <= 176, "Jonx condvar buffer capacity");
_Static_assert(_Alignof(pthread_cond_t) <= 8, "Jonx condvar buffer alignment");
