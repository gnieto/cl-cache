# What's cl-cache?

cl-cache is a Rust library (with C bindings) to simplify the OpenCL's kernels compilation and caching. It will compile them if needed or it will use a pre-compiled binary if we already built this kernel with the same parameters.

# Why cl-cache?

When using OpenCL, you can create the *cl_program* from source or from binary. The easiest way to load a program is load the kernel's source code on memory and call *clCreateProgramWithSource* and then *clBuildProgram*, but it has some drawbacks if you use always that flow:

* Every time you load your host binary, it should load the source on memory and compile it. Depending on the size of the kernel, it may take a while to compile.
* You should distribute your kernel source code with the program source

A part of the use of the source code and compilation time, another issue I've found is that some OpenCL's integrations have developed his own kernel cache system. People is doing the same job over and over on distinct projects. For example, [John the Ripper Jumbo](https://github.com/magnumripper/JohnTheRipper) or the [GROMACS OpenCL fork](https://github.com/StreamComputing/gromacs), have their own caching system.

The aim of this work is create a library which can standardize the way the projects are using the binary kernels cache.

## Rust

I've choose Rust as the development language because it's just an amazing language. All the code that is crossing the C API will be memory and thread safe, so, it can be developed without taking account a sort of bugs that are guaranteed to don't happen if Rust is used.

# Library usage

## Create cache

To create a cache you will need to supply a cache backend (the driver which will be used to store/load the kernels binaries). By the moment, only two drivers are supported:

* Volatile: It will save the binaries on memory. This driver it's only intended to we used with testing purposes
* File system: It needs a route on the file system to save/load the binaries

For example, creating a file system cache it's as easy as:

```rust
extern crate clcache;

use clcache::cache::Cache;
use clcache::cache::disk::FileSystemCache;

let backend_cache = Box::new(FileSystemCache::new("/tmp/demo/".to_string()).unwrap());
let mut cache = Cache::new();
```

## get

With this method, you should provide the source code of the kernel, a list of devices and a context. With all this information, the library will have enough information to generate a proper key and check if it should compile the kernel or if it can use the binary version.

```rust
let pq = PlatformQuery::Index(0);
let platform = ClRoot::get_platform(&pq).unwrap();
let dq = DeviceQuery::Type(DeviceType::All);
let devices = platform.get_devices_query(&dq);
let context = Context::from_devices(&devices);

let program = cache.get(
  get_kernel_code(),
  &devices,
  &ctx
).unwrap();
```

After this code, you will have a `cl::program::Program` instance, which is a type that wraps a raw `cl_program` instance.

## get_with_options

This method is the same as the previous one, but can provide options that will be forwarded to `clBuildPorgram`.

## put_with_tag

This method receives a tag name, a vector of devices and a `cl::program::Program`. It will try to extract the binaries for each of the devices and save it associated with the tag name. After that, it can be recovered just with that tag name, so we can aggressively cache the binaries without even having to load the source code of the kernel.

If a previous entry on the cache exists with the same tag and device, it will be replaced by the new one, so, it's responsibility of the user of the library to save the program with the proper tag.

## get_with_tag

This method receives a tag name, devices and context and is used to recover a kernel that was saved with that tag and devices. If will fail if, at least, one pair of tag name and device is not present on the cache.

The library provides a program to precompile kernels and associate it with tags. You can check this on the wamup section.

# C library usage

This project also provides a C API to use this library. You also can check the [integration](https://github.com/gnieto/JohnTheRipper/commit/3ae618feea4acd01215e2c564882162b9e1ee7a0) that I did (with less than an hour) for John the Ripper. Now it's not checking the modification date of the file, but it shows the amount of code that can be removed (specially, the hashing one).

## Create cache

By the moment, the unique kind of cache that can be created is the file system one. The kind of caches that can be created is open, so we are returning only IDs that will be used to interact with the library.

```c
#include "cl_cache.h"

unsigned int cl_cache_index = cl_cache_create_fs("/tmp/test");
```

If the provided route does not exists, it will be created. If the user has not enough permissions or there's any problem creating it, the result of the call will be 0.

## cl_cache_get

```c
cl_context context;
cl_device_id* devices;

...

cl_program program = cl_cache_get(cl_cache_index, kernel_source, 1, devices, context);
cl_kernel kernel = clCreateKernel(program, "example_kernel", NULL);
```

## cl_cache_get_with_options

```c

cl_context context;
cl_device_id* devices;

...

cl_program program = cl_cache_get_with_options(
  cl_cache_index,
  kernel_source,
  1,
  devices,
  context,
  "-DSOME_PARAMETER=2"
);
cl_kernel kernel = clCreateKernel(program, "example_kernel", NULL);
```

## cl_cache_get_with_tag

```c

cl_context context;
cl_device_id* devices;

...

cl_program program = cl_cache_get_with_tag(cl_cache_index, "kernel_tag", 1, devices, context);
cl_kernel kernel = clCreateKernel(program, "example_kernel", NULL);
```

# Which data is used to build the cache key?

The information that is used to create the cache key is a SHA256 of the concatenation of the next data:

* Source code (or tag)
* Options (if provided)
* Platform name
* Platform version
* Device name

With this data we ensure that the binary will change if one of the next changes: source code, compilation flag or driver, and that we will have a version of each one of the devices presents on the host.

# Command usage

The library provides a program to warmup the cache for the target devices and source kernels. It can be useful to:

* Check if your kernel compiles with the given compilation flags (options)
* Distribute precompiled kernels with you program
* Speed up the boot time of your application, avoiding compile the kernels on demand

## Warmup

You can run the warmup command typing:

```rust
  cargo run --bin=warmup -- -s=<path_to_source>
```

This will compile the OpenCL code found on the target path for the first platform found and all the associated devices. There are several options to select the platform, the devices, the output directory or if it should work recursively. You can check all the options by typing:

```rust
  cargo run --bin=warmup -- --help
```

## YAML

As there are a lot of parameters that can receive the warmup command, we can define all the kernels to compile on a YAML file to make the interaction with the warmup command simpler.

You can run it typing:

```rust
  cargo run --bin=warmup -- -y=<path_to_yaml>
```

Each YAML will describe a set of jobs to be executed. The simpler YAML file you can type is the next:

```yaml
jobs:
    - <job_name>:
        source: <path_to_source>
    - <another_group_of_kernels>:
        source: <path_to_another_source>
```

There are several parameters that can be set to each group of jobs to allow forward the same parameters that we can set from the command line:

* platform_index: It will select the platform with this index.
* platform_regex: It will select the first platform that matches the given regular expression
* device_index: It will select the device with this index given by the platform
* device_regex: It will select the devices that matches with the given regular expression.
* device_type: It will select the devices by device type. Allowed types are: cpu, gpu, all
* output: Output directory where the binaries will be saved
* options: Compilation flags that will be forwarded to `clBuildProgram`
* recursive: If true, it will scan the source path recursively.
* extension: It will scan only the files with the given extension (by default, cl)
* verbose: It sets the level of verbosity of the command, for the given job.
* tag: Save the kernel source with the give tag. Only valid if source is a file.
* force_rebuild: It will force the kernel recompilation and will replace the previous content.

# Examples

You can find a full example of the library usage with Rust on src/bin/demo1.rs
You can find a full simple example of the library usage with C on extern/example/simple.c
You can find a full example using tags with C on extern/example/tagged.c

# Future work

There are several things that can be done to improve the current implementation:

* Add more cache backends
* Implement a `get_from_path` method, that will use the last modification timestamp to decide if the entry of the cache is valid or not
* Add tests to the Rust library
* Add tests to the C API
* Split the project on several repositories:
** Current library
** C API bindings
** Generic cache backend
** High-level Rust OpenCL bindings (or work on [rust-opencl](https://github.com/luqmana/rust-opencl))
* Decrease the amount of dependencies
* Build with Rust stable
* Return errors trough the C API
* Test the thread-safeness of the C API

# Acknowledgments

This library could not be developed without the previous work of [rust-opencl](https://github.com/luqmana/rust-opencl). A part of the "raw" bindings, the high-level bindings that resides on this library are almost the same as the ones on that repository, but adapted to the needs of this project.
