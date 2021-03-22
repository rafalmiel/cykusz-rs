#include <cykusz/syscall.h>
#include <iostream>
#include <signal.h>

void int_handler(int sig) {
	std::cout << "INT signal received" << std::endl;
}

int main() {
	//syscalln0(29);
	
	struct sigaction sact{};
	sact.sa_handler = int_handler;
	sact.sa_flags = SA_RESTART;

	sigaction(SIGINT, &sact, nullptr);

	std::string input{};
	std::cout << "Enter your name: ";

	std::cin >> input;
	std::cout << "Hello, " << input << "!" << std::endl;

	*reinterpret_cast<int*>(0xABABABABABABABAB) = 0xdeadbeef;
}
