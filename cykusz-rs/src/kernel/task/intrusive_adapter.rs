use crate::kernel::task::Task;
use crate::kernel::utils::arc_type::ArcType;
use alloc::sync::Arc;
use core::marker::PhantomData;
use intrusive_collections::PointerOps;

pub struct TaskPointerOps<Pointer>(PhantomData<Pointer>);

impl<Pointer> TaskPointerOps<Pointer> {
    /// Constructs an instance of `DefaultPointerOps`.
    #[inline]
    pub const fn new() -> TaskPointerOps<Pointer> {
        TaskPointerOps(PhantomData)
    }
}

impl<Pointer> Clone for TaskPointerOps<Pointer> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<Pointer> Copy for TaskPointerOps<Pointer> {}

impl<Pointer> Default for TaskPointerOps<Pointer> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl PointerOps for TaskPointerOps<ArcType<Task>> {
    type Value = Task;
    type Pointer = ArcType<Task>;

    #[inline]
    unsafe fn from_raw(&self, raw: *const Task) -> ArcType<Task> {
        ArcType::<Task>::new(Arc::from_raw(raw))
    }

    #[inline]
    fn into_raw(&self, ptr: ArcType<Task>) -> *const Task {
        ArcType::into_raw(ptr)
    }
}

macro_rules! task_intrusive_adapter {
    (@impl
        $(#[$attr:meta])* $vis:vis $name:ident ($($args:tt),*)
        = $pointer:ty: $value:path { $field:ident: $link:ty } $($where_:tt)*
    ) => {
        #[allow(explicit_outlives_requirements)]
        $(#[$attr])*
        $vis struct $name<$($args),*> $($where_)* {
            link_ops: <$link as ::intrusive_collections::DefaultLinkOps>::Ops,
            pointer_ops: $crate::kernel::task::intrusive_adapter::TaskPointerOps<$pointer>,
        }
        unsafe impl<$($args),*> Send for $name<$($args),*> $($where_)* {}
        unsafe impl<$($args),*> Sync for $name<$($args),*> $($where_)* {}
        impl<$($args),*> Copy for $name<$($args),*> $($where_)* {}
        impl<$($args),*> Clone for $name<$($args),*> $($where_)* {
            #[inline]
            fn clone(&self) -> Self {
                *self
            }
        }
        impl<$($args),*> Default for $name<$($args),*> $($where_)* {
            #[inline]
            fn default() -> Self {
                Self::NEW
            }
        }
        #[allow(dead_code)]
        impl<$($args),*> $name<$($args),*> $($where_)* {
            pub const NEW: Self = $name {
                link_ops: <$link as ::intrusive_collections::DefaultLinkOps>::NEW,
                pointer_ops: $crate::kernel::task::intrusive_adapter::TaskPointerOps::<$pointer>::new(),
            };
            #[inline]
            pub fn new() -> Self {
                Self::NEW
            }
        }
        #[allow(dead_code, unsafe_code)]
        unsafe impl<$($args),*> intrusive_collections::Adapter for $name<$($args),*> $($where_)* {
            type LinkOps = <$link as ::intrusive_collections::DefaultLinkOps>::Ops;
            type PointerOps = $crate::kernel::task::intrusive_adapter::TaskPointerOps<$pointer>;

            #[inline]
            unsafe fn get_value(&self, link: <Self::LinkOps as ::intrusive_collections::LinkOps>::LinkPtr) -> *const <Self::PointerOps as intrusive_collections::PointerOps>::Value {
                container_of!(link.as_ptr(), $value, $field)
            }
            #[inline]
            unsafe fn get_link(&self, value: *const <Self::PointerOps as intrusive_collections::PointerOps>::Value) -> <Self::LinkOps as intrusive_collections::LinkOps>::LinkPtr {
                // We need to do this instead of just accessing the field directly
                // to strictly follow the stack borrow rules.
                let ptr = (value as *const u8).add(offset_of!($value, $field));
                core::ptr::NonNull::new_unchecked(ptr as *mut _)
            }
            #[inline]
            fn link_ops(&self) -> &Self::LinkOps {
                &self.link_ops
            }
            #[inline]
            fn link_ops_mut(&mut self) -> &mut Self::LinkOps {
                &mut self.link_ops
            }
            #[inline]
            fn pointer_ops(&self) -> &Self::PointerOps {
                &self.pointer_ops
            }
        }
    };
    (@find_generic
        $(#[$attr:meta])* $vis:vis $name:ident ($($prev:tt)*) > $($rest:tt)*
    ) => {
        task_intrusive_adapter!(@impl
            $(#[$attr])* $vis $name ($($prev)*) $($rest)*
        );
    };
    (@find_generic
        $(#[$attr:meta])* $vis:vis $name:ident ($($prev:tt)*) $cur:tt $($rest:tt)*
    ) => {
        task_intrusive_adapter!(@find_generic
            $(#[$attr])* $vis $name ($($prev)* $cur) $($rest)*
        );
    };
    (@find_if_generic
        $(#[$attr:meta])* $vis:vis $name:ident < $($rest:tt)*
    ) => {
        task_intrusive_adapter!(@find_generic
            $(#[$attr])* $vis $name () $($rest)*
        );
    };
    (@find_if_generic
        $(#[$attr:meta])* $vis:vis $name:ident $($rest:tt)*
    ) => {
        task_intrusive_adapter!(@impl
            $(#[$attr])* $vis $name () $($rest)*
        );
    };
    ($(#[$attr:meta])* $vis:vis $name:ident $($rest:tt)*) => {
        task_intrusive_adapter!(@find_if_generic
            $(#[$attr])* $vis $name $($rest)*
        );
    };
}
