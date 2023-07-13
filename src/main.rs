mod access;
mod cmds;
mod config;
mod keyboard;
mod ngrok;
mod responds;

use std::{error::Error, sync::Arc, time::Duration};

use formatx::formatx;
use teloxide::{
    dispatching::{Dispatcher, UpdateFilterExt, UpdateHandler},
    dptree::{self, case},
    payloads::SendMessageSetters,
    requests::{Request, Requester},
    types::{CallbackQuery, Message, ParseMode, Update},
    Bot,
};
use tokio::time::sleep;

use crate::{
    access::{check_chat_access, check_user_access, Access},
    cmds::{
        error_cmd, help_cmd, kill_ngrok_cmd, list_ngrok_cmd, start_cmd, Command, CommandResponse,
    },
    config::{self as cfg, Config, NgrokCmd},
    keyboard::ButtonQuery,
    ngrok::Ngrok,
    responds::{
        ACCESS_DECLINED, BAD_CHAT_ID, BAD_OPTION_SELECTED, BUTTON_HANDLER_MISSED,
        NGROK_API_CONNECTION_ERROR, NGROK_TUNNEL_OBTAINED,
    },
};

struct State {
    ngrok: Ngrok,
    config: Config,
}

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    ::std::env::set_var("RUST_LOG", "info");

    pretty_env_logger::init();
    log::info!("Starting bot...");

    let config: Config = cfg::load();
    let bot = Bot::new(config.bot_key.clone());

    let state = Arc::new(State {
        config,
        ngrok: Ngrok::new(),
    });

    log::info!("Bot is ready!");

    Dispatcher::builder(bot, create_handlers())
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

fn create_handlers() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Help].endpoint(help_handler))
        .branch(case![Command::Start].endpoint(start_handler))
        .branch(case![Command::Ngrok].endpoint(list_ngrok_handler))
        .branch(case![Command::KillNgrok].endpoint(kill_ngrok_handler));

    dptree::entry()
        .branch(
            Update::filter_message()
                .filter_async(chat_access_filter)
                .branch(command_handler)
                .branch(dptree::endpoint(cmd_not_found_handler)),
        )
        .branch(
            Update::filter_callback_query()
                .filter_map_async(button_query_parser)
                .filter_async(button_access_filter)
                .endpoint(buttons_handler),
        )
}

async fn chat_access_filter(state: Arc<State>, message: Message, bot: Bot) -> bool {
    log::info!("Bot called from: {:?}", message.chat.id);

    if let Some(user) = message.from() {
        log::info!("Called by user {user:?}")
    }

    match check_chat_access(&message.chat, &state.config.permitted_chats) {
        Access::Granted => true,
        Access::Declined => {
            let _ = bot.send_message(message.chat.id, BAD_CHAT_ID).await;
            false
        }
    }
}

async fn start_handler(message: Message, bot: Bot) -> HandlerResult {
    reply(&bot, &message, start_cmd()).await;
    Ok(())
}

async fn help_handler(message: Message, bot: Bot) -> HandlerResult {
    reply(&bot, &message, help_cmd()).await;
    Ok(())
}

async fn list_ngrok_handler(message: Message, bot: Bot, state: Arc<State>) -> HandlerResult {
    reply(
        &bot,
        &message,
        list_ngrok_cmd(message.from(), &state.config.ngrok_cmds),
    )
    .await;
    Ok(())
}

async fn kill_ngrok_handler(message: Message, bot: Bot, state: Arc<State>) -> HandlerResult {
    reply(&bot, &message, kill_ngrok_cmd(&state.ngrok)).await;
    Ok(())
}

async fn cmd_not_found_handler(message: Message, bot: Bot) -> HandlerResult {
    reply(&bot, &message, error_cmd()).await;
    Ok(())
}

