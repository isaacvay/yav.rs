//! Garde validation bridged into the framework's [`FormErrors`] type.
//!
//! This module is only compiled with the `validation` Cargo feature. It keeps
//! `garde` behind that feature while reusing the same error container that the
//! form/action layer already renders.
#![cfg(feature = "validation")]

use crate::forms::{FormErrors, FromFormData};

/// Validate `value` with garde and convert its `garde::Report` into [`FormErrors`].
///
/// Every `(path, error)` pair reported by garde becomes one entry keyed by the
/// stringified field path (`email`, `addresses[0].city`, ...), so existing form
/// error rendering keeps working unchanged. `garde::Validate::validate()`
/// requires a defaultable context, hence the extra bound on `T::Context`.
///
/// If a report unexpectedly contains no entries, a single global `_form`
/// message is produced from `report.to_string()` rather than silently treating
/// the value as valid.
pub fn validate_with_garde<T>(value: &T) -> Result<(), FormErrors>
where
    T: garde::Validate,
    T::Context: Default,
{
    match value.validate() {
        Ok(()) => Ok(()),
        Err(report) => {
            let mut errors = FormErrors::new();
            for (path, error) in report.iter() {
                errors.add(path.to_string(), error.to_string());
            }
            if !errors.has_errors() {
                errors.add("_form", report.to_string());
            }
            Err(errors)
        }
    }
}

/// Marker trait for form payloads that are both parseable from form data and
/// garde-validatable.
///
/// The blanket implementation below means any type satisfying both bounds gets
/// this marker automatically; there is nothing to implement manually.
pub trait ValidatedForm: FromFormData + garde::Validate {}

impl<T> ValidatedForm for T where T: FromFormData + garde::Validate {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Implemented by hand so the tests do not depend on garde's `derive`
    /// feature (which is not enabled by the workspace declaration).
    #[derive(Debug, Default)]
    struct SignupForm {
        name: String,
        email: String,
    }

    impl garde::Validate for SignupForm {
        type Context = ();

        fn validate_into(
            &self,
            _ctx: &Self::Context,
            _parent: &mut dyn FnMut() -> garde::Path,
            report: &mut garde::Report,
        ) {
            if self.name.trim().is_empty() {
                report.append(
                    garde::Path::new("name"),
                    garde::Error::new("name is required"),
                );
            }
            if !self.email.contains('@') {
                report.append(
                    garde::Path::new("email"),
                    garde::Error::new("email is invalid"),
                );
            }
        }
    }

    impl FromFormData for SignupForm {
        fn from_form(_form: &crate::forms::FormData) -> Result<Self, FormErrors> {
            Ok(Self::default())
        }
    }

    #[test]
    fn accepts_a_valid_value() {
        let form = SignupForm {
            name: "Ada".to_string(),
            email: "ada@example.com".to_string(),
        };
        assert!(validate_with_garde(&form).is_ok());
    }

    #[test]
    fn maps_report_paths_and_messages_into_form_errors() {
        let form = SignupForm {
            name: "  ".to_string(),
            email: "not-an-email".to_string(),
        };
        let errors = validate_with_garde(&form).expect_err("two invalid fields");

        assert!(errors.has_errors());
        assert_eq!(errors.all().len(), 2);
        assert_eq!(errors.first("name"), Some("name is required"));
        assert_eq!(errors.first("email"), Some("email is invalid"));
    }

    #[test]
    fn blanket_marker_trait_applies() {
        fn assert_validated_form<T: ValidatedForm>() {}
        assert_validated_form::<SignupForm>();
    }
}
