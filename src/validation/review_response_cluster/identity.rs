use unicase::UniCase;
use unicode_categories::UnicodeCategories;
use unicode_normalization::UnicodeNormalization;

pub(super) fn empty(value: &str) -> bool {
    canonical(value).is_empty()
}

pub(super) fn canonical(value: &str) -> String {
    let normalized = value.nfkc().collect::<String>();
    UniCase::unicode(normalized)
        .to_folded_case()
        .chars()
        .filter(|character| !is_cosmetic(*character))
        .collect()
}

pub(super) fn nonempty_list(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| !empty(value))
}

fn is_cosmetic(character: char) -> bool {
    character.is_whitespace() || character.is_separator() || character.is_punctuation()
}
