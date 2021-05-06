#include <stdio.h>
#include <unistd.h>

int main() {
	setbuf(stdout, NULL);

	while (1) {
		char buf[256];

		int r = read(0, buf, 256);

		printf("main: read %d bytes\n", r);
	}
}
