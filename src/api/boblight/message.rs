use std::{convert::TryFrom, str::FromStr};

use thiserror::Error;

use crate::models::{Color, Led};

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("invalid request")]
    InvalidRequest,
    #[error("invalid get")]
    InvalidGet,
    #[error("invalid set")]
    InvalidSet,
    #[error("invalid light param")]
    InvalidLightParam,
    #[error("invalid priority")]
    InvalidPriority,
    #[error("invalid index")]
    InvalidIndex,
    #[error("invalid color")]
    InvalidColor,
    #[error("not enough data")]
    NotEnoughData,
}

#[derive(Debug)]
pub enum GetArg {
    Version,
    Lights,
}

impl TryFrom<&[&str]> for GetArg {
    type Error = DecodeError;

    fn try_from(value: &[&str]) -> Result<Self, Self::Error> {
        match value.get(0).map(|s| *s) {
            Some("version") => Ok(Self::Version),
            Some("lights") => Ok(Self::Lights),
            _ => Err(DecodeError::InvalidGet),
        }
    }
}

#[derive(Debug)]
pub struct LightParam {
    pub index: usize,
    pub data: LightParamData,
}

impl TryFrom<&[&str]> for LightParam {
    type Error = DecodeError;

    fn try_from(value: &[&str]) -> Result<Self, Self::Error> {
        let index = value
            .get(0)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| DecodeError::InvalidIndex)?;

        if value.len() <= 1 {
            return Err(DecodeError::NotEnoughData);
        }

        Ok(Self {
            index,
            data: LightParamData::try_from(&value[1..])?,
        })
    }
}

#[derive(Debug)]
pub enum LightParamData {
    Color(Color),
    Speed,
    Interpolation,
    Use,
    SingleChange,
}

impl TryFrom<&[&str]> for LightParamData {
    type Error = DecodeError;

    fn try_from(value: &[&str]) -> Result<Self, Self::Error> {
        match value.get(0).map(|s| *s) {
            Some("color") => match value.get(1).map(|s| *s) {
                Some("rgb") => {
                    let r = value
                        .get(2)
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| DecodeError::InvalidColor)?;
                    let g = value
                        .get(3)
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| DecodeError::InvalidColor)?;
                    let b = value
                        .get(4)
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| DecodeError::InvalidColor)?;

                    Ok(Self::Color(Color::new(r, g, b)))
                }
                _ => Err(DecodeError::InvalidColor),
            },
            Some("speed") => Ok(Self::Speed),
            Some("interpolation") => Ok(Self::Interpolation),
            Some("use") => Ok(Self::Use),
            Some("singlechange") => Ok(Self::SingleChange),
            _ => Err(DecodeError::InvalidLightParam),
        }
    }
}

#[derive(Debug)]
pub enum SetArg {
    Light(LightParam),
    Priority(i32),
}

impl TryFrom<&[&str]> for SetArg {
    type Error = DecodeError;

    fn try_from(value: &[&str]) -> Result<Self, Self::Error> {
        match value.get(0).map(|s| *s) {
            Some("light") => {
                if value.len() <= 1 {
                    return Err(DecodeError::NotEnoughData);
                }

                Ok(Self::Light(LightParam::try_from(&value[1..])?))
            }
            Some("priority") => {
                let priority = value
                    .get(1)
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| DecodeError::InvalidPriority)?;

                Ok(Self::Priority(priority))
            }
            _ => Err(DecodeError::InvalidSet),
        }
    }
}

#[derive(Debug)]
pub enum BoblightRequest {
    Hello,
    Ping,
    Get(GetArg),
    Set(SetArg),
    Sync,
}

impl FromStr for BoblightRequest {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let spans: Vec<_> = s
            .trim()
            .split(' ')
            .filter(|s| !s.trim().is_empty())
            .collect();

        match spans.get(0).map(|s| *s) {
            Some("hello") => Ok(Self::Hello),
            Some("ping") => Ok(Self::Ping),
            Some("get") => {
                if spans.len() <= 1 {
                    return Err(DecodeError::NotEnoughData);
                }

                Ok(Self::Get(GetArg::try_from(&spans[1..])?))
            }
            Some("set") => {
                if spans.len() <= 1 {
                    return Err(DecodeError::NotEnoughData);
                }

                Ok(Self::Set(SetArg::try_from(&spans[1..])?))
            }
            Some("sync") => Ok(Self::Sync),
            _ => Err(DecodeError::InvalidRequest),
        }
    }
}

#[derive(Debug)]
pub enum BoblightResponse {
    Hello,
    Ping,
    Version,
    Lights { leds: Vec<Led> },
}

impl std::fmt::Display for BoblightResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoblightResponse::Hello => write!(f, "hello"),
            BoblightResponse::Ping => write!(f, "ping 1"),
            BoblightResponse::Version => write!(f, "version 5"),
            BoblightResponse::Lights { leds } => {
                let n = leds.len();
                if n > 0 {
                    writeln!(f, "lights {}", n)?;

                    for (i, led) in leds.iter().enumerate() {
                        write!(
                            f,
                            "light {:03} scan {} {} {} {}",
                            i,
                            led.hmin * 100.,
                            led.hmax * 100.,
                            led.vmin * 100.,
                            led.vmax * 100.
                        )?;

                        if i < n - 1 {
                            writeln!(f)?;
                        }
                    }

                    Ok(())
                } else {
                    write!(f, "lights 0")
                }
            }
        }
    }
}
