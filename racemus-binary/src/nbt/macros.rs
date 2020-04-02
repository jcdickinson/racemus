#[macro_export]
macro_rules! nbt_compound(
    { $($key:expr => $value:expr),* } => {
        {
            #[allow(unused_mut)]
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key[..].into(), $value);
            )*
            $crate::nbt::Value::Compound(m)
        }
     };
);

#[macro_export]
macro_rules! nbt_list(
    { $($value:expr),* } => {
        {
            #[allow(unused_mut)]
            let mut m = ::std::vec::Vec::new();
            $(
                m.push($value);
            )*
            $crate::nbt::Value::List(m[..].into())
        }
     };
);

#[macro_export]
macro_rules! nbt_byte_array(
    { $($value:expr),* } => {
        {
            #[allow(unused_mut)]
            let mut m = ::std::vec::Vec::new();
            $(
                m.push($value);
            )*
            $crate::nbt::Value::ByteArray(m[..].into())
        }
     };
);

#[macro_export]
macro_rules! nbt_int_array(
    { $($value:expr),* } => {
        {
            #[allow(unused_mut)]
            let mut m = ::std::vec::Vec::new();
            $(
                m.push($value);
            )*
            $crate::nbt::Value::IntArray(m[..].into())
        }
     };
);

#[macro_export]
macro_rules! nbt_long_array(
    { $($value:expr),* } => {
        {
            #[allow(unused_mut)]
            let mut m = ::std::vec::Vec::new();
            $(
                m.push($value);
            )*
            $crate::nbt::Value::LongArray(m[..].into())
        }
     };
);

#[macro_export]
macro_rules! nbt_byte(
    { $value:expr } => {
        {
            $crate::nbt::Value::Byte($value)
        }
     };
);

#[macro_export]
macro_rules! nbt_short(
    { $value:expr } => {
        {
            $crate::nbt::Value::Short($value)
        }
     };
);

#[macro_export]
macro_rules! nbt_int(
    { $value:expr } => {
        {
            $crate::nbt::Value::Int($value)
        }
     };
);

#[macro_export]
macro_rules! nbt_long(
    { $value:expr } => {
        {
            $crate::nbt::Value::Long($value)
        }
     };
);

#[macro_export]
macro_rules! nbt_float(
    { $value:expr } => {
        {
            $crate::nbt::Value::Float($value)
        }
     };
);

#[macro_export]
macro_rules! nbt_double(
    { $value:expr } => {
        {
            $crate::nbt::Value::Double($value)
        }
     };
);

#[macro_export]
macro_rules! nbt_string(
    { $value:expr } => {
        {
            $crate::nbt::Value::String($value[..].into())
        }
     };
);
