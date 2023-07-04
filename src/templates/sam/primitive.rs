use std::cmp::max;

use crate::types::DAMType;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Token<ValType, StopType> {
    Val(ValType),
    Stop(StopType),
    Empty,
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

// pub(crate) use tvec;

fn tmp() {
    let _ = token_vec![u16; u16; 1, 2, 3, "S0", 4, 5, 6, "S1", "D"];
}

impl<ValType: Default, StopType: Default> Default for Token<ValType, StopType> {
    fn default() -> Self {
        Token::Val(ValType::default())
        // panic!("Wrong default used for token");
    }
}

impl<ValType: DAMType, StopType: DAMType> DAMType for Token<ValType, StopType> {
    fn dam_size() -> usize {
        max(ValType::dam_size(), StopType::dam_size()) + 1
    }
}
