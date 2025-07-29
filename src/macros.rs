static mut ERROR_HOOK: Option<fn(&str)> = None;

pub fn register_error_hook(callback: fn(&str)) {
    unsafe { ERROR_HOOK = Some(callback); }
}

#[macro_export]
macro_rules! define_errors {
    (
        $(
            $(#[$meta:meta])* $vis:vis enum $name:ident {
                $( #[kind($kind:ident $(, $($tag:ident = $val:expr),* )?)]
                   $variant:ident $( { $($field:ident : $ftype:ty),* $(,)? } )?, )*
            }
        )*
    ) => {
        $(
            $(#[$meta])* #[derive(Debug)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize))]
            $vis enum $name {
                $( $variant $( { $($field : $ftype),* } )?, )*
            }

            impl $name {
                $(
                    paste::paste! {
                        pub fn [<$variant:lower>]($($($field : $ftype),*)?) -> Self {
                            let instance = Self::$variant $( { $($field),* } )?;
                            #[allow(unsafe_code)]
                            unsafe {
                                if let Some(hook) = $crate::macros::ERROR_HOOK {
                                    let _ = hook(instance.caption());
                                }
                            }
                            instance
                        }
                    }
                )*

                pub fn caption(&self) -> &'static str {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_caption $kind $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn is_retryable(&self) -> bool {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag retryable false $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn is_fatal(&self) -> bool {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag fatal true $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn status_code(&self) -> u16 {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag status 500 $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn exit_code(&self) -> i32 {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag exit 1 $(, $($tag = $val),* )?)
                        } ),*
                    }
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        $( Self::$variant $( { $($field),* } )? => {
                            write!(f, "{}: ", self.caption())?;
                            write!(f, stringify!($variant))?;
                            $( $( write!(f, " | {} = {:?}", stringify!($field), $field)?; )* )?
                            Ok(())
                        } ),*
                    }
                }
            }

            impl std::error::Error for $name {
                fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                    match self {
                        $( Self::$variant $( { $($field),* } )? => {
                            define_errors!(@find_source $($field),*)
                        } ),*
                    }
                }
            }
        )*
    };

    (@find_source $($field:ident),*) => {
        {
            let mut result = None;
            $(
                if result.is_none() {
                    let val = $field;
                    if let Some(err) = (val as &dyn std::any::Any).downcast_ref::<&(dyn std::error::Error + 'static)>() {
                        result = Some(*err);
                    }
                }
            )*
            result
        }
    };

    (@get_caption $kind:ident $(, caption = $caption:expr $(, $($rest:tt)*)? )?) => {
        $crate::define_errors!(@unwrap_caption $kind $(, $caption)? )
    };

    (@unwrap_caption Config, $caption:expr) => { $caption };
    (@unwrap_caption Filesystem, $caption:expr) => { $caption };
    (@unwrap_caption $kind:ident) => { stringify!($kind) };

    (@get_tag $target:ident, $default:expr $(, $($tag:ident = $val:expr),* )?) => {
        {
            let mut found = $default;
            $( $( if stringify!($tag) == stringify!($target) { found = $val; })* )?
            found
        }
    };
}
