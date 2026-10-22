/* C owns AROS startup, argv and the actual return code for both CPU profiles. */
#include <proto/dos.h>
#include <proto/exec.h>
#include <dos/dosextens.h>
#include <exec/nodes.h>

int aros_argc;
char **aros_argv;
extern int aros_acceptance(const char *path);

void aros_probe_write(const char *text)
{
    PutStr((CONST_STRPTR)text);
}

int main(int argc, char **argv)
{
    struct Task *self = FindTask(0);
    if (self->tc_Node.ln_Type == NT_PROCESS)
        ((struct Process *)self)->pr_WindowPtr = (APTR)-1;
    aros_argc = argc;
    aros_argv = argv;
#ifdef AROS_C_PROBE
    aros_probe_write("C: work complete; awaiting CRT exit receipt\n");
    return 0;
#else
    return aros_acceptance(argc > 1 ? argv[1] : "RAM:rust-acceptance.txt");
#endif
}
