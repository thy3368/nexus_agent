```
# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 在此代码库中工作时提供指导。
```

## PromptLine - 智能AI驱动的CLI工具

**PromptLine** 是一个基于Rust的CLI工具，将智能AI代理功能直接带到您的终端。它帮助开发者使用自然语言编写代码、运行命令、管理项目和自动化任务。

## 核心架构概述

### 高层结构

代码库遵循模块化架构，具有清晰的关注点分离：

```
src/
├── main.rs              # CLI入口点和命令处理程序
├── lib.rs               # 库入口点，公开公共API
├── cli.rs               # 使用clap的CLI参数解析
├── config.rs            # 配置管理（基于YAML）
├── agent/               # 代理编排和ReACT循环
├── model/               # LLM提供商（OpenAI、Ollama、Gemini）
├── tools/               # 工具实现（文件操作、shell、git、web、搜索）
├── permissions.rs       # 权限管理系统
├── safety/              # 命令安全验证
├── prompt/              # 提示模板和管理
├── repl.rs              # 交互式聊天界面
├── commands.rs          # 聊天模式的斜杠命令
├── formatter.rs         # 响应格式化工具
├── context/             # 项目上下文检测
└── util/                # 实用函数
```

### 核心组件

1. **代理模块** (`src/agent/mod.rs`)
   - 使用ReACT（推理-行动-观察）循环编排LLM交互
   - 处理工具执行和权限检查
   - 维护对话历史
   - 检测任务完成

2. **模型提供商** (`src/model/`)
   - **OpenAIProvider**：与OpenAI API集成（GPT-3.5/4）
   - **OllamaProvider**：通过Ollama支持本地LLM
   - **GeminiProvider**：Google的Gemini模型支持
   - 所有实现`LanguageModel` trait以实现抽象

3. **工具** (`src/tools/`)
   - 文件操作：读取、写入、列出文件
   - Shell执行：运行系统命令
   - Git操作：状态、差异、提交
   - 网络访问：获取网络内容
   - 代码库搜索：搜索代码片段

4. **权限系统** (`src/permissions.rs`)
   - 三个权限级别：允许、询问、拒绝
   - 持久化和会话级权限
   - 使用`PermissionManager`进行跟踪

5. **配置** (`src/config.rs`)
   - YAML配置文件位于`~/.promptline/config.yaml`
   - 模型提供商设置、工具权限、安全规则
   - 支持环境变量扩展

## 开发命令

### 构建和运行

```bash
# 构建项目
cargo build

# 以发布模式构建
cargo build --release

# 直接运行
cargo run

# 启用详细日志运行
RUST_LOG=info cargo run -- --verbose

# 全局安装
cargo install --path .
```

### 测试

```bash
# 运行所有测试
cargo test

# 运行测试并显示输出
cargo test -- --nocapture

# 运行特定模块的测试
cargo test -p promptline --lib agent

# 运行集成测试
cargo test -p promptline --test permission_integration
```

### 代码质量

```bash
# 使用rustfmt检查代码格式
cargo fmt --check

# 格式化代码
cargo fmt

# 运行clippy进行 linting
cargo clippy

# 运行clippy并将警告视为错误
cargo clippy -- -D warnings
```

### 常见开发任务

#### 添加新工具

1. 在`src/tools/`中创建新工具文件
2. 实现`Tool` trait
3. 在`src/tools/mod.rs`中添加到工具注册表
4. 在`main.rs`的handle_agent和handle_chat函数中注册
5. 在`src/config.rs`中添加权限配置

#### 添加新模型提供商

1. 在`src/model/`中创建新提供商文件
2. 实现`LanguageModel` trait
3. 在`main.rs`中添加提供商检测逻辑

#### 修改配置

- 默认配置值在`src/config.rs`中定义
- 用户配置存储在`~/.promptline/config.yaml`
- 项目特定配置在`./.promptline/config.yaml`

### 调试

```bash
# 启用跟踪日志
RUST_LOG=trace cargo run

# 使用调试器运行
rust-gdb target/debug/promptline

# 生成火焰图
cargo flamegraph
```

## 项目特定细节

### 配置文件

默认配置结构：
```yaml
models:
  default: "llama3"
  providers:
    openai:
      api_key: "${OPENAI_API_KEY}"
      models: ["gpt-3.5-turbo", "gpt-4"]
    ollama:
      base_url: "http://localhost:11434"
      models: ["llama3", "codellama"]

tools:
  file_read: "allow"
  file_write: "ask"
  file_delete: "deny"
  shell_execute: "ask"
  git_status: "ask"
  git_diff: "ask"
  git_commit: "ask"
  web_get: "ask"
  codebase_search: "ask"

safety:
  require_approval: true
  require_diff_preview: true
  max_iterations: 20
  dangerous_commands: ["rm -rf /", "rm -rf *"]
  protected_patterns: ["**/.env", "**/*.key"]

agent:
  default_mode: "plan"
  use_chain_of_thought: true
  explain_before_action: true
```

### 环境变量

- `OPENAI_API_KEY`: OpenAI API密钥
- `OLLAMA_API_KEY`: Ollama API密钥（可选）
- `PROMPTLINE_PROVIDER`: 默认模型提供商（openai/ollama/gemini）
- `RUST_LOG`: 日志级别（trace/debug/info/warn/error）

### 支持的命令

```bash
# 启动交互式聊天
promptline
promptline chat

# 运行特定任务
promptline "您的任务描述"
promptline agent "您的任务描述"

# 规划任务（只读）
promptline plan "您的任务"

# 初始化配置
promptline init

# 检查系统健康
promptline doctor

# 使用AI辅助编辑文件
promptline edit <file> "说明"
```

## 设计原则

1. **模块化架构**：核心逻辑、模型提供商和工具之间的清晰分离
2. **安全第一**：具有可配置批准级别的权限系统
3. **可扩展性**：易于添加新工具和模型提供商
4. **用户控制**：可通过YAML和环境变量配置
5. **代理工作流**：用于推理和执行的ReACT循环

## 主要依赖

- **tokio**: 异步运行时
- **clap**: CLI解析
- **serde**: 序列化/反序列化
- **reqwest**: HTTP客户端
- **async-openai**: OpenAI API客户端
- **rustyline**: REPL界面
- **dialoguer**: 用户提示
- **tracing**: 日志和调试

export OPENAI_API_KEY="your-api-key"