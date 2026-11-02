//! The minimum version of omp and the semver comparison against what `omp --version` prints.
//!
//! `omp --version` prints `omp/<version>`. A version that cannot be read counts as outdated
//! (`requirements/tech#omp`): Weaver has no fallback for older omp.

use std::cmp::Ordering;

/// The oldest omp Weaver works with. One constant, compared as semver.
pub const MIN_OMP_VERSION: &str = "18.15.0";

/// A semantic version: `major.minor.patch`, an optional `-prerelease`, build metadata ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    /// Dot-separated identifiers after `-`; empty for a release.
    pre: Vec<String>,
}

impl Version {
    /// Parses `1.2.3`, `1.2.3-rc.1` and `1.2.3+build`. A leading `v` is accepted.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut numbers = core.split('.');
        let major = number(numbers.next()?)?;
        let minor = number(numbers.next()?)?;
        let patch = number(numbers.next()?)?;
        if numbers.next().is_some() {
            return None;
        }
        let pre = match pre {
            None => Vec::new(),
            Some(pre) => {
                let identifiers: Vec<String> = pre.split('.').map(str::to_owned).collect();
                let valid = identifiers.iter().all(|id| {
                    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                });
                if !valid {
                    return None;
                }
                identifiers
            }
        };
        Some(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    /// The version Weaver needs at least.
    pub fn minimum() -> Self {
        Self::parse(MIN_OMP_VERSION).expect("MIN_OMP_VERSION is a valid version")
    }
}

fn number(text: &str) -> Option<u64> {
    if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                // A release is newer than any of its prereleases.
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => compare_prerelease(&self.pre, &other.pre),
            })
    }
}

fn compare_prerelease(a: &[String], b: &[String]) -> Ordering {
    for (left, right) in a.iter().zip(b) {
        let order = match (number(left), number(right)) {
            (Some(x), Some(y)) => x.cmp(&y),
            // Numeric identifiers are older than alphanumeric ones.
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => left.cmp(right),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        Ok(())
    }
}

/// The version in the output of `omp --version` (`omp/18.15.0`): the first non-empty line, after
/// `omp/`, up to the first space. `None` when the output has another shape.
pub fn parse_omp_version(stdout: &str) -> Option<Version> {
    let line = stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    let rest = line.strip_prefix("omp/")?;
    Version::parse(rest.split_whitespace().next()?)
}

/// Whether a version read from `omp --version` is at least [`MIN_OMP_VERSION`]; an unreadable
/// version (`None`) is not.
pub fn is_supported(version: Option<&Version>) -> bool {
    version.is_some_and(|v| *v >= Version::minimum())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn the_minimum_is_a_valid_version() {
        assert_eq!(Version::minimum().to_string(), MIN_OMP_VERSION);
    }

    #[test]
    fn versions_compare_as_semver() {
        assert!(v("18.15.0") >= Version::minimum());
        assert!(v("18.14.9") < Version::minimum());
        assert!(v("18.15.1") > v("18.15.0"));
        assert!(v("19.0.0") > v("18.99.99"));
        // Numeric, not textual: 18.100.0 is newer than 18.15.0.
        assert!(v("18.100.0") > v("18.15.0"));
        assert!(v("18.15.0-rc.1") < v("18.15.0"));
        assert!(v("1.0.0-alpha") < v("1.0.0-alpha.1"));
        assert!(v("1.0.0-alpha.1") < v("1.0.0-alpha.beta"));
        assert!(v("1.0.0-2") < v("1.0.0-10"));
        assert_eq!(v("1.0.0+build.5"), v("1.0.0"));
    }

    #[test]
    fn text_that_is_not_semver_is_refused() {
        for bad in [
            "",
            "18",
            "18.15",
            "18.15.0.1",
            "a.b.c",
            "18.15.x",
            "1.0.0-",
            "1.0.0-a..b",
            "-1.0.0",
        ] {
            assert_eq!(Version::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn omp_version_output_is_read_after_the_prefix() {
        assert_eq!(parse_omp_version("omp/18.15.0\n"), Some(v("18.15.0")));
        assert_eq!(
            parse_omp_version("\n  omp/19.0.1 (build 5)\nmore"),
            Some(v("19.0.1"))
        );
        assert_eq!(parse_omp_version("18.15.0"), None);
        assert_eq!(parse_omp_version("bun/1.2.3"), None);
        assert_eq!(parse_omp_version(""), None);
    }

    #[test]
    fn an_unreadable_version_is_outdated() {
        assert!(!is_supported(None));
        assert!(!is_supported(Some(&v("18.14.0"))));
        assert!(is_supported(Some(&v("18.15.0"))));
        assert!(is_supported(Some(&v("20.1.0"))));
    }
}
