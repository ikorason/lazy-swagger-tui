pub fn mask_token(token: &str) -> String {
    let len = token.len();
    if len <= 15 {
        // Too short to safely show, just show dots
        return "●".repeat(len);
    }

    let first = &token[..7];
    let last = &token[len - 6..];
    format!("{}...{}", first, last)
}
