use anyhow::{Result, bail};

pub(crate) const MAX_COMPONENT: u32 = 2_147_483_647;

/// Requires the repository's bounded, canonical MAJOR.MINOR.PATCH version.
pub(crate) fn require(version: &str) -> Result<()> {
    let mut parts = version.split('.');
    let valid = (0..3).all(|_| {
        let Some(part) = parts.next() else {
            return false;
        };
        !part.is_empty()
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && (part == "0" || !part.starts_with('0'))
            && part
                .parse::<u32>()
                .is_ok_and(|component| component <= MAX_COMPONENT)
    }) && parts.next().is_none();
    if valid {
        Ok(())
    } else {
        bail!("version must be semver-like MAJOR.MINOR.PATCH: {version:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::require;

    #[test]
    fn components_are_bounded_and_canonical() {
        for version in ["0.0.0", "2147483647.0.0"] {
            assert!(require(version).is_ok(), "{version}");
        }
        for version in ["2147483648.0.0", "999999999999999999999.0.0", "01.0.0"] {
            assert!(require(version).is_err(), "{version}");
        }
    }
}
