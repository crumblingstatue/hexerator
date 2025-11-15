pub trait AnyhowConv<T, E>
where
    anyhow::Error: From<E>,
{
    fn how(self) -> anyhow::Result<T>;
}

impl<T, E> AnyhowConv<T, E> for Result<T, E>
where
    anyhow::Error: From<E>,
{
    fn how(self) -> anyhow::Result<T> {
        self.map_err(anyhow::Error::from)
    }
}
