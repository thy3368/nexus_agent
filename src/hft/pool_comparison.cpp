#include <iostream>
#include <chrono>
#include <vector>
#include <iomanip>
#include "object_pool.h"
#include "fast_object_pool.h"

// 简单测试对象
struct SimpleObject {
    int id;
    double value;
    
    SimpleObject() : id(0), value(0.0) {}
    
    void reset() {
        id = 0;
        value = 0.0;
    }
};

// 复杂测试对象
struct ComplexObject {
    int id;
    std::string name;
    std::vector<double> data;
    
    ComplexObject() : id(0), name(""), data(10, 0.0) {}
    
    void reset() {
        id = 0;
        name.clear();
        std::fill(data.begin(), data.end(), 0.0);
    }
};

// 为 FastObjectPool 特化 reset 方法
template<>
void FastObjectPool<SimpleObject>::reset_object(SimpleObject& obj) {
    obj.reset();
}

template<>
void FastObjectPool<ComplexObject>::reset_object(ComplexObject& obj) {
    obj.reset();
}

template<>
void ObjectPool<SimpleObject>::reset_object(SimpleObject& obj) {
    obj.reset();
}

template<>
void ObjectPool<ComplexObject>::reset_object(ComplexObject& obj) {
    obj.reset();
}

void test_simple_object_performance() {
    std::cout << "\n=== 简单对象性能测试 ===\n";
    
    const int iterations = 10000000;
    const int pool_size = 1000;
    
    std::cout << "测试规模: " << iterations << " 次操作\n";
    std::cout << "对象池大小: " << pool_size << "\n\n";
    
    // 测试1: 直接分配
    auto start = std::chrono::high_resolution_clock::now();
    {
        for (int i = 0; i < iterations; ++i) {
            auto obj = std::make_unique<SimpleObject>();
            obj->id = i;
            obj->value = i * 1.1;
        }
    }
    auto direct_time = std::chrono::high_resolution_clock::now() - start;
    auto direct_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(direct_time).count();
    
    // 测试2: 原始对象池
    start = std::chrono::high_resolution_clock::now();
    {
        ObjectPool<SimpleObject> pool(pool_size);
        for (int i = 0; i < iterations; ++i) {
            auto obj = pool.acquire();
            obj->id = i;
            obj->value = i * 1.1;
        }
    }
    auto pool_time = std::chrono::high_resolution_clock::now() - start;
    auto pool_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(pool_time).count();
    
    // 测试3: 快速对象池
    start = std::chrono::high_resolution_clock::now();
    {
        FastObjectPool<SimpleObject> fast_pool(pool_size);
        for (int i = 0; i < iterations; ++i) {
            auto obj = acquire_from_pool(fast_pool);
            obj->id = i;
            obj->value = i * 1.1;
        }
    }
    auto fast_time = std::chrono::high_resolution_clock::now() - start;
    auto fast_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(fast_time).count();
    
    // 测试4: 裸指针快速池
    start = std::chrono::high_resolution_clock::now();
    {
        FastObjectPool<SimpleObject> raw_pool(pool_size);
        for (int i = 0; i < iterations; ++i) {
            auto* obj = raw_pool.acquire_fast();
            obj->id = i;
            obj->value = i * 1.1;
            raw_pool.release_fast(obj);
        }
    }
    auto raw_time = std::chrono::high_resolution_clock::now() - start;
    auto raw_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(raw_time).count();
    
    // 输出结果
    std::cout << "性能测试结果 (简单对象):\n";
    std::cout << "直接分配:     " << std::setw(8) << direct_ns / 1000000.0 << " ms ("
              << std::setw(6) << static_cast<double>(direct_ns) / iterations << " ns/op)\n";
    std::cout << "原始对象池:   " << std::setw(8) << pool_ns / 1000000.0 << " ms ("
              << std::setw(6) << static_cast<double>(pool_ns) / iterations << " ns/op)\n";
    std::cout << "快速对象池:   " << std::setw(8) << fast_ns / 1000000.0 << " ms ("
              << std::setw(6) << static_cast<double>(fast_ns) / iterations << " ns/op)\n";
    std::cout << "裸指针池:     " << std::setw(8) << raw_ns / 1000000.0 << " ms ("
              << std::setw(6) << static_cast<double>(raw_ns) / iterations << " ns/op)\n";
    
    std::cout << "\n相对性能 (以直接分配为基准):\n";
    if (direct_ns > 0) {
        std::cout << "原始对象池: " << std::fixed << std::setprecision(2) 
                  << static_cast<double>(direct_ns) / pool_ns << "x\n";
        std::cout << "快速对象池: " << std::fixed << std::setprecision(2) 
                  << static_cast<double>(direct_ns) / fast_ns << "x\n";
        std::cout << "裸指针池:   " << std::fixed << std::setprecision(2) 
                  << static_cast<double>(direct_ns) / raw_ns << "x\n";
    }
}

