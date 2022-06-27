use crate::{
    bitmap::Bitmap,
    math::{point::Point, rgba8::Rgba8},
    reduce::{Reduce, Reduced, Unreduce},
};

impl<Context, T> Reduce<Context> for Point<T>
where
    T: Reduce<Context>,
{
    fn reduce(&self, context: &mut Context) -> anyhow::Result<Reduced> {
        [&self.x, &self.y].reduce(context)
    }
}

impl<Context, T> Unreduce<Context> for Point<T>
where
    T: Unreduce<Context>,
{
    fn unreduce(reduced: &Reduced, context: &Context) -> anyhow::Result<Self> {
        let pair: [T; 2] = reduced.unreduce(context)?;
        Ok(pair.into())
    }
}

impl<Context> Reduce<Context> for Rgba8 {
    fn reduce(&self, context: &mut Context) -> anyhow::Result<Reduced> {
        self.to_array().reduce(context)
    }
}

impl<Context> Unreduce<Context> for Rgba8 {
    fn unreduce(reduced: &Reduced, context: &Context) -> anyhow::Result<Self> {
        let array: [u8; 4] = reduced.unreduce(context)?;
        Ok(array.into())
    }
}

impl<Context> Reduce<Context> for Bitmap {
    fn reduce(&self, context: &mut Context) -> anyhow::Result<Reduced> {
        let u8_pixels = bytemuck::cast_slice(self.linear_slice());
        let reduced_pixels = Reduced::from_bytes(u8_pixels);

        Ok(Reduced::object_from([
            ("width", self.width().reduce(context)?),
            ("height", self.height().reduce(context)?),
            ("pixels", reduced_pixels),
        ]))
    }
}

impl<Context> Unreduce<Context> for Bitmap {
    fn unreduce(reduced: &Reduced, context: &Context) -> anyhow::Result<Self> {
        let u8_pixels = reduced.get_key("pixels")?.to_bytes()?;
        let pixels = Vec::from(bytemuck::cast_slice(&u8_pixels));

        Ok(Bitmap::from_linear(
            reduced.get_key("width")?.unreduce(context)?,
            reduced.get_key("height")?.unreduce(context)?,
            pixels,
        ))
    }
}
