use anyhow::anyhow;
use data_encoding::BASE64;
use smallvec::SmallVec;
use std::{collections::HashMap, fmt::Debug, hash::Hash, path::Path};

#[derive(Debug, Clone)]
pub enum Reduced {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec(Vec<Reduced>),
    Object(HashMap<String, Reduced>),
}

impl Reduced {
    // TODO: Write serde Serializer instead, so we can skip the intermediate representation
    pub fn to_serde_json(self) -> serde_json::Value {
        match self {
            Self::Bool(this) => serde_json::Value::from(this),
            Self::Int(this) => serde_json::Value::from(this),
            Self::Float(this) => serde_json::Value::from(this),
            Self::String(this) => serde_json::Value::from(this),
            Self::Vec(this) => {
                let j_items: Vec<_> = this.into_iter().map(Reduced::to_serde_json).collect();
                serde_json::Value::from(j_items)
            }
            Self::Object(this) => {
                let j_map = this
                    .into_iter()
                    .map(|(key, item)| (key, item.to_serde_json()));
                serde_json::Value::from_iter(j_map)
            }
            Self::Null => serde_json::Value::Null,
        }
    }

    // TODO: Write a serde Deserializer instead
    pub fn from_serde_json(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(n_i64) = n.as_i64() {
                    Self::Int(n_i64)
                } else if let Some(n_f64) = n.as_f64() {
                    Self::Float(n_f64)
                } else {
                    unreachable!()
                }
            }
            serde_json::Value::String(s) => Self::String(s),
            serde_json::Value::Array(a) => {
                let items: Vec<_> = a.into_iter().map(Self::from_serde_json).collect();
                Self::Vec(items)
            }
            serde_json::Value::Object(o) => {
                let map: HashMap<String, Self> = o
                    .into_iter()
                    .map(|(key, value)| (key, Self::from_serde_json(value)))
                    .collect();
                Self::Object(map)
            }
        }
    }

    pub fn from_json_str(json_str: &str) -> anyhow::Result<Self> {
        let j_value: serde_json::Value = serde_json::from_str(json_str)?;
        Ok(Self::from_serde_json(j_value))
    }

    pub fn from_json_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let json_str = std::fs::read_to_string(path)?;
        Self::from_json_str(&json_str)
    }

    pub fn save(self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let j_value = self.to_serde_json();
        let json_str = serde_json::to_string_pretty(&j_value)?;
        std::fs::write(path, &json_str)?;
        Ok(())
    }

    pub fn object_from<const N: usize>(pairs: [(&str, Self); N]) -> Self {
        let map: HashMap<_, _> = pairs
            .into_iter()
            .map(|(key, reduced)| (key.to_owned(), reduced))
            .collect();
        Self::Object(map)
    }

    pub fn object_one(key: &str, value: Self) -> Self {
        let map = HashMap::from([(key.to_owned(), value)]);
        Self::Object(map)
    }

    pub fn vec_from<const N: usize>(items: [Self; N]) -> Self {
        Self::Vec(Vec::from(items))
    }

    pub fn as_slice(&self) -> anyhow::Result<&[Self]> {
        match self {
            Self::Vec(vec) => Ok(vec.as_slice()),
            _ => Err(anyhow::anyhow!("Not a Vec")),
        }
    }

    pub fn as_slice_with_len(&self, len: usize) -> anyhow::Result<&[Self]> {
        let slice = self.as_slice()?;
        if slice.len() != len {
            Err(anyhow!("Wrong len"))
        } else {
            Ok(slice)
        }
    }

    pub fn as_array<const N: usize>(&self) -> anyhow::Result<[&Self; N]> {
        let small_vec: SmallVec<[&Self; N]> = self.as_slice_with_len(N)?.iter().collect();
        Ok(small_vec.into_inner().unwrap())
    }

    pub fn as_map(&self) -> anyhow::Result<&HashMap<String, Self>> {
        match self {
            Self::Object(map) => Ok(map),
            _ => Err(anyhow::anyhow!("Not an Object")),
        }
    }

    pub fn as_i64(&self) -> anyhow::Result<i64> {
        match self {
            Self::Int(i) => Ok(*i),
            _ => Err(anyhow!("Not an i64")),
        }
    }

    pub fn as_f64(&self) -> anyhow::Result<f64> {
        match self {
            Self::Float(f) => Ok(*f),
            _ => Err(anyhow!("Not an f64")),
        }
    }

    pub fn as_bool(&self) -> anyhow::Result<bool> {
        match self {
            Self::Bool(b) => Ok(*b),
            _ => Err(anyhow!("Not a bool")),
        }
    }

    pub fn as_string(&self) -> anyhow::Result<&String> {
        match self {
            Reduced::String(s) => Ok(s),
            _ => Err(anyhow!("Not a String")),
        }
    }

    pub fn get_index(&self, index: usize) -> anyhow::Result<&Reduced> {
        self.as_slice()?
            .get(index)
            .ok_or_else(|| anyhow!("Index out of bounds"))
    }

    pub fn get_key(&self, key: &str) -> anyhow::Result<&Reduced> {
        self.as_map()?
            .get(key)
            .ok_or_else(|| anyhow!("Key not contained"))
    }

    pub fn unreduce<Context, T>(&self, context: &Context) -> anyhow::Result<T>
    where
        T: Unreduce<Context>,
    {
        Unreduce::unreduce(self, context)
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let base64_pixels = BASE64.encode(bytes);
        Self::String(base64_pixels)
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let base64_bytes = self.as_string()?;
        let bytes = BASE64.decode(base64_bytes.as_bytes())?;
        Ok(bytes)
    }
}

