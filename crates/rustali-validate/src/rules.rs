//! Rule predicates and default messages used by `#[derive(Validate)]`.
//!
//! Messages follow Laravel's wording so documentation and muscle memory carry over;
//! the i18n package will make them translatable.

use std::fmt::Display;

/// Turn `first_name` into `first name` for messages.
#[must_use]
pub fn humanize(field: &str) -> String {
    field.replace(['_', '-'], " ")
}

/// `required`: a string must contain a non-whitespace character.
#[must_use]
pub fn required_str(value: &str) -> bool {
    !value.trim().is_empty()
}

/// `required` on `Option<T>`: must be `Some`; strings inside must be non-blank.
#[must_use]
pub fn required_opt<T: AsRef<str>>(value: Option<&T>) -> bool {
    value.is_some_and(|v| required_str(v.as_ref()))
}

/// `required` on any other `Option<T>`.
#[must_use]
pub const fn required_some<T>(value: Option<&T>) -> bool {
    value.is_some()
}

/// `length(min, max)` counts characters, not bytes.
#[must_use]
pub fn length(value: &str, min: Option<usize>, max: Option<usize>) -> bool {
    let len = value.chars().count();
    min.is_none_or(|m| len >= m) && max.is_none_or(|m| len <= m)
}

/// `range(min, max)` on numbers.
#[must_use]
pub fn range(value: f64, min: Option<f64>, max: Option<f64>) -> bool {
    min.is_none_or(|m| value >= m) && max.is_none_or(|m| value <= m)
}

/// `email`: pragmatic shape check (`local@domain.tld`), not RFC 5322.
#[must_use]
pub fn email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !local.contains(char::is_whitespace)
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains(char::is_whitespace)
        && !domain.contains("..")
}

/// `url`: `http://` or `https://` followed by a host.
#[must_use]
pub fn url(value: &str) -> bool {
    let rest = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .unwrap_or("");
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    !host.is_empty() && !host.contains(char::is_whitespace)
}

/// `one_of`: value must be one of the allowed options.
#[must_use]
pub fn one_of(value: &str, allowed: &[&str]) -> bool {
    allowed.contains(&value)
}

/// `alpha_num`: letters and digits only.
#[must_use]
pub fn alpha_num(value: &str) -> bool {
    !value.is_empty() && value.chars().all(char::is_alphanumeric)
}

/// Default messages.
pub mod messages {
    use super::humanize;

    /// `required`
    #[must_use]
    pub fn required(field: &str) -> String {
        format!("The {} field is required.", humanize(field))
    }

    /// `length`
    #[must_use]
    pub fn length(field: &str, min: Option<usize>, max: Option<usize>) -> String {
        let field = humanize(field);
        match (min, max) {
            (Some(min), Some(max)) => {
                format!("The {field} must be between {min} and {max} characters.")
            }
            (Some(min), None) => format!("The {field} must be at least {min} characters."),
            (None, Some(max)) => format!("The {field} may not be greater than {max} characters."),
            (None, None) => format!("The {field} has an invalid length."),
        }
    }

    /// `range`
    #[must_use]
    pub fn range(field: &str, min: Option<f64>, max: Option<f64>) -> String {
        let field = humanize(field);
        let fmt = |n: f64| {
            if n.fract() == 0.0 {
                format!("{n:.0}")
            } else {
                n.to_string()
            }
        };
        match (min, max) {
            (Some(min), Some(max)) => {
                format!("The {field} must be between {} and {}.", fmt(min), fmt(max))
            }
            (Some(min), None) => format!("The {field} must be at least {}.", fmt(min)),
            (None, Some(max)) => format!("The {field} may not be greater than {}.", fmt(max)),
            (None, None) => format!("The {field} is out of range."),
        }
    }

    /// `email`
    #[must_use]
    pub fn email(field: &str) -> String {
        format!("The {} must be a valid email address.", humanize(field))
    }

    /// `url`
    #[must_use]
    pub fn url(field: &str) -> String {
        format!("The {} must be a valid URL.", humanize(field))
    }

    /// `one_of`
    #[must_use]
    pub fn one_of(field: &str, allowed: &[&str]) -> String {
        format!(
            "The selected {} is invalid (allowed: {}).",
            humanize(field),
            allowed.join(", ")
        )
    }

    /// `alpha_num`
    #[must_use]
    pub fn alpha_num(field: &str) -> String {
        format!(
            "The {} may only contain letters and numbers.",
            humanize(field)
        )
    }

    /// `confirmed`
    #[must_use]
    pub fn confirmed(field: &str) -> String {
        format!("The {} confirmation does not match.", humanize(field))
    }

    /// `custom` rules return their own message; this wraps a bare `Display`.
    pub fn custom(message: impl super::Display) -> String {
        message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_rules() {
        assert!(required_str("x") && !required_str("  "));
        assert!(
            required_opt(Some(&"x")) && !required_opt(Some(&" ")) && !required_opt::<&str>(None)
        );
        assert!(length("héllo", Some(5), Some(5)));
        assert!(!length("hi", Some(3), None));
        assert!(email("a@b.co") && !email("a@b") && !email("@b.co") && !email("a@.b.co"));
        assert!(url("https://x.io/p?q") && !url("ftp://x") && !url("http://"));
        assert!(one_of("b", &["a", "b"]) && !one_of("c", &["a", "b"]));
        assert!(alpha_num("abc123") && !alpha_num("a b"));
    }

    #[test]
    fn number_rules() {
        assert!(range(3.0, Some(1.0), Some(5.0)));
        assert!(!range(0.5, Some(1.0), None));
        assert!(!range(6.0, None, Some(5.0)));
    }

    #[test]
    fn messages_read_like_laravel() {
        assert_eq!(
            messages::required("first_name"),
            "The first name field is required."
        );
        assert_eq!(
            messages::length("title", Some(3), Some(200)),
            "The title must be between 3 and 200 characters."
        );
        assert_eq!(
            messages::range("rating", Some(1.0), Some(5.0)),
            "The rating must be between 1 and 5."
        );
        assert_eq!(
            messages::range("price", Some(0.5), None),
            "The price must be at least 0.5."
        );
        assert_eq!(
            messages::confirmed("password"),
            "The password confirmation does not match."
        );
    }
}
