/* Reach exec.library the way C does, so that no exec stub symbol is referenced.
 *
 * libexec.a has three members, and exec_regcall_stubs.o carries *every* exec
 * stub in one object -- including `open` and `close`. Referencing any exec
 * function by symbol therefore drags that whole object in, and its `close`
 * collides with crt_close_stub.o in libcrt.a. That is what forces
 * -Wl,--allow-multiple-definition on anything mixing exec calls with C file
 * I/O.
 *
 * proto/exec.h avoids it entirely: AllocMem is a macro over an inline function
 * that calls through SysBase, so nothing undefined is emitted and the archive
 * member is never pulled. Wrapping the calls here gives Rust the same property.
 */
#include <proto/exec.h>

void *aros_glue_AllocMem(unsigned long size, unsigned long requirements)
{
    return AllocMem(size, requirements);
}

void aros_glue_FreeMem(void *mem, unsigned long size)
{
    FreeMem(mem, size);
}

/* `pr_WindowPtr`, which is why a missing volume is not a hang.
 *
 * `Lock` on a path whose volume is not mounted asks the user to insert it:
 * dos.library puts up "Please insert volume X in any drive" and waits. A
 * program that only wants to know whether the path is there must set the
 * process's `pr_WindowPtr` to -1 first, which turns the requester into an
 * error return. The field lives inside `struct Process`, so it is set here,
 * where the SDK's own headers decide the offsets, rather than from Rust
 * against a layout written out by hand. Returns the old value, to put back.
 */
#include <dos/dosextens.h>

void *aros_glue_set_window_ptr(void *ptr)
{
    struct Process *me = (struct Process *)FindTask(NULL);
    void *old = me->pr_WindowPtr;
    me->pr_WindowPtr = ptr;
    return old;
}
