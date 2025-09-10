cmake_minimum_required(VERSION 3.30)
project(hft)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 启用编译器优化和调试信息
set(CMAKE_CXX_FLAGS_DEBUG "-g -O0")
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG")

# 包含头文件目录
include_directories(.)

# 主程序
add_executable(hft main.cpp)

# 对象池测试程序
add_executable(test_object_pool test_object_pool.cpp)
target_link_libraries(test_object_pool pthread)

# 使用示例程序
add_executable(usage_example usage_example.cpp)
target_link_libraries(usage_example pthread)

# 对象池性能对比程序
add_executable(pool_comparison pool_comparison.cpp)
target_link_libraries(pool_comparison pthread)
