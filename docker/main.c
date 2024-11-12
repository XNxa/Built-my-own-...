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


#define STACK_SIZE (1024 * 1024)    /* Stack size for cloned child */

struct args
{
    int argc;
    char** argv;
};

void usage() {
    fprintf(stderr, "Usage:\n\tccrun run <command> <args>\n\n");
    exit(1);
}

static int child(void *args) {
    struct args *arguments = (struct args*) args;

    if (sethostname("container", 9) == -1) {
        fprintf(stderr, "Erreur sethostname : %s\n", strerror(errno));
        exit(1);
    }

    if (chroot("alpine/") == -1) {
        fprintf(stderr, "Erreur chroot : %s\n", strerror(errno));
        exit(1);
    }

    if (chdir("/") == -1) {
        fprintf(stderr, "Erreur chdir : %s\n", strerror(errno));
        exit(1);
    }
    
    if (mount("proc", "/proc", "proc", 0, NULL) == -1) {
        fprintf(stderr, "Erreur mount /proc : %s\n", strerror(errno));
        exit(1);
    }
    
    // Exécution de la commande
    if (arguments->argc > 3) {
        execvp(arguments->argv[2], arguments->argv + 2);
    } else {
        char* empty[] = { arguments->argv[2], NULL };
        execvp(arguments->argv[2], empty);
    }

    fprintf(stderr, "Erreur execvp : %s\n", strerror(errno));
    exit(1);
}

int main(int argc, char** argv) {
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

    char* stack = malloc(STACK_SIZE);    
    if (!stack) {
        fprintf(stderr, "Erreur d'allocation mémoire (malloc).\n");
        exit(1);
    }

    char* stackTop = stack + STACK_SIZE;  /* On suppose que la pile croît vers le bas */
    
    struct args arguments = {
        .argc = argc,
        .argv = argv
    };

    int pid = clone(child, stackTop, CLONE_NEWNS | CLONE_NEWUSER | CLONE_NEWPID | CLONE_NEWUTS | SIGCHLD, &arguments);
    if (pid == -1) {
        fprintf(stderr, "Erreur clone : %s\n", strerror(errno));
        free(stack);
        exit(1);
    }

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