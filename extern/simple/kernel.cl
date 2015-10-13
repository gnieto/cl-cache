__kernel void vector_add(__global const long *A, __global const long *B, __global long *C) {
	int i = get_global_id(0);
	C[i] = A[i] + B[i];
}