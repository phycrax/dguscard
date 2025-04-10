use crate::response::ResponseData;

/// TODO
pub trait Dispatch<Key> {
    /// TODO
    async fn handle(&mut self, key: &Key, data: ResponseData);
}

#[macro_export]
/// Define a dispatch struct with a context and a handler for each endpoint.
macro_rules! define_dispatch {
    (name: $name:ident; key: $key_ty:ty; context: $context_ty:ty; endpoints: {$($key:tt => $handler:ident)+};) => {
        pub struct $name {
            pub context: $context_ty,
        }

        impl $name {
            pub fn new(context: $context_ty) -> Self {
                Self { context }
            }
        }

        impl Dispatch<$key_ty> for $name {
            async fn handle(&mut self, key: &$key_ty, data: dguscard::response::ResponseData<'_>) {
                match key {
                    $(
                        $key => $handler(&mut self.context, data).await,
                    )+
                    _ => defmt::error!("Unknown response"),
                }
            }
        }
    }
}