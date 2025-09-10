#include <iostream>
#include <vector>
#include <chrono>
#include <iomanip>
#include "object_pool.h"

// 交易订单类示例
struct Order {
    int order_id;
    double price;
    int quantity;
    std::string symbol;
    bool is_buy;
    std::chrono::high_resolution_clock::time_point timestamp;
    
    Order() : order_id(0), price(0.0), quantity(0), symbol(""), is_buy(true) {
        timestamp = std::chrono::high_resolution_clock::now();
    }
    
    void reset() {
        order_id = 0;
        price = 0.0;
        quantity = 0;
        symbol.clear();
        is_buy = true;
        timestamp = std::chrono::high_resolution_clock::now();
    }
    
    void print() const {
        std::cout << "订单 #" << order_id 
                  << " | " << symbol 
                  << " | " << (is_buy ? "买入" : "卖出")
                  << " | 数量: " << quantity 
                  << " | 价格: " << price << std::endl;
    }
};

// 为Order类型特化reset方法
template<>
void ObjectPool<Order>::reset_object(Order& obj) {
    obj.reset();
}

// 函数声明
void simulate_hft_trading();
void performance_comparison();
void test_allocation_frequency();

// 模拟高频交易场景
void simulate_hft_trading() {
    std::cout << "\n=== 高频交易对象池使用示例 ===\n";
    
    // 创建订单对象池，预分配100个订单对象
    ObjectPool<Order> order_pool(100);
    
    std::cout << "初始对象池状态: " << order_pool.available() 
              << "/" << order_pool.capacity() << " 可用\n";
    
    // 模拟处理1000笔交易订单
    const int num_orders = 1000;
    std::vector<std::string> symbols = {"AAPL", "GOOGL", "TSLA", "MSFT", "AMZN"};
    
    auto start_time = std::chrono::high_resolution_clock::now();
    
    for (int i = 0; i < num_orders; ++i) {
        // 从对象池获取订单对象
        auto order = order_pool.acquire();
        
        // 设置订单数据
        order->order_id = i + 1;
        order->symbol = symbols[i % symbols.size()];
        order->price = 100.0 + (i % 1000) * 0.01;
        order->quantity = 100 + (i % 10) * 10;
        order->is_buy = (i % 2 == 0);
        
        // 在实际应用中，这里会进行订单处理逻辑
        // 例如：验证、风控检查、发送到交易所等
        
        if (i % 200 == 0) {
            order->print();
            std::cout << "对象池状态: " << order_pool.available() 
                      << "/" << order_pool.capacity() << " 可用\n";
        }
        
        // order对象会在作用域结束时自动返回到对象池
    }
    
    auto end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
    
    std::cout << "\n处理 " << num_orders << " 笔订单耗时: " 
              << duration.count() << " 微秒\n";
    std::cout << "平均每笔订单耗时: " 
              << static_cast<double>(duration.count()) / num_orders << " 微秒\n";
    
    std::cout << "最终对象池状态: " << order_pool.available() 
              << "/" << order_pool.capacity() << " 可用\n";
}

// 性能对比测试 - 优化版本
void performance_comparison() {
    std::cout << "\n=== 性能对比测试 ===\n";
    
    const int iterations = 1000000;  // 增加测试规模
    const int warmup_iterations = 10000;  // 预热阶段
    const int pool_size = 1000;  // 增加池大小减少锁竞争
    
    std::cout << "测试规模: " << iterations << " 次分配/释放\n";
    std::cout << "预热阶段: " << warmup_iterations << " 次\n\n";
    
    // 预热阶段 - 对象池
    {
        ObjectPool<Order> warmup_pool(pool_size);
        for (int i = 0; i < warmup_iterations; ++i) {
            auto order = warmup_pool.acquire();
            order->order_id = i;
        }
    }
    
    // 预热阶段 - 直接分配
    {
        for (int i = 0; i < warmup_iterations; ++i) {
            auto order = std::make_unique<Order>();
            order->order_id = i;
        }
    }
    
    // 正式测试 - 对象池方式
    std::cout << "测试对象池方式...";
    auto start = std::chrono::high_resolution_clock::now();
    {
        ObjectPool<Order> pool(pool_size);
        for (int i = 0; i < iterations; ++i) {
            auto order = pool.acquire();
            order->order_id = i;
            order->price = 100.0 + (i % 1000) * 0.01;
            order->quantity = 100 + (i % 10);
            // 模拟一些计算
            volatile double temp = order->price * order->quantity;
            (void)temp;  // 避免编译器优化
        }
    }
    auto pool_time = std::chrono::high_resolution_clock::now() - start;
    auto pool_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(pool_time).count();
    std::cout << " 完成\n";
    
    // 正式测试 - 直接分配方式  
    std::cout << "测试直接分配方式...";
    start = std::chrono::high_resolution_clock::now();
    {
        for (int i = 0; i < iterations; ++i) {
            auto order = std::make_unique<Order>();
            order->order_id = i;
            order->price = 100.0 + (i % 1000) * 0.01;
            order->quantity = 100 + (i % 10);
            // 模拟相同的计算
            volatile double temp = order->price * order->quantity;
            (void)temp;  // 避免编译器优化
        }
    }
    auto direct_time = std::chrono::high_resolution_clock::now() - start;
    auto direct_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(direct_time).count();
    std::cout << " 完成\n\n";
    
    // 输出详细结果
    std::cout << "=== 详细性能报告 ===\n";
    std::cout << "对象池方式:\n";
    std::cout << "  总耗时: " << pool_ns / 1000000.0 << " 毫秒\n";
    std::cout << "  平均每次: " << static_cast<double>(pool_ns) / iterations << " 纳秒\n";
    
    std::cout << "直接分配方式:\n";
    std::cout << "  总耗时: " << direct_ns / 1000000.0 << " 毫秒\n";
    std::cout << "  平均每次: " << static_cast<double>(direct_ns) / iterations << " 纳秒\n";
    
    if (pool_ns > 0 && direct_ns > 0) {
        if (direct_ns > pool_ns) {
            double improvement = static_cast<double>(direct_ns) / pool_ns;
            std::cout << "\n🎯 对象池性能提升: " << std::fixed << std::setprecision(2) 
                      << improvement << "x\n";
        } else {
            double degradation = static_cast<double>(pool_ns) / direct_ns;
            std::cout << "\n⚠️  对象池性能下降: " << std::fixed << std::setprecision(2) 
                      << degradation << "x (可能原因: 锁竞争、测试规模过小)\n";
        }
    }
    
    // 内存分配频率测试
    test_allocation_frequency();
}

