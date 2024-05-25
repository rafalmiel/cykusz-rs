#include <stdio.h>
#include <unistd.h>

int main(int argc, char** argv) {
    FILE* fd = fopen(argv[1], "r");

    printf("opened %d\n", fd);

    sleep(30);
}