#include <cykusz/syscall.h>
#include <iostream>
#include <vector>
#include <signal.h>
#include <unistd.h>
#include <thread>
#include <mutex>
#include <cstring>

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
		//for (int i = 0;i < 10; ++i) {
		for(;;) {
			//std::unique_lock<std::mutex> lck{MUT};
			//std::cout << "Hello from thread one " << i << std::endl;
			//write(1, "TT1. Hello printf one\n", 22);
//			printf("Hello printf one\n");
		}
}

void print_thread2(int v) {
		//for (int i = 0;i < 10; ++i) {
		pid_t tid = syscalln0(SYS_GETTID);
		if (false && v != 7) {
			for (int i = 0;; ++i) {
				//printf("%d ", tid);
				char buf[8];
				sprintf(buf, "%d|", tid);
				syscalln3(SYS_WRITE, 0, (uint64_t)buf, strlen(buf));
			}
			//for (;;) {
			//	//printf("%d", v);
	//		//	printf("Hello, from thread two\n");
			//	//std::unique_lock<std::mutex> lck{MUT};
			//	//std::cout << "Hello from thread two " << i << std::endl;
			//	//write(1, "TT2. Hello printf two\n", 22);
			//}
		} else {
			//for (int i = 0; i < 1000; ++i) {
			for (int i = 0; ; ++i) {
				char buf[8];
				sprintf(buf, "%d|", tid);
				syscalln3(SYS_WRITE, 1, (uint64_t)buf, strlen(buf));
			}

            char buf[20];
            sprintf(buf, "exec stack\n");
            syscalln3(SYS_WRITE, 1, (uint64_t)buf, strlen(buf));

			char* args[] = {"/bin/stack", "-arg1", "-arg2", 0};
			char* envs[] = {"PATH=/usr/bin:/bin", 0};

			execve("/bin/stack", args, envs);
		}
}

int main(int argc, char *argv[]) {
	//for (int i = 0; i < argc; ++i) {
	//	std::cout << "hello arg: " << argv[i] << std::endl;
	//}
	
	//struct sigaction sact{};
	//sact.sa_handler = int_handler;
	//sact.sa_flags = SA_RESTART;

	//sigaction(SIGINT, &sact, nullptr);

	std::string input{};
	//std::cout << "Enter your name: ";

    //for (int i = 0; i < 1000; ++i) {
        std::thread thr1{print_thread2, 1};
        std::thread thr2{print_thread2, 2};
        std::thread thr3{print_thread2, 3};
        std::thread thr4{print_thread2, 4};
        std::thread thr5{print_thread2, 5};
        std::thread thr6{print_thread2, 6};
        std::thread thr7{print_thread2, 7};
        std::thread thr8{print_thread2, 8};
        std::thread thr9{print_thread2, 9};

        thr1.join();
        thr2.join();
        thr3.join();
        thr4.join();
        thr5.join();
        thr6.join();
        thr7.join();
        thr8.join();
        thr9.join();
    //}

	//for(int i = 0;i < 10; ++i) {
	//for (;;) {
		//printf("%d", 0);
		//std::unique_lock<std::mutex> lck{MUT};
		//std::cout << "Hello, from main " << i << std::endl;
		//write(1, "TT0. Hello printf main\n", 23);
		//lock.unlock();
	//	printf("Hello printf main\n");
	//}

	//std::cin >> input;
	std::cout << "Hello, " << input << "!" << std::endl;

	std::vector<int> vec = {1, 2, 3, 4, 5};
	for(auto a: vec) {
		std::cout << a << " ";
	}
	std::cout << std::endl;

	//char* const args[3] = {"-arg1", "-arg2", nullptr};
	//char* const envs[1] = {nullptr};

}
