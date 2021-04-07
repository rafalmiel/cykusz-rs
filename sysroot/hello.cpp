#include <cykusz/syscall.h>
#include <iostream>
#include <vector>
#include <signal.h>
#include <unistd.h>
#include <thread>
#include <mutex>

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

static int CNT = 0;

static std::mutex MUT;

void print_thread() {
		for (int i = 0;i < 10; ++i) {
		//for(;;) {
			//std::unique_lock<std::mutex> lck{MUT};
			//std::cout << "Hello from thread one " << i << std::endl;
			//write(1, "TT1. Hello printf one\n", 22);
			printf("Hello printf one\n");
		}
}

void print_thread2() {
		for (int i = 0;i < 10; ++i) {
		//for (;;) {
			printf("Hello, from thread two\n");
			//std::unique_lock<std::mutex> lck{MUT};
			//std::cout << "Hello from thread two " << i << std::endl;
			//write(1, "TT2. Hello printf two\n", 22);
		}
}

int main(int argc, char *argv[]) {
	//for (int i = 0; i < argc; ++i) {
	//	std::cout << "hello arg: " << argv[i] << std::endl;
	//}
	
	struct sigaction sact{};
	sact.sa_handler = int_handler;
	sact.sa_flags = SA_RESTART;

	sigaction(SIGINT, &sact, nullptr);

	std::string input{};
	//std::cout << "Enter your name: ";

	std::thread thr{print_thread};
	std::thread thr2{print_thread2};

	for(int i = 0;i < 10; ++i) {
	//for (;;) {
		//std::unique_lock<std::mutex> lck{MUT};
		//std::cout << "Hello, from main " << i << std::endl;
		//write(1, "TT0. Hello printf main\n", 23);
		//lock.unlock();
		printf("Hello printf main\n");
	}

	//std::cin >> input;
	std::cout << "Hello, " << input << "!" << std::endl;

	std::vector<int> vec = {1, 2, 3, 4, 5};
	for(auto a: vec) {
		std::cout << a << " ";
	}
	std::cout << std::endl;

	char* const args[3] = {"-arg1", "-arg2", nullptr};
	char* const envs[1] = {nullptr};

	thr.join();
	thr2.join();
}
