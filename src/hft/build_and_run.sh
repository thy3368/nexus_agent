#!/bin/bash

echo "=== 编译对象池使用示例 ==="

# 编译 usage_example
echo "编译 usage_example.cpp..."
c++ -std=c++20 -I. -O2 -o usage_example usage_example.cpp
if [ $? -eq 0 ]; then
    echo "编译成功!"
    echo
    echo "=== 运行对象池示例 ==="
    ./usage_example
else
    echo "编译失败，尝试其他编译器..."
    clang++ -std=c++20 -I. -O2 -o usage_example usage_example.cpp
    if [ $? -eq 0 ]; then
        echo "使用 clang++ 编译成功!"
        echo
        echo "=== 运行对象池示例 ==="
        ./usage_example
    else
        echo "编译失败"
        exit 1
    fi
fi

echo
echo "=== 编译测试程序 ==="

# 编译 test_object_pool
echo "编译 test_object_pool.cpp..."
c++ -std=c++20 -I. -O2 -o test_object_pool test_object_pool.cpp -pthread
if [ $? -eq 0 ]; then
    echo "测试程序编译成功!"
else
    echo "测试程序编译失败"
fi

echo
echo "=== 编译性能对比程序 ==="

# 编译 pool_comparison
echo "编译 pool_comparison.cpp..."
c++ -std=c++20 -I. -O2 -o pool_comparison pool_comparison.cpp -pthread
if [ $? -eq 0 ]; then
    echo "性能对比程序编译成功!"
    echo
    echo "=== 运行性能对比测试 ==="
    ./pool_comparison
else
    echo "性能对比程序编译失败"
fi