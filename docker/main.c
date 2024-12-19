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
#include <unistd.h>
#include <limits.h>
#include "main.h"

#define STACK_SIZE (1024 * 1024)    /* Stack size for cloned child */

struct args {
    int argc;
    char **argv;
    int pipe_fd[2];  /* Pipe for parent-child synchronization */
};

void usage() {
    fprintf(stderr, "Usage:\n\tccrun run <command> <args>\n\n");
    exit(1);
}

int update_map(const char *map, const char *map_path) {
    FILE *f = fopen(map_path, "w");
    if (f == NULL) {
        perror("fopen");
        return -1;
    }
    fprintf(f, "%s", map);
    fclose(f);
    return 0;
}

void write_uid_gid_mappings(int pid)
{
    // Parent process waits for the child to update UID/GID mappings
    char map_buf[100];
    const int MAP_BUF_SIZE = 100;
    char map_path[PATH_MAX];

    // Update UID map for the child
    snprintf(map_path, PATH_MAX, "/proc/%d/uid_map", pid);
    snprintf(map_buf, MAP_BUF_SIZE, "0 %d 1", getuid()); // Map UID 0 in the parent to UID 0 in the child
    update_map(map_buf, map_path);

    // Update GID map for the child
    snprintf(map_path, PATH_MAX, "/proc/%d/gid_map", pid);
    snprintf(map_buf, MAP_BUF_SIZE, "0 %d 1", getgid()); // Map GID 0 in the parent to GID 0 in the child
    update_map(map_buf, map_path);
}


static int child(void *args) {
    struct args *arguments = (struct args *)args;
    
    // Wait until the parent has updated UID and GID mappings.
    close(arguments->pipe_fd[1]);    // Close write end of the pipe
    char ch;
    if (read(arguments->pipe_fd[0], &ch, 1) != 0) {
        fprintf(stderr, "Failure in child: read from pipe returned != 0\n");
        exit(1);
    }
    close(arguments->pipe_fd[0]);

    // Set hostname for the new container.
    if (sethostname("container", 9) == -1) {
        fprintf(stderr, "Erreur sethostname : %s\n", strerror(errno));
        exit(1);
    }

    // Chroot into the "alpine/" directory and make sure this exists.
    if (chroot("alpine/") == -1) {
        fprintf(stderr, "Erreur chroot : %s\n", strerror(errno));
        exit(1);
    }

    // Change to the new root.
    if (chdir("/") == -1) {
        fprintf(stderr, "Erreur chdir : %s\n", strerror(errno));
        exit(1);
    }
    
    // Mount the proc filesystem for the new namespace.
    if (mount("proc", "/proc", "proc", 0, NULL) == -1) {
        fprintf(stderr, "Erreur mount /proc : %s\n", strerror(errno));
        exit(1);
    }

    // Execute the command passed by the parent process.
    if (arguments->argc > 3) {
        execvp(arguments->argv[2], arguments->argv + 2);
    } else {
        char *empty[] = { arguments->argv[2], NULL };
        execvp(arguments->argv[2], empty);
    }

    fprintf(stderr, "Erreur execvp : %s\n", strerror(errno));
    exit(1);
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "Erreur : Aucun argument n'est spécifié.\n");
        usage();
    }

    if (strcmp(argv[1], "run") != 0) {
        fprintf(stderr, "Erreur : Option inconnue '%s'.\n", argv[1]);
        usage();
    }

    if (argc < 3) {
        fprintf(stderr, "Erreur : Aucune commande spécifiée à exécuter.\n");
        usage();
    }

    char *stack = malloc(STACK_SIZE);    
    if (!stack) {
        fprintf(stderr, "Erreur d'allocation mémoire (malloc).\n");
        exit(1);
    }

    char *stackTop = stack + STACK_SIZE;  /* Assume stack grows downwards */
    
    struct args arguments = {
        .argc = argc,
        .argv = argv
    };

    // Create the pipe for parent-child synchronization
    if (pipe(arguments.pipe_fd) == -1) {
        perror("pipe");
        free(stack);
        exit(1);
    }

    // Create the child in new namespaces.
    int pid = clone(child, stackTop, CLONE_NEWNS | CLONE_NEWUSER | CLONE_NEWPID | CLONE_NEWUTS | SIGCHLD, &arguments);
    if (pid == -1) {
        fprintf(stderr, "Erreur clone : %s\n", strerror(errno));
        free(stack);
        exit(1);
    }

    write_uid_gid_mappings(pid);

    // Close the write end of the pipe to signal the child process it can proceed
    close(arguments.pipe_fd[1]);

    int status;
    if (waitpid(pid, &status, 0) == -1) {
        fprintf(stderr, "Erreur waitpid : %s\n", strerror(errno));
        free(stack);
        exit(1);
    }

    free(stack);

    if (WIFEXITED(status)) {
        return WEXITSTATUS(status);
    } else {
        fprintf(stderr, "Erreur : le processus enfant n'a pas terminé normalement.\n");
        return 1;
    }
}