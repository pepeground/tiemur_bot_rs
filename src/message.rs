use telegram_bot::{Api, Message, CanReplySendMessage};

pub fn process(message: Message, api: &Api) {
    api.spawn(message.text_reply("Hello World"));
}
