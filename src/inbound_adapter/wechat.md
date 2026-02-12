🤖 Rust 微信机器人完整指南

在 Rust 中构建微信机器人有多种方案，我将为你详细介绍最主流和稳定的几种实现方式。

📊 方案对比

方案 使用技术 稳定性 功能完整性 推荐度

wechaty-rust 基于 Puppeteer/IPC 较高 完整 ⭐⭐⭐⭐⭐

微信官方 API 企业微信/公众号 最高 有限 ⭐⭐⭐⭐

逆向协议 逆向工程 较低 完整 ⭐⭐

第三方服务 API 网关 高 中等 ⭐⭐⭐

🎯 推荐方案：wechaty-rust

wechaty-rust 是目前 Rust 生态中最成熟的微信机器人框架，支持个人微信和企业微信。

1. 创建项目

cargo new wechat-bot-rs
cd wechat-bot-rs


2. 添加依赖

# Cargo.toml
[package]
name = "wechat-bot-rs"
version = "0.1.0"
edition = "2021"

[dependencies]
wechaty = "0.1"
tokio = { version = "1.0", features = ["full", "macros"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
dotenv = "0.15"
anyhow = "1.0"
futures = "0.3"


3. 完整机器人实现

// src/main.rs
use wechaty::prelude::*;
use dotenv::dotenv;
use std::env;
use tracing::{info, warn, error};
use tokio::time::{sleep, Duration};
use serde_json::json;
use anyhow::Result;

#[derive(Clone)]
struct WeChatBot {
config: Config,
// 可以添加数据库连接等
}

#[derive(Clone)]
struct Config {
bot_name: String,
auto_reply: bool,
keywords: Vec<String>,
admin_users: Vec<String>,
}

impl WeChatBot {
fn new() -> Self {
Self {
config: Config {
bot_name: env::var("BOT_NAME").unwrap_or_else(|_| "RustWeChatBot".to_string()),
auto_reply: env::var("AUTO_REPLY")
.map(|v| v.to_lowercase() == "true")
.unwrap_or(true),
keywords: env::var("KEYWORDS")
.unwrap_or_else(|_| "帮助,菜单,状态".to_string())
.split(',')
.map(|s| s.trim().to_string())
.collect(),
admin_users: env::var("ADMIN_USERS")
.unwrap_or_default()
.split(',')
.map(|s| s.trim().to_string())
.collect(),
},
}
}

    async fn run(&self) -> Result<()> {
        info!("🤖 启动微信机器人: {}", self.config.bot_name);
        
        // 创建 Wechaty 实例
        let mut wechaty = Wechaty::new();
        
        // 注册事件处理器
        self.register_handlers(&mut wechaty).await?;
        
        // 启动机器人
        wechaty.start().await?;
        
        info!("✅ 微信机器人已启动，等待消息...");
        
        // 保持运行
        tokio::signal::ctrl_c().await?;
        
        Ok(())
    }
    
    async fn register_handlers(&self, wechaty: &mut Wechaty) -> Result<()> {
        // 登录事件
        wechaty.on_login(Box::new(|context: LoginContext| {
            Box::pin(async move {
                info!("✅ 登录成功！用户: {}", context.contact.name().await.unwrap_or_default());
            })
        }));
        
        // 登出事件
        wechaty.on_logout(Box::new(|context: LogoutContext| {
            Box::pin(async move {
                info!("⚠️ 用户登出: {}", context.contact.name().await.unwrap_or_default());
            })
        }));
        
        // 消息事件
        wechaty.on_message(Box::new({
            let bot = self.clone();
            move |context: MessageContext| {
                let bot = bot.clone();
                Box::pin(async move {
                    if let Err(e) = bot.handle_message(context).await {
                        error!("处理消息失败: {}", e);
                    }
                })
            }
        }));
        
        // 好友请求事件
        wechaty.on_friendship(Box::new({
            let bot = self.clone();
            move |context: FriendshipContext| {
                let bot = bot.clone();
                Box::pin(async move {
                    if let Err(e) = bot.handle_friendship(context).await {
                        error!("处理好友请求失败: {}", e);
                    }
                })
            }
        }));
        
        // 群邀请事件
        wechaty.on_room_invite(Box::new(|context: RoomInvitationContext| {
            Box::pin(async move {
                info!("收到群邀请: {:?}", context.invitation);
                // 自动接受群邀请
                context.invitation.accept().await.ok();
            })
        }));
        
        Ok(())
    }
    
    async fn handle_message(&self, context: MessageContext) -> Result<()> {
        let msg = context.message;
        
        // 获取消息信息
        let talker = msg.talker();
        let room = msg.room();
        let text = msg.text();
        let msg_type = msg.message_type();
        let msg_id = msg.id();
        
        // 获取发送者信息
        let talker_name = talker.name().await.unwrap_or_else(|| "未知用户".to_string());
        let talker_id = talker.id();
        
        // 判断消息类型
        match msg_type {
            MessageType::Text => {
                if let Some(room) = room {
                    // 群消息
                    let room_name = room.topic().await.unwrap_or_else(|| "未知群".to_string());
                    info!("👥 群 [{}] - {}: {}", room_name, talker_name, text);
                    
                    // 处理群消息
                    self.handle_group_message(&msg, room, &text, &talker).await?;
                } else {
                    // 私聊消息
                    info!("💬 {}: {}", talker_name, text);
                    
                    // 处理私聊消息
                    self.handle_private_message(&msg, &text, &talker).await?;
                }
            }
            MessageType::Image => {
                info!("🖼️ 收到图片消息 from {}", talker_name);
                // 可以在这里处理图片消息
            }
            MessageType::Attachment => {
                info!("📎 收到文件消息 from {}", talker_name);
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn handle_private_message(
        &self,
        msg: &Message,
        text: &str,
        talker: &Contact,
    ) -> Result<()> {
        // 转换为小写方便匹配
        let text_lower = text.to_lowercase();
        
        match text_lower.as_str() {
            "帮助" | "help" | "菜单" => {
                self.send_help_message(msg, talker).await?;
            }
            "状态" | "status" => {
                self.send_status_message(msg, talker).await?;
            }
            "时间" | "time" => {
                let now = chrono::Local::now();
                msg.say(&format!("当前时间: {}", now.format("%Y-%m-%d %H:%M:%S")))
                    .await?;
            }
            "echo" if text.len() > 5 => {
                let echo_text = &text[5..].trim();
                msg.say(&format!("回显: {}", echo_text)).await?;
            }
            _ => {
                // 关键词回复
                for keyword in &self.config.keywords {
                    if text.contains(keyword) {
                        let reply = format!("您提到了「{}」，有什么可以帮您的吗？", keyword);
                        msg.say(&reply).await?;
                        return Ok(());
                    }
                }
                
                // 默认回复
                if self.config.auto_reply {
                    self.send_default_reply(msg, talker).await?;
                }
            }
        }
        
        Ok(())
    }
    
    async fn handle_group_message(
        &self,
        msg: &Message,
        room: Room,
        text: &str,
        talker: &Contact,
    ) -> Result<()> {
        // 检查是否是@机器人的消息
        let bot_self = msg.self_contact().await?;
        if msg.is_mention(&bot_self).await.unwrap_or(false) {
            // 提取实际消息内容（去除@部分）
            let pure_text = text.replace(&format!("@{}", bot_self.name().await.unwrap_or_default()), "").trim().to_string();
            
            info!("🤖 被@的消息: {}", pure_text);
            
            // 处理@消息
            self.handle_mention_message(msg, room, &pure_text, talker).await?;
        } else {
            // 普通群消息
            // 可以在这里添加群消息监控、关键词提醒等功能
        }
        
        Ok(())
    }
    
    async fn handle_mention_message(
        &self,
        msg: &Message,
        room: Room,
        text: &str,
        talker: &Contact,
    ) -> Result<()> {
        match text.to_lowercase().as_str() {
            "帮助" | "help" => {
                let help_text = "🤖 可用命令:\n\
                                 • 帮助 - 显示此帮助信息\n\
                                 • 状态 - 查看机器人状态\n\
                                 • 时间 - 显示当前时间\n\
                                 • @机器人 + 消息 - 与机器人对话";
                room.say(&help_text, Some(&msg)).await?;
            }
            "状态" | "status" => {
                let status = format!("🟢 机器人运行正常\n👤 发送者: {}", talker.name().await.unwrap_or_default());
                room.say(&status, Some(&msg)).await?;
            }
            _ => {
                // 默认回复
                let reply = format!("👤 {} 你好！我收到了你的消息: {}", 
                    talker.name().await.unwrap_or_default(), text);
                room.say(&reply, Some(&msg)).await?;
            }
        }
        
        Ok(())
    }
    
    async fn handle_friendship(&self, context: FriendshipContext) -> Result<()> {
        let friendship = context.friendship;
        
        match friendship.type_().await? {
            FriendshipType::Receive => {
                info!("收到好友请求");
                
                // 自动接受好友请求
                friendship.accept().await?;
                
                // 发送欢迎消息
                let contact = friendship.contact().await?;
                let welcome_msg = "👋 你好！我是基于 Rust 开发的微信机器人\n\n\
                                   💬 发送「帮助」查看可用功能";
                contact.say(&welcome_msg).await?;
            }
            FriendshipType::Confirm => {
                info!("好友关系已确认");
            }
            FriendshipType::Verify => {
                info!("需要验证好友");
            }
        }
        
        Ok(())
    }
    
    async fn send_help_message(&self, msg: &Message, talker: &Contact) -> Result<()> {
        let help_text = format!("🤖 {} 帮助菜单\n\n\
                                📋 可用命令:\n\
                                • 帮助 - 显示此帮助信息\n\
                                • 状态 - 查看机器人状态\n\
                                • 时间 - 显示当前时间\n\n\
                                🎯 关键词回复: {}\n\n\
                                ⚙️ 自动回复: {}",
                                self.config.bot_name,
                                self.config.keywords.join(", "),
                                if self.config.auto_reply { "开启" } else { "关闭" });
        
        msg.say(&help_text).await?;
        Ok(())
    }
    
    async fn send_status_message(&self, msg: &Message, talker: &Contact) -> Result<()> {
        let status_text = format!("📊 机器人状态\n\n\
                                  🏷️ 名称: {}\n\
                                  🔧 自动回复: {}\n\
                                  📅 启动时间: {}\n\
                                  👤 管理员: {}",
                                  self.config.bot_name,
                                  if self.config.auto_reply { "✅" } else { "❌" },
                                  chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                                  self.config.admin_users.join(", "));
        
        msg.say(&status_text).await?;
        Ok(())
    }
    
    async fn send_default_reply(&self, msg: &Message, talker: &Contact) -> Result<()> {
        let replies = vec![
            "我在呢！有什么可以帮您？",
            "您好！我是机器人助手",
            "请输入「帮助」查看可用功能",
            "抱歉，我还在学习中，请说得更明确些",
        ];
        
        use rand::seq::SliceRandom;
        let reply = replies.choose(&mut rand::thread_rng()).unwrap();
        
        msg.say(reply).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
// 初始化日志
tracing_subscriber::fmt()
.with_max_level(tracing::Level::INFO)
.init();

    // 加载环境变量
    dotenv().ok();
    
    info!("🚀 启动微信机器人...");
    
    // 创建并运行机器人
    let bot = WeChatBot::new();
    
    // 重试机制
    let max_retries = 3;
    for attempt in 1..=max_retries {
        match bot.run().await {
            Ok(_) => break,
            Err(e) if attempt < max_retries => {
                error!("第 {} 次启动失败: {}，{} 秒后重试...", 
                       attempt, e, attempt * 5);
                sleep(Duration::from_secs((attempt * 5) as u64)).await;
            }
            Err(e) => {
                error!("启动失败，已达到最大重试次数: {}", e);
                return Err(e);
            }
        }
    }
    
    Ok(())
}


4. 添加配置文件

创建 .env 文件：
# 机器人配置
BOT_NAME=RustWeChatBot
AUTO_REPLY=true
KEYWORDS=帮助,菜单,状态,时间,天气
ADMIN_USERS=admin1,admin2

# Wechaty 配置 (如果需要)
WECHATY_ENDPOINT=
WECHATY_TOKEN=


创建 config.toml：
# config.toml
[bot]
name = "RustWeChatBot"
auto_reply = true
keywords = ["帮助", "菜单", "状态", "时间", "天气"]
admin_users = ["admin1", "admin2"]

[database]
path = "data/bot.db"

[schedule]
auto_reply_interval = 60
check_friends_interval = 3600

[webhook]
enabled = false
url = "http://localhost:3000/webhook"


5. 高级功能扩展

数据库支持

// src/database.rs
use rusqlite::{Connection, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRecord {
pub id: i64,
pub msg_id: String,
pub talker_id: String,
pub talker_name: String,
pub room_id: Option<String>,
pub room_name: Option<String>,
pub message_type: String,
pub content: String,
pub created_at: DateTime<Local>,
pub is_handled: bool,
}

pub struct Database {
conn: Connection,
}

impl Database {
pub fn new(path: &str) -> Result<Self> {
let conn = Connection::open(path)?;

        // 创建表
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                msg_id TEXT UNIQUE,
                talker_id TEXT NOT NULL,
                talker_name TEXT,
                room_id TEXT,
                room_name TEXT,
                message_type TEXT,
                content TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                is_handled BOOLEAN DEFAULT 0
            );
            
            CREATE TABLE IF NOT EXISTS contacts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                contact_id TEXT UNIQUE,
                contact_name TEXT,
                alias TEXT,
                friend_status INTEGER DEFAULT 0,
                last_contact TIMESTAMP,
                tags TEXT
            );
            
            CREATE TABLE IF NOT EXISTS groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                room_id TEXT UNIQUE,
                room_name TEXT,
                topic TEXT,
                member_count INTEGER,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );"
        )?;
        
        Ok(Self { conn })
    }
    
    pub fn save_message(&self, record: &MessageRecord) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO messages 
            (msg_id, talker_id, talker_name, room_id, room_name, message_type, content) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                &record.msg_id,
                &record.talker_id,
                &record.talker_name,
                &record.room_id,
                &record.room_name,
                &record.message_type,
                &record.content,
            ],
        )?;
        Ok(())
    }
}


