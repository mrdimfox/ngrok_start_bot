use teloxide::{
    types::{ReplyMarkup, User},
    utils::command::BotCommands,
};

use crate::{
    access::{check_user_access, Access},
    cfg::NgrokCmds,
    keyboard::{make_ngrok_cmd_keyboard, make_startup_keyboard},
    ngrok::Ngrok,
};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "Start")]
    Start,
    #[command(description = "Launch ngrok")]
    Ngrok,
    #[command(description = "Kill ngrok")]
    KillNgrok,
    #[command(description = "Help me")]
    Help,
}

impl Command {
    fn gather_commands_as_str() -> String {
        Command::bot_commands()
            .iter()
            .map(|c| c.command.clone())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

pub enum CommandResponse {
    WithMarkup { msg: String, keyboard: ReplyMarkup },
    WithoutMarkup { msg: String },
}

pub fn start_cmd() -> CommandResponse {
    CommandResponse::WithMarkup {
        msg: "🤖 Press /ngrok".into(),
        keyboard: make_startup_keyboard().into(),
    }
}

pub fn list_ngrok_cmd(user: Option<&User>, ngrok_cmds: &NgrokCmds) -> CommandResponse {
    if let Some(user) = user {
        let allowed_cmds: Vec<_> = ngrok_cmds
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, cmd)| (index, cmd))
            .filter(|(_, cmd)| match check_user_access(user, cmd) {
                Access::Granted => true,
                Access::Declined => false,
            })
            .collect();

        if allowed_cmds.is_empty() {
            return CommandResponse::WithoutMarkup {
                msg: "🤖 No commands for you, pal. Sorry. Ask permission from chat owner, maybe?"
                    .into(),
            };
        }

        CommandResponse::WithMarkup {
            msg: "🤖 Choose ngrok config to start expose target:".into(),
            keyboard: make_ngrok_cmd_keyboard(&allowed_cmds).into(),
        }
    } else {
        CommandResponse::WithoutMarkup {
            msg: "🤖 I don't know who the fuck are you. Sorry, mate. Don't speak to strangers."
                .into(),
        }
    }
}

pub fn kill_ngrok_cmd(ngrok: &Ngrok) -> CommandResponse {
    if ngrok.is_run() {
        ngrok.kill();
        CommandResponse::WithoutMarkup {
            msg: "🫡 Ngrok killed!".into(),
        }
    } else {
        CommandResponse::WithoutMarkup {
            msg: "💀 Ngrok actually dead...".into(),
        }
    }
}

pub fn help_cmd() -> CommandResponse {
    CommandResponse::WithoutMarkup {
        msg: format!("🤖 Available commands:\n{}.", {
            Command::gather_commands_as_str()
        }),
    }
}

pub fn error_cmd() -> CommandResponse {
    CommandResponse::WithoutMarkup {
        msg: format!(
            "💅🏻 This command does not exist, you dummy!\
            \n\nHere are available commands:\n{}.",
            { Command::gather_commands_as_str() }
        ),
    }
}
