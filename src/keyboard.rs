use serde::{Deserialize, Serialize};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup};

use crate::{cfg::NgrokCmd, responds::BAD_INLINE_DATA_TYPE};

pub type CommandIdx = usize;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ButtonQuery {
    Ngrok { cmd_idx: CommandIdx },
}

impl TryFrom<&str> for ButtonQuery {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let res = serde_json::from_str::<ButtonQuery>(value);
        res.map_err(|_| BAD_INLINE_DATA_TYPE.into())
    }
}

pub fn make_ngrok_cmd_keyboard(cmds: &[(CommandIdx, NgrokCmd)]) -> InlineKeyboardMarkup {
    let keyboard: Vec<Vec<_>> = cmds
        .iter()
        .map(|(i, cmd)| {
            let data = ButtonQuery::Ngrok { cmd_idx: *i };
            vec![InlineKeyboardButton::callback(
                cmd.description.to_owned(),
                serde_json::to_string(&data).unwrap(),
            )]
        })
        .collect();

    InlineKeyboardMarkup::new(keyboard)
}

pub fn make_startup_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new(vec![vec![
        KeyboardButton::new("/ngrok"),
        KeyboardButton::new("/killngrok"),
    ]])
    .resize_keyboard(true)
}
