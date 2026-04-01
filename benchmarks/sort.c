#include <stdlib.h>
#include <stdint.h>

int main(void) {
    int N = 10000;
    uint64_t *buf = (uint64_t *)malloc(N * sizeof(uint64_t));
    for (int i = 0; i < N; i++) buf[i] = N - i;

    int swapped = 1;
    while (swapped) {
        swapped = 0;
        for (int i = 0; i < N - 1; i++) {
            if (buf[i] > buf[i+1]) {
                uint64_t tmp = buf[i];
                buf[i] = buf[i+1];
                buf[i+1] = tmp;
                swapped = 1;
            }
        }
    }
    return (int)(buf[0] & 0xFF);
}
