#include <cykusz/syscall.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <stdio.h>
#include <stdlib.h>

static int* val = 0ull;

int main() {

	val = malloc(4);
	*val = 30;

	//printf("before fork: %p\n", val);

	syscalln0(29);

	pid_t pid = fork();

	printf("after fork: %p\n", &val);
	asm volatile ("xchg %bx, %bx");

	if (pid == 0) {
		//char *args[] = {"/bin/stack", "-a", "-b", 0ull};
		//printf("val: %d\n", *val);

		exit(0);
		//printf("before execve: %d\n", *val);
		//execve("/bin/stack", args, 0ull);
	} else {
		int status = 0;

		//printf("before wait: %d\n", *val);
		pid_t res = waitpid(pid, &status, 0);
		printf("Finished %d %d\n", res, status);
	}
	syscalln0(29);

	printf("val 2: %d\n", *val);
}


