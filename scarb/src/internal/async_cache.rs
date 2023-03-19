use std::collections::HashMap;
use std::hash::Hash;

use anyhow::Result;
use futures::future::{LocalBoxFuture, Shared};
use futures::prelude::*;
use tokio::sync::RwLock;

use crate::internal::cloneable_error::CloneableResult;

pub type TryLoadFuture<'a, V> = LocalBoxFuture<'a, CloneableResult<V>>;

pub struct AsyncCache<'a, K, V, C> {
    futures: RwLock<HashMap<K, Shared<TryLoadFuture<'a, V>>>>,
    load_fn: Box<dyn Fn(K, C) -> TryLoadFuture<'a, V> + 'a>,
    context: C,
}

impl<'a, K, V, C> AsyncCache<'a, K, V, C>
where
    K: Clone + Eq + Hash,
    V: Clone + 'a,
    C: Clone,
{
    pub fn new(context: C, load_fn: impl Fn(K, C) -> TryLoadFuture<'a, V> + 'a) -> Self {
        Self {
            futures: RwLock::new(HashMap::with_capacity(128)),
            load_fn: Box::new(load_fn),
            context,
        }
    }

    pub async fn load(&self, key: K) -> Result<V> {
        let cached_future = self.futures.read().await.get(&key).cloned();
        if let Some(future) = cached_future {
            Ok(future.await?)
        } else {
            let future = {
                let mut futures = self.futures.write().await;
                let future = (self.load_fn)(key.clone(), self.context.clone())
                    .boxed_local()
                    .shared();
                futures.insert(key.clone(), future.clone());
                future
            };

            Ok(future.await?)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU8, Ordering};

    use anyhow::anyhow;
    use futures::prelude::*;

    use super::AsyncCache;

    #[tokio::test]
    async fn load() {
        let cache = AsyncCache::new((), |key: usize, _ctx: ()| {
            static COUNTER: AtomicU8 = AtomicU8::new(0);
            async move { Ok((key, COUNTER.fetch_add(1, Ordering::Relaxed))) }.boxed_local()
        });

        assert_eq!(cache.load(1).await.unwrap(), (1, 0));
        assert_eq!(cache.load(1).await.unwrap(), (1, 0));
        assert_eq!(cache.load(2).await.unwrap(), (2, 1));
        assert_eq!(cache.load(2).await.unwrap(), (2, 1));
    }

    #[tokio::test]
    async fn load_err() {
        let cache = AsyncCache::new((), |key: usize, _ctx: ()| {
            static COUNTER: AtomicU8 = AtomicU8::new(0);
            async move {
                Err(anyhow!("{key} {}", COUNTER.fetch_add(1, Ordering::Relaxed)))?;
                Ok(())
            }
            .boxed_local()
        });

        assert_eq!(cache.load(1).await.unwrap_err().to_string(), "1 0");
        assert_eq!(cache.load(1).await.unwrap_err().to_string(), "1 0");
        assert_eq!(cache.load(2).await.unwrap_err().to_string(), "2 1");
        assert_eq!(cache.load(2).await.unwrap_err().to_string(), "2 1");
    }
}
