// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::{ValidFactor, ValidName};
use anyhow::Error;
use fake::{Dummy, Fake, Faker, StringFaker};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgHasArrayType;
use std::cmp::Ordering;

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone)]
#[sqlx(type_name = "score")]
pub struct Score {
    pub name: ValidName,
    pub factor: ValidFactor,
}

impl Score {
    pub fn new<T: AsRef<str>>(name: T, factor: f64) -> Result<Self, Error> {
        Ok(Score {
            name: ValidName::parse(name.as_ref().to_string())?,
            factor: ValidFactor::parse(factor)?,
        })
    }
}

impl PgHasArrayType for Score {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_score")
    }
}

impl TryFrom<ScoreTest> for Score {
    type Error = Error;

    fn try_from(value: ScoreTest) -> Result<Self, Self::Error> {
        Ok(Score {
            name: ValidName::parse(value.name.ok_or_else(|| anyhow::anyhow!("name is None"))?)?,
            factor: ValidFactor::parse(
                value
                    .factor
                    .ok_or_else(|| anyhow::anyhow!("factor is None"))?,
            )?,
        })
    }
}

impl PartialEq<Score> for Score {
    fn eq(&self, other: &Self) -> bool {
        let Score {
            name: s_name,
            factor: s_factor,
        } = self;
        let Score {
            name: o_name,
            factor: o_factor,
        } = other;

        let s_fac = f64::abs(*s_factor.as_ref());
        let o_fac = f64::abs(*o_factor.as_ref());

        let (diff, biggest) = if s_fac > o_fac {
            (s_fac - o_fac, s_fac)
        } else {
            (o_fac - s_fac, o_fac)
        };

        s_name.as_ref() == o_name.as_ref()
            && (diff < f64::EPSILON || diff < biggest * f64::EPSILON.sqrt())
    }
}

impl Eq for Score {}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.name.as_ref().cmp(other.name.as_ref()))
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.as_ref().cmp(other.name.as_ref())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ScoreTest {
    pub name: Option<String>,
    pub factor: Option<f64>,
}

impl ScoreTest {
    pub fn new() -> Self {
        ScoreTest {
            name: None,
            factor: None,
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_factor(mut self, factor: f64) -> Self {
        self.factor = Some(factor);
        self
    }
}

impl PartialOrd for ScoreTest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            self.name
                .as_ref()
                .unwrap()
                .cmp(other.name.as_ref().unwrap()),
        )
    }
}

impl Ord for ScoreTest {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name
            .as_ref()
            .unwrap()
            .cmp(other.name.as_ref().unwrap())
    }
}

impl PartialEq<ScoreTest> for ScoreTest {
    fn eq(&self, other: &Self) -> bool {
        let ScoreTest {
            name: s_name,
            factor: s_factor,
        } = self;
        let ScoreTest {
            name: o_name,
            factor: o_factor,
        } = other;

        let s_fac = f64::abs(*s_factor.as_ref().unwrap());
        let o_fac = f64::abs(*o_factor.as_ref().unwrap());

        let (diff, biggest) = if s_fac > o_fac {
            (s_fac - o_fac, s_fac)
        } else {
            (o_fac - s_fac, o_fac)
        };

        s_name.as_ref().unwrap() == o_name.as_ref().unwrap()
            && (diff < f64::EPSILON || diff < biggest * f64::EPSILON.sqrt())
    }
}

impl Eq for ScoreTest {}

impl PartialEq<Score> for ScoreTest {
    fn eq(&self, other: &Score) -> bool {
        let ScoreTest {
            name: s_name,
            factor: s_factor,
        } = self;
        let Score {
            name: o_name,
            factor: o_factor,
        } = other;

        // Can't be equal if any field in ScoreTest is None
        if s_name.is_none() || s_factor.is_none() {
            return false;
        }

        let s_fac = f64::abs(*s_factor.as_ref().unwrap());
        let o_fac = f64::abs(*o_factor.as_ref());

        let (diff, biggest) = if s_fac > o_fac {
            (s_fac - o_fac, s_fac)
        } else {
            (o_fac - s_fac, o_fac)
        };

        s_name.as_ref().unwrap() == o_name.as_ref()
            && (diff < f64::EPSILON || diff < biggest * f64::EPSILON.sqrt())
    }
}

impl PartialEq<ScoreTest> for Score {
    fn eq(&self, other: &ScoreTest) -> bool {
        other.eq(self)
    }
}

impl Dummy<Faker> for ScoreTest {
    fn dummy_with_rng<R: Rng + ?Sized>(_: &Faker, rng: &mut R) -> ScoreTest {
        let name = StringFaker::with(
            String::from("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789*&^%$#@!~")
                .into_bytes(),
            1..256,
        )
        .fake_with_rng(rng);
        ScoreTest {
            name: Some(name),
            factor: Some((0.0..f64::MAX).fake_with_rng(rng)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::assert_ok;

    impl quickcheck::Arbitrary for ScoreTest {
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            Faker.fake()
        }
    }

    #[quickcheck]
    fn a_valid_name_is_parsed_successfully(score: ScoreTest) {
        assert_ok!(Score::try_from(score));
    }
}
