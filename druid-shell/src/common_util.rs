/// Strip the access keys from the menu string.
///
/// Changes "E&xit" to "Exit". Actual ampersands are escaped as "&&".
pub fn strip_access_key(raw_menu_text: &str) -> String {
    // TODO this is copied from mac/menu.rs maybe this should be moved somewhere common?
    let mut saw_ampersand = false;
    let mut result = String::new();
    for c in raw_menu_text.chars() {
        if c == '&' {
            if saw_ampersand {
                result.push(c);
            }
            saw_ampersand = !saw_ampersand;
        } else {
            result.push(c);
            saw_ampersand = false;
        }
    }
    result
}