async fn reply(bot: &Bot, message: &Message, response: CommandResponse) {
    let res = match response {
        CommandResponse::WithMarkup { msg, keyboard } => {
            bot.send_message(message.chat.id, msg)
                .reply_markup(keyboard)
                .await
        }
        CommandResponse::WithoutMarkup { msg } => bot.send_message(message.chat.id, msg).await,
    };

    match res {
        Ok(_) => {}
        Err(err) => log::error!("Can't send a message: {}", err),
    }
}

async fn button_query_parser(
    bot: Bot,
    query: CallbackQuery,
    state: Arc<State>,
) -> Option<NgrokCmd> {
    let button_query = //.
        if let Some(ref raw_data) = query.data {
            ButtonQuery::try_from(raw_data.as_str())
        } else {
            Err(BUTTON_HANDLER_MISSED.into())
        };

    let ngrok_cmd = button_query.and_then(|query| match query {
        ButtonQuery::Ngrok { cmd_idx } => {
            if cmd_idx >= state.config.ngrok_cmds.len() {
                Err(BAD_OPTION_SELECTED.into())
            } else {
                Ok(state.config.ngrok_cmds[cmd_idx].clone())
            }
        }
    });

    if let Some(err) = ngrok_cmd.as_ref().err() {
        reply_from_query(&bot, &query, err.to_string()).await;
    }

    ngrok_cmd.ok()
}

async fn button_access_filter(bot: Bot, query: CallbackQuery, ngrok_cmd: NgrokCmd) -> bool {
    log::info!(
        "Command called by: {} ({})",
        query.from.full_name(),
        query.from.id.0
    );

    let access_check_result: Result<(), String> = //.
        match check_user_access(&query.from, &ngrok_cmd) {
            Access::Granted => Ok(()),
            Access::Declined => Err(
                formatx!(ACCESS_DECLINED, accessed_cmd = ngrok_cmd.description.clone()).unwrap()
            ),
        };

    if let Some(err) = access_check_result.as_ref().err() {
        reply_from_query(&bot, &query, err.to_string()).await;
    }

    access_check_result.is_ok()
}

async fn buttons_handler(
    bot: Bot,
    state: Arc<State>,
    query: CallbackQuery,
    ngrok_cmd: NgrokCmd,
) -> HandlerResult {
    let ngrok_connection_result = start_ngrok(&state.ngrok, &ngrok_cmd).await;

    if let Some(Message { id, chat, .. }) = query.message {
        let mut msg =
            bot.edit_message_text(chat.id, id, ngrok_connection_result.replace('.', r#"\."#));
        msg.parse_mode = Some(ParseMode::MarkdownV2);
        msg.send().await?;
    }

    Ok(())
}

async fn reply_from_query(bot: &Bot, query: &CallbackQuery, text: String) {
    if let Some(ref msg) = query.message {
        let resp = CommandResponse::WithoutMarkup { msg: text };
        reply(bot, msg, resp).await;
    }
}

async fn start_ngrok(ngrok: &Ngrok, cmd: &NgrokCmd) -> String {
    let ngrok_start_result = ngrok.start(&cmd.connection_type, cmd.port, true);

    log::info!("Chosen ngrok config: {:?}", cmd);
    sleep(Duration::from_millis(500)).await;
    log::info!("Ngrok started!");

    let answer = match ngrok_start_result {
        // TODO: Get rid of unwrap!
        Ok(result_str) => match ngrok.fetch_url().await {
            Ok(url) => Ok(formatx!(
                NGROK_TUNNEL_OBTAINED,
                connection_report = result_str,
                url = url.clone(),
                host = url.host_str().unwrap(),
                port = url.port().unwrap(),
                howto = cmd
                    .howto
                    .clone()
                    .unwrap_or_else(|| "Nothing special, just use it".into())
            )
            .unwrap()),
            Err(err) => Err(formatx!(NGROK_API_CONNECTION_ERROR, error_description = err).unwrap()),
        },
        Err(res) => Err(res),
    };

    answer.unwrap_or_else(|ans| {
        ngrok.kill();
        log::error!("{}", ans);
        ans
    })
}
