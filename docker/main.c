#define _GNU_SOURCE
#include <errno.h>
#include <sched.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/mount.h>
#include <sys/types.h>
#include <sys/utsname.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <unistd.h>
#include <limits.h>

#define STACK_SIZE (1024 * 1024) /* Stack size for cloned child */

struct args
{
    int argc;
    char **argv;
    int pipe_fd[2]; /* Pipe for parent-child synchronization */
};

void error_exit(const char *msg) {
    fprintf(stderr, "Error: %s - %s\n", msg, strerror(errno));
    exit(1);
}

void usage()
{
    fprintf(stderr, "Usage:\n\tccrun run <command> <args>\n\n");
    exit(1);
}

int update_map(const char *map, const char *map_path)
{
    FILE *f = fopen(map_path, "w");
    if (f == NULL)
    {
        error_exit("Failed to open map file for writing");
    }
    fprintf(f, "%s", map);
    fclose(f);
    return 0;
}

void write_uid_gid_mappings(int pid)
{
    char map_buf[100];
    const int MAP_BUF_SIZE = 100;
    char map_path[PATH_MAX];

    snprintf(map_path, PATH_MAX, "/proc/%d/uid_map", pid);
    snprintf(map_buf, MAP_BUF_SIZE, "0 %d 1", getuid());
    if (update_map(map_buf, map_path) != 0)
    {
        error_exit("Failed to update UID map");
    }

    snprintf(map_path, PATH_MAX, "/proc/%d/gid_map", pid);
    snprintf(map_buf, MAP_BUF_SIZE, "0 %d 1", getgid());
    if (update_map(map_buf, map_path) != 0)
    {
        error_exit("Failed to update GID map");
    }
}

int write_limits()
{
    const char *cgroup_path = "/sys/fs/cgroup/container-ccrun";
    pid_t pid = getpid();

    if (mkdir(cgroup_path, 0777) != 0)
    {
        error_exit("Failed to create cgroup directory");
    }

    // Limit the CPU
    char cpu_max_path[256];
    snprintf(cpu_max_path, sizeof(cpu_max_path), "%s/cpu.max", cgroup_path);
    FILE *f_cpu = fopen(cpu_max_path, "w");
    if (f_cpu == NULL)
    {
        error_exit("Failed to open cpu.max file");
    }
    fprintf(f_cpu, "10000 100000");
    fclose(f_cpu);

    // Set memory limits
    char memory_max_path[256];
    snprintf(memory_max_path, sizeof(memory_max_path), "%s/memory.max", cgroup_path);
    FILE *f_mem = fopen(memory_max_path, "w");
    if (f_mem == NULL)
    {
        error_exit("Failed to open memory.max file");
    }
    fprintf(f_mem, "67108864");
    fclose(f_mem);

    // Add process to cgroup
    char cgroup_procs_path[256];
    snprintf(cgroup_procs_path, sizeof(cgroup_procs_path), "%s/cgroup.procs", cgroup_path);
    FILE *f_procs = fopen(cgroup_procs_path, "w");
    if (f_procs == NULL)
    {
        error_exit("Failed to open cgroup.procs file");
    }
    fprintf(f_procs, "%d", pid);
    fclose(f_procs);

    return 0;
}

int remove_cgroup_directory()
{
    const char *cgroup_path = "/sys/fs/cgroup/container-ccrun";

    // Attempt to remove the cgroup directory
    if (rmdir(cgroup_path) == 0)
    {
        return 0;
    }
    else
    {
        if (errno == ENOENT)
        {
            printf("Cgroup directory %s does not exist.\n", cgroup_path);
        }
        else
        {
            perror("Error removing cgroup directory");
        }
        return 1;
    }
}

static int child(void *args)
{
    struct args *arguments = (struct args *)args;

    close(arguments->pipe_fd[1]);
    char ch;
    if (read(arguments->pipe_fd[0], &ch, 1) != 0)
    {
        error_exit("Failed to read from pipe in child");
    }
    close(arguments->pipe_fd[0]);

    if (write_limits() != 0)
    {
        error_exit("Failed to write resource limits");
    }

    if (sethostname("container", 9) == -1)
    {
        error_exit("Failed to set hostname");
    }

    if (chroot("alpine/") == -1)
    {
        error_exit("Failed to change root directory");
    }

    if (chdir("/") == -1)
    {
        error_exit("Failed to change directory to new root");
    }

    if (mount("proc", "/proc", "proc", 0, NULL) == -1)
    {
        error_exit("Failed to mount /proc filesystem");
    }

    if (arguments->argc > 3)
    {
        execvp(arguments->argv[2], arguments->argv + 2);
    }
    else
    {
        char *empty[] = {arguments->argv[2], NULL};
        execvp(arguments->argv[2], empty);
    }

    error_exit("execvp failed");
    return 1; // Unreachable
}

int main(int argc, char **argv)
{
    if (argc < 2)
    {
        fprintf(stderr, "Error: No arguments specified.\n");
        usage();
    }

    if (strcmp(argv[1], "run") != 0)
    {
        fprintf(stderr, "Error: Unknown option '%s'.\n", argv[1]);
        usage();
    }

    if (argc < 3)
    {
        fprintf(stderr, "Error: No command specified to execute.\n");
        usage();
    }

    char *stack = malloc(STACK_SIZE);
    if (!stack)
    {
        error_exit("Memory allocation failed for stack");
    }

    char *stackTop = stack + STACK_SIZE;

    struct args arguments = {
        .argc = argc,
        .argv = argv};

    if (pipe(arguments.pipe_fd) == -1)
    {
        error_exit("Failed to create pipe");
    }

    int pid = clone(child, stackTop, CLONE_NEWNS | CLONE_NEWUSER | CLONE_NEWPID | CLONE_NEWUTS | SIGCHLD, &arguments);
    if (pid == -1)
    {
        error_exit("Failed to create child process using clone");
    }

    write_uid_gid_mappings(pid);

    close(arguments.pipe_fd[1]);

    int status;
    if (waitpid(pid, &status, 0) == -1)
    {
        error_exit("Failed to wait for child process");
    }

    remove_cgroup_directory(pid);

    free(stack);

    if (WIFEXITED(status))
    {
        return WEXITSTATUS(status);
    }
    else
    {
        error_exit("Child process did not exit normally");
    }
}
