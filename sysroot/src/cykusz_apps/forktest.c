#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>

int main() {

    int pid = fork();

    if (pid == 0) {
        int pid2 = fork();

        if (pid2 == 0) {
            sleep(10);

        } else {
            printf("Child2 pid is: %d\n", pid2);
            sleep(5);
        }
    } else {
        printf("Child pid is: %d\n", pid);

        while (1) {
            int status;
            pid = waitpid(-1, &status, WUNTRACED | WCONTINUED);

            if (pid == -1) {
                return 0;
            }

            printf("got waitpid %d: %d\n", pid, status);
        }
    }

}
