/// WeChat Bot 使用示例
///
/// 这个示例展示如何使用 wechaty-rust 创建一个完整的微信机器人
///
/// 运行方式:
/// ```bash
/// # 设置环境变量
/// export BOT_NAME="RustWeChatBot"
/// export AUTO_REPLY="true"
/// export KEYWORDS="帮助,菜单,状态"
/// export WECHATY_PUPPET_SERVICE_ENDPOINT="http://localhost:8080"
///
/// # 运行示例
/// cargo run --example wechat_bot_example
/// ```

use tracing_subscriber;
use nexus_agent::inbound_adapter::wechat::{run_wechat_bot, WeChatConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 创建自定义配置
    let config = WeChatConfig {
        bot_name: "RustWeChatBot".to_string(),
        auto_reply: true,
        keywords: vec![
            "帮助".to_string(),
            "菜单".to_string(),
            "状态".to_string(),
            "时间".to_string(),
        ],
        admin_users: vec!["admin@example.com".to_string()],
    };

    // 启动机器人
    run_wechat_bot(Some(config)).await?;

    Ok(())
}
