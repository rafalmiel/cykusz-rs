#include <cykusz/syscall.h>
#include <iostream>

int main() {
	syscalln0(29);
	std::string input{};
	std::cin >> input;
	std::cout << "Hello! " << input << std::endl;
}
