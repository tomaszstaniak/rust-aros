/* The native AArch64 toolchain (Clang with compiler-rt builtins) has no
 * unwinder library. Rust for this target aborts on panic, so the personality
 * routine never runs; std still refers to these entry points for it and for
 * backtraces. They report an empty stack. Weak: a real unwinder linked by the
 * program replaces them. */
#include <stdint.h>
#include <stddef.h>

#define RS_WEAK __attribute__((weak))
struct _Unwind_Context;
typedef int (*rs_unwind_trace_fn)(struct _Unwind_Context *, void *);

RS_WEAK int _Unwind_Backtrace(rs_unwind_trace_fn fn, void *arg) { (void)fn; (void)arg; return 5; /* _URC_END_OF_STACK */ }
RS_WEAK void *_Unwind_FindEnclosingFunction(void *pc) { (void)pc; return NULL; }
RS_WEAK uintptr_t _Unwind_GetCFA(struct _Unwind_Context *c) { (void)c; return 0; }
RS_WEAK uintptr_t _Unwind_GetDataRelBase(struct _Unwind_Context *c) { (void)c; return 0; }
RS_WEAK uintptr_t _Unwind_GetTextRelBase(struct _Unwind_Context *c) { (void)c; return 0; }
RS_WEAK uintptr_t _Unwind_GetIP(struct _Unwind_Context *c) { (void)c; return 0; }
RS_WEAK uintptr_t _Unwind_GetIPInfo(struct _Unwind_Context *c, int *before) { (void)c; if (before) *before = 0; return 0; }
RS_WEAK void *_Unwind_GetLanguageSpecificData(struct _Unwind_Context *c) { (void)c; return NULL; }
RS_WEAK uintptr_t _Unwind_GetRegionStart(struct _Unwind_Context *c) { (void)c; return 0; }
RS_WEAK void _Unwind_SetGR(struct _Unwind_Context *c, int reg, uintptr_t value) { (void)c; (void)reg; (void)value; }
RS_WEAK void _Unwind_SetIP(struct _Unwind_Context *c, uintptr_t value) { (void)c; (void)value; }
