pub(super) fn unquoted_text(text: &str) -> String {
    let mut quote = None;
    text.chars()
        .map(|character| match quote {
            Some(delimiter) if character == delimiter => {
                quote = None;
                ' '
            }
            Some(_) => ' ',
            None if character == '"' => {
                quote = Some(character);
                ' '
            }
            None => character,
        })
        .collect()
}
