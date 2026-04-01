#include <stdlib.h>
#include <stdint.h>
#include <string.h>

int main(void) {
    int N = 200;
    uint64_t *A = (uint64_t *)calloc(N * N, sizeof(uint64_t));
    uint64_t *B = (uint64_t *)calloc(N * N, sizeof(uint64_t));
    uint64_t *C = (uint64_t *)calloc(N * N, sizeof(uint64_t));
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++) {
            A[i*N+j] = (i + j) & 0xFF;
            B[i*N+j] = (i + j) & 0xFF;
        }
    for (int i = 0; i < N; i++)
        for (int k = 0; k < N; k++) {
            uint64_t aik = A[i*N+k];
            for (int j = 0; j < N; j++)
                C[i*N+j] += aik * B[k*N+j];
        }
    return (int)(C[0] & 0xFF);
}
