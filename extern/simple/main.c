#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/stat.h>

#ifdef __APPLE__
    #include "OpenCL/opencl.h"
#else
    #include "CL/cl.h"
#endif
#include "cl_cache.h"

#define DATA_SIZE (1024)

cl_device_id* get_devices(cl_platform_id platform, cl_uint num_devices);
cl_uint get_num_devices(cl_platform_id platform);
cl_context create_context(cl_device_id* devices, cl_uint num_devices);
cl_command_queue create_command_queue(cl_context context, cl_device_id device);

int main(int argc, char** argv)
{
    int err;
      
    long a[DATA_SIZE], b[DATA_SIZE];
    long results[DATA_SIZE];

    size_t global;
    size_t local;

    cl_program program;
    cl_kernel kernel;    
    cl_mem input_a, input_b;
    cl_mem output;

    cl_platform_id platform_id;
    clGetPlatformIDs(1, &platform_id, NULL);

    for(int i = 0; i < DATA_SIZE; i++) {
        a[i] = i;
        b[i] = DATA_SIZE - i;
    }
    
    cl_uint num_devices = get_num_devices(platform_id);
    cl_device_id* devices = get_devices(platform_id, num_devices);
    cl_device_id device_id = devices[0];
    cl_context context = create_context(devices, num_devices);
    cl_command_queue commands = create_command_queue(context, devices[0]);

    unsigned short cache = cl_cache_create_fs("/tmp/test");
    if (cache == -1) {
        printf("Could not get the cache\n");
        exit(1);
    }

    program = cl_cache_get_with_tag(cache, "tag_test", 1, &device_id, context);
    if (program == NULL) {
        printf("Could not get program with tag!!\n");
        exit(1);
    }

    kernel = clCreateKernel(program, "vector_add", &err);
    if (!kernel || err != CL_SUCCESS)
    {
        printf("Could not create kernel! %d\n", err);
        exit(1);
    }

    input_a = clCreateBuffer(context, CL_MEM_READ_ONLY, sizeof(long) * DATA_SIZE, NULL, NULL);
    input_b = clCreateBuffer(context, CL_MEM_READ_ONLY, sizeof(long) * DATA_SIZE, NULL, NULL);
    output = clCreateBuffer(context, CL_MEM_WRITE_ONLY, sizeof(long) * DATA_SIZE, NULL, NULL);
    if (!input_a || !input_b || !output)
    {
        printf("Some buffer failed!\n");
        exit(1);
    }    
    
    err = clEnqueueWriteBuffer(commands, input_a, CL_TRUE, 0, sizeof(long) * DATA_SIZE, a, 0, NULL, NULL);
    if (err != CL_SUCCESS)
    {
        printf("Could not write buffer a!\n");
        exit(1);
    }

    err = clEnqueueWriteBuffer(commands, input_b, CL_TRUE, 0, sizeof(long) * DATA_SIZE, b, 0, NULL, NULL);
    if (err != CL_SUCCESS)
    {
        printf("Could not write buffer b\n");
        exit(1);
    }

    err = 0;
    err  = clSetKernelArg(kernel, 0, sizeof(cl_mem), &input_a);
    err |= clSetKernelArg(kernel, 1, sizeof(cl_mem), &input_b);
    err |= clSetKernelArg(kernel, 2, sizeof(cl_mem), &output);
    if (err != CL_SUCCESS)
    {
        printf("Could not set some of the arguments!\n");
        exit(1);
    }

    global = DATA_SIZE;
    err = clEnqueueNDRangeKernel(commands, kernel, 1, NULL, &global, NULL, 0, NULL, NULL);
    if (err)
    {
        printf("Could not queue kernel execution!\n");
        return EXIT_FAILURE;
    }

    clFinish(commands);

    err = clEnqueueReadBuffer(commands, output, CL_TRUE, 0, sizeof(long) * DATA_SIZE, results, 0, NULL, NULL);  
    if (err != CL_SUCCESS)
    {
        printf("Could not read the output buffer!\n");
        exit(1);
    }
    
    unsigned int correct = 0;
    for (unsigned int i = 0; i < DATA_SIZE; ++i)
    {
        if(results[i] == a[i] + b[i]) {
            ++correct;
        } else {
            printf("%d: %ld = %ld + %ld\n", i, results[i], a[i], b[i]);
        }
    }

    if (correct == DATA_SIZE) {
        printf("Vectors are equal\n");
    } else {
        printf("Vectors are not equal\n");
    }
    
    clReleaseMemObject(input_a);
    clReleaseMemObject(input_b);
    clReleaseMemObject(output);
    clReleaseProgram(program);
    clReleaseKernel(kernel);
    clReleaseCommandQueue(commands);
    clReleaseContext(context);

    return 0;
}

cl_uint get_num_devices(cl_platform_id platform) {
    // TODO: Check errors
    cl_uint num_devices;
    clGetDeviceIDs(
        platform,
        CL_DEVICE_TYPE_ALL,
        (cl_uint) 0,
        NULL,
        &num_devices
    );

    return num_devices;
}

cl_device_id* get_devices(cl_platform_id platform, cl_uint num_devices) {
    cl_device_id* devices = malloc(sizeof(cl_device_id) * num_devices);

    clGetDeviceIDs(
        platform,
        CL_DEVICE_TYPE_ALL,
        num_devices,
        devices,
        NULL
    );

    return devices;
}

cl_context create_context(cl_device_id* devices, cl_uint num_devices) {
    // TODO: Check errors
    cl_context context = clCreateContext(0, num_devices, devices, NULL, NULL, NULL);
    if (!context)
    {
        printf("Error: Failed to create a compute context!\n");
        exit(1);
    }

    return context;
}

cl_command_queue create_command_queue(cl_context context, cl_device_id device) {
    cl_command_queue commands = clCreateCommandQueue(context, device, 0, NULL);

    if (!commands)
    {
        printf("Error: Failed to create a command commands!\n");
        exit(1);
    }

    return commands;
}