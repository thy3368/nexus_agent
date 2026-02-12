# Rust之从0-1低时延CEX：使用 CLI Agent 进行 Spot 交易下单的详细指南

## 概述

`trade_ops` 工具是 PromptLine 提供的一个强大的 Spot 交易操作工具，支持通过自然语言或命令参数进行下单、测试下单和取消订单等操作。它与本地的 Spot 交易服务集成，提供快速、安全的交易执行。

## 工具功能特性

- **支持多种订单类型**: 限价单、市价单、止损单、止盈单等
- **安全的测试模式**: 提供测试下单功能，不产生真实交易
- **订单管理**: 支持取消订单操作
- **实时验证**: 完整的参数验证和错误处理
- **与本地服务集成**: 默认连接到本地 Spot 交易服务

## 安装和配置

### 1. 确保 PromptLine 已安装

```bash
# 从源代码构建
cargo install --path .

# 验证安装
promptline --version
```

### 2. 启动本地 Spot 交易服务

在使用 `trade_ops` 工具之前，需要确保本地的 Spot 交易服务正在运行。具体启动方法取决于您的服务实现。

### 3. 配置验证

运行健康检查确保系统正常：

```bash
promptline doctor
```

## 使用方法

### 方式一：通过 Agent 模式自然语言交互

```bash
# 启动 Agent 模式
promptline agent "帮我下一个 BTCUSDT 的限价买入订单，价格 50000，数量 0.01"

# 或者使用聊天模式
promptline chat
```

在聊天模式中，您可以直接输入：
> "下一个 BTCUSDT 的市价买入订单，数量 0.01"
> "测试下单 BTCUSDT 限价卖出，价格 55000，数量 0.02"
> "取消订单 ID 12345"

### 方式二：通过直接命令调用

```bash
# 使用 JSON 参数调用
promptline agent '{
  "tool": "trade_ops",
  "command": "new_order",
  "symbol": "BTCUSDT",
  "side": "BUY",
  "order_type": "LIMIT",
  "quantity": 0.01,
  "price": 50000,
  "time_in_force": "GTC"
}'
```

## 详细命令说明

### 1. 下单命令 (`new_order`)

创建真实的 Spot 交易订单。

**参数说明：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `command` | string | 是 | 命令类型，固定为 `"new_order"` |
| `symbol` | string | 是 | 交易对，支持：`BTCUSDT`, `ETHUSDT`, `BTCETH` |
| `side` | string | 是 | 订单方向：`BUY`(买入) 或 `SELL`(卖出) |
| `order_type` | string | 是 | 订单类型（见下） |
| `quantity` | number | 是 | 订单数量 |
| `price` | number | 否 | 订单价格（限价单必填） |
| `time_in_force` | string | 否 | 有效时间（默认 GTC） |
| `new_client_order_id` | string | 否 | 自定义客户端订单ID |

**支持的订单类型：**

- `LIMIT`: 限价单（需要 `price` 参数）
- `MARKET`: 市价单（不需要 `price` 参数）
- `STOP_LOSS`: 止损单
- `STOP_LOSS_LIMIT`: 止损限价单（需要 `price` 参数）
- `TAKE_PROFIT`: 止盈单
- `TAKE_PROFIT_LIMIT`: 止盈限价单（需要 `price` 参数）
- `LIMIT_MAKER`: 限价挂单（需要 `price` 参数）

**支持的有效时间：**

- `GTC`: 一直有效（默认）
- `IOC`: 立即成交或取消
- `FOK`: 全部成交或取消

**示例：**

```bash
# 限价买入订单
promptline agent '{
  "tool": "trade_ops",
  "command": "new_order",
  "symbol": "BTCUSDT",
  "side": "BUY",
  "order_type": "LIMIT",
  "quantity": 0.01,
  "price": 50000,
  "time_in_force": "GTC"
}'

# 市价卖出订单
promptline agent '{
  "tool": "trade_ops",
  "command": "new_order",
  "symbol": "ETHUSDT",
  "side": "SELL",
  "order_type": "MARKET",
  "quantity": 0.1
}'
```

### 2. 测试下单命令 (`test_order`)

模拟下单操作，不产生真实交易，用于测试和验证。

**参数与 `new_order` 相同**

**示例：**

```bash
# 测试限价订单
promptline agent '{
  "tool": "trade_ops",
  "command": "test_order",
  "symbol": "BTCUSDT",
  "side": "BUY",
  "order_type": "LIMIT",
  "quantity": 0.01,
  "price": 50000,
  "time_in_force": "GTC"
}'
```

### 3. 取消订单命令 (`cancel_order`)

取消已提交的订单。

