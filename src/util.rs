pub(crate) struct Cleanup<F>
where
    F: FnOnce(),
{
    f: Option<F>,
}

impl<F> Cleanup<F>
where
    F: FnOnce(),
{
    pub(crate) fn new(f: F) -> Self {
        Self { f: Some(f) }
    }

    pub(crate) fn cancel(&mut self) {
        self.f = None;
    }
}

impl<F> Drop for Cleanup<F>
where
    F: FnOnce(),
{
    fn drop(&mut self) {
        if let Some(f) = self.f.take() {
            f()
        };
    }
}
