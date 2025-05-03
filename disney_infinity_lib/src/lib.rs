use hidapi::HidError;

pub(crate) mod commands{
    pub const SOLID_COLOR : u8 = 0x90;
    pub const ACTIVATE : u8 = 0x80;
    pub const INDEXES : u8 = 0xa1;
    pub const RESPONSE : u8 = 0xaa;
    pub const GET_TAG : u8 = 0xb4;
    pub const NOTIFY_OBSERVERS : u8 = 0xab;

}
#[derive(Debug)]
pub enum InfinityError{
    TexPoison,
    HidError(HidError)
}
impl From<HidError> for InfinityError{
    fn from(value: HidError) -> Self {
        Self::HidError(value)
    }
}
pub type InfnityResult<T> = Result<T, InfinityError>;
//pub(crate) const TIMEOUT_MS : i32 = 1;
pub(crate) const ACTIVATE_MESSAGE : &str = "(c) Disney 2013";

#[cfg(feature = "threaded")]
pub use infinity_base::*;
#[cfg(feature = "threaded")]
mod infinity_comms;
#[cfg(feature = "threaded")]
mod infinity_base;

#[cfg(feature = "async")]
mod async_base;
#[cfg(feature = "async")]
mod async_comms;
#[cfg(feature= "async")]
pub use async_base::*;