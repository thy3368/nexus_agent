---
allowed-tools: all
description: 集成测试驱动开发命令 - 测试优先开发

---

# 集成测试驱动开发命令

**基于2步核心流程的集成测试驱动开发**

## 开发流程

**两步核心流程：**

1. **生成失败测试** - 生成测试代码并验证失败状态（TDD红阶段）
2. **实现通过代码** - 实现让测试通过的最小业务代码（TDD绿阶段）





## 集成测试驱动场景

集成测试优先，避免过度细粒度的单元测试。重点关注：
- **端到端业务流程**：完整用户场景的验证
- **系统边界集成**：外部服务、数据库、API的真实交互
- **关键业务逻辑**：核心算法和决策逻辑的综合验证
- **错误恢复机制**：系统级异常处理和恢复能力

## 详细工作流程

### 第1步：`/dev gen <特性>` - 生成失败测试
**根据特性描述生成集成测试代码并验证其失败状态（TDD红阶段）。**

**功能说明：**
- 分析特性需求，识别核心业务流程
- 生成可运行的集成测试代码
- 自动运行测试验证失败状态
- 输出失败测试的详细信息

**输入要求：**
- 提供清晰的特性描述，包含具体业务场景
- 关注端到端用户体验，而非技术实现细节
- 重点关注正向业务流程，确保主要用例的完整性

**集成测试代码模板：**
```rust
#[tokio::test]
async fn test_multi_exchange_realtime_data_flow() -> Result<()> {
    // 业务规格：多交易所实时数据获取
    // - 同时连接Binance、Bitget、OKX三个交易所
    // - 订阅BTC/USDT订单簿数据流
    // - 验证数据接收完整性和格式一致性
    // - 确保连接成功率>99%, 数据延迟<10ms
    
    println!("🚀 开始多交易所实时数据流测试");
    
    // 配置多交易所连接器（使用真实API签名）
    let config = ExchangeConfig::new()
        .add_exchange("binance", BinanceConfig::testnet())
        .add_exchange("bitget", BitgetConfig::testnet())  
        .add_exchange("okx", OkxConfig::testnet());
    
    println!("📋 配置信息: {:#?}", config);
    
    let collector = MultiExchangeCollector::new(config);
    
    // 建立WebSocket连接
    println!("🔌 正在连接交易所...");
    let start_time = std::time::Instant::now();
    let connections = collector.connect_all().await?;
    let connection_time = start_time.elapsed();
    
    println!("✅ 连接成功: {} 个交易所, 耗时: {:?}", connections.len(), connection_time);
    for (i, conn) in connections.iter().enumerate() {
        println!("  [{}.] {}: {} (延迟: {:?})", 
                i+1, conn.exchange, conn.status, conn.latency);
    }
    assert_eq!(connections.len(), 3, "所有交易所连接成功");
    
    // 订阅市场数据
    println!("📊 订阅 BTC/USDT 市场数据...");
    let stream = collector
        .subscribe_market_data("BTC/USDT", DataType::OrderBook)
        .await?;
    
    // 验证数据接收
    println!("📥 收集市场数据 (100条)...");
    let data_start = std::time::Instant::now();
    let data = stream.take(100).collect::<Vec<_>>().await;
    let data_time = data_start.elapsed();
    
    println!("🎯 数据收集完成: {} 条数据, 耗时: {:?}", data.len(), data_time);
    assert!(!data.is_empty(), "成功接收市场数据");
    
    // 打印前3条和后3条数据样本
    println!("📈 数据样本 (前3条):");
    for (i, item) in data.iter().take(3).enumerate() {
        println!("  [{}] {}: price={:.6}, volume={:.2}, timestamp={}", 
                i+1, item.exchange, item.price, item.volume, item.timestamp);
    }
    
    if data.len() > 6 {
        println!("📈 数据样本 (后3条):");
        for (i, item) in data.iter().skip(data.len()-3).enumerate() {
            println!("  [{}] {}: price={:.6}, volume={:.2}, timestamp={}", 
                    data.len()-3+i+1, item.exchange, item.price, item.volume, item.timestamp);
        }
    }
    
    // 统计各交易所数据分布
    let mut exchange_stats: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for item in &data {
        *exchange_stats.entry(item.exchange.clone()).or_insert(0) += 1;
    }
    println!("📊 交易所数据分布:");
    for (exchange, count) in &exchange_stats {
        println!("  {}: {} 条 ({:.1}%)", 
                exchange, count, (*count as f32 / data.len() as f32) * 100.0);
    }
    
    // 验证数据质量
    println!("🔍 验证数据质量...");
    let mut valid_count = 0;
    let mut price_range = (f64::MAX, f64::MIN);
    
    for (i, item) in data.iter().enumerate() {
        if item.price <= 0.0 {
            println!("❌ [{}] 无效价格: {}", i, item.price);
        } else {
            valid_count += 1;
            price_range.0 = price_range.0.min(item.price);
            price_range.1 = price_range.1.max(item.price);
        }
        
        if !["binance", "bitget", "okx"].contains(&item.exchange.as_str()) {
            println!("❌ [{}] 无效交易所标识: {}", i, item.exchange);
        }
        
        assert!(item.price > 0.0, "价格有效");
        assert!(["binance", "bitget", "okx"].contains(&item.exchange.as_str()), "交易所标识正确");
    }
    
    println!("✅ 数据质量检查: {}/{} 条有效", valid_count, data.len());
    println!("💰 价格区间: {:.6} - {:.6}", price_range.0, price_range.1);
    
    // 验证数据标准化
    println!("🔄 执行数据标准化...");
    let norm_start = std::time::Instant::now();
    let normalized = collector.normalize_data(&data)?;
    let norm_time = norm_start.elapsed();
    
    println!("✅ 标准化完成: 格式={:?}, 耗时={:?}", normalized.format, norm_time);
    println!("📋 标准化后数据统计:");
    println!("  - 字段数量: {}", normalized.fields.len());
    println!("  - 数据大小: {} bytes", normalized.data.len());
    println!("  - 压缩率: {:.1}%", 
            (1.0 - normalized.data.len() as f32 / (data.len() * 128) as f32) * 100.0);
    
    assert_eq!(normalized.format, DataFormat::Standard, "格式标准化成功");
    
    println!("🎉 测试完成: 多交易所实时数据流验证通过");
    Ok(())
}
```

