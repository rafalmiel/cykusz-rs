#include <cykusz/syscall.h>
#include <stdio.h>

int main() {
	printf("Hello, World!\n");

	syscalln0(SYS_MAPS);

	return 0;
}
