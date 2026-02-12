# PromptLine Agent - 通用 AI Agent 框架需求

## 核心目标
构建一个通用、可扩展的 AI Agent 框架，支持多模型、多渠道、事件驱动、分布式部署。参考 OpenClaw 架构，实现本地优先的个人 AI 助手平台。

---

## Phase 1: 核心架构（必需）

### 1. 事件驱动系统
- **事件类型**：
  - `llm_command` → `llm_result`
  - `tool_command` → `tool_result`
  - `state_change` (global/session/local)
  - `agent_lifecycle` (start/pause/resume/end)
  - `permission_request` / `permission_granted`

- **事件总线**：支持事件发布/订阅、事件重放、审计日志
- **持久化**：事件存储到文件/数据库，支持事件溯源

### 2. 分布式网关架构（参考 OpenClaw）
```
Gateway (WebSocket 控制平面)
  ├── Session Manager
  ├── Channel Router (CLI/HTTP/WebSocket/Slack/Discord...)
  ├── Tool Registry
  ├── Event Bus
  └── State Store

Agent Runtime (RPC 模式)
  ├── ReACT Loop
  ├── Tool Executor
  ├── Stream Handler
  └── Permission Manager
```

### 3. 会话管理系统
- **Session 结构**：
  ```
  Session {
    id: String,
    user_id: String,
    channel: String,           // cli/http/slack/discord/...
    context: ContextState,
    history: Vec<Message>,
    tools: Vec<ToolRef>,
    permissions: PermissionSet,
    created_at: DateTime,
    expires_at: DateTime,
    metadata: HashMap<String, Value>,
  }
  ```
- 会话隔离和生命周期管理
- 会话持久化和恢复

### 4. 状态管理系统
- **三层状态**：
  - `global`: 全局共享状态
  - `session`: 会话级状态
  - `local`: 临时/本地状态

- **存储后端**：
  - 文件系统（开发/本地）
  - SQLite（单机）
  - PostgreSQL（分布式）
  - Redis（缓存/会话）

- **状态变更通知**：通过事件总线发布

### 5. 多模型支持
- **已支持**：OpenAI, Ollama, Gemini
- **新增**：Kimi, DeepSeek, Claude API, 本地模型
- **模型能力声明**：
  ```
  ModelCapabilities {
    supports_vision: bool,
    supports_function_calling: bool,
    supports_streaming: bool,
    max_tokens: usize,
    context_window: usize,
  }
  ```
- **模型路由**：根据任务类型选择最优模型
- **负载均衡**：多实例模型的请求分发

---

## Phase 2: 多渠道集成（扩展）

### 6. 多交互接口
- **CLI**（已有）：增强 REPL 和命令行体验
- **HTTP REST API**：标准 RESTful 接口
- **WebSocket**：实时双向通信
- **聊天平台**：
  - Slack Bot
  - Discord Bot
  - Telegram Bot
  - WeChat（企业号/公众号）
  - 钉钉

- **语音接口**（可选）：
  - 语音输入（STT）
  - 语音输出（TTS）
  - 集成 ElevenLabs

### 7. 工具流式处理
- **流式工具结果**：支持长时间运行任务的增量返回
- **进度报告**：工具执行进度实时反馈
- **取消机制**：支持中断长时间运行的工具
- **超时控制**：工具执行超时管理

### 8. 插件系统
- **MCP 服务器集成**：支持标准 MCP 协议
- **Skill 动态加载**：运行时加载/卸载 Skill
- **自定义工具注册**：用户定义工具接口
- **工具市场**：工具发现和共享机制

---

## Phase 3: 安全与可视化（增强）

### 9. 安全隔离
- **会话级沙箱**：
  - Docker 容器隔离（group/channel sessions）
  - 资源限制（CPU/内存/时间）
  - 文件系统隔离

- **工具权限细粒度控制**：
  - 按工具、按会话、按用户的权限
  - 权限继承和组合
  - 动态权限调整

- **审计日志**：
  - 所有操作记录
  - 敏感操作告警
  - 合规性报告

