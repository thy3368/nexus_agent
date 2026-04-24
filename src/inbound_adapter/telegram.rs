use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

// 定义支持的命令
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "支持以下命令:")]
enum Command {
    #[command(description = "开始使用机器人")]
    Start,
    #[command(description = "打个招呼吧")]
    Hello,
    #[command(description = "显示帮助信息")]
    Help,
}

// 处理 /start 命令
async fn start_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    bot.send_message(
        msg.chat.id,
        "👋 你好！我是用 Rust 和 teloxide 编写的机器人！\n\n发送 /hello 试试看！",
    )
    .await?;
    Ok(())
}

// 处理 /hello 命令
async fn hello_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    let username = msg
        .from()
        .and_then(|user| user.username.as_ref())
        .map(|name| format!("@{name}"))
        .unwrap_or_else(|| "朋友".to_string());

    bot.send_message(
        msg.chat.id,
        format!(
            "👋 你好，{}！\n\n很高兴见到你！\n\n试试发送一些文字消息，我会复读给你。",
            username
        ),
    )
    .await?;
    Ok(())
}

// 处理 /help 命令
async fn help_handler(bot: Bot, msg: Message, _cmd: Command) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

// 处理所有命令
async fn command_handler(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Start => start_handler(bot, msg).await,
        Command::Hello => hello_handler(bot, msg).await,
        Command::Help => help_handler(bot, msg, cmd).await,
    }
}

// 处理普通文本消息（复读机功能）
async fn message_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        println!("收到消息: {}", text);

        // 添加一些简单的回复逻辑
        let reply = if text.to_lowercase().contains("你好") {
            "你也好！😊"
        } else if text.to_lowercase().contains("rust") {
            "Rust 是最好的语言！🦀"
        } else {
            text
        };

        bot.send_message(msg.chat.id, reply).await?;
    }
    Ok(())
}

pub async fn run_telegram_bot() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("🚀 启动 Telegram 机器人...");

    // 从环境变量获取 Token
    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("请设置 TELEGRAM_BOT_TOKEN 环境变量");

    let bot = Bot::new(token);

    // 设置命令列表（显示在 Telegram 客户端中）
    if let Err(e) = bot.set_my_commands(Command::bot_commands()).await {
        log::warn!("⚠️ 无法设置命令列表: {}", e);
    }

    // 创建 Dispatcher
    let handler = dptree::entry()
        // 先处理命令
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(command_handler),
        )
        // 再处理普通消息
        .branch(Update::filter_message().endpoint(message_handler));

    // 启动机器人
    log::info!("🤖 机器人已启动！");
    log::info!("📱 在 Telegram 中搜索你的机器人，发送 /start 开始对话");

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler() // 支持 Ctrl+C 优雅关闭
        .build()
        .dispatch()
        .await;

    Ok(())
}
