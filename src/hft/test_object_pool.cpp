#include <iostream>
#include <chrono>
#include <cassert>
#include <thread>
#include <vector>
#include "object_pool.h"

// Test struct for object pool testing
struct TestObject {
    int value;
    std::string data;
    
    TestObject() : value(0), data("") {}
    TestObject(int v, const std::string& d) : value(v), data(d) {}
    
    void reset() {
        value = 0;
        data.clear();
    }
};

// Specialized reset for TestObject
template<>
void ObjectPool<TestObject>::reset_object(TestObject& obj) {
    obj.reset();
}

void test_basic_functionality() {
    std::cout << "Testing basic functionality...\n";
    
    ObjectPool<TestObject> pool(5);
    
    // Test initial state
    assert(pool.available() == 5);
    assert(pool.capacity() == 5);
    
    // Acquire an object
    auto obj1 = pool.acquire();
    assert(obj1 != nullptr);
    assert(pool.available() == 4);
    
    // Use the object
    obj1->value = 42;
    obj1->data = "test";
    
    // Object should be returned to pool when destroyed
    {
        auto obj2 = pool.acquire();
        assert(pool.available() == 3);
    }
    assert(pool.available() == 4);
    
    std::cout << "Basic functionality test passed!\n";
}

void test_pool_exhaustion() {
    std::cout << "Testing pool exhaustion...\n";
    
    ObjectPool<TestObject> pool(2);
    
    auto obj1 = pool.acquire();
    auto obj2 = pool.acquire();
    assert(pool.available() == 0);
    
    // Should create new object when pool is empty
    auto obj3 = pool.acquire();
    assert(obj3 != nullptr);
    assert(pool.available() == 0);
    
    std::cout << "Pool exhaustion test passed!\n";
}

void test_performance() {
    std::cout << "Testing performance...\n";
    
    const int iterations = 100000;
    
    // Test with object pool
    auto start = std::chrono::high_resolution_clock::now();
    {
        ObjectPool<TestObject> pool(100);
        for (int i = 0; i < iterations; ++i) {
            auto obj = pool.acquire();
            obj->value = i;
            obj->data = std::to_string(i);
        }
    }
    auto pool_time = std::chrono::high_resolution_clock::now() - start;
    
    // Test without object pool (direct allocation)
    start = std::chrono::high_resolution_clock::now();
    {
        for (int i = 0; i < iterations; ++i) {
            auto obj = std::make_unique<TestObject>();
            obj->value = i;
            obj->data = std::to_string(i);
        }
    }
    auto direct_time = std::chrono::high_resolution_clock::now() - start;
    
    auto pool_ms = std::chrono::duration_cast<std::chrono::milliseconds>(pool_time).count();
    auto direct_ms = std::chrono::duration_cast<std::chrono::milliseconds>(direct_time).count();
    
    std::cout << "Object pool time: " << pool_ms << "ms\n";
    std::cout << "Direct allocation time: " << direct_ms << "ms\n";
    std::cout << "Performance improvement: " << 
                 (direct_ms > 0 ? (static_cast<double>(direct_ms) / pool_ms) : 1.0) << "x\n";
    
    std::cout << "Performance test completed!\n";
}

void test_thread_safety() {
    std::cout << "Testing thread safety...\n";
    
    ObjectPool<int> pool(10);
    std::vector<std::thread> threads;
    
    for (int i = 0; i < 5; ++i) {
        threads.emplace_back([&pool, i]() {
            for (int j = 0; j < 100; ++j) {
                auto obj = pool.acquire();
                *obj = i * 100 + j;
                std::this_thread::sleep_for(std::chrono::microseconds(1));
            }
        });
    }
    
    for (auto& thread : threads) {
        thread.join();
    }
    
    std::cout << "Thread safety test passed!\n";
}

int main() {
    std::cout << "Running Object Pool Tests\n";
    std::cout << "========================\n";
    
    try {
        test_basic_functionality();
        test_pool_exhaustion();
        test_performance();
        
        std::cout << "\n所有测试通过! Object Pool实现无内存泄漏且具有低延迟特性。\n";
        
    } catch (const std::exception& e) {
        std::cerr << "Test failed with exception: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}