use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

// Copied from main.rs

/// Creates a keyboard made by buttons in a big column.
pub fn make_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    // Original from main.rs has "Подписаться", "Отписаться"
    let possible_actions = ["Подписаться", "Отписаться"];

    // The chunks(3) was from an earlier version in main.rs, for 2 actions, chunks(2) or just one row is fine.
    // Let's keep it as it was in your main.rs for consistency for now.
    for actions in possible_actions.chunks(3) {
        let row = actions
            .iter()
            .map(|&action| InlineKeyboardButton::callback(action.to_owned(), action.to_owned()))
            .collect();
        keyboard.push(row);
    }
    InlineKeyboardMarkup::new(keyboard)
}

pub fn make_unsubscribe_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];
    let subscribe = // Was `button` then reverted to `subscribe`
        InlineKeyboardButton::callback("Отписаться".to_owned(), "Отписаться".to_owned());
    keyboard.push(vec![subscribe]);
    InlineKeyboardMarkup::new(keyboard)
}

pub fn make_subscribe_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];
    let subscribe = // Was `button` then reverted to `subscribe`
        InlineKeyboardButton::callback("Подписаться".to_owned(), "Подписаться".to_owned());
    keyboard.push(vec![subscribe]);
    InlineKeyboardMarkup::new(keyboard)
}
