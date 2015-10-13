extern unsigned int cl_cache_create_fs(char* path);
extern cl_program cl_cache_get(unsigned int cache_id, char* source, unsigned char num_devices, void* devices, void *context);
extern cl_program cl_cache_get_with_options(unsigned int cache_id, char* source, unsigned char num_devices, void* devices, void *context, char* options);
extern cl_program cl_cache_get_with_tag(unsigned int cache_id, char* tag, unsigned char num_devices, void* devices, void *context);
extern cl_program cl_cache_put_with_tag(unsigned int cache_id, char* tag, unsigned char num_devices, void* devices, void *program);