定时任务

// src/scheduler.rs
use tokio_cron_scheduler::{Job, JobScheduler};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::info;

pub struct Scheduler {
bot: Arc<WeChatBot>,
}

impl Scheduler {
pub fn new(bot: Arc<WeChatBot>) -> Self {
Self { bot }
}

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let scheduler = JobScheduler::new().await?;
        
        // 每天早上9点发送问候
        scheduler.add(Job::new_async("0 0 9 * * *", |_uuid, _l| {
            Box::pin(async move {
                info!("⏰ 发送每日问候");
                // 这里可以调用机器人发送消息的方法
            })
        })?).await?;
        
        // 每小时检查好友状态
        scheduler.add(Job::new_async("0 0 * * * *", |_uuid, _l| {
            Box::pin(async move {
                info!("👥 检查好友状态");
            })
        })?).await?;
        
        scheduler.start().await?;
        
        // 保持运行
        tokio::signal::ctrl_c().await?;
        
        Ok(())
    }
}


HTTP API 接口

// src/api.rs
use warp::Filter;
use std::sync::Arc;
use serde_json::json;

pub struct ApiServer {
bot: Arc<WeChatBot>,
port: u16,
}

impl ApiServer {
pub fn new(bot: Arc<WeChatBot>, port: u16) -> Self {
Self { bot, port }
}

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let bot = Arc::clone(&self.bot);
        
