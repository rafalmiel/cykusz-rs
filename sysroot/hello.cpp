#include <cykusz/syscall.h>
#include <iostream>
#include <vector>
#include <signal.h>
#include <unistd.h>

class TestCtr {
	public:
		TestCtr() {
			std::cout << "Ctr()" << std::endl;
		}

		~TestCtr() {
			std::cout << "~Ctr()" << std::endl;
		}
};

static TestCtr TEST{};

void int_handler(int sig) {
	std::cout << "INT signal received" << std::endl;
}

int main(int argc, char *argv[]) {
	syscalln0(29);

	for (int i = 0; i < argc; ++i) {
		std::cout << "hello arg: " << argv[i] << std::endl;
	}
	
	struct sigaction sact{};
	sact.sa_handler = int_handler;
	sact.sa_flags = SA_RESTART;

	sigaction(SIGINT, &sact, nullptr);

	std::string input{};
	std::cout << "Enter your name: ";

	std::cin >> input;
	std::cout << "Hello, " << input << "!" << std::endl;

	std::vector<int> vec = {1, 2, 3, 4, 5};
	for(auto a: vec) {
		std::cout << a << " ";
	}
	std::cout << std::endl;

	char* const args[3] = {"-arg1", "-arg2", nullptr};
	char* const envs[1] = {nullptr};
}
