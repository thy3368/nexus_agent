#pragma once

#include <vector>
#include <memory>
#include <atomic>
#include <functional>

// 高性能对象池 - 针对单线程优化版本
template<typename T>
class FastObjectPool {
private:
    std::vector<std::unique_ptr<T>> pool_;
    std::vector<T*> available_;
    size_t next_index_;
    size_t pool_size_;
    
public:
    explicit FastObjectPool(size_t initial_size = 100) 
        : next_index_(0), pool_size_(initial_size) {
        pool_.reserve(initial_size);
        available_.reserve(initial_size);
        
        // 预分配所有对象
        for (size_t i = 0; i < initial_size; ++i) {
            auto obj = std::make_unique<T>();
            available_.push_back(obj.get());
            pool_.push_back(std::move(obj));
        }
    }
    
    // 快速获取对象 - 无锁版本
    T* acquire_fast() {
        if (next_index_ > 0) {
            return available_[--next_index_];
        }
        
        // 池已耗尽，创建新对象
        auto obj = std::make_unique<T>();
        T* ptr = obj.get();
        pool_.push_back(std::move(obj));
        return ptr;
    }
    
    // 快速释放对象 - 无锁版本
    void release_fast(T* obj) {
        if (!obj) return;
        
        // 重置对象状态
        reset_object(*obj);
        
        // 如果池未满，则返回池中
        if (next_index_ < pool_size_) {
            available_[next_index_++] = obj;
        }
    }
    
    size_t available() const {
        return next_index_;
    }
    
    size_t capacity() const {
        return pool_size_;
    }

private:
    void reset_object(T& obj) {
        // 默认实现 - 可以被特化
        if constexpr (std::is_arithmetic_v<T>) {
            obj = T{};
        }
    }
};

// RAII包装器用于自动释放
template<typename T>
class PooledObject {
private:
    T* obj_;
    FastObjectPool<T>* pool_;
    
public:
    PooledObject(T* obj, FastObjectPool<T>* pool) : obj_(obj), pool_(pool) {}
    
    ~PooledObject() {
        if (obj_ && pool_) {
            pool_->release_fast(obj_);
        }
    }
    
    // 禁止拷贝
    PooledObject(const PooledObject&) = delete;
    PooledObject& operator=(const PooledObject&) = delete;
    
    // 允许移动
    PooledObject(PooledObject&& other) noexcept 
        : obj_(other.obj_), pool_(other.pool_) {
        other.obj_ = nullptr;
        other.pool_ = nullptr;
    }
    
    PooledObject& operator=(PooledObject&& other) noexcept {
        if (this != &other) {
            if (obj_ && pool_) {
                pool_->release_fast(obj_);
            }
            obj_ = other.obj_;
            pool_ = other.pool_;
            other.obj_ = nullptr;
            other.pool_ = nullptr;
        }
        return *this;
    }
    
    T* get() const { return obj_; }
    T& operator*() const { return *obj_; }
    T* operator->() const { return obj_; }
    
    explicit operator bool() const { return obj_ != nullptr; }
};

// 便捷的获取函数
template<typename T>
PooledObject<T> acquire_from_pool(FastObjectPool<T>& pool) {
    return PooledObject<T>(pool.acquire_fast(), &pool);
}

// 线程安全版本的对象池
template<typename T>
class ThreadSafeObjectPool {
private:
    std::vector<std::unique_ptr<T>> pool_;
    std::atomic<size_t> next_index_;
    size_t pool_size_;
    std::mutex mutex_;
    
public:
    explicit ThreadSafeObjectPool(size_t initial_size = 100) 
        : next_index_(initial_size), pool_size_(initial_size) {
        pool_.reserve(initial_size * 2); // 预留扩展空间
        
        // 预分配所有对象
        for (size_t i = 0; i < initial_size; ++i) {
            pool_.push_back(std::make_unique<T>());
        }
    }
    
    T* acquire() {
        size_t index = next_index_.fetch_sub(1, std::memory_order_relaxed);
        
        if (index > 0) {
            return pool_[index - 1].get();
        }
        
        // 需要创建新对象
        std::lock_guard<std::mutex> lock(mutex_);
        auto obj = std::make_unique<T>();
        T* ptr = obj.get();
        pool_.push_back(std::move(obj));
        return ptr;
    }
    
    void release(T* obj) {
        if (!obj) return;
        
        reset_object(*obj);
        
        size_t current = next_index_.load(std::memory_order_relaxed);
        if (current < pool_size_) {
            size_t expected = current;
            while (!next_index_.compare_exchange_weak(expected, current + 1, 
                                                     std::memory_order_relaxed)) {
                current = expected;
                if (current >= pool_size_) {
                    return; // 池已满
                }
            }
        }
    }
    
    size_t available() const {
        return next_index_.load(std::memory_order_relaxed);
    }
    
    size_t capacity() const {
        return pool_size_;
    }

private:
    void reset_object(T& obj) {
        if constexpr (std::is_arithmetic_v<T>) {
            obj = T{};
        }
    }
};