        // 健康检查
        let health = warp::path!("health")
            .map(|| warp::reply::json(&json!({"status": "ok"})));
        
        // 发送消息
        let send_message = warp::path!("send")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |body: serde_json::Value| {
                let bot = Arc::clone(&bot);
                async move {
                    match bot.send_message(
                        &body["to"].as_str().unwrap_or(""),
                        &body["content"].as_str().unwrap_or(""),
                    ).await {
                        Ok(_) => Ok(warp::reply::json(&json!({"success": true}))),
                        Err(e) => Ok(warp::reply::json(&json!({"success": false, "error": e.to_string()}))),
                    }
                }
            });
        
        let routes = health.or(send_message);
        
        warp::serve(routes)
            .run(([0, 0, 0, 0], self.port))
            .await;
        
        Ok(())
    }
}


6. 企业微信机器人

如果你需要使用企业微信，这里有一个简单的示例：
// src/wecom.rs
use reqwest::Client;
use serde_json::json;
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct WeComBot {
corp_id: String,
corp_secret: String,
agent_id: u32,
client: Client,
access_token: Option<String>,
token_expire_time: u64,
}

impl WeComBot {
pub fn new(corp_id: &str, corp_secret: &str, agent_id: u32) -> Self {
Self {
corp_id: corp_id.to_string(),
corp_secret: corp_secret.to_string(),
agent_id,
client: Client::new(),
access_token: None,
token_expire_time: 0,
}
}