**参数说明：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `command` | string | 是 | 命令类型，固定为 `"cancel_order"` |
| `symbol` | string | 是 | 交易对 |
| `order_id` | integer | 否 | 订单ID（与 `orig_client_order_id` 二选一） |
| `orig_client_order_id` | string | 否 | 原始客户端订单ID（与 `order_id` 二选一） |
| `new_client_order_id` | string | 否 | 新的客户端订单ID |

**示例：**

```bash
# 通过订单ID取消
promptline agent '{
  "tool": "trade_ops",
  "command": "cancel_order",
  "symbol": "BTCUSDT",
  "order_id": 12345
}'

# 通过客户端订单ID取消
promptline agent '{
  "tool": "trade_ops",
  "command": "cancel_order",
  "symbol": "BTCUSDT",
  "orig_client_order_id": "my-order-123"
}'
```

## 响应格式

### 成功响应

```
✓ Task completed successfully
Iterations: 1
Tools used: trade_ops

Result:
下单成功: ... (详细响应信息)
```

### 错误响应

```
✗ Task failed
Iterations: 1
Tools used: trade_ops

Result:
下单失败: 无效的订单类型
```

## 使用建议和最佳实践

### 1. 先测试后交易

在进行真实交易之前，建议先使用 `test_order` 命令进行测试：

```bash
# 先测试
promptline agent '{
  "tool": "trade_ops",
  "command": "test_order",
  "symbol": "BTCUSDT",
  "side": "BUY",
  "order_type": "LIMIT",
  "quantity": 0.01,
  "price": 50000
}'

# 确认无误后再执行真实下单
promptline agent '{
  "tool": "trade_ops",
  "command": "new_order",
  "symbol": "BTCUSDT",
  "side": "BUY",
  "order_type": "LIMIT",
  "quantity": 0.01,
  "price": 50000
}'
```

### 2. 使用合适的订单类型

- **市价单**：快速成交，但价格不确定
- **限价单**：价格确定，但可能不会立即成交
- **止损单**：行情触发时自动下单
- **止盈单**：盈利目标触发时自动下单

### 3. 风险控制

- 控制单次交易数量，避免过大仓位
- 设置合理的止损和止盈价格
- 定期检查和调整订单状态

### 4. 自动化场景

```bash
# 脚本化下单
#!/bin/bash
promptline agent '{
  "tool": "trade_ops",
  "command": "new_order",
  "symbol": "ETHUSDT",
  "side": "BUY",
  "order_type": "LIMIT",
  "quantity": 0.1,
  "price": '"$(curl -s 'https://api.binance.com/api/v3/ticker/price?symbol=ETHUSDT' | jq -r '.price | tonumber - 10')"'
}'
```

## 常见问题

### 1. 连接失败

```
下单失败: Failed to connect to localhost:8080
```

**解决方法：**
- 确保 Spot 交易服务正在运行
- 检查服务是否监听正确的端口（默认 8080）
- 检查防火墙设置

### 2. 无效的交易对

```
下单失败: 不支持的交易对: ADAUSDT
```

**解决方法：**
- 确认使用的交易对在支持列表中：BTCUSDT, ETHUSDT, BTCETH

### 3. 参数验证失败

```
下单失败: 限价单需要 price 字段
```

**解决方法：**
- 检查订单类型是否与所需参数匹配
- 限价单、止损限价单、止盈限价单和限价挂单需要价格参数

### 4. 权限问题

```
Error: Permission denied for tool 'trade_ops'
```

**解决方法：**
- 检查配置文件 `~/.promptline/config.yaml` 中的工具权限设置
- 确保 `trade_ops` 工具的权限不是 `deny`

## 调试和日志

### 启用详细日志

```bash
RUST_LOG=info promptline agent "下单命令"
```

### 查看系统日志

```bash
# 查看服务日志（取决于您的服务实现）
journalctl -u spot-trade-service

# 或直接查看控制台输出
```

## 扩展功能

### 添加新的交易对

在 `src/tools/trade_ops.rs` 中添加：

```rust
// 在 execute_new_order 和 execute_test_order 方法中
let trading_pair = match symbol {
    "BTCUSDT" => TradingPair::BtcUsdt,
    "ETHUSDT" => TradingPair::EthUsdt,
    "BTCETH" => TradingPair::BtcEth,
    "ADAUSDT" => TradingPair::AdaUsdt, // 新增
    _ => return Ok(ToolResult::error(format!("不支持的交易对: {}", symbol))),
};
```

### 修改默认配置

在 `src/config.rs` 中修改默认配置参数。

## 总结

`trade_ops` 工具为 PromptLine 提供了强大的 Spot 交易能力，支持多种订单类型和安全的测试模式。通过 Agent 模式，您可以使用自然语言进行交易操作，大大简化了量化交易的开发和测试流程。

记住，在进行真实交易时始终要谨慎，合理控制风险！
