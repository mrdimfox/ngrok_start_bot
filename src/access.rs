use teloxide::types::{User, Chat};

use crate::cfg::NgrokCmd;

pub enum Access {
    Granted,
    Declined
}

pub fn check_user_access(user: &User, cmd: &NgrokCmd) -> Access {
    if cmd.permitted_users.contains(&user.id.0) {
        Access::Granted
    }
    else {
        Access::Declined
    }
}

pub fn check_chat_access(chat: &Chat, permitted_chats: &[i64])-> Access {
    if permitted_chats.contains(&chat.id.0) {
        Access::Granted
    }
    else {
        Access::Declined
    }
}