// 测试高频分配场景
void test_allocation_frequency() {
    std::cout << "\n=== 高频分配场景测试 ===\n";
    
    const int burst_size = 10000;
    const int burst_count = 100;
    
    // 对象池方式 - 突发分配测试
    std::cout << "测试对象池突发分配 (" << burst_count << "轮 x " << burst_size << "次)...\n";
    auto start = std::chrono::high_resolution_clock::now();
    {
        ObjectPool<Order> pool(burst_size / 2);  // 故意设置较小的池
        for (int burst = 0; burst < burst_count; ++burst) {
            std::vector<ObjectPool<Order>::ObjectPtr> orders;
            orders.reserve(burst_size);
            
            // 突发分配
            for (int i = 0; i < burst_size; ++i) {
                orders.push_back(pool.acquire());
                orders.back()->order_id = i;
            }
            
            // 突发释放 (通过清空vector自动释放)
            orders.clear();
        }
    }
    auto pool_burst_time = std::chrono::high_resolution_clock::now() - start;
    
    // 直接分配方式 - 突发分配测试
    std::cout << "测试直接分配突发分配...\n";
    start = std::chrono::high_resolution_clock::now();
    {
        for (int burst = 0; burst < burst_count; ++burst) {
            std::vector<std::unique_ptr<Order>> orders;
            orders.reserve(burst_size);
            
            // 突发分配
            for (int i = 0; i < burst_size; ++i) {
                orders.push_back(std::make_unique<Order>());
                orders.back()->order_id = i;
            }
            
            // 突发释放
            orders.clear();
        }
    }
    auto direct_burst_time = std::chrono::high_resolution_clock::now() - start;
    
    auto pool_burst_ms = std::chrono::duration_cast<std::chrono::milliseconds>(pool_burst_time).count();
    auto direct_burst_ms = std::chrono::duration_cast<std::chrono::milliseconds>(direct_burst_time).count();
    
    std::cout << "\n突发分配测试结果:\n";
    std::cout << "对象池方式: " << pool_burst_ms << " 毫秒\n";
    std::cout << "直接分配方式: " << direct_burst_ms << " 毫秒\n";
    
    if (direct_burst_ms > pool_burst_ms && pool_burst_ms > 0) {
        double improvement = static_cast<double>(direct_burst_ms) / pool_burst_ms;
        std::cout << "突发场景性能提升: " << std::fixed << std::setprecision(2) 
                  << improvement << "x\n";
    }
}

int main() {
    std::cout << "Object Pool 性能测试与使用示例\n";
    std::cout << "===============================\n";
    
    std::cout << "本示例演示对象池在高频交易系统中的优势:\n";
    std::cout << "1. 避免频繁的内存分配和释放\n";
    std::cout << "2. 减少内存碎片和分配器压力\n";
    std::cout << "3. 在突发负载下保持稳定性能\n";
    std::cout << "4. 降低GC压力 (如果使用GC语言)\n\n";
    
    simulate_hft_trading();
    performance_comparison();
    
    std::cout << "\n=== 总结与建议 ===\n";
    std::cout << "✅ 对象池适用场景:\n";
    std::cout << "  - 对象创建成本高 (复杂构造函数)\n";
    std::cout << "  - 高频率的分配/释放模式\n";
    std::cout << "  - 突发性负载\n";
    std::cout << "  - 多线程环境下的资源竞争\n\n";
    std::cout << "⚠️  注意事项:\n";
    std::cout << "  - 单线程+简单对象可能不会有性能提升\n";
    std::cout << "  - 需要合理设置池大小\n";
    std::cout << "  - 考虑内存占用 vs 性能的权衡\n";
    
    return 0;
}