    async fn get_access_token(&mut self) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        if self.access_token.is_some() && now < self.token_expire_time {
            return Ok(self.access_token.clone().unwrap());
        }
        
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/gettoken?corpid={}&corpsecret={}",
            self.corp_id, self.corp_secret
        );
        
        let response: serde_json::Value = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;
        
        if let Some(token) = response["access_token"].as_str() {
            self.access_token = Some(token.to_string());
            self.token_expire_time = now + 7000; // 提前刷新
            Ok(token.to_string())
        } else {
            anyhow::bail!("获取 access_token 失败: {:?}", response)
        }
    }
    
    pub async fn send_text_message(&mut self, content: &str, to_user: Option<&str>) -> Result<()> {
        let token = self.get_access_token().await?;
        
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
            token
        );
        
        let mut data = json!({
            "touser": to_user.unwrap_or("@all"),
            "msgtype": "text",
            "agentid": self.agent_id,
            "text": {
                "content": content
            }
        });
        
        let response: serde_json::Value = self.client
            .post(&url)
            .json(&data)
            .send()
            .await?
            .json()
            .await?;
        
        if response["errcode"].as_i64() == Some(0) {
            Ok(())
        } else {
            anyhow::bail!("发送消息失败: {:?}", response)
        }
    }
    
    pub async fn send_markdown_message(&mut self, content: &str, to_user: Option<&str>) -> Result<()> {
        let token = self.get_access_token().await?;
        
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
            token
        );
        
        let data = json!({
            "touser": to_user.unwrap_or("@all"),
            "msgtype": "markdown",
            "agentid": self.agent_id,
            "markdown": {
                "content": content
            }
        });
        
        let response: serde_json::Value = self.client
            .post(&url)
            .json(&data)
            .send()
            .await?
            .json()
            .await?;
        
        if response["errcode"].as_i64() == Some(0) {
            Ok(())
        } else {
            anyhow::bail!("发送消息失败: {:?}", response)
        }
    }
}


