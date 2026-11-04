from pathlib import Path
import subprocess
import tempfile
import unittest
ROOT = Path(__file__).resolve().parents[1]
HARNESS = r'''
#include <assert.h>
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
static int no_memory, stat_error, mode = 0755, kind = S_IFREG, exec_error = ENOEXEC, calls;
static char **expected_args;
void *rs_test_malloc(size_t size) { return no_memory ? NULL : malloc(size); }
void rs_test_free(void *p) { free(p); }
int rs_test_stat(const char *path, struct stat *out) {
    (void)path;
    if (stat_error) { errno = stat_error; return -1; }
    memset(out, 0, sizeof(*out)); out->st_mode = kind | mode;
    return 0;
}
int rs_test_execvp(const char *file, char *const args[]) {
    (void)file; calls++;
    int i;
    for (i = 0; expected_args[i]; i++) {
        const char *expected = i && !expected_args[i][0] ? "\"\"" : expected_args[i];
        assert(args[i] && strcmp(args[i], expected) == 0);
    }
    assert(args[i] == NULL);
    errno = exec_error; return -1;
}
int rs_execvp(const char *, char *const []);
int main(void) {
    char *args[] = {"program", "", "two words", "Zażółć", NULL};
    expected_args = args;
    assert(rs_execvp("RAM:program", args) == -1 && errno == ENOEXEC && calls == 1);
    assert(strcmp(args[1], "") == 0); /* caller-owned argv remains intact */
    stat_error = ENOENT;
    assert(rs_execvp("RAM:missing", args) == -1 && errno == ENOENT && calls == 1);
    stat_error = 0; mode = 0644;
    assert(rs_execvp("RAM:program", args) == -1 && errno == EACCES && calls == 1);
    mode = 0755; no_memory = 1;
    assert(rs_execvp("RAM:program", args) == -1 && errno == ENOMEM && calls == 1);
    no_memory = 0; exec_error = 0;
    assert(rs_execvp("RAM:program", args) == -1 && errno == EIO && calls == 2);
    /* Bare program names retain the SDK's PATH lookup; preflight must not stat
       the same name in cwd and accidentally change command resolution. */
    stat_error = ENOENT; exec_error = ENOENT;
    assert(rs_execvp("program", args) == -1 && errno == ENOENT && calls == 3);
    stat_error = 0; kind = S_IFDIR;
    assert(rs_execvp("RAM:directory", args) == -1 && errno == EACCES && calls == 3);
    kind = S_IFREG;
    char *many[] = {"program", "", "", "tail", "", NULL};
    expected_args = many;
    assert(rs_execvp("RAM:program", many) == -1 && errno == ENOENT && calls == 4);
    char *nonempty[] = {"program", "ordinary", NULL};
    expected_args = nonempty; no_memory = 1;
    assert(rs_execvp("RAM:program", nonempty) == -1 && errno == ENOENT && calls == 5);
    assert(rs_execvp(NULL, args) == -1 && errno == EINVAL);
    assert(rs_execvp("program", NULL) == -1 && errno == EINVAL);
}
'''
class ExecAdapter(unittest.TestCase):
    def test_empty_arguments_and_error_contract(self):
        with tempfile.TemporaryDirectory(prefix='aros-exec-') as directory:
            root = Path(directory)
            harness = root / 'check.c'; harness.write_text(HARNESS)
            obj = root / 'adapter.o'; binary = root / 'check'
            result = subprocess.run(['cc', '-Wall', '-Wextra', '-Werror',
                                     '-Dexecvp=rs_test_execvp', '-Dstat=rs_test_stat',
                                     '-Dmalloc=rs_test_malloc', '-Dfree=rs_test_free',
                                     '-c', str(ROOT / 'std-groundwork/libc-aros/csrc/exec_adapter.c'),
                                     '-o', str(obj)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            result = subprocess.run(['cc', str(harness), str(obj), '-o', str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            result = subprocess.run([str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