**使用示例：**
```bash
# 第1步：生成失败测试
/dev gen 从币安、Bitget、OKX实时获取代币委托及成交数据
# 输出：生成测试代码，运行测试，显示失败信息

# 第2步：实现业务代码
/dev imp # 实现让所有失败测试通过的代码
# 或指定特定测试：/dev imp test_multi_exchange_realtime_data_flow
```



### 第2步：`/dev imp [测试名]` - 实现通过代码
**基于失败测试实现最小业务代码，并验证测试通过（TDD绿阶段）。**

**前提条件：**
- 存在运行失败的测试（来自第1步）

**功能说明：**
- 不指定测试名：实现所有失败测试的代码
- 指定测试名：只实现特定失败测试的代码
- 采用最小实现策略，确保测试通过
- 自动运行测试验证通过状态
- 提供后续重构建议

**实现原则：**
- 最小实现原则，避免过度设计
- 测试驱动的接口设计
- 关注业务逻辑，而非技术细节


## 命令总结

| 命令 | 功能 | 输入 | 输出 |
|------|------|------|------|
| `/dev gen <特性>` | 生成测试代码并验证失败状态 | 特性描述 | 失败测试代码+失败信息 |
| `/dev imp [测试名]` | 实现代码并验证测试通过 | 可选测试名 | 通过测试的最小代码 |

---

**核心理念**：测试先行 → 代码实现，确保每一步都有明确的验证标准。