pub struct NoContext {}

pub trait Reduce<Context> {
    fn reduce(&self, context: &mut Context) -> anyhow::Result<Reduced>;
}

pub trait Unreduce<Context>
where
    Self: Sized,
{
    fn unreduce(reduced: &Reduced, context: &Context) -> anyhow::Result<Self>;
}

impl<Context, T> Reduce<Context> for &T
where
    T: Reduce<Context>,
{
    fn reduce(&self, context: &mut Context) -> anyhow::Result<Reduced> {
        (*self).reduce(context)
    }
}

macro_rules! impl_reduce_int {
    ($t: ty) => {
        impl<Context> Reduce<Context> for $t {
            fn reduce(&self, _context: &mut Context) -> anyhow::Result<Reduced> {
                let i: i64 = (*self).try_into()?;
                Ok(Reduced::Int(i))
            }
        }

        impl<Context> Unreduce<Context> for $t {
            fn unreduce(reduced: &Reduced, _context: &Context) -> anyhow::Result<Self> {
                Ok(reduced.as_i64()?.try_into()?)
            }
        }
    };
}

impl_reduce_int!(u8);
impl_reduce_int!(u16);
impl_reduce_int!(u32);
impl_reduce_int!(u64);
impl_reduce_int!(usize);
impl_reduce_int!(i8);
impl_reduce_int!(i16);
impl_reduce_int!(i32);
impl_reduce_int!(i64);
impl_reduce_int!(isize);

impl<Context> Reduce<Context> for f64 {
    fn reduce(&self, _context: &mut Context) -> anyhow::Result<Reduced> {
        Ok(Reduced::Float(*self))
    }
}

impl<Context> Unreduce<Context> for f64 {
    fn unreduce(reduced: &Reduced, _context: &Context) -> anyhow::Result<Self> {
        reduced.as_f64()
    }
}

impl<Context> Reduce<Context> for bool {
    fn reduce(&self, _context: &mut Context) -> anyhow::Result<Reduced> {
        Ok(Reduced::Bool(*self))
    }
}

impl<Context> Unreduce<Context> for bool {
    fn unreduce(reduced: &Reduced, _context: &Context) -> anyhow::Result<Self> {
        reduced.as_bool()
    }
}

impl<Context> Reduce<Context> for String {
    fn reduce(&self, _context: &mut Context) -> anyhow::Result<Reduced> {
        Ok(Reduced::String(self.clone()))
    }
}

