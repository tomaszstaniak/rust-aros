#include <signal.h>
#include <stddef.h>

_Static_assert(SA_RESTART == 8 && SA_SIGINFO == 16, "signal flags");
_Static_assert(sizeof(union sigval) == 8 && _Alignof(union sigval) == 8, "sigval");
_Static_assert(sizeof(struct sigaction) == 32 && _Alignof(struct sigaction) == 8, "sigaction");
_Static_assert(offsetof(struct sigaction, sa_handler) == 0, "handler union");
_Static_assert(offsetof(struct sigaction, sa_sigaction) == 0, "action union");
_Static_assert(offsetof(struct sigaction, sa_mask) == 8, "mask");
_Static_assert(offsetof(struct sigaction, sa_flags) == 24, "flags");
_Static_assert(sizeof(siginfo_t) == 64 && _Alignof(siginfo_t) == 8, "siginfo");
_Static_assert(offsetof(siginfo_t, si_pid) == 16, "pid");
_Static_assert(offsetof(siginfo_t, si_uid) == 24, "uid");
_Static_assert(offsetof(siginfo_t, si_addr) == 32, "address");
_Static_assert(offsetof(siginfo_t, si_status) == 40, "status");
_Static_assert(offsetof(siginfo_t, si_band) == 48, "band");
_Static_assert(offsetof(siginfo_t, si_value) == 56, "value");
