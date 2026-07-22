use std::sync::OnceLock;

use regex::Regex;
use unicase::UniCase;
use unicode_categories::UnicodeCategories;
use unicode_normalization::UnicodeNormalization;

static DEFAULT_IGNORABLE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();

pub(super) fn empty(value: &str) -> bool {
    canonical(value).is_empty()
}

pub(super) fn canonical(value: &str) -> String {
    let normalized = value.nfkc().collect::<String>();
    let mut has_base = false;
    UniCase::unicode(normalized)
        .to_folded_case()
        .chars()
        .filter_map(|character| {
            if is_non_material(character) {
                return None;
            }
            if character.is_mark() {
                return has_base.then_some(character);
            }
            has_base = true;
            Some(character)
        })
        .collect()
}

pub(super) fn nonempty_list(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| !empty(value))
}

fn is_cosmetic(character: char) -> bool {
    character.is_whitespace() || character.is_separator() || character.is_punctuation()
}

fn is_non_material(character: char) -> bool {
    is_default_ignorable(character)
        || character.is_other_control()
        || character.is_other_format()
        || is_cosmetic(character)
}

fn is_default_ignorable(character: char) -> bool {
    let mut utf8 = [0; 4];
    DEFAULT_IGNORABLE
        .get_or_init(|| Regex::new(r"\p{Default_Ignorable_Code_Point}"))
        .as_ref()
        .is_ok_and(|pattern| pattern.is_match(character.encode_utf8(&mut utf8)))
}
