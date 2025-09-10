 #pragma once

#include <vector>
#include <memory>
#include <mutex>
#include <functional>

template<typename T>
class ObjectPool {
public:
    using ObjectPtr = std::unique_ptr<T, std::function<void(T*)>>;
    
    explicit ObjectPool(size_t initial_size = 10) : pool_size_(initial_size) {
        pool_.reserve(initial_size);
        for (size_t i = 0; i < initial_size; ++i) {
            pool_.emplace_back(std::make_unique<T>());
        }
    }
    
    ObjectPtr acquire() {
        std::lock_guard<std::mutex> lock(mutex_);
        
        if (pool_.empty()) {
            // Create new object if pool is empty
            auto deleter = [this](T* ptr) {
                this->release(std::unique_ptr<T>(ptr));
            };
            return ObjectPtr(new T(), deleter);
        }
        
        auto obj = std::move(pool_.back());
        pool_.pop_back();
        
        auto deleter = [this](T* ptr) {
            this->release(std::unique_ptr<T>(ptr));
        };
        
        return ObjectPtr(obj.release(), deleter);
    }
    
    void release(std::unique_ptr<T> obj) {
        if (!obj) return;
        
        std::lock_guard<std::mutex> lock(mutex_);
        
        // Reset object state if needed (can be customized)
        reset_object(*obj);
        
        // Return to pool if not at capacity
        if (pool_.size() < pool_size_) {
            pool_.emplace_back(std::move(obj));
        }
        // Otherwise let it be destroyed naturally
    }
    
    size_t available() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return pool_.size();
    }
    
    size_t capacity() const {
        return pool_size_;
    }

private:
    void reset_object(T& obj) {
        // Default implementation - can be specialized for specific types
        // For POD types or types with reset methods
        if constexpr (std::is_arithmetic_v<T>) {
            obj = T{};
        }
    }
    
    std::vector<std::unique_ptr<T>> pool_;
    size_t pool_size_;
    mutable std::mutex mutex_;
};