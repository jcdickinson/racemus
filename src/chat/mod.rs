use serde_json::json;

pub fn trivial(chat: &str) -> Result<String, serde_json::error::Error> {
    let v = json!({
        "text": chat
    });
    serde_json::to_string(&v)
}

