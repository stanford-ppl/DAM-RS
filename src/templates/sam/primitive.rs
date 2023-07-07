

use crate::types::DAMType;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Token<ValType, StopType> {
    Val(ValType),
    Stop(StopType),
    Empty,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Repsiggen {
    Repeat,
    Stop,
    Done,
}

impl<ValType: DAMType, StopType> From<ValType> for Token<ValType, StopType> {
    fn from(value: ValType) -> Self {
        Self::Val(value)
    }
}

impl<ValType, StopType: core::str::FromStr> TryFrom<&str> for Token<ValType, StopType> {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.starts_with("D") {
            Ok(Self::Done)
        } else if value.starts_with("N") {
            Ok(Self::Empty)
        } else if value.starts_with("S") {
            value[1..].parse().map(Self::Stop).map_err(|_| ())
        } else {
            Err(())
        }
    }
}

impl TryFrom<&str> for Repsiggen {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.starts_with("R") {
            Ok(Self::Repeat)
        } else if value.starts_with("S") {
            Ok(Self::Stop)
        } else if value.starts_with("D") {
            Ok(Self::Done)
        } else {
            Err(())
        }
    }
}

#[macro_export]
macro_rules! token_vec {
    [$toktype: tt; $stoptype: tt; $($val:expr),*] => {
        ({
            let mut res = Vec::new();
            $(
                {
                    res.push(Token::<$toktype, $stoptype>::try_from($val).unwrap());
                }
            )*
            res
        })
    };
}

#[macro_export]
macro_rules! repsig_vec {
    [$($val:expr),*] => {
        ({
            let mut res = Vec::new();
            $(
                {
                    res.push(Repsiggen::try_from($val).unwrap());
                }
            )*
            res
        })
    };
}

// pub(crate) use tvec;

fn tmp() {
    let _ = token_vec![u16; u16; 1, 2, 3, "S0", 4, 5, 6, "S1", "D"];
    let _ = repsig_vec!("R", "R", "S", "D");
}

impl<ValType: Default, StopType: Default> Default for Token<ValType, StopType> {
    fn default() -> Self {
        Token::Val(ValType::default())
    }
}

impl Default for Repsiggen {
    fn default() -> Self {
        Repsiggen::Repeat
    }
}

impl<ValType: DAMType, StopType: DAMType> DAMType for Token<ValType, StopType> {
    fn dam_size(&self) -> usize {
        2 + match self {
            Token::Val(val) => val.dam_size(),
            Token::Stop(stkn) => stkn.dam_size(),
            Token::Empty => 0,
            Token::Done => 0,
        }
    }
}

impl DAMType for Repsiggen {
    fn dam_size(&self) -> usize {
        2 + match self {
            // Not sure exact size beyond 2 bits so using match just in case to update later
            Repsiggen::Repeat => 0,
            Repsiggen::Stop => 0,
            Repsiggen::Done => 0,
        }
    }
}
