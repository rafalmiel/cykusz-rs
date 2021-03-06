#include <cykusz/syscall.h>
#include <iostream>

int main() {
	//syscalln0(29);
	std::string input{};
	std::cout << "Enter your name: ";
	std::cin >> input;
	std::cout << "Hello, " << input << "!" << std::endl;
}