impl<Context> Unreduce<Context> for String {
    fn unreduce(reduced: &Reduced, _context: &Context) -> anyhow::Result<Self> {
        reduced.as_string().cloned()
    }
}

/// Reduce slice as list
impl<Context, T> Reduce<Context> for [T]
where
    T: Reduce<Context>,
{
    fn reduce(&self, context: &mut Context) -> anyhow::Result<Reduced> {
        let mut reduced_items = Vec::new();
        for item in self {
            reduced_items.push(item.reduce(context)?);
        }
        Ok(Reduced::Vec(reduced_items))
    }
}

/// Reduce Vec as list
impl<Context, T> Reduce<Context> for Vec<T>
where
    T: Reduce<Context>,
{
    fn reduce(&self, context: &mut Context) -> anyhow::Result<Reduced> {
        self.as_slice().reduce(context)
    }
}

impl<Context, T> Unreduce<Context> for Vec<T>
where
    T: Unreduce<Context>,
{
    fn unreduce(reduced: &Reduced, context: &Context) -> anyhow::Result<Self> {
        let mut items = Vec::new();
        for reduced_item in reduced.as_slice()? {
            items.push(reduced_item.unreduce(context)?);
        }
        Ok(items)
    }
}

/// Reduce fixed size array as list
impl<Context, T, const N: usize> Reduce<Context> for [T; N]
where
    T: Reduce<Context>,
{
    fn reduce(&self, context: &mut Context) -> anyhow::Result<Reduced> {
        self.as_slice().reduce(context)
    }
}

impl<Context, T, const N: usize> Unreduce<Context> for [T; N]
where
    T: Unreduce<Context>,
{
    fn unreduce(reduced: &Reduced, context: &Context) -> anyhow::Result<Self> {
        let reduced_items = reduced.as_slice_with_len(N)?;

        let mut items: SmallVec<[T; N]> = SmallVec::new();
        for reduced_item in reduced_items {
            items.push(reduced_item.unreduce(context)?);
        }

        Ok(items.into_inner().unwrap_or_else(|_| unreachable!()))
    }
}

/// Reduce 2-tuple as list
impl<Context, T0, T1> Reduce<Context> for (T0, T1)
where
    T0: Reduce<Context>,
    T1: Reduce<Context>,
{
    fn reduce(&self, context: &mut Context) -> anyhow::Result<Reduced> {
        let reduced_0 = self.0.reduce(context)?;
        let reduced_1 = self.1.reduce(context)?;
        let pair = Reduced::vec_from([reduced_0, reduced_1]);
        Ok(pair)
    }
}

impl<Context, T0, T1> Unreduce<Context> for (T0, T1)
where
    T0: Unreduce<Context>,
    T1: Unreduce<Context>,
{
    fn unreduce(reduced: &Reduced, context: &Context) -> anyhow::Result<Self> {
        let [reduced_0, reduced_1] = reduced.as_array::<2>()?;
        Ok((reduced_0.unreduce(context)?, reduced_1.unreduce(context)?))
    }
}

/// Reduce HashMap as list of key-value pairs
impl<Context, Lhs, Rhs> Reduce<Context> for HashMap<Lhs, Rhs>
where
    Lhs: Reduce<Context>,
    Rhs: Reduce<Context>,
{
    fn reduce(&self, context: &mut Context) -> anyhow::Result<Reduced> {
        let vec: Vec<_> = self.iter().collect();
        vec.reduce(context)
    }
}

impl<Context, Lhs, Rhs> Unreduce<Context> for HashMap<Lhs, Rhs>
where
    Lhs: Unreduce<Context> + Hash + Eq,
    Rhs: Unreduce<Context>,
{
    fn unreduce(reduced: &Reduced, context: &Context) -> anyhow::Result<Self> {
        let vec: Vec<(Lhs, Rhs)> = reduced.unreduce(context)?;
        Ok(vec.into_iter().collect())
    }
}
