# OKX API 完整文档

> 更新时间: 2026-02-26
> 文档来源: https://www.okx.com/docs-v5/

---

## 目录

1. [概述](#1-概述)
2. [API 分类](#2-api-分类)
3. [认证方式](#3-认证方式)
4. [REST API 端点](#4-rest-api-端点)
5. [WebSocket API](#5-websocket-api)
6. [速率限制](#6-速率限制)
7. [API 地址](#7-api-地址)
8. [产品类型](#8-产品类型)

---

## 1. 概述

OKX（欧易）提供完整的 REST 和 WebSocket API 以满足您的交易需求。平台支持现货交易、衍生品、交割、期权以及杠杆交易。

**账户模式：**
- 现货模式 (Spot Mode)
- 合约模式 (Contract Mode)
- 跨币种保证金模式 (Cross-Margin Mode)
- 组合保证金模式 (Portfolio Margin Mode)

---

## 2. API 分类

| 分类 | 描述 | 前缀 |
|------|------|------|
| **交易账户 API** | 账户配置、余额、持仓、杠杆 | `/api/v5/account/*` |
| **撮合交易 API** | 下单、撤单、修改订单 | `/api/v5/trade/*` |
| **公共数据 API** | 行情、K线、深度 | `/api/v5/market/*` |
| **资金账户 API** | 充值、提现、转账 | `/api/v5/asset/*` |
| **子账户 API** | 子账户管理 | `/api/v5/account/subaccount/*` |
| **金融产品 API** | 理财、质押、借贷 | `/api/v5/finance/*` |
| **跟单交易 API** | 跟单交易 | `/api/v5/copytrading/*` |
| **网格交易 API** | 网格策略 | `/api/v5/tradingBot/grid/*` |
| **信号交易 API** | 信号策略 | `/api/v5/tradingBot/signal/*` |
| **定投 API** | 定投策略 | `/api/v5/tradingBot/recurring/*` |
| **大宗交易 API** | 大宗交易 | `/api/v5/sprd/*` |
| **RFQ API** | 报价请求 | `/api/v5/rfq/*` |
| **公共数据 API** | 平台数据 | `/api/v5/public/*` |
| **市场数据 API** | 大数据 | `/api/v5/rubik/*` |
| **法币 API** | 法币交易 | `/api/v5/fiat/*` |

---

## 3. 认证方式

### 3.1 REST API 认证

所有私有 REST 请求需要以下请求头：

```
OK-ACCESS-KEY: API密钥
OK-ACCESS-SIGN: HMAC SHA256 签名 (Base64编码)
OK-ACCESS-TIMESTAMP: 请求时间戳 (UTC)
OK-ACCESS-PASSPHRASE: API密钥密码
```

**签名生成算法：**

```
sign = Base64(HMAC-SHA256(timestamp + method + requestPath + body, secretKey))
```

示例：
```javascript
const timestamp = '' + Date.now() / 1000
const method = 'GET'
const requestPath = '/api/v5/account/balance'
const body = ''
const sign = CryptoJS.enc.Base64.stringify(CryptoJS.HmacSHA256(timestamp + method + requestPath + body, secretKey))
```

### 3.2 WebSocket 认证

```json
{
  "op": "login",
  "args": [{
    "apiKey": "YOUR_API_KEY",
    "passphrase": "YOUR_PASSPHRASE",
    "timestamp": "1538054050",
    "sign": "YOUR_SIGNATURE"
  }]
}
```

WebSocket 签名算法：
```javascript
const timestamp = '' + Math.floor(Date.now() / 1000)
const method = 'GET'
const requestPath = '/users/self/verify'
const sign = CryptoJS.enc.Base64.stringify(CryptoJS.HmacSHA256(timestamp + method + requestPath, secretKey))
```

---

## 4. REST API 端点

### 4.1 交易账户 API (`/api/v5/account/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/account/instruments` | 获取交易产品基础信息 |
| GET | `/api/v5/account/balance` | 查看账户余额 |
| GET | `/api/v5/account/positions` | 查看持仓信息 |
| GET | `/api/v5/account/positions-history` | 查看历史持仓 |
| GET | `/api/v5/account/account-position-risk` | 账户仓位风险 |
| GET | `/api/v5/account/bills` | 查看账单 (近7天) |
| GET | `/api/v5/account/bills-archive` | 查看账单 (近3月) |
| GET | `/api/v5/account/bills-history-archive` | 账单历史归档 |
| GET | `/api/v5/account/config` | 获取账户配置 |
| POST | `/api/v5/account/set-position-mode` | 设置持仓模式 |
| POST | `/api/v5/account/set-leverage` | 设置杠杆倍数 |
| GET | `/api/v5/account/leverage-info` | 杠杆信息 |
| GET | `/api/v5/account/adjust-leverage-info` | 可调杠杆信息 |
| GET | `/api/v5/account/max-loan` | 最大可借 |
| GET | `/api/v5/account/max-size` | 最大开仓数量 |
| GET | `/api/v5/account/max-avail-size` | 最大可用数量 |
| POST | `/api/v5/account/position/margin-balance` | 调整保证金 |
| GET | `/api/v5/account/trade-fee` | 交易手续费率 |
| GET | `/api/v5/account/interest-accrued` | 应计利息 |
| GET | `/api/v5/account/interest-rate` | 利率 |
| POST | `/api/v5/account/set-fee-type` | 设置手续费类型 |
| POST | `/api/v5/account/set-greeks` | 设置希腊值显示 |
| POST | `/api/v5/account/set-isolated-mode` | 设置逐仓模式 |
| GET | `/api/v5/account/max-withdrawal` | 最大可提数量 |
| GET | `/api/v5/account/risk-state` | 风险状态 |
| GET | `/api/v5/account/greeks` | 希腊值 |
| GET | `/api/v5/account/position-tiers` | 持仓档位 |
| POST | `/api/v5/account/activate-option` | 激活期权 |
| GET | `/api/v5/account/collateral-assets` | 抵押资产 |
| POST | `/api/v5/account/set-collateral-assets` | 设置抵押资产 |
| POST | `/api/v5/account/set-auto-loan` | 设置自动借币 |
| POST | `/api/v5/account/set-auto-repay` | 设置自动还币 |
| POST | `/api/v5/account/spot-manual-borrow-repay` | 手动借还币 |
| GET | `/api/v5/account/spot-borrow-repay-history` | 借还币历史 |
| GET | `/api/v5/account/interest-limits` | 借币限额 |
| POST | `/api/v5/account/account-level-switch-preset` | 预设账户等级切换 |
| POST | `/api/v5/account/move-positions` | 仓位迁移 |
| GET | `/api/v5/account/move-positions-history` | 仓位迁移历史 |

### 4.2 子账户 API (`/api/v5/account/subaccount/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/account/subaccount/balances` | 子账户余额 |
| GET | `/api/v5/account/subaccount/max-withdrawal` | 子账户最大可提 |
| POST | `/api/v5/account/subaccount/transfer` | 子账户转账 |

### 4.3 撮合交易 API (`/api/v5/trade/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/v5/trade/order` | 下单 |
| POST | `/api/v5/trade/batch-orders` | 批量下单 |
| POST | `/api/v5/trade/cancel-order` | 撤单 |
| POST | `/api/v5/trade/cancel-batch-orders` | 批量撤单 |
| POST | `/api/v5/trade/amend-order` | 修改订单 |
| POST | `/api/v5/trade/amend-batch-orders` | 批量修改订单 |
| GET | `/api/v5/trade/order` | 订单详情 |
| GET | `/api/v5/trade/orders-pending` | 未成交订单 |
| GET | `/api/v5/trade/orders-history` | 历史订单 (近7天) |
| GET | `/api/v5/trade/orders-history-archive` | 历史订单 (近3月) |
| GET | `/api/v5/trade/fills` | 成交明细 (近7天) |
| GET | `/api/v5/trade/fills-history` | 成交明细 (近3月) |
| POST | `/api/v5/trade/close-position` | 平仓 |
| POST | `/api/v5/trade/cancel-all-after` | 定时撤单 |
| POST | `/api/v5/trade/mass-cancel` | 批量撤单 |
| POST | `/api/v5/trade/easy-convert` | 一键转换 |
| GET | `/api/v5/trade/easy-convert-currency-list` | 一键转换币种列表 |
| GET | `/api/v5/trade/easy-convert-history` | 一键转换历史 |
| POST | `/api/v5/trade/one-click-repay` | 一键还币 |
| POST | `/api/v5/trade/one-click-repay-v2` | 一键还币V2 |
| GET | `/api/v5/trade/one-click-repay-currency-list` | 一键还币币种列表 |
| GET | `/api/v5/trade/one-click-repay-currency-list-v2` | 一键还币币种列表V2 |
| GET | `/api/v5/trade/one-click-repay-history` | 一键还币历史 |
| GET | `/api/v5/trade/one-click-repay-history-v2` | 一键还币历史V2 |
| GET | `/api/v5/trade/order-precheck` | 订单预检查 |
| GET | `/api/v5/trade/account-rate-limit` | 账户限速 |

### 4.4 算法订单 API (`/api/v5/trade/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/v5/trade/order-algo` | 下算法单 |
| POST | `/api/v5/trade/amend-algos` | 修改算法单 |
| POST | `/api/v5/trade/cancel-algos` | 取消算法单 |
| GET | `/api/v5/trade/orders-algo-pending` | 未完成算法单 |
| GET | `/api/v5/trade/orders-algo-history` | 算法单历史 |

### 4.5 公共数据 API (`/api/v5/market/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/market/ticker` | 行情 (单个产品) |
| GET | `/api/v5/market/tickers` | 行情 (所有产品) |
| GET | `/api/v5/market/candles` | K线数据 |
| GET | `/api/v5/market/history-candles` | 历史K线 |
| GET | `/api/v5/market/books` | 深度数据 |
| GET | `/api/v5/market/books-full` | 完整深度 |
| GET | `/api/v5/market/books-sbe` | 深度 (自选) |
| GET | `/api/v5/market/trades` | 近期成交 |
| GET | `/api/v5/market/history-trades` | 历史成交 |
| GET | `/api/v5/market/index-candles` | 指数K线 |
| GET | `/api/v5/market/history-index-candles` | 指数历史K线 |
| GET | `/api/v5/market/index-tickers` | 指数行情 |
| GET | `/api/v5/market/mark-price-candles` | 标记价格K线 |
| GET | `/api/v5/market/history-mark-price-candles` | 标记价格历史K线 |
| GET | `/api/v5/market/call-auction-details` | 集合竞价详情 |
| GET | `/api/v5/market/exchange-rate` | 汇率 |
| GET | `/api/v5/market/platform-24-volume` | 平台24h成交量 |
| GET | `/api/v5/market/block-ticker` | 大宗交易 ticker |
| GET | `/api/v5/market/block-tickers` | 大宗交易 tickers |
| GET | `/api/v5/market/sprd-ticker` | 价差 ticker |
| GET | `/api/v5/market/sprd-candles` | 价差K线 |
| GET | `/api/v5/market/sprd-history-candles` | 价差历史K线 |

### 4.6 公共接口 API (`/api/v5/public/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/public/instruments` | 产品信息 |
| GET | `/api/v5/public/delivery-exercise-history` | 交割/行权历史 |
| GET | `/api/v5/public/estimated-price` | 预估交割价 |
| GET | `/api/v5/public/estimated-settlement-info` | 预估结算信息 |
| GET | `/api/v5/public/funding-rate` | 资金费率 |
| GET | `/api/v5/public/funding-rate-history` | 资金费率历史 |
| GET | `/api/v5/public/price-limit` | 限价范围 |
| GET | `/api/v5/public/opt-summary` | 期权概况 |
| GET | `/api/v5/public/open-interest` | 持仓量 |
| GET | `/api/v5/public/underlying` | 标的资产 |
| GET | `/api/v5/public/settlement-history` | 结算历史 |
| GET | `/api/v5/public/insurance-fund` | 保险基金 |
| GET | `/api/v5/public/time` | 服务器时间 |
| GET | `/api/v5/public/mark-price` | 标记价格 |
| GET | `/api/v5/public/position-tiers` | 持仓档位 |
| GET | `/api/v5/public/instrument-tick-bands` | 限价档位 |
| GET | `/api/v5/public/discount-rate-interest-free-quota` | 折扣率 |
| GET | `/api/v5/public/interest-rate-loan-quota` | 借币利率 |
| GET | `/api/v5/public/economic-calendar` | 经济日历 |

### 4.7 资金账户 API (`/api/v5/asset/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/asset/balances` | 资产余额 |
| GET | `/api/v5/asset/bills` | 账单 (近30天) |
| GET | `/api/v5/asset/bills-history` | 账单历史 |
| GET | `/api/v5/asset/currencies` | 币种信息 |
| GET | `/api/v5/asset/deposit-address` | 充值地址 |
| GET | `/api/v5/asset/deposit-history` | 充值历史 |
| POST | `/api/v5/asset/withdrawal` | 提币 |
| GET | `/api/v5/asset/withdrawal-history` | 提币历史 |
| POST | `/api/v5/asset/transfer` | 转账 |
| GET | `/api/v5/asset/transfer-state` | 转账状态 |
| GET | `/api/v5/asset/deposit-withdraw-status` | 充值提现状态 |
| POST | `/api/v5/asset/cancel-withdrawal` | 取消提币 |
| GET | `/api/v5/asset/monthly-statement` | 月度账单 |
| GET | `/api/v5/asset/non-tradable-assets` | 不可交易资产 |
| GET | `/api/v5/asset/asset-valuation` | 资产估值 |
| GET | `/api/v5/asset/exchange-list` | 交易平台列表 |

### 4.8 闪兑 API (`/api/v5/asset/convert/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/asset/convert/currencies` | 闪兑币种 |
| GET | `/api/v5/asset/convert/currency-pair` | 闪兑交易对 |
| GET | `/api/v5/asset/convert/estimate-quote` | 闪兑预估报价 |
| POST | `/api/v5/asset/convert/trade` | 闪兑交易 |
| GET | `/api/v5/asset/convert/history` | 闪兑历史 |

### 4.9 子账户资金 API (`/api/v5/asset/subaccount/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/asset/subaccount/balances` | 子账户余额 |
| GET | `/api/v5/asset/subaccount/bills` | 子账户账单 |
| POST | `/api/v5/asset/subaccount/transfer` | 子账户转账 |
| GET | `/api/v5/asset/subaccount/managed-subaccount-bills` | 托管子账户账单 |

### 4.10 跟单交易 API (`/api/v5/copytrading/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/copytrading/config` | 跟单配置 |
| GET | `/api/v5/copytrading/copy-settings` | 跟单设置 |
| POST | `/api/v5/copytrading/copy-settings` | 设置跟单 |
| POST | `/api/v5/copytrading/amend-copy-settings` | 修改跟单设置 |
| POST | `/api/v5/copytrading/first-copy-settings` | 首次跟单 |
| POST | `/api/v5/copytrading/stop-copy-trading` | 停止跟单 |
| POST | `/api/v5/copytrading/amend-profit-sharing-ratio` | 修改分润比例 |
| POST | `/api/v5/copytrading/close-subposition` | 关闭跟单仓位 |
| GET | `/api/v5/copytrading/current-subpositions` | 当前跟单仓位 |
| GET | `/api/v5/copytrading/subpositions-history` | 跟单仓位历史 |
| GET | `/api/v5/copytrading/profit-sharing-details` | 分润明细 |
| GET | `/api/v5/copytrading/total-profit-sharing` | 总分润 |
| GET | `/api/v5/copytrading/total-unrealized-profit-sharing` | 总未实现分润 |
| GET | `/api/v5/copytrading/unrealized-profit-sharing-details` | 未实现分润明细 |
| GET | `/api/v5/copytrading/public-config` | 公共配置 |
| GET | `/api/v5/copytrading/public-copy-traders` | 公共交易员 |
| GET | `/api/v5/copytrading/public-lead-traders` | 公共带单员 |
| GET | `/api/v5/copytrading/public-stats` | 交易员统计 |
| GET | `/api/v5/copytrading/public-pnl` | 交易员收益 |
| GET | `/api/v5/copytrading/public-weekly-pnl` | 交易员周收益 |
| GET | `/api/v5/copytrading/public-current-subpositions` | 公共跟单仓位 |
| GET | `/api/v5/copytrading/public-subpositions-history` | 公共跟单仓位历史 |
| GET | `/api/v5/copytrading/public-preference-currency` | 交易员偏好币种 |

### 4.11 网格交易 API (`/api/v5/tradingBot/grid/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/v5/tradingBot/grid/order-algo` | 网格下单 |
| POST | `/api/v5/tradingBot/grid/amend-order-algo` | 修改网格 |
| POST | `/api/v5/tradingBot/grid/stop-order-algo` | 停止网格 |
| GET | `/api/v5/tradingBot/grid/orders-algo-pending` | 网格挂单 |
| GET | `/api/v5/tradingBot/grid/orders-algo-history` | 网格历史 |
| GET | `/api/v5/tradingBot/grid/orders-algo-details` | 网格详情 |
| GET | `/api/v5/tradingBot/grid/positions` | 网格持仓 |
| GET | `/api/v5/tradingBot/grid/sub-orders` | 网格子单 |
| POST | `/api/v5/tradingBot/grid/close-position` | 网格平仓 |
| POST | `/api/v5/tradingBot/grid/compute-margin-balance` | 计算保证金 |
| POST | `/api/v5/tradingBot/grid/margin-balance` | 调整保证金 |
| GET | `/api/v5/tradingBot/grid/ai-param` | AI参数 |
| GET | `/api/v5/tradingBot/grid/min-investment` | 最小投资 |
| GET | `/api/v5/tradingBot/grid/grid-quantity` | 网格数量 |
| POST | `/api/v5/tradingBot/grid/adjust-investment` | 调整投资 |
| POST | `/api/v5/tradingBot/grid/cancel-close-order` | 取消平仓单 |
| POST | `/api/v5/tradingBot/grid/order-instant-trigger` | 立即触发 |
| GET | `/api/v5/tradingBot/grid/withdraw-income` | 提取收益 |

### 4.12 信号交易 API (`/api/v5/tradingBot/signal/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/v5/tradingBot/signal/create-signal` | 创建信号策略 |
| POST | `/api/v5/tradingBot/signal/amendTPSL` | 修改止盈止损 |
| POST | `/api/v5/tradingBot/signal/cancel-sub-order` | 取消子单 |
| POST | `/api/v5/tradingBot/signal/close-position` | 平仓 |
| GET | `/api/v5/tradingBot/signal/orders-algo-pending` | 活跃信号策略 |
| GET | `/api/v5/tradingBot/signal/orders-algo-history` | 信号策略历史 |
| GET | `/api/v5/tradingBot/signal/orders-algo-details` | 信号策略详情 |
| GET | `/api/v5/tradingBot/signal/positions` | 信号策略持仓 |
| GET | `/api/v5/tradingBot/signal/sub-orders` | 信号子单 |
| GET | `/api/v5/tradingBot/signal/event-history` | 事件历史 |

### 4.13 定投 API (`/api/v5/tradingBot/recurring/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/v5/tradingBot/recurring/order-algo` | 创建定投 |
| POST | `/api/v5/tradingBot/recurring/amend-order-algo` | 修改定投 |
| POST | `/api/v5/tradingBot/recurring/stop-order-algo` | 停止定投 |
| GET | `/api/v5/tradingBot/recurring/orders-algo-pending` | 活跃定投 |
| GET | `/api/v5/tradingBot/recurring/orders-algo-history` | 定投历史 |
| GET | `/api/v5/tradingBot/recurring/orders-algo-details` | 定投详情 |
| GET | `/api/v5/tradingBot/recurring/sub-orders` | 定投子单 |

### 4.14 大宗交易 API (`/api/v5/sprd/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/v5/sprd/order` | 大宗下单 |
| POST | `/api/v5/sprd/amend-order` | 修改大宗订单 |
| POST | `/api/v5/sprd/cancel-order` | 取消大宗订单 |
| GET | `/api/v5/sprd/orders-pending` | 大宗挂单 |
| GET | `/api/v5/sprd/orders-history` | 大宗历史 |
| GET | `/api/v5/sprd/orders-history-archive` | 大宗历史归档 |
| POST | `/api/v5/sprd/mass-cancel` | 批量取消 |
| POST | `/api/v5/sprd/cancel-all-after` | 定时取消 |
| GET | `/api/v5/sprd/spreads` | 价差 |
| GET | `/api/v5/sprd/books` | 深度 |
| GET | `/api/v5/sprd/trades` | 成交 |
| GET | `/api/v5/sprd/public-trades` | 公共成交 |

### 4.15 RFQ API (`/api/v5/rfq/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/v5/rfq/create-rfq` | 创建询价 |
| POST | `/api/v5/rfq/cancel-rfq` | 取消询价 |
| POST | `/api/v5/rfq/cancel-batch-rfqs` | 批量取消询价 |
| POST | `/api/v5/rfq/cancel-all-rfqs` | 取消全部询价 |
| POST | `/api/v5/rfq/create-quote` | 创建报价 |
| POST | `/api/v5/rfq/cancel-quote` | 取消报价 |
| POST | `/api/v5/rfq/cancel-batch-quotes` | 批量取消报价 |
| POST | `/api/v5/rfq/cancel-all-quotes` | 取消全部报价 |
| POST | `/api/v5/rfq/execute-quote` | 执行报价 |
| GET | `/api/v5/rfq/rfqs` | 询价列表 |
| GET | `/api/v5/rfq/quotes` | 报价列表 |
| GET | `/api/v5/rfq/trades` | 成交列表 |
| GET | `/api/v5/rfq/counterparties` | 交易对手 |
| POST | `/api/v5/rfq/counterparties` | 设置交易对手 |
| GET | `/api/v5/rfq/mmp-config` | MMP配置 |
| POST | `/api/v5/rfq/mmp-reset` | 重置MMP |
| POST | `/api/v5/rfq/maker-instrument-settings` | 设置报价币对 |
| GET | `/api/v5/rfq/public-trades` | 公共成交 |

### 4.16 金融产品 API (`/api/v5/finance/*`)

#### 4.16.1 活期理财 (`/api/v5/finance/savings/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/finance/savings/balance` | 活期余额 |
| POST | `/api/v5/finance/savings/purchase-redempt` | 申购/赎回 |
| GET | `/api/v5/finance/savings/lending-history` | 借出历史 |
| GET | `/api/v5/finance/savings/lending-rate` | 借出利率 |
| GET | `/api/v5/finance/savings/lending-rate-history` | 借出利率历史 |

#### 4.16.2 灵活借贷 (`/api/v5/finance/flexible-loan/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/finance/flexible-loan/loan-info` | 贷款信息 |
| GET | `/api/v5/finance/flexible-loan/borrow-currencies` | 可借币种 |
| GET | `/api/v5/finance/flexible-loan/collateral-assets` | 抵押资产 |
| GET | `/api/v5/finance/flexible-loan/max-loan` | 最大可借 |
| GET | `/api/v5/finance/flexible-loan/max-collateral-redeem-amount` | 最大可赎回 |
| GET | `/api/v5/finance/flexible-loan/interest-accrued` | 应计利息 |
| GET | `/api/v5/finance/flexible-loan/loan-history` | 贷款历史 |
| POST | `/api/v5/finance/flexible-loan/adjust-collateral` | 调整抵押品 |

#### 4.16.3 Staking (`/api/v5/finance/staking-defi/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/finance/staking-defi/offers` | Staking 产品 |
| GET | `/api/v5/finance/staking-defi/eth/product-info` | ETH 产品信息 |
| POST | `/api/v5/finance/staking-defi/eth/purchase` | ETH 质押 |
| POST | `/api/v5/finance/staking-defi/eth/redeem` | ETH 赎回 |
| POST | `/api/v5/finance/staking-defi/eth/cancel-redeem` | 取消赎回 |
| GET | `/api/v5/finance/staking-defi/eth/balance` | ETH 余额 |
| GET | `/api/v5/finance/staking-defi/eth/apy-history` | ETH APY 历史 |
| GET | `/api/v5/finance/staking-defi/eth/purchase-redeem-history` | ETH 申购赎回历史 |
| GET | `/api/v5/finance/staking-defi/sol/product-info` | SOL 产品信息 |
| POST | `/api/v5/finance/staking-defi/sol/purchase` | SOL 质押 |
| POST | `/api/v5/finance/staking-defi/sol/redeem` | SOL 赎回 |
| GET | `/api/v5/finance/staking-defi/sol/balance` | SOL 余额 |
| GET | `/api/v5/finance/staking-defi/sol/apy-history` | SOL APY 历史 |
| GET | `/api/v5/finance/staking-defi/sol/purchase-redeem-history` | SOL 申购赎回历史 |
| POST | `/api/v5/finance/staking-defi/purchase` | 通用质押 |
| POST | `/api/v5/finance/staking-defi/redeem` | 通用赎回 |
| POST | `/api/v5/finance/staking-defi/cancel` | 取消质押 |
| GET | `/api/v5/finance/staking-defi/orders-active` | 活跃订单 |
| GET | `/api/v5/finance/staking-defi/orders-history` | 历史订单 |

### 4.17 市场大数据 API (`/api/v5/rubik/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/rubik/stat/taker-volume` | Taker 成交量 |
| GET | `/api/v5/rubik/stat/taker-volume-contract` | 合约 Taker 成交量 |
| GET | `/api/v5/rubik/stat/contracts/long-short-account-ratio` | 多空账户比 |
| GET | `/api/v5/rubik/stat/contracts/long-short-account-ratio-contract` | 合约多空账户比 |
| GET | `/api/v5/rubik/stat/contracts/long-short-position-ratio-contract-top-trader` | 顶部交易员多空比 |
| GET | `/api/v5/rubik/stat/contracts/long-short-account-ratio-contract-top-trader` | 顶部交易员多空账户比 |
| GET | `/api/vubik/stat/contracts/open-interest-history` | 持仓量历史 |
| GET | `/api/v5/rubik/stat/contracts/open-interest-volume` | 持仓量/成交量 |
| GET | `/api/v5/rubik/stat/margin/loan-ratio` | 借贷率 |
| GET | `/api/v5/rubik/stat/option/open-interest-volume` | 期权持仓量 |
| GET | `/api/v5/rubik/stat/option/open-interest-volume-expiry` | 期权持仓量到期分布 |
| GET | `/api/v5/rubik/stat/option/open-interest-volume-strike` | 期权持仓量行权价分布 |
| GET | `/api/v5/rubik/stat/option/open-interest-volume-ratio` | 期权持仓量比率 |
| GET | `/api/v5/rubik/stat/option/taker-block-volume` | 期权 Taker 大额成交量 |
| GET | `/api/v5/rubik/stat/trading-data/support-coin` | 币种支持 |

### 4.18 法币 API (`/api/v5/fiat/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/fiat/buy-sell/currencies` | 法币币种 |
| GET | `/api/v5/fiat/buy-sell/currency-pair` | 法币交易对 |
| POST | `/api/v5/fiat/buy-sell/quote` | 询价 |
| POST | `/api/v5/fiat/buy-sell/trade` | 成交 |
| GET | `/api/v5/fiat/buy-sell/history` | 历史记录 |

### 4.19 系统 API (`/api/v5/*`)

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v5/system/status` | 系统状态 |
| GET | `/api/v5/system/time` | 服务器时间 |
| GET | `/api/v5/support/announcements` | 公告 |
| GET | `/api/v5/support/announcement-types` | 公告类型 |

---

## 5. WebSocket API

### 5.1 连接地址

| 环境 | 地址 |
|------|------|
| 实盘公共 | `wss://ws.okx.com:8443/ws/v5/public` |
| 实盘私有 | `wss://ws.okx.com:8443/ws/v5/private` |
| 实盘业务 | `wss://ws.okx.com:8443/ws/v5/business` |
| 模拟盘公共 | `wss://wspap.okx.com:8443/ws/v5/public` |
| 模拟盘私有 | `wss://wspap.okx.com:8443/ws/v5/private` |
| 模拟盘业务 | `wss://wspap.okx.com:8443/ws/v5/business` |

### 5.2 公共频道

| 频道 | 描述 |
|------|------|
| `tickers` | 行情 |
| `candles` | K线 |
| `books` | 深度 |
| `books-l2` | 深度 (L2) |
| `books5` | 深度 (5档) |
| `trades` | 成交 |
| `funding-rate` | 资金费率 |
| `price-limit` | 限价范围 |
| `mark-price` | 标记价格 |
| `mark-price-candles` | 标记价格K线 |
| `open-interest` | 持仓量 |
| `index-tickers` | 指数行情 |
| `index-candles` | 指数K线 |
| `index-depth` | 指数深度 |
| `estimated-price` | 预估价格 |
| `option/instrument-family-trades` | 期权成交 |

### 5.3 私有频道

| 频道 | 描述 |
|------|------|
| `account` | 账户 |
| `positions` | 持仓 |
| `orders` | 订单 |
| `orders-algo` | 算法订单 |
| `position-algo` | 算法持仓 |
| `account-greeks` | 希腊值 |
| `liquidity-info` | 流动性信息 |

### 5.4 业务频道

| 频道 | 描述 |
|------|------|
| `grid-orders` | 网格订单 |
| `grid-positions` | 网格持仓 |
| `grid-sub-orders` | 网格子单 |
| `signal-orders` | 信号订单 |
| `signal-positions` | 信号持仓 |
| `recurring-orders` | 定投订单 |
| `copytrading-subpositions` | 跟单仓位 |

---

## 6. 速率限制

### 6.1 通用限速

| 类型 | 限速规则 |
|------|----------|
| WebSocket 登录/订阅 | 基于连接 |
| 公共 REST (未认证) | 基于 IP |
| 私有 REST | 基于 User ID |
| WebSocket 订单管理 | 基于 User ID |

### 6.2 订单限速

- 子账户每2秒最多 1000 个订单请求
- 每个交易产品最多 500 个未成交订单
- 最多 4,000 个未成交订单总数

### 6.3 VIP 限速 (VIP5+)

根据成交比率，限速可达 10,000 请求/2秒

---

## 7. API 地址

| 环境 | REST | WebSocket |
|------|------|-----------|
| 实盘 | `https://www.okx.com` | `wss://ws.okx.com:8443` |
| 模拟盘 | `https://www.okx.com` | `wss://wspap.okx.com:8443` |

**注意**: 模拟盘请求需要添加 header: `x-simulated-trading: 1`

---

## 8. 产品类型

| 代码 | 名称 |
|------|------|
| `SPOT` | 币币 |
| `MARGIN` | 币币杠杆 |
| `SWAP` | 永续合约 |
| `FUTURES` | 交割合约 |
| `OPTION` | 期权 |

---

## 相关链接

- [OKX API 官方文档](https://www.okx.com/docs-v5/)
- [OKX API 英文文档](https://www.okx.com/docs-v5/en/)
- [Python SDK](https://www.okx.com/docs-v5/en/#python-sdk)
- [做市商代码示例](https://www.okx.com/docs-v5/en/#market-maker)

---

*本文档由 AI 自动生成，内容来自 OKX 官方 API 文档。*