### 10. 可视化支持
- **Live Canvas**（类似 OpenClaw A2UI）：
  - Agent 驱动的 UI 生成
  - 实时状态展示
  - 交互式控制面板

- **Web Dashboard**：
  - 会话管理界面
  - 工具监控
  - 事件查看器
  - 性能指标

---

## 技术栈

### 后端
- **框架**：Tokio (async runtime)
- **Web**：Axum (HTTP) + Tokio-tungstenite (WebSocket)
- **数据库**：SQLx (PostgreSQL/SQLite)
- **消息队列**：Kafka (可选，分布式)
- **缓存**：Redis (可选)
- **序列化**：Serde (JSON/MessagePack)

### 前端（可视化）
- **Web UI**：React/Vue + TypeScript
- **实时通信**：WebSocket
- **状态管理**：Redux/Pinia

### 部署
- **容器化**：Docker + Docker Compose
- **编排**：Kubernetes (可选)
- **配置管理**：YAML + 环境变量

---

## 配置示例

```yaml
# ~/.promptline/config.yaml

gateway:
  host: "0.0.0.0"
  port: 8080
  websocket_port: 8081

models:
  default: "gpt-4"
  providers:
    openai:
      api_key: "${OPENAI_API_KEY}"
      models: ["gpt-4", "gpt-3.5-turbo"]
    kimi:
      api_key: "${KIMI_API_KEY}"
      models: ["kimi-9b-exp"]
    deepseek:
      api_key: "${DEEPSEEK_API_KEY}"
      models: ["deepseek-chat"]
    ollama:
      base_url: "http://localhost:11434"
      models: ["llama2", "mistral"]

channels:
  cli:
    enabled: true
  http:
    enabled: true
    port: 8080
  websocket:
    enabled: true
    port: 8081
  slack:
    enabled: false
    bot_token: "${SLACK_BOT_TOKEN}"
  discord:
    enabled: false
    bot_token: "${DISCORD_BOT_TOKEN}"

state:
  backend: "sqlite"  # sqlite/postgres/redis
  sqlite_path: "~/.promptline/state.db"
  postgres_url: "${DATABASE_URL}"
  redis_url: "${REDIS_URL}"

tools:
  file_read: "allow"
  file_write: "ask"
  file_delete: "deny"
  shell_execute: "ask"
  git_status: "ask"
  git_commit: "ask"
  web_get: "ask"
  codebase_search: "ask"

security:
  require_approval: true
  sandbox_enabled: false
  resource_limits:
    cpu_percent: 80
    memory_mb: 512
    timeout_secs: 300
  audit_log: true

agent:
  default_mode: "plan"
  max_iterations: 20
  use_chain_of_thought: true
  stream_enabled: true
```

---

## 实现路线图

### Week 1-2: 事件系统 + 会话管理
- [ ] 事件总线实现
- [ ] Session 管理器
- [ ] 基础事件类型定义

### Week 3-4: 状态管理 + 多存储后端
- [ ] 状态管理系统
- [ ] SQLite/PostgreSQL 适配器
- [ ] 状态持久化

### Week 5-6: HTTP/WebSocket 接口
- [ ] REST API 设计
- [ ] WebSocket 服务器
- [ ] 客户端库

### Week 7-8: 聊天平台集成
- [ ] Slack Bot
- [ ] Discord Bot
- [ ] 通用 Bot 框架

### Week 9-10: 工具流式处理 + 插件系统
- [ ] 流式工具执行
- [ ] MCP 集成
- [ ] Skill 加载器

### Week 11-12: 可视化 + 安全隔离
- [ ] Web Dashboard
- [ ] Docker 沙箱
- [ ] 审计日志

---

## 参考架构

**OpenClaw 特性借鉴**：
- 本地优先网关设计
- 多渠道集成模式
- 会话隔离机制
- 事件驱动架构
- 工具流式处理

**PromptLine 现有优势**：
- 成熟的 ReACT 循环
- 灵活的权限系统
- 多模型支持
- 安全验证机制