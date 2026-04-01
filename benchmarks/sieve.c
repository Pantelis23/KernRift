#include <stdlib.h>
#include <stdint.h>
#include <string.h>

int main(void) {
    int N = 1000000;
    uint8_t *sieve = (uint8_t *)malloc(N);
    memset(sieve, 1, N);
    for (int p = 2; (long)p * p < N; p++) {
        if (sieve[p]) {
            for (int j = p * p; j < N; j += p)
                sieve[j] = 0;
        }
    }
    int count = 0;
    for (int i = 2; i < N; i++)
        if (sieve[i]) count++;
    return count & 0xFF;
}
