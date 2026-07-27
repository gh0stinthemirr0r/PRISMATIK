//! Apache-2.0 publish checklist floor (`P9-OD-01`).

use thiserror::Error;

/// Pre-publish gate for a PRISMATIK OSS crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishChecklist {
    /// Crate / package name under review.
    pub crate_name: String,
    /// SPDX / license field is Apache-2.0 (or approved equivalent).
    pub license_ok: bool,
    /// Root `NOTICE` (or equivalent attribution) is present and current.
    pub notice_ok: bool,
    /// Public API stability floor satisfied (semver / freeze policy).
    pub api_stability_ok: bool,
}

/// Publish gate failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PublishError {
    /// One or more checklist flags are false.
    #[error(
        "crate {crate_name} not publishable: license_ok={license_ok}, notice_ok={notice_ok}, api_stability_ok={api_stability_ok}"
    )]
    NotPublishable {
        /// Crate name.
        crate_name: String,
        /// License flag.
        license_ok: bool,
        /// NOTICE flag.
        notice_ok: bool,
        /// API stability flag.
        api_stability_ok: bool,
    },
}

impl PublishChecklist {
    /// Fail if any of `license_ok`, `notice_ok`, or `api_stability_ok` is false.
    pub fn assert_publishable(&self) -> Result<(), PublishError> {
        if self.license_ok && self.notice_ok && self.api_stability_ok {
            return Ok(());
        }
        Err(PublishError::NotPublishable {
            crate_name: self.crate_name.clone(),
            license_ok: self.license_ok,
            notice_ok: self.notice_ok,
            api_stability_ok: self.api_stability_ok,
        })
    }
}

/// Convenience wrapper around [`PublishChecklist::assert_publishable`].
pub fn assert_publishable(checklist: &PublishChecklist) -> Result<(), PublishError> {
    checklist.assert_publishable()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_checklist() -> PublishChecklist {
        PublishChecklist {
            crate_name: "prismatik-plugin-sdk".into(),
            license_ok: true,
            notice_ok: true,
            api_stability_ok: true,
        }
    }

    #[test]
    fn publishable_when_all_true() {
        assert_publishable(&ok_checklist()).unwrap();
    }

    #[test]
    fn fails_when_license_false() {
        let mut c = ok_checklist();
        c.license_ok = false;
        let err = assert_publishable(&c).unwrap_err();
        assert!(matches!(
            err,
            PublishError::NotPublishable {
                license_ok: false,
                notice_ok: true,
                api_stability_ok: true,
                ..
            }
        ));
    }

    #[test]
    fn fails_when_notice_false() {
        let mut c = ok_checklist();
        c.notice_ok = false;
        assert!(assert_publishable(&c).is_err());
    }

    #[test]
    fn fails_when_api_stability_false() {
        let mut c = ok_checklist();
        c.api_stability_ok = false;
        assert!(assert_publishable(&c).is_err());
    }

    #[test]
    fn fails_when_any_combination_false() {
        let mut c = ok_checklist();
        c.license_ok = false;
        c.notice_ok = false;
        c.api_stability_ok = false;
        let err = assert_publishable(&c).unwrap_err();
        assert_eq!(
            err,
            PublishError::NotPublishable {
                crate_name: "prismatik-plugin-sdk".into(),
                license_ok: false,
                notice_ok: false,
                api_stability_ok: false,
            }
        );
    }
}
