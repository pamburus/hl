use std::cmp::min;

pub trait Push<T> {
    fn push(&mut self, value: T);
    fn extend_from_slice(&mut self, values: &[T]);
}

impl<T> Push<T> for Vec<T>
where
    T: Clone,
{
    fn push(&mut self, value: T) {
        Vec::push(self, value)
    }

    fn extend_from_slice(&mut self, values: &[T]) {
        Vec::extend_from_slice(self, values)
    }
}

pub struct Counter {
    value: usize,
}

impl Counter {
    pub fn new() -> Self {
        Self { value: 0 }
    }

    pub fn result(&self) -> usize {
        self.value
    }
}

impl<T> Push<T> for Counter {
    fn push(&mut self, _: T) {
        self.value += 1
    }

    fn extend_from_slice(&mut self, values: &[T]) {
        self.value += values.len()
    }
}

pub struct Aligner<'a, T, B>
where
    T: Clone,
    B: Push<T>,
{
    buf: &'a mut B,
    width: usize,
    cur: usize,
    filler: T,
}

impl<'a, T, B> Aligner<'a, T, B>
where
    T: Clone,
    B: Push<T>,
{
    fn new(buf: &'a mut B, width: usize, filler: T) -> Self {
        Self {
            buf,
            width,
            cur: 0,
            filler,
        }
    }

    pub fn push(&mut self, value: T) {
        if self.cur < self.width {
            self.buf.push(value);
            self.cur += 1;
        }
    }

    pub fn extend_from_slice(&mut self, s: &[T]) {
        if self.cur < self.width {
            let n = min(self.width - self.cur, s.len());
            self.buf.extend_from_slice(&s[..n]);
            self.cur += n;
        }
    }

    pub fn centered(&mut self, s: &[T]) {
        let b = (self.width - s.len()) / 2;
        for _ in self.cur..b {
            self.buf.push(self.filler.clone());
        }
        self.extend_from_slice(s)
    }
}

impl<'a, T, B> Push<T> for Aligner<'a, T, B>
where
    T: Clone,
    B: Push<T>,
{
    fn push(&mut self, value: T) {
        Aligner::push(self, value)
    }

    fn extend_from_slice(&mut self, values: &[T]) {
        Aligner::extend_from_slice(self, values)
    }
}

impl<'a, T, B> Drop for Aligner<'a, T, B>
where
    T: Clone,
    B: Push<T>,
{
    fn drop(&mut self) {
        for _ in self.cur..self.width {
            self.buf.push(self.filler.clone());
        }
    }
}

pub fn aligned<'a, T, B, F>(buf: &'a mut B, width: usize, filler: T, f: F)
where
    T: Clone,
    B: Push<T>,
    F: FnOnce(Aligner<'a, T, B>),
{
    f(Aligner::new(buf, width, filler));
}