7. Docker 部署

# Dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM ubuntu:22.04
WORKDIR /app

# 安装依赖
RUN apt-get update && apt-get install -y \
ca-certificates \
libssl-dev \
ca-certificates \
&& rm -rf /var/lib/apt/lists/*

# 复制可执行文件
COPY --from=builder /app/target/release/wechat-bot-rs /app/wechat-bot
COPY --from=builder /app/config.toml /app/config.toml
COPY --from=builder /app/.env /app/.env

# 创建数据目录
RUN mkdir -p /app/data

# 设置非root用户
RUN useradd -m -u 1000 botuser
RUN chown -R botuser:botuser /app
USER botuser

CMD ["/app/wechat-bot"]

# docker-compose.yml
version: '3.8'

services:
wechat-bot:
build: .
container_name: wechat-bot
restart: unless-stopped
volumes:
- ./data:/app/data
- ./logs:/app/logs
environment:
- TZ=Asia/Shanghai
env_file:
- .env


8. 运行和使用

1. 安装依赖：
# 安装 puppeteer 依赖（wechaty 需要）
sudo apt-get install -y \
gconf-service \
libasound2 \
libatk1.0-0 \
libc6 \
libcairo2 \
libcups2 \
libdbus-1-3 \
libexpat1 \
libfontconfig1 \
libgcc1 \
libgconf-2-4 \
libgdk-pixbuf2.0-0 \
libglib2.0-0 \
libgtk-3-0 \
libnspr4 \
libpango-1.0-0 \
libpangocairo-1.0-0 \
libstdc++6 \
libx11-6 \
libx11-xcb1 \
libxcb1 \
libxcomposite1 \
libxcursor1 \
libxdamage1 \
libxext6 \
libxfixes3 \
libxi6 \
libxrandr2 \
libxrender1 \
libxss1 \
libxtst6 \
ca-certificates \
fonts-liberation \
libappindicator1 \
libnss3 \
lsb-release \
xdg-utils \
wget


2. 运行机器人：
# 开发模式
cargo run

# 生产模式
cargo build --release
./target/release/wechat-bot-rs


3. 扫码登录：
   程序启动后会显示二维码，使用微信扫码登录即可。

9. 注意事项

1. 合规性：使用微信机器人需要遵守微信的使用条款
2. 账号安全：建议使用小号，避免主号被封
3. 频率限制：控制消息发送频率，避免被限制
4. 功能限制：某些高级功能可能需要开通微信支付等
5. 更新维护：微信协议经常更新，需要定期更新依赖

10. 替代方案

如果 wechaty-rust 不满足需求，可以考虑：

1. itchat-rs：另一个 Rust 微信机器人库
2. Python 方案：使用 itchat/wxpy + Rust FFI
3. HTTP 代理：通过 mitmproxy 拦截微信流量
4. Windows 自动化：通过 RPA 技术控制微信客户端

🔐 安全建议

1. 使用环境变量存储敏感信息
2. 限制机器人权限，避免敏感操作
3. 实现消息审核，过滤敏感内容
4. 定期备份数据库和配置
5. 监控运行状态，设置告警

这个指南提供了完整的 Rust 微信机器人实现方案。根据你的具体需求选择合适的方案，建议从 wechaty-rust 开始，它是最成熟和活跃的 Rust 微信机器人框架。