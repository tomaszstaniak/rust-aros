#include <sys/types.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <dirent.h>
#include <time.h>
#include <pthread.h>
#include <fcntl.h>
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

struct stat            v_stat;
struct dirent          v_dirent;
struct timespec        v_timespec;
struct timeval         v_timeval;
struct tm              v_tm;
pthread_t              v_pthread_t;
pthread_attr_t         v_pthread_attr_t;
pthread_mutex_t        v_pthread_mutex_t;
pthread_mutexattr_t    v_pthread_mutexattr_t;
pthread_cond_t         v_pthread_cond_t;
pthread_condattr_t     v_pthread_condattr_t;
pthread_rwlock_t       v_pthread_rwlock_t;
pthread_key_t          v_pthread_key_t;
off_t                  v_off_t;
mode_t                 v_mode_t;
pid_t                  v_pid_t;
size_t                 v_size_t;
ssize_t                v_ssize_t;
time_t                 v_time_t;
ino_t                  v_ino_t;
dev_t                  v_dev_t;
nlink_t                v_nlink_t;
uid_t                  v_uid_t;
gid_t                  v_gid_t;
blksize_t              v_blksize_t;
blkcnt_t               v_blkcnt_t;
clockid_t              v_clockid_t;
int main(void){ return 0; }
