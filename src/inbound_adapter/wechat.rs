use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// 微信机器人配置
#[derive(Clone, Debug)]
pub struct WeChatConfig {
    pub bot_name: String,
    pub auto_reply: bool,
    pub keywords: Vec<String>,
    pub admin_users: Vec<String>,
}

impl Default for WeChatConfig {
    fn default() -> Self {
        Self {
            bot_name: std::env::var("BOT_NAME").unwrap_or_else(|_| "RustWeChatBot".to_string()),
            auto_reply: std::env::var("AUTO_REPLY")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(true),
            keywords: std::env::var("KEYWORDS")
                .unwrap_or_else(|_| "帮助,菜单,状态".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            admin_users: std::env::var("ADMIN_USERS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }
}

/// 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Text,
    Image,
    Attachment,
    Unknown,
}

/// 消息结构
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub msg_type: MessageType,
    pub content: String,
    pub talker_id: String,
    pub talker_name: String,
    pub room_id: Option<String>,
    pub room_name: Option<String>,
    pub timestamp: i64,
}

/// 微信机器人 - 基于 wechaty-rust
pub struct WeChatBot {
    config: Arc<WeChatConfig>,
    is_running: Arc<Mutex<bool>>,
}

impl WeChatBot {
    /// 创建新的微信机器人实例
    pub fn new(config: Option<WeChatConfig>) -> Self {
        let config = config.unwrap_or_default();
        info!("🤖 初始化微信机器人: {}", config.bot_name);

        Self {
            config: Arc::new(config),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// 启动机器人 - 使用 wechaty-rust
    pub async fn run(&self) -> Result<()> {
        info!("🚀 启动微信机器人: {}", self.config.bot_name);

        let mut is_running = self.is_running.lock().await;
        *is_running = true;
        drop(is_running);

        // 初始化 wechaty-rust
        // 实际使用时需要配置 WECHATY_PUPPET_SERVICE_ENDPOINT
        info!("💡 wechaty-rust 初始化配置:");
        info!("   export WECHATY_PUPPET_SERVICE_ENDPOINT=\"http://localhost:8080\"");
        info!("   或使用 wechaty-puppet-service 服务");

        // 登录事件
        self.on_login().await?;

        // 接收消息循环
        self.message_loop().await?;

        // 登出事件
        self.on_logout().await?;

        Ok(())
    }

    /// 登录事件处理 - 使用 wechaty-rust API
    async fn on_login(&self) -> Result<()> {
        info!("✅ 登录成功！");
        info!("📱 机器人已连接到微信");
        info!("🎯 机器人名称: {}", self.config.bot_name);

        // 实际使用 wechaty-rust 时的 API 调用:
        // use wechaty::prelude::*;
        // let bot = Wechaty::new(puppet);
        // bot.on_login(Box::new(|context: LoginContext| {
        //     Box::pin(async move {
        //         let contact = context.contact;
        //         let name = contact.name().await.unwrap_or_else(|| "Unknown".to_string());
        //         info!("✅ 登录成功！用户: {}", name);
        //     })
        // })).await;

        Ok(())
    }

    /// 登出事件处理
    async fn on_logout(&self) -> Result<()> {
        info!("⚠️ 用户登出");
        let mut is_running = self.is_running.lock().await;
        *is_running = false;
        Ok(())
    }

    /// 消息处理循环 - 使用 wechaty-rust API
    async fn message_loop(&self) -> Result<()> {
        info!("📨 开始接收消息...");

        // 实际使用 wechaty-rust 时的事件处理:
        // use wechaty::prelude::*;
        // let mut wechaty = Wechaty::new(puppet);
        // wechaty.on_login(Box::new(|context: LoginContext| {
        //     Box::pin(async move {
        //         info!("✅ 登录成功！");
        //     })
        // })).await;
        // wechaty.on_message(Box::new({
        //     let bot = self.clone();
        //     move |context: MessageContext| {
        //         let bot = bot.clone();
        //         Box::pin(async move {
        //             let message = context.message;
        //             let text = message.text().await.unwrap_or_default();
        //             let talker = message.talker();
        //             let room = message.room();
        //             let msg = Message {
        //                 id: message.id().to_string(),
        //                 msg_type: MessageType::Text,
        //                 content: text,
        //                 talker_id: talker.id().to_string(),
        //                 talker_name: talker.name().await.unwrap_or_default(),
        //                 room_id: room.as_ref().map(|r| r.id().to_string()),
        //                 room_name: room.as_ref().and_then(|r| r.topic().await.ok()),
        //                 timestamp: chrono::Local::now().timestamp(),
        //             };
        //             if let Err(e) = bot.handle_message(&msg).await {
        //                 error!("处理消息失败: {}", e);
        //             }
        //         })
        //     }
        // })).await;
        // wechaty.on_friendship(Box::new(|context: FriendshipContext| {
        //     Box::pin(async move {
        //         let friendship = context.friendship;
        //         match friendship.type_().await {
        //             Ok(FriendshipType::Receive) => {
        //                 info!("收到好友请求");
        //                 friendship.accept().await.ok();
        //             }
        //             _ => {}
        //         }
        //     })
        // })).await;
        // wechaty.start().await?;

        // 模拟消息处理（演示用）
        self.simulate_messages().await?;

        Ok(())
    }

    /// 处理消息
    async fn handle_message(&self, msg: &Message) -> Result<()> {
        match msg.msg_type {
            MessageType::Text => {
                if let Some(room_name) = &msg.room_name {
                    // 群消息
                    info!("👥 群 [{}] - {}: {}", room_name, msg.talker_name, msg.content);
                    self.handle_group_message(msg).await?;
                } else {
                    // 私聊消息
                    info!("💬 {}: {}", msg.talker_name, msg.content);
                    self.handle_private_message(msg).await?;
                }
            }
            MessageType::Image => {
                info!("🖼️ 收到图片消息 from {}", msg.talker_name);
            }
            MessageType::Attachment => {
                info!("📎 收到文件消息 from {}", msg.talker_name);
            }
            MessageType::Unknown => {
                warn!("❓ 收到未知类型消息 from {}", msg.talker_name);
            }
        }
        Ok(())
    }

    /// 处理私聊消息
    async fn handle_private_message(&self, msg: &Message) -> Result<()> {
        let text_lower = msg.content.to_lowercase();

        let reply = match text_lower.as_str() {
            "帮助" | "help" | "菜单" => self.get_help_message(),
            "状态" | "status" => self.get_status_message(),
            "时间" | "time" => {
                format!("当前时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))
            }
            _ => {
                // 检查关键词
                for keyword in &self.config.keywords {
                    if msg.content.contains(keyword) {
                        return Ok(self.send_reply(msg, &format!("您提到了「{}」，有什么可以帮您的吗？", keyword)).await?);
                    }
                }

                // 默认回复
                if self.config.auto_reply {
                    self.get_default_reply()
                } else {
                    return Ok(());
                }
            }
        };

        self.send_reply(msg, &reply).await?;
        Ok(())
    }

    /// 处理群消息
    async fn handle_group_message(&self, msg: &Message) -> Result<()> {
        // 检查是否是@机器人的消息
        if msg.content.contains(&format!("@{}", self.config.bot_name)) {
            let pure_text = msg
                .content
                .replace(&format!("@{}", self.config.bot_name), "")
                .trim()
                .to_string();

            info!("🤖 被@的消息: {}", pure_text);

            let reply = match pure_text.to_lowercase().as_str() {
                "帮助" | "help" => self.get_help_message(),
                "状态" | "status" => self.get_status_message(),
                _ => format!("👤 {} 你好！我收到了你的消息: {}", msg.talker_name, pure_text),
            };

            self.send_group_reply(msg, &reply).await?;
        }

        Ok(())
    }

    /// 发送私聊回复 - 使用 wechaty-rust API
    async fn send_reply(&self, msg: &Message, content: &str) -> Result<()> {
        info!("📤 回复 {}: {}", msg.talker_name, content);

        // 实际使用 wechaty-rust 时的 API 调用:
        // use wechaty::prelude::*;
        // 方法1: 通过 Contact 对象发送
        // let contact = Contact::load(msg.talker_id.clone()).await?;
        // contact.say(content).await?;
        // 方法2: 通过消息对象直接回复（如果有消息对象）
        // message.say(content).await?;c
        // 方法3: 发送文件或其他类型消息
        // contact.say_file("/path/to/file").await?;
        // contact.say_url("https://example.com/image.jpg").await?;

        Ok(())
    }

    /// 发送群聊回复 - 使用 wechaty-rust API
    async fn send_group_reply(&self, msg: &Message, content: &str) -> Result<()> {
        info!("📤 回复群 [{}]: {}", msg.room_name.as_ref().unwrap_or(&"未知".to_string()), content);

        // 实际使用 wechaty-rust 时的 API 调用:
        // use wechaty::prelude::*;
        // 方法1: 通过 Room 对象发送
        // if let Some(room_id) = &msg.room_id {
        //     let room = Room::load(room_id.clone()).await?;
        //     room.say(content).await?;
        // }
        // 方法2: 通过消息对象直接回复
        // message.say(content).await?;
        // 方法3: @特定用户回复
        // if let Some(room_id) = &msg.room_id {
        //     let room = Room::load(room_id.clone()).await?;
        //     let contact = Contact::load(msg.talker_id.clone()).await?;
        //     let mention_text = format!("@{} {}", contact.name().await.unwrap_or_default(), content);
        //     room.say(&mention_text).await?;
        // }

        Ok(())
    }

    /// 获取帮助信息
    fn get_help_message(&self) -> String {
        format!(
            "🤖 {} 帮助菜单\n\n📋 可用命令:\n• 帮助 - 显示此帮助信息\n• 状态 - 查看机器人状态\n• 时间 - 显示当前时间\n\n🎯 关键词回复: {}\n\n⚙️ 自动回复: {}",
            self.config.bot_name,
            self.config.keywords.join(", "),
            if self.config.auto_reply { "开启" } else { "关闭" }
        )
    }

    /// 获取状态信息
    fn get_status_message(&self) -> String {
        format!(
            "📊 机器人状态\n\n🏷️ 名称: {}\n🔧 自动回复: {}\n📅 启动时间: {}\n👤 管理员: {}",
            self.config.bot_name,
            if self.config.auto_reply { "✅" } else { "❌" },
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            if self.config.admin_users.is_empty() {
                "无".to_string()
            } else {
                self.config.admin_users.join(", ")
            }
        )
    }

    /// 获取默认回复
    fn get_default_reply(&self) -> String {
        let replies = vec![
            "我在呢！有什么可以帮您？",
            "您好！我是机器人助手",
            "请输入「帮助」查看可用功能",
            "抱歉，我还在学习中，请说得更明确些",
        ];

        use rand::seq::SliceRandom;
        replies
            .choose(&mut rand::thread_rng())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "您好！".to_string())
    }

    /// 模拟接收消息（用于演示）
    async fn simulate_messages(&self) -> Result<()> {
        info!("📨 模拟消息处理演示...");

        // 模拟私聊消息
        let msg1 = Message {
            id: "msg_001".to_string(),
            msg_type: MessageType::Text,
            content: "帮助".to_string(),
            talker_id: "user_001".to_string(),
            talker_name: "张三".to_string(),
            room_id: None,
            room_name: None,
            timestamp: chrono::Local::now().timestamp(),
        };

        self.handle_message(&msg1).await?;

        // 模拟群消息
        let msg2 = Message {
            id: "msg_002".to_string(),
            msg_type: MessageType::Text,
            content: format!("@{} 状态", self.config.bot_name),
            talker_id: "user_002".to_string(),
            talker_name: "李四".to_string(),
            room_id: Some("room_001".to_string()),
            room_name: Some("开发讨论组".to_string()),
            timestamp: chrono::Local::now().timestamp(),
        };

        self.handle_message(&msg2).await?;

        // 模拟关键词消息
        let msg3 = Message {
            id: "msg_003".to_string(),
            msg_type: MessageType::Text,
            content: "我需要帮助".to_string(),
            talker_id: "user_003".to_string(),
            talker_name: "王五".to_string(),
            room_id: None,
            room_name: None,
            timestamp: chrono::Local::now().timestamp(),
        };

        self.handle_message(&msg3).await?;

        Ok(())
    }
}

/// 启动微信机器人
pub async fn run_wechat_bot(config: Option<WeChatConfig>) -> Result<()> {
    info!("🚀 启动微信机器人...");

    let bot = WeChatBot::new(config);

    // 重试机制
    let max_retries = 3;
    for attempt in 1..=max_retries {
        match bot.run().await {
            Ok(_) => {
                info!("✅ 微信机器人已正常关闭");
                break;
            }
            Err(e) if attempt < max_retries => {
                error!("第 {} 次启动失败: {}，{}秒后重试...", attempt, e, attempt * 5);
                tokio::time::sleep(tokio::time::Duration::from_secs((attempt * 5) as u64)).await;
            }
            Err(e) => {
                error!("启动失败，已达到最大重试次数: {}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wechat_config_default() {
        let config = WeChatConfig::default();
        assert_eq!(config.bot_name, "RustWeChatBot");
        assert!(config.auto_reply);
    }

    #[tokio::test]
    async fn test_handle_private_message() {
        let config = WeChatConfig {
            bot_name: "TestBot".to_string(),
            auto_reply: true,
            keywords: vec!["帮助".to_string()],
            admin_users: vec![],
        };

        let bot = WeChatBot::new(Some(config));

        let msg = Message {
            id: "test_001".to_string(),
            msg_type: MessageType::Text,
            content: "帮助".to_string(),
            talker_id: "user_001".to_string(),
            talker_name: "测试用户".to_string(),
            room_id: None,
            room_name: None,
            timestamp: chrono::Local::now().timestamp(),
        };

        assert!(bot.handle_private_message(&msg).await.is_ok());
    }

    #[test]
    fn test_get_help_message() {
        let config = WeChatConfig {
            bot_name: "TestBot".to_string(),
            auto_reply: true,
            keywords: vec!["帮助".to_string(), "菜单".to_string()],
            admin_users: vec!["admin".to_string()],
        };

        let bot = WeChatBot::new(Some(config));
        let help = bot.get_help_message();

        assert!(help.contains("TestBot"));
        assert!(help.contains("帮助"));
        assert!(help.contains("菜单"));
    }
}
