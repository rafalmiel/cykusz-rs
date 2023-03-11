#include <sys/stat.h>
#include <stdio.h>

int main() {
    struct stat st;

    stat("/hello.cpp", &st);
    printf("stat mode: %d\n", st.st_mode);

}
