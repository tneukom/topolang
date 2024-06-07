# Copy or Reference

https://news.ycombinator.com/item?id=20798033

"In the below example Rust switches to using a pointer when you might think 
it's doing pass by copy/move. This works because in Rust the moved value can 
never be referenced after calling the function, so the compiler can just pass a 
pointer and clean up the value after the function has returned."


https://www.forrestthewoods.com/blog/should-small-rust-structs-be-passed-by-copy-or-by-borrow/

https://www.reddit.com/r/rust/comments/3g30fw/how_efficient_is_moving_a_struct_into_a_function/

Conclusion: Use copy for all primitive int/float math structs: Point, Rect, Interval, 
Matrix2, Matrix3, AffineMap, ...

# Links
https://www.reddit.com/r/rust/comments/13fvpey/object_oriented_programming_in_rustyuck_and_yet/

# Impl ops for move and reference
Forward all ops to &lhs op &rhs
- Point<T> + Point<T> to &Point<T> + &Point<T>

Forward 
- &Point<T> + Point<T> to Point<&T> + Point<T>
- &Point<T> + &Point<T> to Point<&T> + Point<&T>
- Point<T> + &Point<T> to Point<T> + Point<&T>
```rust
impl<Lhs, Rhs> Add<Point<Rhs>> for Point<Lhs> where
    Lhs: Add<Rhs>
{
    type Output = Point<<Lhs as Add<Rhs>>::Output>;

    fn add(self, rhs: Point<Rhs>) -> Self::Output {
        Self::Output::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<T> Add<&Point<T>> for Point<T> where
        for<'a> Point<T>: Add<Point<&'a T>, Output=Point<T>>,
{
    type Output = Point<T>;

    fn add(self, rhs: &Point<T>) -> Self::Output {
        self + Point::new(&rhs.x, &rhs.y)
    }
}
```

Looks like the rust compiler can properly inline add(Point<&T>, Point<&T>) into
add(&Point<T>, &Point<T>), see https://godbolt.org/z/ceTPbMKh9.

# Rust in place collect
https://www.reddit.com/r/rust/comments/16hx79e/when_does_vecinto_itermapcollect_reallocate_and/

https://doc.rust-lang.org/nightly/src/alloc/vec/in_place_collect.rs.html