void test_complex_object_performance() {
    std::cout << "\n=== 复杂对象性能测试 ===\n";
    
    const int iterations = 10000000;  // 减少迭代次数，因为对象更复杂
    const int pool_size = 500;
    
    std::cout << "测试规模: " << iterations << " 次操作\n";
    std::cout << "对象池大小: " << pool_size << "\n\n";
    
    // 测试1: 直接分配
    auto start = std::chrono::high_resolution_clock::now();
    {
        for (int i = 0; i < iterations; ++i) {
            auto obj = std::make_unique<ComplexObject>();
            obj->id = i;
            obj->name = "Object" + std::to_string(i);
            obj->data[0] = i * 1.1;
        }
    }
    auto direct_time = std::chrono::high_resolution_clock::now() - start;
    auto direct_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(direct_time).count();
    
    // 测试2: 快速对象池
    start = std::chrono::high_resolution_clock::now();
    {
        FastObjectPool<ComplexObject> fast_pool(pool_size);
        for (int i = 0; i < iterations; ++i) {
            auto obj = acquire_from_pool(fast_pool);
            obj->id = i;
            obj->name = "Object" + std::to_string(i);
            obj->data[0] = i * 1.1;
        }
    }
    auto fast_time = std::chrono::high_resolution_clock::now() - start;
    auto fast_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(fast_time).count();
    
    // 测试3: 裸指针快速池
    start = std::chrono::high_resolution_clock::now();
    {
        FastObjectPool<ComplexObject> raw_pool(pool_size);
        for (int i = 0; i < iterations; ++i) {
            auto* obj = raw_pool.acquire_fast();
            obj->id = i;
            obj->name = "Object" + std::to_string(i);
            obj->data[0] = i * 1.1;
            raw_pool.release_fast(obj);
        }
    }
    auto raw_time = std::chrono::high_resolution_clock::now() - start;
    auto raw_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(raw_time).count();
    
    // 输出结果
    std::cout << "性能测试结果 (复杂对象):\n";
    std::cout << "直接分配:     " << std::setw(8) << direct_ns / 1000000.0 << " ms ("
              << std::setw(6) << static_cast<double>(direct_ns) / iterations << " ns/op)\n";
    std::cout << "快速对象池:   " << std::setw(8) << fast_ns / 1000000.0 << " ms ("
              << std::setw(6) << static_cast<double>(fast_ns) / iterations << " ns/op)\n";
    std::cout << "裸指针池:     " << std::setw(8) << raw_ns / 1000000.0 << " ms ("
              << std::setw(6) << static_cast<double>(raw_ns) / iterations << " ns/op)\n";
    
    std::cout << "\n相对性能 (以直接分配为基准):\n";
    if (direct_ns > 0) {
        std::cout << "快速对象池: " << std::fixed << std::setprecision(2) 
                  << static_cast<double>(direct_ns) / fast_ns << "x\n";
        std::cout << "裸指针池:   " << std::fixed << std::setprecision(2) 
                  << static_cast<double>(direct_ns) / raw_ns << "x\n";
    }
}

void test_burst_allocation() {
    std::cout << "\n=== 突发分配性能测试 ===\n";
    
    const int burst_size = 5000;
    const int burst_count = 50;
    
    std::cout << "突发测试: " << burst_count << " 轮 x " << burst_size << " 次分配\n\n";
    
    // 测试直接分配
    auto start = std::chrono::high_resolution_clock::now();
    {
        for (int burst = 0; burst < burst_count; ++burst) {
            std::vector<std::unique_ptr<SimpleObject>> objects;
            objects.reserve(burst_size);
            
            for (int i = 0; i < burst_size; ++i) {
                objects.push_back(std::make_unique<SimpleObject>());
                objects.back()->id = i;
            }
            objects.clear();
        }
    }
    auto direct_burst_time = std::chrono::high_resolution_clock::now() - start;
    
    // 测试快速对象池
    start = std::chrono::high_resolution_clock::now();
    {
        FastObjectPool<SimpleObject> pool(burst_size / 2); // 故意设小一些
        for (int burst = 0; burst < burst_count; ++burst) {
            std::vector<SimpleObject*> objects;
            objects.reserve(burst_size);
            
            for (int i = 0; i < burst_size; ++i) {
                objects.push_back(pool.acquire_fast());
                objects.back()->id = i;
            }
            
            for (auto* obj : objects) {
                pool.release_fast(obj);
            }
            objects.clear();
        }
    }
    auto pool_burst_time = std::chrono::high_resolution_clock::now() - start;
    
    auto direct_ms = std::chrono::duration_cast<std::chrono::milliseconds>(direct_burst_time).count();
    auto pool_ms = std::chrono::duration_cast<std::chrono::milliseconds>(pool_burst_time).count();
    
    std::cout << "突发分配测试结果:\n";
    std::cout << "直接分配: " << direct_ms << " ms\n";
    std::cout << "快速池:   " << pool_ms << " ms\n";
    
    if (direct_ms > pool_ms && pool_ms > 0) {
        double improvement = static_cast<double>(direct_ms) / pool_ms;
        std::cout << "性能提升: " << std::fixed << std::setprecision(2) << improvement << "x\n";
    } else if (pool_ms > direct_ms && direct_ms > 0) {
        double degradation = static_cast<double>(pool_ms) / direct_ms;
        std::cout << "性能下降: " << std::fixed << std::setprecision(2) << degradation << "x\n";
    }
}

int main() {
    std::cout << "对象池性能对比测试\n";
    std::cout << "==================\n";
    std::cout << "测试不同对象池实现在各种场景下的性能表现\n";
    
    test_simple_object_performance();
    test_complex_object_performance();
    test_burst_allocation();
    
    std::cout << "\n=== 测试总结 ===\n";
    std::cout << "1. 对于简单对象，对象池可能没有显著优势，甚至可能更慢\n";
    std::cout << "2. 对于复杂对象（含有动态分配），对象池优势明显\n";
    std::cout << "3. 在突发分配场景下，对象池能显著减少分配器压力\n";
    std::cout << "4. 裸指针版本的对象池性能最好，但需要手动管理生命周期\n";
    std::cout << "5. RAII包装版本在保证安全性的同时，性能损失较小\n";
    
    return 0;
}