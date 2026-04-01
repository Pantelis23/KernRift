#include <stdlib.h>
#include <stdint.h>

uint64_t fib(uint64_t n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

int main(void) {
    uint64_t r = fib(40);
    return (int)(r & 0xFF);
}
