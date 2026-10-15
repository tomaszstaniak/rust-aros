/* Diagnostic reference: GCC handles the selected SDK's varargs ABI. */
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
int rs_probe_pipe(int *fds) { return pipe(fds); }
int rs_probe_fcntl(int fd, int cmd, int arg, int has_arg) {
    if (has_arg) return fcntl(fd, cmd, arg);
    return fcntl(fd, cmd);
}
int rs_probe_close(int fd) { return close(fd); }
int rs_probe_errno(void) { return errno; }
void rs_probe_clear_errno(void) { errno = 0; }

#include <sys/wait.h>
#include <signal.h>
#include <stdint.h>
struct rs_spawn_probe {
    int64_t pid, waited;
    uintptr_t signal_result;
    int child_errno, parent_errno, status, exec_return;
};
/* Keep the entire returns-twice region in a GCC-compiled C frame. */
void rs_probe_spawn(const char *program, int reset_signal, struct rs_spawn_probe *out) {
    char *const args[] = {(char *)program, "--child", "exit", (reset_signal & 2) ? "RESULTS:cspb1015-execvp-child.txt" : "RESULTS:cspb1015-execv-child.txt", NULL};
    pid_t pid = vfork();
    if (pid == 0) {
        if (reset_signal & 1) {
            errno = 0;
            out->signal_result = (uintptr_t)signal(SIGPIPE, SIG_DFL);
            out->child_errno = errno;
            if (out->signal_result == (uintptr_t)SIG_ERR) _exit(21);
        }
        out->exec_return = (reset_signal & 2) ? execvp(program, args) : execv(program, args);
        out->child_errno = errno;
        _exit(20);
    }
    out->pid = pid;
    out->parent_errno = errno;
    if (pid > 0) {
        out->waited = waitpid(pid, &out->status, 0);
        out->parent_errno = errno;
    }
}

#include <proto/dos.h>
/* Trace a diagnostic std build without allocating in the vfork region. */
void rs_probe_trace(int stage, intptr_t value) {
    int saved_errno = errno;
    char path[] = "RESULTS:dbgsp1015-00.txt";
    char bytes[64], *end = bytes + sizeof(bytes), *p = end;
    uintptr_t magnitude = value < 0 ? (uintptr_t)(-(value + 1)) + 1 : (uintptr_t)value;
    *--p = '\n';
    do { *--p = '0' + magnitude % 10; magnitude /= 10; } while (magnitude);
    if (value < 0) *--p = '-';
    path[17] = '0' + stage / 10;
    path[18] = '0' + stage % 10;
    BPTR file = Open(path, MODE_NEWFILE);
    if (file) { Write(file, p, end - p); Close(file); }
    errno = saved_errno;
}
