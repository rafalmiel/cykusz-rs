#include <stdio.h>
#include <inttypes.h>
extern char **environ;

int main(int argc, char **argv) {
        unsigned long long a, *x;
	unsigned char *y;

	printf("argv = %08" PRIx64 "\n", (unsigned long long) argv);
	printf("argv[0] = %08" PRIx64 "\n", (unsigned long long) argv[0]);
	printf("environ = %08" PRIx64 "\n", (unsigned long long) environ);
	printf("environ[0] = %08" PRIx64 "\n", (unsigned long long) environ[0]);
	printf("\n\n");

        x = (unsigned long long *) ((unsigned long long) &a & ~0xf);
        while ((unsigned long long) x < (unsigned long long) argv[0]) {
                printf("%08" PRIx64 ":", (unsigned long long) x);
                for (a=0; a<4; a++)
                        printf(" %08x", x[a]);
                printf("\n");
                x += 4;
        }

	printf("\n\n");
	y = (unsigned char *) ((unsigned long long) argv[0] & ~0xf) - 16;
        while ((unsigned long long) y < 0x800000000000) {
                printf("%08" PRIx64 ":", (unsigned long long) y);
                for (a=0; a<16; a++)
                        printf(" %02" PRIx64, y[a]);
		printf("   ");
		for (a=0; a<16; a++)
			putchar((y[a] > 32 && y[a] < 127) ? y[a] : '.');
                printf("\n");
                y += 16;
        }

	printf("\n\n");
	printf("arg strings:\n");
	int i = 0;
	for (; i < argc; ++i) {
		printf("%s\n", argv[i]);

	}
	printf("env strings:\n");
	char** env = environ;
	while (*env != 0) {
		printf("%s\n", *env);
		env += 1;
	}
        return 0;
}
